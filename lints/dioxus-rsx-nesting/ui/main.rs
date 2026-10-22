// dioxus-lint-fixture
// Clean: one level of nesting.
#[cfg(any())]
fn clean() {
    rsx! {
        div {
            span { "one level" }
        }
    }
}

fn main() {}
