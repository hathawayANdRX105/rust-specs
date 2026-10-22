// dioxus-lint-fixture
// Bad: match / while / loop / let block inside rsx. `if` is handled by
// dioxus-rsx-no-if-else; `for` is allowed.
#[cfg(any())]
fn view(state: i32, running: bool, items: Vec<i32>) {
    rsx! {
        div {
            { match state { _ => "idle" } }
            { while running { tick() } }
            { loop { break } }
            { let label = compute(); label }
            for item in items {
                span { "{item}" }
            }
        }
    }
}

fn main() {}
