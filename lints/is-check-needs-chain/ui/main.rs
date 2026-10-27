#![allow(dead_code)]

// 形 B: 单独 is_some 判断后 if/else 仍产出 Option → 命中（应写 map 链）
fn some_branch(o: Option<u32>) -> Option<u32> {
    if o.is_some() {
        Some(1)
    } else {
        None
    }
}

// 形 B: is_none 反向 → 命中
fn none_reversed(o: Option<u32>) -> Option<u32> {
    if o.is_none() {
        None
    } else {
        Some(2)
    }
}

// 形 B: Result 同型 → 命中
fn ok_branch(r: Result<u32, ()>) -> Result<u32, ()> {
    if r.is_ok() {
        Ok(1)
    } else {
        Err(())
    }
}

// 形 A: is_some().then(..) 结果仍是 Option → 命中
fn then_after_check(o: Option<u32>) -> Option<u32> {
    o.is_some().then(|| 1)
}

// 形 A: is_some().then_some(..) 结果仍是 Option → 命中
fn then_some_after_check(o: Option<u32>) -> Option<u32> {
    o.is_some().then_some(1)
}

// 形 A: Result 接收者同理 → 命中
fn then_some_on_result(r: Result<u32, ()>) -> Option<u32> {
    r.is_ok().then_some(1)
}

// bool → Option → 普通值（unwrap_or 收尾）: 许可写法 → 不命中
fn manual_default(o: Option<u32>) -> u32 {
    o.is_none().then_some(3).unwrap_or(4)
}

// 同上（then + unwrap_or_else 收尾）: 不命中
fn manual_else(o: Option<String>, fallback: fn() -> String) -> String {
    o.is_none().then(fallback).unwrap_or_else(String::new)
}

// 单独 is_some 判断落在 bool → 不命中
fn is_present(o: Option<u32>) -> bool {
    o.is_some()
}

// 普通 if/else 二选一（非 Option 判断、非 Option 产出）→ 不命中
fn provider(p: String) -> String {
    if p.is_empty() {
        "manual".to_string()
    } else {
        p
    }
}

// 复合谓词不是单独判断 → 不命中
fn guarded(o: Option<u32>, flag: bool) -> u32 {
    if o.is_some() && flag { 1 } else { 0 }
}

// 无 else 的 if（unit 返回）→ 不命中
fn log_some(o: Option<u32>) {
    if o.is_some() {
        println!("some");
    }
}

// is_some_and 已是链式惯用法 → 不命中
fn and_form(o: Option<u32>) -> bool {
    o.is_some_and(|v| v > 1)
}

fn main() {}
