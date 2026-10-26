// Bad: a let-bound pick_with whose two branches are no-arg closures.
#[cfg(any())]
fn bad_pick_with(c: bool) {
    let _node = pick_with(c, || 1, || 2);
}

// Bad: an if/else whose two branch tails are no-arg closures.
#[cfg(any())]
fn bad_if_closures(c: bool) {
    let _node = if c { || 1 } else { || 2 };
}

// Good: an ordinary let-if without closures.
#[cfg(any())]
fn good_plain(c: bool) {
    let n = if c { 1 } else { 2 };
    let _ = n;
}

// Good: pick selecting between two rsx branches.
#[cfg(any())]
fn good_pick_rsx(c: bool) {
    let _ = pick(c, rsx! { span { "a" } }, rsx! { span { "b" } });
}

fn main() {}
