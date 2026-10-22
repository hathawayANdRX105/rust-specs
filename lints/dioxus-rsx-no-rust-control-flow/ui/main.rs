// dioxus-lint-fixture
// Clean rsx: no if / match / while / loop / let. `for` is allowed.
#[cfg(any())]
fn clean() {
    rsx! {
        div {
            for item in items {
                span { "{item}" }
            }
            span { "ok" }
        }
    }
}

fn main() {}
