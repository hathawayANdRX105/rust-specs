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
