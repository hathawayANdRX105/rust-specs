#![allow(dead_code)]

fn load_option(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.unwrap())
}

fn chinese_expect(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.expect("值必须存在"))
}

fn english_expect(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.expect("value must be present"))
}

fn ok_case(v: Option<u32>) -> Result<u32, ()> {
    Ok(v.ok_or(())?)
}

fn main() {}
