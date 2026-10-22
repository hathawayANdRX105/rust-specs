#![allow(dead_code)]

// 非 Result/Option 函数里用 expect → 命中（? 不可用）
fn non_result(v: Option<u32>) -> u32 {
    v.expect("value must be present")
}

// 返回 Result → 不命中（? 可用，应改用 ?）
fn returns_result(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.expect("value must be present"))
}

// 返回 Option → 不命中
fn returns_option(v: Option<u32>) -> Option<u32> {
    Some(v.expect("value must be present"))
}

// 闭包 → 不命中（闭包无 fn_sig，规则跳过）
fn closure_case(v: Option<u32>) -> u32 {
    let f = || v.expect("value must be present");
    f()
}

fn main() {}