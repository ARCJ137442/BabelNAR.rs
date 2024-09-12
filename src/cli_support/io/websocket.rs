//! 基于[`ws`]为CLI提供Websocket IO支持
//! * ✨简单的「地址生成」「服务端启动」等逻辑
//! * ⚠️不涉及具体业务代码
//! * 📝【2024-04-03 16:01:37】可以启动IPv6服务端，但尚且没有测试方法
//!   * 📌语法：`[主机地址]:连接端口`
//!   * 📄示例：`[::]:3012`
//!   * 🔗参考：<https://github.com/housleyjk/ws-rs/issues/341>

use anyhow::Result;
use std::{
    fmt,
    net::ToSocketAddrs,
    thread::{self, JoinHandle},
};
use ws::{Factory, Handler, Sender, WebSocket};

/// 从「主机地址」与「连接端口」格式化到「完整地址」
/// * ✨兼容IPv4 和 IPv6
/// * 📌端口号为十六位无符号整数（0~65535，含两端）
pub fn to_address(host: &str, port: u16) -> String {
    match is_ipv6_host(host) {
        // IPv6
        true => format!("[{host}]:{port}"),
        // IPv4
        false => format!("{host}:{port}"),
    }
}

/// 判断**主机地址**是否为IPv6地址
/// * 🚩【2024-04-03 17:20:59】目前判断标准：地址中是否包含冒号
///   * 📄`::1`
///   * 📄`fe80::abcd:fade:dad1`
pub fn is_ipv6_host(host: &str) -> bool {
    host.contains(':')
}

/// 生成一个Websocket监听线程
/// * 🎯简单生成一个Websocket监听线程
/// * ⚠️线程在生成后立即开始运行
#[inline]
pub fn spawn_on<A, F, H>(addr: A, message_listener: F) -> JoinHandle<Result<()>>
where
    A: ToSocketAddrs + fmt::Debug + Send + Sync + 'static,
    F: FnMut(Sender) -> H + Send + Sync + 'static,
    H: Handler,
{
    spawn_server(addr, message_listener)
}

/// 生成一个Websocket监听线程，使用特定的服务端
/// * 🎯使用自定义的Websocket「工厂」[`Factory`]生成服务端与连接处理者（侦听器）
/// * 📄地址格式：`127.0.0.1:8080`、`localhost:8080`
/// * ⚠️线程在生成后立即开始运行
/// * 📝与[`ws::listen`]不同的是：允许在[`factory`]处自定义各种连接、侦听逻辑
/// * 🔗参考：<https://docs.rs/ws/latest/ws/trait.Factory.html>
#[inline]
pub fn spawn_server<A, F, H>(address: A, factory: F) -> JoinHandle<Result<()>>
where
    A: ToSocketAddrs + fmt::Debug + Send + Sync + 'static,
    F: Factory<Handler = H> + Send + Sync + 'static,
    H: Handler,
{
    thread::spawn(move || {
        let server = WebSocket::new(factory)?;
        server.listen(address)?;
        Ok(())
    })
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use ws::util::{Timeout, Token};

    #[test]
    fn main() {
        let t = spawn_on("127.0.0.1:3012", |sender| {
            println!("Websocket启动成功");
            move |msg| {
                println!("Received: {}", msg);
                sender.send(msg)
            }
        });
        t.join().expect("Websocket失败！").expect("Websocket出错！");
    }

    /// 📄简单的回传连接处理者
    /// * 🎯处理单个Websocket连接
    #[derive(Debug)]
    struct EchoHandler {
        sender: Sender,
    }

    impl Handler for EchoHandler {
        fn on_shutdown(&mut self) {
            println!("Handler received WebSocket shutdown request.");
        }

        fn on_open(&mut self, shake: ws::Handshake) -> ws::Result<()> {
            if let Some(addr) = shake.remote_addr()? {
                println!("Connection with {} now open", addr);
            }
            Ok(())
        }

        fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
            println!("Received message {:?}", msg);
            self.sender.send(msg)?;
            Ok(())
        }

        fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
            println!("Connection closing due to ({:?}) {}", code, reason);
        }

        fn on_error(&mut self, err: ws::Error) {
            // Ignore connection reset errors by default, but allow library clients to see them by
            // overriding this method if they want
            if let ws::ErrorKind::Io(ref err) = err.kind {
                if let Some(104) = err.raw_os_error() {
                    return;
                }
            }

            eprintln!("{:?}", err);
        }

        fn on_request(&mut self, req: &ws::Request) -> ws::Result<ws::Response> {
            println!("Handler received request:\n{}", req);
            ws::Response::from_request(req)
        }

        fn on_response(&mut self, res: &ws::Response) -> ws::Result<()> {
            println!("Handler received response:\n{}", res);
            Ok(())
        }

        fn on_timeout(&mut self, event: Token) -> ws::Result<()> {
            println!("Handler received timeout token: {:?}", event);
            Ok(())
        }

        fn on_new_timeout(&mut self, _: Token, _: Timeout) -> ws::Result<()> {
            // default implementation discards the timeout handle
            Ok(())
        }

        fn on_frame(&mut self, frame: ws::Frame) -> ws::Result<Option<ws::Frame>> {
            println!("Handler received: {}", frame);
            // default implementation doesn't allow for reserved bits to be set
            if frame.has_rsv1() || frame.has_rsv2() || frame.has_rsv3() {
                Err(ws::Error::new(
                    ws::ErrorKind::Protocol,
                    "Encountered frame with reserved bits set.",
                ))
            } else {
                Ok(Some(frame))
            }
        }

        fn on_send_frame(&mut self, frame: ws::Frame) -> ws::Result<Option<ws::Frame>> {
            println!("Handler will send: {}", frame);
            // default implementation doesn't allow for reserved bits to be set
            if frame.has_rsv1() || frame.has_rsv2() || frame.has_rsv3() {
                Err(ws::Error::new(
                    ws::ErrorKind::Protocol,
                    "Encountered frame with reserved bits set.",
                ))
            } else {
                Ok(Some(frame))
            }
        }
    }

    struct EchoFactory;

    impl Factory for EchoFactory {
        type Handler = EchoHandler;

        fn connection_made(&mut self, sender: Sender) -> EchoHandler {
            dbg!(EchoHandler { sender })
        }

        fn on_shutdown(&mut self) {
            println!("Factory received WebSocket shutdown request.");
        }

        fn client_connected(&mut self, ws: Sender) -> Self::Handler {
            self.connection_made(ws)
        }

        fn server_connected(&mut self, ws: Sender) -> Self::Handler {
            self.connection_made(ws)
        }

        fn connection_lost(&mut self, handler: Self::Handler) {
            println!("Connection lost of {handler:?}");
        }
    }

    #[test]
    fn test_echo_localhost() -> Result<()> {
        let ws = WebSocket::new(EchoFactory)?;
        ws.listen("localhost:3012")?;
        Ok(())
    }

    #[test]
    fn test_echo_ipv6() -> Result<()> {
        let ws = WebSocket::new(EchoFactory)?;
        ws.listen("[::]:3012")?;
        Ok(())
    }
}
