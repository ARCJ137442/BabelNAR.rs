//! 🚩【2024-03-30 23:36:48】曾经的尝试：
//!   * 所有「路径构建器」都返回一个动态的「虚拟机启动器」类型
//!   * 启动时只需在一个「Anyhow虚拟机启动器」列表中选择

trait Turned {
    fn say(&self);
}
trait Unturned {
    type Target: Turned;
    fn turn(self) -> Self::Target;
    fn turn_box(self: Box<Self>) -> Box<Self::Target>;
    fn turn_box_sized(self: Box<Self>) -> Box<Self::Target>
    where
        Self: Sized,
    {
        Box::new(self.turn())
    }
}
struct U(usize);
struct T(usize);
impl Turned for T {
    fn say(&self) {
        print!("I'm T({})", self.0)
    }
}
impl Unturned for U {
    type Target = T;
    fn turn(self) -> T {
        T(self.0)
    }
    fn turn_box(self: Box<Self>) -> Box<Self::Target> {
        self.turn_box_sized()
    }
}
struct AnyhowUnturned<T: Turned = AnyhowTurned> {
    inner: Box<dyn Unturned<Target = T>>,
}
struct AnyhowTurned {
    inner: Box<dyn Turned>,
}
impl Turned for AnyhowTurned {
    fn say(&self) {
        self.inner.say()
    }
}
impl Unturned for AnyhowUnturned<T> {
    type Target = AnyhowTurned;
    fn turn(self) -> AnyhowTurned {
        AnyhowTurned {
            inner: self.inner.turn_box(),
        }
    }

    fn turn_box(self: Box<Self>) -> Box<Self::Target> {
        self.turn_box_sized()
    }
}
impl<T: Turned, U: Unturned<Target = T>> From<U> for AnyhowUnturned<T> {
    fn from(value: U) -> Self {
        Self {
            inner: Box::new(value),
        }
    }
}
struct AnyhowUnturned2 {
    inner: AnyhowTurned,
}

fn main() {
    let unturned: AnyhowUnturned<_> = U(1).into();
}

// pub struct AnyhowLauncher<'a, Runtime: VmRuntime + 'a> {
//     pub launcher: Box<dyn VmLauncher<Runtime> + 'a>,
// }

// impl<'a, Runtime: VmRuntime + 'a> AnyhowLauncher<'a, Runtime> {
//     pub fn new<Launcher>(launcher: impl VmLauncher<Runtime> + 'a) -> Self
//     where
//         Launcher: VmLauncher<Runtime> + 'a,
//     {
//         Self {
//             launcher: Box::new(launcher),
//         }
//     }
// }

// /// ! Box<Runtime>不能充当`VmLauncher`的参数：未实现`VmRuntime`
// impl<'a, Runtime: VmRuntime + 'a> VmLauncher<AnyhowRuntime<'a>> for AnyhowLauncher<'a, Runtime> {
//     fn launch(self) -> AnyhowRuntime<'a> {
//         AnyhowRuntime {
//             inner: Box::new(self.launcher.launch()),
//         }
//     }
// }

// struct AnyhowRuntime<'a> {
//     inner: Box<dyn VmRuntime + 'a>,
// }

// impl AnyhowRuntime<'_> {
//     fn new(inner: impl VmRuntime) -> Self {
//         Self {
//             inner: Box::new(inner),
//         }
//     }
// }

// impl VmRuntime for AnyhowRuntime<'_> {
//     fn input_cmd(&mut self, cmd: navm::cmd::Cmd) -> anyhow::Result<()> {
//         self.inner.input_cmd(cmd)
//     }

//     fn fetch_output(&mut self) -> anyhow::Result<navm::output::Output> {
//         self.inner.fetch_output()
//     }

//     fn try_fetch_output(&mut self) -> anyhow::Result<Option<navm::output::Output>> {
//         self.inner.try_fetch_output()
//     }

//     fn terminate(self) -> anyhow::Result<()> {
//         self.inner.terminate()
//     }
// }
