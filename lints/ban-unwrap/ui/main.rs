#![allow(dead_code)]

fn load_option(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.unwrap())
}

fn load_expected(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.expect("value missing"))
}

fn ok_case(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.ok_or(())?)
}

fn load_infallible(v: Option<u32>) -> u32 {
    v.unwrap_or(0)
}

fn main() {}
