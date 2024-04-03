//! BabelNAR CLI的Websocket交互逻辑
//! * 🎯为BabelNAR CLI实现Websocket IO
//! * 🎯实现专有的Websocket服务端逻辑

use crate::{LaunchConfig, RuntimeManager};
use anyhow::Result;
use babel_nar::{
    cli_support::io::{
        navm_output_cache::{ArcMutex, OutputCache},
        websocket::{spawn_server, to_address},
    },
    eprintln_cli, println_cli,
};
use navm::vm::VmRuntime;
use std::{sync::Arc, thread::JoinHandle};
use ws::{Factory, Handler, Sender};

/// 工具宏：尝试执行，如果失败则上抛错误
/// * 🎯在「无法使用[`anyhow::Result`]上抛错误」的情况下适用
macro_rules! try_or_return_err {
    ($value:expr; $e_id:ident => $($error_msg:tt)*) => {
        match $value {
            Ok(value) => value,
            Err($e_id) => {
                // 生成并输出错误信息
                let error_msg = format!($($error_msg)*);
                println_cli!([Error] "{error_msg}");
                // 转换错误 | 使用Websocket的「内部错误」以指示是CLI的错误
                let error = ws::Error::new(ws::ErrorKind::Internal, error_msg);
                return Err(error);
            }
        }
    };
}

/// 入口代码
/// * 🎯生成一个Websocket服务端线程
/// * 🚩不管参数`config`中的地址：可能没有
pub fn spawn_ws_server<R>(
    manager: &RuntimeManager<R>,
    host: &str,
    port: u16,
) -> JoinHandle<Result<()>>
where
    R: VmRuntime + Send + Sync,
{
    // 合并地址
    let address = to_address(host, port);

    // 获取服务端「处理者工厂」
    // * 🚩拷贝[`Arc`]
    let factory = WSServer {
        runtime: manager.runtime.clone(),
        output_cache: manager.output_cache.clone(),
        config: manager.config.clone(),
    };

    // 根据专有服务端逻辑，生成子线程并返回
    let server = spawn_server(address.clone(), factory);
    println_cli!([Info] "Websocket服务器已在 {:?} 启动", address);
    server
}

/// 一个Websocket连接
/// * 🎯处理单个Websocket连接
#[derive(Debug)]
pub struct Connection<R>
where
    R: VmRuntime + Send + Sync,
{
    /// 所涉及的运行时
    pub(crate) runtime: ArcMutex<R>,

    /// 所涉及的运行时配置
    pub(crate) config: Arc<LaunchConfig>,

    /// 所涉及的运行时
    pub(crate) output_cache: ArcMutex<OutputCache>,
    // /// 连接（服务端这方的）发送者
    // /// * 🚩【2024-04-03 19:44:58】现在不再需要
    // pub(crate) sender: Sender,
    /// 连接id
    pub(crate) id: u32,
}

impl<R> Handler for Connection<R>
where
    R: VmRuntime + Send + Sync + 'static,
{
    fn on_shutdown(&mut self) {
        println_cli!([Info] "Websocket连接已关停")
    }

    fn on_open(&mut self, shake: ws::Handshake) -> ws::Result<()> {
        if let Some(addr) = shake.remote_addr()? {
            println_cli!([Info] "Websocket连接已打开：{addr}")
        }
        Ok(())
    }

    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        println_cli!([Debug] "Websocket收到消息：{msg}");
        // 获取所需的参数信息 | 在此时独占锁
        let runtime = &mut *try_or_return_err!(self.runtime.lock(); poison => "在Websocket连接中获取运行时失败：{poison}");
        let config = &self.config;
        let output_cache = &mut *try_or_return_err!(self.output_cache.lock(); err => "在Websocket连接中获取输出缓存失败：{err}");

        // 输入信息，并监控缓存的新输出
        if let Err(err) =
            RuntimeManager::input_line_to_vm(runtime, &msg.to_string(), config, output_cache)
        {
            eprintln_cli!([Error] "在Websocket连接中输入「{msg}」时发生错误：{err}")
        }

        // ! 🚩此处无法回传输出：输出捕捉在缓存中处理的地方
        // if new_len_cache > old_len_cache {
        //     let mut output;
        //     let mut json_text;
        //     // 逐个获取
        //     for i in (old_len_cache - 1)..new_len_cache {
        //         output = &output_cache.borrow_inner()[i];
        //         json_text = output.to_json_string();
        //         // 回传，若出错仅输出错误
        //         if let Err(e) = self.sender.send(json_text.clone()) {
        //             eprintln_cli!([Error] "尝试回传消息「{json_text}」时发生错误：{e}");
        //         }
        //     }
        // }

        Ok(())
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        println_cli!([Info] "Websocket连接关闭（退出码：{code:?}；原因：「{reason}」）")
    }

    fn on_error(&mut self, err: ws::Error) {
        // Ignore connection reset errors by default, but allow library clients to see them by
        // overriding this method if they want
        if let ws::ErrorKind::Io(ref err) = err.kind {
            if let Some(104) = err.raw_os_error() {
                return;
            }
        }

        println_cli!([Error] "连接发生错误：{err:?}");
    }

    fn on_timeout(&mut self, event: ws::util::Token) -> ws::Result<()> {
        println_cli!([Warn] "连接超时：{:?}", event);
        Ok(())
    }

    fn on_new_timeout(&mut self, _: ws::util::Token, _: ws::util::Timeout) -> ws::Result<()> {
        // default implementation discards the timeout handle
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WSServer<R>
where
    R: VmRuntime,
{
    /// 所涉及的虚拟机运行时
    pub(crate) runtime: ArcMutex<R>,

    /// 所涉及的虚拟机配置
    pub(crate) config: Arc<LaunchConfig>,

    /// 所涉及的输出缓存
    pub(crate) output_cache: ArcMutex<OutputCache>,
}

impl<R> Factory for WSServer<R>
where
    R: VmRuntime + Send + Sync + 'static,
{
    type Handler = Connection<R>;

    fn connection_made(&mut self, sender: Sender) -> Connection<R> {
        println_cli!([Info] "Websocket连接已建立");
        let id = sender.connection_id();
        // 尝试添加「发送者」
        match self.output_cache.lock() {
            Ok(mut output_cache) => {
                let output_cache = &mut *output_cache;
                // 添加「发送者」
                output_cache.websocket_senders.push(sender);
            }
            Err(err) => {
                // 输出错误
                println_cli!([Error] "Websocket输出侦听器添加失败：{err}");
            }
        }
        // 返回连接
        Connection {
            runtime: self.runtime.clone(),
            config: self.config.clone(),
            output_cache: self.output_cache.clone(),
            id,
        }
    }

    fn on_shutdown(&mut self) {
        println_cli!([Info] "Websocket服务器已关停")
    }

    fn connection_lost(&mut self, handler: Self::Handler) {
        eprintln_cli!([Error] "与id为 {} 的客户端断开连接！", handler.id);
    }
}
