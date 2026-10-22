// dioxus-lint-fixture
// Clean: business-id key.
#[cfg(any())]
fn clean() {
    rsx! {
        div {
            for item in items {
                div { key: "{item.id}", "{item.name}" }
            }
        }
    }
}

fn main() {}
