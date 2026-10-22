#![allow(dead_code)]

// for 算 1 层、else-if 链平铺 → 视觉深度 4（for/if-let/for/if-let）+ 内层 if = 5 → 命中
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

// for + else-if 链：视觉深度 2（for → if），不应命中
fn else_if_chain(items: &[&str]) -> u32 {
    let mut cjk = 0;
    let mut latin = 0;
    let mut digit = 0;
    let mut other = 0;
    for ch in items.iter().flat_map(|s| s.chars()) {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            cjk += 1;
        } else if ch.is_ascii_alphabetic() {
            latin += 1;
        } else if ch.is_ascii_digit() {
            digit += 1;
        } else if !ch.is_ascii_whitespace() {
            other += 1;
        }
    }
    cjk + latin + digit + other
}

// 纯 if 嵌套 4 层 → 不命中
fn if_four(flag: bool, v: Option<u32>) -> u32 {
    if flag {
        if let Some(x) = v {
            if x > 0 {
                return x;
            }
        }
    }
    0
}

fn main() {}

// while 算 1 层：while -> if -> match -> if -> if = 5 → 命中
fn while_deep(items: &[u32]) -> usize {
    let mut sum = 0usize;
    let mut rest = items.len();
    while rest > 0 {
        if rest % 2 == 0 {
            match rest {
                0 => {}
                n => {
                    if n > 3 {
                        if n > 5 {
                            sum += n;
                        }
                    }
                }
            }
        }
        rest -= 1;
    }
    sum
}

// `<expr>.await` desugars into a compiler-generated `match` wrapped in a
// generator `loop`. Neither is nesting the source shows, so 4 real levels
// plus an `await` must stay at depth 4 and not be reported.
async fn await_is_not_a_level(a: bool, b: bool, c: bool, d: bool) -> u32 {
    if a {
        if b {
            if c {
                if d {
                    let x = ready().await;
                    return x;
                }
            }
        }
    }
    0
}

async fn ready() -> u32 {
    7
}

// Five real levels still report, `await` or not.
async fn five_real_levels(
    a: bool,
    b: bool,
    c: bool,
    d: bool,
    e: bool,
) -> u32 {
    if a {
        if b {
            if c {
                if d {
                    if e {
                        return ready().await;
                    }
                }
            }
        }
    }
    0
}

// while 算 1 层：while -> if -> if -> if = 4 层，不应命中
// （回归：普通 while 的 desugar 条件 If 不能计入层数）
fn while_four(items: &[u32]) -> usize {
    let mut sum = 0usize;
    let mut i = 0usize;
    while i < items.len() {
        if i > 0 {
            if i > 1 {
                if i > 2 {
                    sum += i;
                }
            }
        }
        i += 1;
    }
    sum
}
