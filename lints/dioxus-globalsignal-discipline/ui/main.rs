// dioxus-lint-fixture
// Clean: one documented static, no non-static GlobalSignal use.
struct GlobalSignal<T>(T);

/// Global by design: every page needs the current value.
static LOCALE: GlobalSignal<i32> = GlobalSignal(1);

fn main() {}
