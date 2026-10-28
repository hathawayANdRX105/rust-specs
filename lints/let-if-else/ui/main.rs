// dioxus-lint-fixture
// Bad: hoisting an if/if-else with rsx branches into a let binding.
#[cfg(any())]
fn bad(c: bool) {
    let node = if c {
        rsx! { span { "a" } }
    } else {
        rsx! { span { "b" } }
    };
    let _ = node;
    let _ = rsx! { div {} };
}

// Good: pick keeps the choice in one expression.
#[cfg(any())]
fn good_pick(c: bool) {
    let _ = rsx! {
        div {
            { pick(c, rsx! { span { "a" } }, rsx! { span { "b" } }) }
        }
    };
}

// Good: hoisting pick (a function call, not an if expression) into a let is
// not this lint's job — only `Expr::If` initializers are inspected.
#[cfg(any())]
fn good_hoisted_pick(c: bool) {
    let node = pick(c, rsx! { span { "a" } }, rsx! { span { "b" } });
    let _ = rsx! { div { {node} } };
}

// Good: an ordinary let-if without an rsx branch.
#[cfg(any())]
fn good_plain(c: bool) {
    let n = if c { 1 } else { 2 };
    let _ = n;
}

// Good: an if inside rsx is dioxus-rsx-no-if-else's job, not this lint's.
#[cfg(any())]
fn good_rsx_if(c: bool) {
    let _ = rsx! {
        div {
            if c { span { "a" } }
        }
    };
}

fn main() {}
