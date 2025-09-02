// 原始版本
//
// q|w|e|a|s|d：移动光标。
// .：开始绘制。
// #AARRGGBBM：结束绘制，AARRGGBB 为颜色值，M 为绘制方式（d 轮廓，f 填充，l 线段）。
// +x：变量 x 自增 1。
// -x：变量 x 自减 1。
// =x：变量 x 设为 0。
// %x：使用变量 x 的值作为重复次数。
// 3c：重复指令 c 3 次。
// [...]：将内部指令视为一个整体。
// {x...}：定义宏 x，内部为宏内容。
// @x：调用宏 x。
// *x：将当前位置保存为标记 x。
// ^x：移动到标记 x 处。
// 变量名、宏名、标记名均为单个字母（区分大小写）。
// 若无绘制指令，则输出所有变量的值。

use core::slice;
use linked_hash_map::LinkedHashMap as HashMap;

#[derive(Debug)]
pub enum Error {
    UndefinedMacro(Var, Loc),
    UndefinedMark(Var, Loc),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var(pub char);

impl std::fmt::Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Loc(pub usize);

#[derive(Debug)]
pub enum Cmd {
    Stack(Var),
    Set(Var),
    Add(Var),
    Sub(Var),
    Num(i64),
    Var(Var),
    Group(Vec<Cmd>),
    Macro(Var, Vec<Cmd>),
    Call(Var, Loc),
    Mark(Var),
    Goto(Var, Loc),
}

impl Cmd {
    pub fn preset_count(&self, count: Option<i64>) -> i64 {
        let default = match self {
            Cmd::Stack(..) => 0,
            Cmd::Set(..) => 0,
            Cmd::Add(..) => 1,
            Cmd::Sub(..) => 1,
            Cmd::Num(..) => 1,
            Cmd::Var(..) => 1,
            Cmd::Group(..) => 1,
            Cmd::Macro(..) => 1,
            Cmd::Call(..) => 1,
            Cmd::Mark(..) => 1,
            Cmd::Goto(..) => 1,
        };
        count.unwrap_or(default)
    }
}

peg::parser!(pub grammar parser() for str {
    rule _() = __* comment()?
    rule __() = quiet! {[' '|'\t'|'\r'|'\n']}
    rule comment() = ";" quiet!{[^'\n']}* _
    rule num() -> i64 = s:$(['0'..='9']+) {? s.parse().map_err(|_| "valided-number") }
    rule var() -> Var = !__ ch:[_] { Var(ch) }
    rule cmd_inner(loc: Loc) -> Cmd
        = "=" _ v:var() { Cmd::Set(v) }
        / "$" _ v:var() { Cmd::Stack(v) }
        / "+" _ v:var() { Cmd::Add(v) }
        / "-" _ v:var() { Cmd::Sub(v) }
        / "%" _ v:var() { Cmd::Var(v) }
        / "@" _ v:var() { Cmd::Call(v, loc) }
        / "*" _ v:var() { Cmd::Mark(v) }
        / "^" _ v:var() { Cmd::Goto(v, loc) }
        / "[" cmds:cmds() "]" { Cmd::Group(cmds) }
        / "{" _ v:var() cmds:cmds() "}" { Cmd::Macro(v, cmds) }
        / n:num() { Cmd::Num(n) }
    rule cmd() -> Cmd
        = p:position!() c:cmd_inner(Loc(p)) {c}
    pub rule cmds() -> Vec<Cmd>
        = _ c:(c:cmd() _ {c})* {c}
});

/// 或是栈, 栈情况加减操作尾部元素,
/// 等号清零, 如果有参数则增减栈
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Num(i64),
    Stack(Vec<i64>),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Num(n) => n.fmt(f),
            Self::Stack(nums) => nums.fmt(f),
        }
    }
}

impl From<Vec<i64>> for Value {
    fn from(v: Vec<i64>) -> Self {
        Self::Stack(v)
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Num(0)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Num(v)
    }
}

impl Value {
    pub fn num(&self) -> i64 {
        match self {
            Value::Num(n) => *n,
            Value::Stack(s) => s.last().copied().unwrap_or(0),
        }
    }

    pub fn inc(&mut self, count: i64) {
        let slot = match self {
            Value::Num(n) => n,
            Value::Stack(items) => {
                if items.is_empty() { items.push(0) }
                items.last_mut().unwrap()
            },
        };
        *slot += count;
    }

    pub fn set(&mut self, count: i64) {
        match self {
            Value::Num(n) => *n = count,
            Value::Stack(items) => {
                if let Some(slot) = items.last_mut() {
                    *slot = count;
                } else {
                    items.push(count);
                }
            },
        }
    }

