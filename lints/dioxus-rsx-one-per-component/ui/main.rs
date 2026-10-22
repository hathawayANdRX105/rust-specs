// dioxus-lint-fixture
// Clean: one rsx per component.
#[cfg(any())]
#[component]
fn Card() -> Element {
    rsx! { div { "one" } }
}

fn main() {}
