// dioxus-lint-fixture
// Bad: if / if-else inside rsx. `for` is allowed.
#[cfg(any())]
fn view(show: bool, items: Vec<i32>) {
    rsx! {
        div {
            if show { span { "visible" } }
            if show { span { "a" } } else { span { "b" } }
            for item in items {
                span { "{item}" }
            }
        }
    }
}

fn main() {}

// Regression: an `if` on a line that already holds non-ASCII text. The column
// from `proc_macro2` counts characters, so treating it as a byte offset used to
// land the span inside a multi-byte character, panicking rustc and aborting the
// whole crate's lint pass (every later finding was silently dropped).
#[cfg(any())]
fn bad_after_non_ascii(c: bool) {
    let _ = rsx! { div { "探针中文" if c { span { "a" } } } };
}
