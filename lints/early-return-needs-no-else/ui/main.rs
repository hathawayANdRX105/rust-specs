#![allow(dead_code)]

fn with_else(flag: bool) -> u32 {
    if flag {
        return 0;
    } else {
        1
    }
}

fn without_else(flag: bool) -> u32 {
    if flag {
        return 0;
    }
    1
}

fn main() {}
