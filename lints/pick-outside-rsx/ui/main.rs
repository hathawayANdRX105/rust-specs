// dioxus-lint-fixture
// The qualified-path case is covered by the unit tests: the expected-output
// file lives under `ui/`, and the test harness rewrites that path prefix
// anywhere it appears — including inside the source line a finding points at.

// Bad: plain-value pick outside rsx — use a plain if/else expression.
#[cfg(any())]
fn bad_bare_pick(active: bool) {
    let class = pick(active, "row-active", "row");
    let _ = class;
}

// Bad: pick_with outside rsx.
#[cfg(any())]
fn bad_pick_with(active: bool) {
    let n = pick_with(active, || 1, || 2);
    let _ = n;
}

// Good: pick inside rsx selects between two elements.
#[cfg(any())]
fn good_pick_inside_rsx(active: bool) {
    let _ = rsx! {
        div {
            { pick(active, rsx! { span { "active" } }, rsx! {}) }
        }
    };
}

// Good: an unrelated method named pick is not this lint's business.
#[cfg(any())]
fn good_unrelated_pick(selector: &Selector) {
    let _ = selector.pick(&refs);
}

// Good: another module's pick is not the project's helper.
#[cfg(any())]
fn good_other_module_pick(active: bool) {
    let n = other::pick(active, 1, 2);
    let _ = n;
}

fn main() {}
