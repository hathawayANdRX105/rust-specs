// Fixture for the `cjk-in-code` rule. Chinese is deliberate: this rule reports Chinese in
// comments and in string literals outside test scope, and never reports rsx copy (that is
// `dioxus-rsx-cjk-text`'s job).

// 行内注释 → reported

/// 文档注释 → reported

#[cfg(any())]
fn rsx_copy_is_not_double_reported() {
    rsx! { span { "保存" } }
}

#[cfg(any())]
fn rsx_data_attributes_are_not_copy() {
    rsx! { div { "data-testid": "保存按钮" } }
}

#[cfg(any())]
fn chinese_identifier_is_reported() {
    let 保存 = 1;
    println!("{}", 保存);
}

#[cfg(test)]
mod tests {
    // 测试里的中文注释 → reported
    #[test]
    fn renders_the_save_label() {
        let copy = "保存";
        assert_eq!(copy, "保存");
    }
}

fn main() {
    // 生产代码里的中文注释 → reported
    let copy = "保存按钮";
    println!("{copy}");
}
