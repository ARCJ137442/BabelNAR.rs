use std::{
    thread::{self, sleep},
    time::Duration,
};
extern crate ws;

fn main() {
    loop {
        let _ = ws::connect("ws://127.0.0.1:8765", |sender| {
            // 生成一个不断发送消息的线程
            thread::spawn(move || loop {
                let _ = sender.send("NSE A.".to_string());
                let _ = sender.send("NSE B.".to_string());
                let _ = sender.send("NSE A?".to_string());
            });

            // handle received message
            move |msg| {
                println!("Got message: {}", msg);
                // out.close(CloseCode::Normal)
                Ok(())
            }
        });
        sleep(Duration::from_secs(1));
    }
}

/// 压力测试
/// * 🔗GitHub issue：<https://github.com/housleyjk/ws-rs/issues/346>
#[test]
fn main_server() {
    // A client that sends tons of messages to the server
    thread::spawn(move || {
        let _ = ws::connect("ws://127.0.0.1:3012", |sender| {
            let mut num_send = 0_usize;
            // Generate a thread that constantly sends messages for testing
            thread::spawn(move || loop {
                num_send += 1;
                // The content is just for example, the actual situation has more variety
                let _ = sender.send(format!("overwhelming message #{num_send}!"));
            });

            // Handle nothing
            move |_| Ok(())
        });
    });

    // A server that echoes messages back to the client
    ws::Builder::new()
        .with_settings(ws::Settings {
            max_connections: 0x40,
            // * ↓Change this setting to `usize::MAX` actually can't be allowed: It might run out of memory
            queue_size: 0x300,
            // ! ↓Even enabled it, it still can't stop the blocking
            panic_on_queue: true,
            ..Default::default()
        })
        .build(|sender: ws::Sender| {
            // handle received message
            move |msg| {
                println!("Got message: {}", msg);
                println!("from {sender:?}");
                // ! It will block on ↓this line when the `SyncSender` is full
                let _ = sender.send(msg);
                // * ↑If uncomment this line of code, the server will not be blocked
                Ok(())
            }
        })
        .unwrap()
        .listen("127.0.0.1:3012")
        .unwrap();
}
