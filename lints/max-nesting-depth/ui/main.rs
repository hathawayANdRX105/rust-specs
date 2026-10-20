#![allow(dead_code)]

fn deep(items: &[Option<Vec<Option<u32>>>]) -> u32 {
    let mut sum = 0;
    for x in items {
        if let Some(inner) = x {
            for y in inner {
                if let Some(v) = y {
                    if *v > 0 {
                        sum += *v;
                    }
                }
            }
        }
    }
    sum
}

fn shallow(flag: bool, v: Option<u32>) -> u32 {
    if flag {
        if let Some(x) = v {
            return x;
        }
    }
    0
}

fn main() {}
