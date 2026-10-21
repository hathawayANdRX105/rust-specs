#![allow(dead_code)]

// 真正的 else：if 分支早退 → 命中
fn with_else(flag: bool) -> u32 {
    if flag {
        return 0;
    } else {
        1
    }
}

// else-if 链：链里的 break 属于另一个 if，没有可删的 else → 不命中
fn else_if_chain(c: char) -> u32 {
    let mut total = 0;
    if c == '\\' {
        total += 1;
    } else if c == '"' {
        break_or(total);
    } else {
        total += 2;
    }
    total
}

fn break_or(v: u32) -> u32 {
    v
}

fn main() {}
