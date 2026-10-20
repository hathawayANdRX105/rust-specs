#![allow(dead_code)]

fn three_loops(a: &[u32], b: &[u32], c: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for x in a {
        out.push(x + 1);
    }
    for x in b {
        out.push(x + 2);
    }
    for x in c {
        out.push(x + 3);
    }
    out
}

fn two_loops(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for x in a {
        out.push(x + 1);
    }
    for x in b {
        out.push(x + 2);
    }
    out
}

fn main() {}
