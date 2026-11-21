#![allow(dead_code)]

enum Shape {
    Circle(u32),
    Square { side: u32 },
}

fn area_of(shape: Shape) -> u32 {
    match shape {
        Shape::Circle(r) => r * r,
        // 命中：`_` 作为整个 arm 的模式，吞掉负载却不命名
        _ => 0,
    }
}

fn default_of(opt: Option<u32>) -> u32 {
    match opt {
        Some(x) => x,
        // 命中：Option 上的 bare `_` 整 arm 模式
        _ => 0,
    }
}

// 否定：兜底臂改用命名绑定，负载有名可查
fn with_binding(opt: Option<u32>) -> u32 {
    match opt {
        Some(x) => x,
        other => 0,
    }
}

// 否定：模式内部的 `_` 子模式（Some(_)）不视为整 arm 通配，允许
fn subpattern_ok(opt: Option<u32>) -> u32 {
    match opt {
        Some(_) => 0,
        None => 1,
    }
}

// 否定：三臂匹配用 `other` 作为兜底
fn three_arm(shape: Shape) -> u32 {
    match shape {
        Shape::Circle(r) => r,
        Shape::Square { side } => side,
        other => 0,
    }
}

// 否定：全部具体臂，无通配臂
fn exhaustive(shape: Shape) -> u32 {
    match shape {
        Shape::Circle(r) => r,
        Shape::Square { side } => side,
    }
}

macro_rules! wrap_match {
    ($e:expr) => {
        match $e {
            Some(_) => 1,
            _ => 0,
        }
    };
}

// 否定：宏展开里的 arm 跳过（span.from_expansion()）
fn from_macro(opt: Option<u32>) -> u32 {
    wrap_match!(opt)
}

fn main() {}
