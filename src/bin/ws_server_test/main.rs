use std::{
    cell::RefCell,
    thread::{self, sleep},
    time::Duration,
};
extern crate ws;

fn main() {
    /// 单次训练过程
    fn train(sender: ws::Sender) -> impl Fn(ws::Message) -> Result<(), ws::Error> {
        // 尝试注册操作
        let _ = sender.send("REG left".to_string());
        let _ = sender.send("REG right".to_string());

        // 预先经验
        for _ in 0..5 {
            // 背景事件
            let _ = sender.send("NSE <a --> b>. :|:".to_string());
            // 自身操作
            let _ = sender.send("NSE <(*, {SELF}) --> ^left>. :|:".to_string());
            let _ = sender.send("NSE <(*, {SELF}) --> ^right>. :|:".to_string());
            // 一定间隔
            let _ = sender.send("CYC 10".to_string());
            // 自身状态
            let _ = sender.send("NSE <{SELF} --> [good]>. :|:".to_string());
        }
        // 再间隔一段时间，开始训练
        let _ = sender.send("CYC 100".to_string());

        let sender2 = sender.clone();
        // 生成一个不断发送消息的线程
        thread::spawn(move || loop {
            let _ = sender2.send("NSE <a --> b>. :|:".to_string());
            let _ = sender2.send("CYC 10".to_string());
            let _ = sender2.send("NSE <{SELF} --> [good]>! :|:".to_string());
            // let _ = sender2.send("NSE <?1 =/> <{SELF} --> [good]>>? :|:".to_string());
            thread::sleep(Duration::from_secs_f64(0.03));
        });

        // * 📝Websocket Handler不能可变，就用RefCell实现内部可变性
        let right_side = RefCell::new(false);
        let num_good = RefCell::new(0_usize);
        let output_steps = RefCell::new(0_usize);
        let minimum_fitness_period = RefCell::new(usize::MAX);
        const MAX_GOOD: usize = 20;
        move |msg: ws::Message| {
            // println!("Got message: {}", msg);
            let msg = msg.to_string();
            // 记录步数
            let output_steps = &mut *output_steps.borrow_mut();
            *output_steps += 1;
            // 操作
            if msg.contains("EXE") {
                // 左右操作状态
                let left = msg.contains(r#"["left","{SELF}"]"#);
                let right = msg.contains(r#"["right","{SELF}"]"#);
                if !left && !right {
                    return Ok(());
                }
                let minimum_fitness_period = &mut *minimum_fitness_period.borrow_mut();
                // * 🔬可以尝试「左右颠倒」以观察NARS的适应能力
                let num_good = &mut *num_good.borrow_mut();
                let right_side = &mut *right_side.borrow_mut();
                let lr = if *right_side { "right" } else { "left" };
                // 奖励
                if left && !*right_side || right && *right_side {
                    let _ = sender.send("NSE <{SELF} --> [good]>. :|: %1.0; 0.5%".to_string());
                    println!("good\t{lr}\tfor {num_good}!\t{minimum_fitness_period}");
                    *num_good += 1;
                    // 改变模式
                    if *num_good > MAX_GOOD {
                        let b = *right_side;
                        *right_side = !b;
                        *num_good = 0;
                        // 一个轮回⇒以「轮回数」记录「适应性」
                        if b {
                            *minimum_fitness_period = *minimum_fitness_period.min(output_steps);
                            *output_steps = 0;
                        }
                    }
                }
                // 惩罚
                else {
                    let _ = sender.send("NSE <{SELF} --> [good]>. :|: %0.0; 0.5%".to_string());
                    println!("bad\t{lr}\tfor {num_good}!\t{minimum_fitness_period}");
                }
            }
            // out.close(CloseCode::Normal)
            Ok(())
        }
    }

    // 循环
    loop {
        let _ = ws::connect("ws://127.0.0.1:8765", train);
        // 连接失败则延迟等待
        sleep(Duration::from_secs(1));
    }
}

#[test]
fn test_overwhelming_nse() {
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
