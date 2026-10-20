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

fn main() {}
