//! BabelNAR CLI的Websocket交互逻辑
//! * 🎯为BabelNAR CLI实现Websocket IO
//! * 🎯实现专有的Websocket服务端逻辑

use crate::{LaunchConfigWebsocket, RuntimeConfig, RuntimeManager};
use anyhow::Result;
use babel_nar::{
    cli_support::{
        error_handling_boost::error_anyhow,
        io::{
            navm_output_cache::{ArcMutex, OutputCache},
            websocket::to_address,
        },
    },
    eprintln_cli, if_let_err_eprintln_cli, println_cli,
};
use navm::{output::Output, vm::VmRuntime};
use std::{
    sync::Arc,
    thread::{self, JoinHandle},
};
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

/// 通信用代码
/// * 🎯统一有关「通信消息格式」的内容
/// * 📌形式：JSON**对象数组**
///  * ⚠️【2024-04-08 19:08:15】即便一次只回传一条消息，也需包装上方括号`[{...}]`
#[inline]
pub fn format_output_message(output: &Output) -> String {
    // 包装成「对象数组」
    format!("[{}]", output.to_json_string())
}

/// 入口代码
/// * 🎯生成一个Websocket服务端线程
/// * ⚠️此处要求**manager.config.websocket**必须非空，否则会直接panic
/// * 🚩此处手动生成Websocket服务端并启动：提升其「待发消息缓冲区」容量到24576
///   * ❗【2024-04-09 01:20:57】问题缘起：服务端在「突然收到大量消息需要重发」时，可能会直接阻塞线程
///   * 📌【2024-04-09 01:21:37】现在通过配置「最大连接数」与「队列大小」以**暂时缓解**此问题
///   * 🔗参考：<https://docs.rs/ws/latest/ws/struct.Settings.html>
///   * 🔗GitHub issue：<https://github.com/housleyjk/ws-rs/issues/346>
pub fn spawn_ws_server<R>(manager: &mut RuntimeManager<R>) -> Result<JoinHandle<Result<()>>>
where
    R: VmRuntime + Send + Sync,
{
    // 提取并合并地址
    let LaunchConfigWebsocket { host, port } = manager
        .config
        .websocket
        .as_ref()
        .expect("尝试在无配置时启动Websocket服务器");
    let address = to_address(host, *port);

    // 获取服务端「处理者工厂」
    // * 🚩拷贝[`Arc`]
    let server = WSServer {
        runtime: manager.runtime.clone(),
        output_cache: manager.output_cache.clone(),
        config: manager.config.clone(),
    };

    // 生成定制版的Websocket服务端
    // * 🎯获取生成的[`WebSocket`]（服务端）对象，调用[`WebSocket::boardcaster`]方法快速广播
    // * ❌【2024-04-08 23:23:08】无法独立为单独的函数：此中NAVM运行时「R」的生命周期问题（难以参与推导）
    let (handle, sender) = {
        let factory = server;
        let address = address.clone();
        let ws_setting = ws::Settings {
            // * 📝使用`ws::Builder`结合`ws::Settings`生成配置
            // * ✅在配置中调节「队列大小」以扩宽「连续消息接收限制」
            // * 默认：100（最大连接）×5（最长队列）→500条后阻塞
            // * 🚩【2024-04-09 01:03:52】现在调整成「最多32个连接，每个连接最多768条消息」
            // * ⚠️仍然会在24576条消息后产生阻塞——但相比原先500条，情况少很多
            max_connections: 0x20,
            queue_size: 0x300,
            ..Default::default()
        };
        let server = ws::Builder::new()
            .with_settings(ws_setting)
            .build(factory)?;
        let sender = server.broadcaster();
        let handle = thread::spawn(move || {
            server.listen(address)?;
            // ! ❌此处不能缩并：必须转换为`anyhow::Error`
            Ok(())
        });
        (handle, sender)
    };
    println_cli!([Info] "Websocket服务器已在 {:?} 启动", address);

    // 向（服务端自身）「输出缓存」添加侦听器
    if_let_err_eprintln_cli! {
        // ! 此处需要可变的`manager`
        register_listener(&mut manager.output_cache, sender)
        => e => [Error] "无法为服务端注册侦听器：{e}"
    }

    // 返回线程句柄
    Ok(handle)
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
    pub(crate) config: Arc<RuntimeConfig>,

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
        // * 📝【2024-04-08 22:10:17】现在查明「Websocket线程阻塞」问题在Websocket「回传发送者」的`send`调用中
        if_let_err_eprintln_cli! {
            RuntimeManager::input_line_to_vm(
                runtime,
                &msg.to_string(),
                config,
                output_cache,
                &config.config_path
            )
            => err => [Error] "在Websocket连接中输入「{msg}」时发生错误：{err}"
        }

        Ok(())
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        println_cli!([Info] "Websocket连接关闭（退出码：{code:?}；原因：「{reason}」）");
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
    pub(crate) config: Arc<RuntimeConfig>,

    /// 所涉及的输出缓存
    pub(crate) output_cache: ArcMutex<OutputCache>,
}

/// 向所有「回传发送者」广播NAVM输出
/// * 🎯回传所侦听到的NAVM输出
pub(crate) fn broadcast_to_senders(
    // senders: &mut ArcMutex<ResendSenders>,
    broadcaster: &mut Sender,
    output: &Output,
) -> Result<()> {
    let output_str = format_output_message(output);

    // println_cli!([Debug] "🏗️正在向接收者回传消息：\n{output_str}");
    // * 通过一个`broadcaster`直接向所有连接广播消息
    if_let_err_eprintln_cli! {
        broadcaster.send(output_str.to_string())
        => e => [Error] "广播消息失败：{e}"
    };

    // println_cli!([Debug] "✅向接收者回传消息完成：\n{output_str}");

    Ok(())
}

/// 向「输出缓存」注册侦听器
/// * 🎯绑定侦听器到输出缓存中，以便在「侦听器有输出」时广播
/// * 🎯现在只有「输出缓存」会留存：因为`WebSocket.broadcaster`只在服务器启动后创建
pub(crate) fn register_listener(
    output_cache: &mut ArcMutex<OutputCache>,
    mut broadcaster: Sender,
) -> Result<()> {
    // 尝试解包「输出缓存」
    let output_cache = &mut *output_cache.lock().map_err(error_anyhow)?;
    output_cache.output_handlers.add_handler(move |output| {
        // 广播
        if_let_err_eprintln_cli! {
            broadcast_to_senders(&mut broadcaster, &output)
            => e => [Error] "Websocket回传广播到发送者时出现错误：{:?}", e
        }
        // 返回
        Some(output)
    });
    Ok(())
}

impl<R> Factory for WSServer<R>
where
    R: VmRuntime + Send + Sync + 'static,
{
    type Handler = Connection<R>;

    fn connection_made(&mut self, sender: Sender) -> Connection<R> {
        let id = sender.connection_id();
        println_cli!([Info] "Websocket连接已在id {id} 处建立");
        // 返回连接
        Connection {
            runtime: self.runtime.clone(),
            config: self.config.clone(),
            output_cache: self.output_cache.clone(),
            id,
        }
    }

    fn on_shutdown(&mut self) {
        // 打印消息
        println_cli!([Info] "Websocket服务器已关停")
    }

    fn connection_lost(&mut self, handler: Self::Handler) {
        eprintln_cli!([Error] "与id为 {} 的客户端断开连接！", handler.id);
    }
}

// TODO: ❓【2024-04-07 12:42:51】单元测试不好做：网络连接难以被模拟
