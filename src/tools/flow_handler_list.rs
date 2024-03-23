//! 模块：流式处理者列表
//! * 🎯用于流式处理物件，并在这其中灵活控制处理流程
//! * 📌组合式处理流程：多个处理者在一个处理函数中处理
//! * 📌截断式消耗过程：处理的物件可能会在中途被处理者消耗
//!
//! ? 【2024-03-23 14:45:53】是否需要整合进[`nar_dev_utils`]中去

use std::marker::PhantomData;

/// 枚举：处理结果
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandleResult<Item, HandlerIndex> {
    /// 物件通过了所有处理者，并最终返回
    Passed(Item),
    /// 物件在处理中途被消耗，指示「消耗了物件的处理者」
    Consumed(HandlerIndex),
}

/// 流式处理者列表
/// * 🚩处理者的特征约束：`FnMut(Item) -> Option<Item>`
/// * 📝不能显式声明「处理者」类型
///   * ❗若作为泛型参数，则意味着「需要统一所有类型」
///   * 📌而各个闭包彼此之间类型都是不同的
pub struct FlowHandlerList<Item> {
    /// 存储所有的处理者
    /// * 🚩使用[`Box`]以容纳不同类型的闭包
    handlers: Vec<Box<dyn FnMut(Item) -> Option<Item>>>,

    /// 用于对未直接作为字段的`Item`类型的占位符
    /// * 🔗标准库文档：<https://rustwiki.org/zh-CN/std/marker/struct.PhantomData.html>
    _marker: PhantomData<Item>,
}

impl<Item> FlowHandlerList<Item> {
    /// 构造函数/从某个[`Box`]迭代器中构造
    /// * ℹ️若需构造一个空列表，可使用[`FlowHandlerList::default`]
    /// * 📝【2024-03-23 15:09:58】避免不了装箱：存储的是特征对象，不能不装箱就迭代
    /// * ❌【2024-03-23 15:31:48】停用：对参数`([Box::new(|x| Some(x)),],)`也无法使用
    ///   * 🚩已改用快捷构造宏
    pub fn new() -> Self {
        Self::from_vec(vec![])
    }

    /// 构造函数/直接从[`Vec`]构造
    /// * 需要自己手动装箱
    /// * ℹ️若需构造一个空列表，可使用[`FlowHandlerList::default`]
    pub fn from_vec(vec: Vec<Box<dyn FnMut(Item) -> Option<Item>>>) -> Self {
        Self {
            handlers: vec,
            _marker: PhantomData,
        }
    }

    // 核心逻辑 //

    /// 【核心】处理
    /// * 🚩主要思路：不断让`Item`值通过各个处理者，直到「全部通过」或「有处理者消耗」
    /// * ⚙️返回值：全部通过后的物件 / 被消耗的处理者索引
    /// * 📝实际上也可不用额外的`let item`，直接使用传入所有权的参数变量
    pub fn handle(&mut self, mut item: Item) -> HandleResult<Item, usize> {
        // // 预置好物件变量
        // let mut item = item;
        // 逐个遍历处理者
        for (index, handler) in self.handlers.iter_mut().enumerate() {
            // 调用处理者处理物件，并对返回值做分支
            match handler(item) {
                // 有返回值⇒继续
                // ! 这里的返回值有可能已【不是】原来的那个了
                Some(new_item) => item = new_item,
                // 没返回值⇒报告处理者所在索引
                None => return HandleResult::Consumed(index),
            }
        }
        // 最终通过
        HandleResult::Passed(item)
    }

    // 对「处理者列表」的操作 //

    /// 获取某个位置的处理者（不可变）
    pub fn get_handler(&self, index: usize) -> Option<&dyn FnMut(Item) -> Option<Item>> {
        // 获取指定位置的box，然后将其转为索引
        self.handlers.get(index).map(Box::as_ref)
    }

    // ! 【2024-03-23 15:16:08】废稿：可变引用的生命周期类型是【invariant】的
    // * 📝生命周期中`'self : 'handler`不代表`&mut 'self`
    // * 🔗参考：<https://doc.rust-lang.org/nomicon/subtyping.html>
    // /// 获取某个位置的处理者（可变）
    // /// * ℹ️[`Self::get_handler`]的可变引用版本
    // pub fn get_handler_mut(
    //     &mut self,
    //     index: usize,
    // ) -> Option<&mut dyn FnMut(Item) -> Option<Item>> {
    //     self.handlers.get_mut(index).map(Box::as_mut)
    // }

    /// 添加新的处理者
    /// * ⚠️虽然结构体定义时无需对「处理者」类型约束为`'static`静态周期，
    ///   * 但此处传入作为参数（的函数指针）是需要的
    pub fn add_handler(&mut self, handler: impl FnMut(Item) -> Option<Item> + 'static) {
        self.handlers.push(Box::new(handler))
    }
}

/// 默认构造函数：空数组
impl<Item> Default for FlowHandlerList<Item> {
    fn default() -> Self {
        Self::new()
    }
}

/// 快捷构造宏
#[macro_export]
macro_rules! flow_handler_list {
    [ $($handler:expr),* $(,)? ] => {
        // * ❌【2024-03-23 15:34:04】暂时不使用`$crate`：模块路径尚未固定
        FlowHandlerList::from_vec(
            vec![$(Box::new($handler)),*]
        )
    };
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use util::*;
    use HandleResult::*;

    /// 基础功能测试
    #[test]
    fn test_flow_handler_list() {
        // * 📝`|x| Some(x)`可以直接使用构造函数调用，写成`Some`
        let handler1 = Some;
        let handler2 = |x| Some(x + 1);
        let handler3 = |x| if x > 1 { Some(x) } else { None };

        let mut list = FlowHandlerList::new();

        asserts! {
            // 第一个闭包
            list.add_handler(handler1) => (),
            list.handle(0) =>Passed(0),
            // 第二个闭包
            list.add_handler(handler2) => (),
            list.handle(0) => Passed(1),
            // 第三个闭包
            list.add_handler(handler3) => (),
            list.handle(0) => Consumed(2), // 被消耗，索引在最后一个
            list.handle(1) => Passed(2), // 通过
        }

        let mut list = flow_handler_list![
            Some,
            |x: usize| Some(x + 1),
            |x| Some(dbg!(x)),
            |x: usize| Some(x - 1),
        ];

        asserts! {
            list.handle(0) => Passed(0)
        }
    }

    /// 联动「NAVM输出」测试
    #[test]
    fn test_navm_output() {
        use narsese::conversion::string::impl_lexical::shortcuts::*;
        use navm::output::*;
        // 构造输出
        let answer = Output::ANSWER {
            content_raw: "<A --> B>.".into(),
            narsese: Some(nse!(<A --> B>.)), // * ✨直接使用新版快捷构造宏
        };
        let out = Output::OUT {
            content_raw: "<A --> C>".into(),
            narsese: Some(nse!(<A --> C>.)),
        };
        // 构造处理者列表
        let mut list = flow_handler_list![
            // 展示
            |out: Output| Some(dbg!(out)),
            // 截获回答
            |out| match out {
                Output::ANSWER {
                    content_raw,
                    narsese,
                } => {
                    println!("截获到回答：{content_raw:?} | {narsese:?}");
                    None
                }
                _ => Some(out),
            },
            // 展示
            |out| {
                println!("这是其它输出：{out:?}");
                Some(out)
            },
        ];
        // 测试处理
        asserts! {
            // 回答被截获
            list.handle(answer) => Consumed(1),
            // 其它被通过
            list.handle(out.clone()) => Passed(out),
        }
    }
}
