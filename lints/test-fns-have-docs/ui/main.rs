// compile-flags: --test
#![allow(dead_code)]

// 正例1：顶层的 `#[test]` 函数没有文档注释 → 应该报告
#[test]
fn top_level_undocumented() {}

// 正例2：`#[cfg(test)] mod tests` 里的 `#[test]` 函数同样必须写文档注释 → 应该报告
#[cfg(test)]
mod tests {
    // 这个测试函数没有文档注释 → 应该报告
    #[test]
    fn module_test_undocumented() {}
}

// 反例1：有文档注释的 `#[test]` 函数 → 不报告
/// Returns true when the input slice is empty.
#[test]
fn empty_input_is_empty() {
    assert!(true);
}

// 反例2：不是测试的函数，即使没有文档注释 → 不报告
fn undocumented_helper() {}

// 反例3：测试模块里有文档注释的 `#[test]` 函数 → 不报告
#[cfg(test)]
mod documented_tests {
    /// Adds two numbers and asserts the expected sum.
    #[test]
    fn adds_two_numbers() {
        assert_eq!(2 + 2, 4);
    }
}

fn main() {}