    pub fn set_stack(&mut self, count: i64) {
        match self {
            &mut Value::Num(n) => {
                *self = Self::Stack(vec![n; count.max(0) as usize]);
            },
            Value::Stack(items) => {
                let new_len = (items.len() as i64).saturating_add(count);
                items.resize(new_len as usize, 0);
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct Vm<'a> {
    count: Option<Value>,
    pub vars: HashMap<Var, Value>,
    macros: HashMap<Var, &'a Vec<Cmd>>,
    marks: HashMap<Var, slice::Iter<'a, Cmd>>,
}

impl<'a> Vm<'a> {
    pub fn get_var(&mut self, var: &Var) -> &mut Value {
        self.vars.entry(var.clone())
            .or_default()
    }

    pub fn run(&mut self, cmds: &mut slice::Iter<'a, Cmd>) -> Result<(), Option<Error>> {
        let cmd = cmds.next().ok_or(None)?;
        let count = self.count.take();
        let num = count.map(|c| c.num());
        let count = cmd.preset_count(num);

        self.run_cmd(cmds, cmd, count)?;

        Ok(())
    }

    fn run_cmd(
        &mut self,
        cmds: &slice::Iter<'a, Cmd>,
        cmd: &'a Cmd,
        count: i64,
    ) -> Result<(), Option<Error>> {
        /// repeat! {}
        macro_rules! repeat {
            ($($t:tt)*) => {{
                for _ in 0..count {
                    $($t)*
                }
            }};
        }
        match cmd {
            Cmd::Stack(var) => self.get_var(var).set_stack(count),
            Cmd::Set(var) => self.get_var(var).set(count),
            Cmd::Add(var) => self.get_var(var).inc(count),
            Cmd::Sub(var) => self.get_var(var).inc(-count),
            Cmd::Num(n) => self.count = Some((count**n).into()),
            Cmd::Var(var) => self.count = Some((count*self.get_var(var).num()).into()),
            Cmd::Group(sub_cmds) => repeat! {
                self.run_to_finish(&mut sub_cmds.iter())?
            },
            Cmd::Macro(var, sub_cmds) => {
                self.macros.insert(var.clone(), sub_cmds);
            },
            Cmd::Call(var, loc) => repeat! {
                let &sub_cmds = self.macros.get(var)
                    .ok_or_else(|| Error::UndefinedMacro(var.clone(), *loc))?;
                self.run_to_finish(&mut sub_cmds.iter())?;
            },
            Cmd::Mark(var) => {
                self.marks.insert(var.clone(), cmds.clone());
            },
            Cmd::Goto(var, loc) => repeat! {
                let mut sub_cmds = self.marks.get(var)
                    .cloned()
                    .ok_or_else(|| Error::UndefinedMark(var.clone(), *loc))?;
                self.run_to_finish(&mut sub_cmds)?;
            },
        }
        Ok(())
    }

    pub fn run_to_finish(&mut self, cmds: &mut slice::Iter<'a, Cmd>) -> Result<(), Error> {
        loop {
            match self.run(cmds) {
                Ok(()) => {},
                Err(None) => break Ok(()),
                Err(Some(e)) => break Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn check_run(src: &str, expected: HashMap<Var, Value>) {
        let cmds = parser::cmds(src).unwrap();
        let mut vm = Vm::default();
        vm.run_to_finish(&mut cmds.iter()).unwrap();
        if vm.vars != expected {
            panic!("vars not equal\n left: {:#?}\nright: {expected:#?}", vm.vars)
        }
    }

    /// value_map! {}
    macro_rules! value_map {
        () => {HashMap::new()};
        ($($k:ident : $v:expr),+ $(,)?) => {{
            let mut map = HashMap::new();
            $(
                let s = stringify!($k);
                assert_eq!(s.chars().count(), 1);
                map.insert(Var(s.chars().next().unwrap()), Value::from($v));
            )+
            map
        }};
    }

    #[test]
    fn it_works() {
        check_run("=a", value_map! { a: 0 });
        check_run("+a", value_map! { a: 1 });
        check_run("2+a", value_map! { a: 2 });
        check_run("2[+a]", value_map! { a: 2 });
        check_run("=a+a", value_map! { a: 1 });
        check_run("2=a", value_map! { a: 2 });
        check_run("2[=a]", value_map! { a: 0 });
        check_run("2=a-a", value_map! { a: 1 });
        check_run("2 3=a", value_map! { a: 6 });
        check_run("-x-x%x3=a", value_map! { x: -2, a: -6 });
        check_run("$a", value_map! { a: vec![] });
        check_run("2$a", value_map! { a: vec![0, 0] });
        check_run("2$a+a", value_map! { a: vec![0, 1] });
        check_run("2$a+a+a", value_map! { a: vec![0, 2] });
        check_run("2$a+a$a", value_map! { a: vec![0, 1] });
        check_run("2$a+a=a", value_map! { a: vec![0, 0] });
        check_run("2$a+a-b%b$a", value_map! { a: vec![0], b: -1 });
        check_run("2$a+a1$a", value_map! { a: vec![0, 1, 0] });
        check_run("4=x*m+a-x%x^m", value_map! { x: -3, a: 7 });
        check_run("{m+a+a+a}@m@m", value_map! { a: 6 });
        check_run("+a;...\n+a", value_map! { a: 2 });
    }
}
