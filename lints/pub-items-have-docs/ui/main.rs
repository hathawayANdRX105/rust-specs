#![allow(dead_code)]

/// Documented pub function — OK (negative case).
pub fn documented_fn() {}

/// Documented pub struct — OK (negative case).
pub struct DocumentedStruct;

/// Documented pub enum — OK (negative case).
pub enum DocumentedEnum {
    A,
}

/// Documented pub const — OK (negative case).
pub const DOCUMENTED_CONST: u32 = 1;

/// Documented pub trait — OK (negative case).
pub trait DocumentedTrait {}

/// Documented pub static — OK (negative case).
pub static DOCUMENTED_STATIC: u32 = 0;

// `pub(crate)` without docs — OK: not plainly `pub`.
pub(crate) fn crate_only() {}

// `pub use` re-export — OK: `use` declarations are not reported.
pub use std::fmt::Debug;

// `mod tests` with `#[cfg(test)]` — OK: test modules are exempt.
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

// --- Positive cases (missing doc comments) ---

pub fn undocumented_fn() {} // 公共函数缺少文档注释

pub struct UndocumentedStruct; // 公共结构体缺少文档注释

pub enum UndocumentedEnum { // 公共枚举缺少文档注释
    A,
}

pub const UNDOCUMENTED_CONST: u32 = 2; // 公共常量缺少文档注释

pub static UNDOCUMENTED_STATIC: u32 = 3; // 公共静态量缺少文档注释

pub trait UndocumentedTrait {} // 公共 trait 缺少文档注释

pub type UndocumentedAlias = u32; // 公共类型别名缺少文档注释

pub union UndocumentedUnion { // 公共 union 缺少文档注释
    a: u32,
}

pub mod undocumented_mod {} // 公共模块缺少文档注释

fn main() {}
