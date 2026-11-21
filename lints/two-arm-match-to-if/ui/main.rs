#![allow(dead_code, unreachable_patterns)]

/// Positive: literal pair — a string literal against `_`.
fn lit_pair(s: &str) -> u32 {
    match s {
        "a" => 1,
        _ => 2,
    }
}

/// Positive: unit variant `None` against `_`.
fn unit_variant(v: Option<u32>) -> u32 {
    match v {
        None => 1,
        _ => 2,
    }
}

/// Positive: bool literal `true` against `_`.
fn bool_lit(b: bool) -> u32 {
    match b {
        true => 1,
        _ => 2,
    }
}

/// Positive: range pattern without bindings against `_`.
fn range_no_binding(n: u32) -> u32 {
    match n {
        1..=5 => 1,
        _ => 2,
    }
}

/// Negative: three arms.
fn three_arms(n: u32) -> u32 {
    match n {
        1 => 1,
        2 => 2,
        _ => 3,
    }
}

/// Negative: arm with a guard.
fn guard_arm(v: Option<u32>) -> u32 {
    match v {
        Some(x) if x > 0 => 1,
        _ => 2,
    }
}

/// Negative: non-catch-all arm binds a payload (`x`).
fn binding_payload(v: Option<u32>) -> u32 {
    match v {
        Some(x) => 1,
        _ => 2,
    }
}

/// Negative: both arms catch-all.
fn both_wild() -> u32 {
    match () {
        _ => 1,
        _ => 2,
    }
}

/// Negative: both arms specific, no catch-all.
fn no_catch_all(b: bool) -> u32 {
    match b {
        true => 1,
        false => 2,
    }
}

fn main() {}
