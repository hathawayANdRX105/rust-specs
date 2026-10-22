// dioxus-lint-fixture
// Clean effect: reads a signal but never writes it.
struct Signal<T>(T);

impl<T: Copy> Signal<T> {
    fn new(value: T) -> Self {
        Signal(value)
    }

    fn read(&self) -> T {
        self.0
    }

    fn set(&self, _value: T) {}
}

fn use_effect<F: FnMut()>(_f: F) {}

fn clean() {
    let value = Signal::new(1);
    use_effect(move || {
        let _ = value.read();
    });
}

fn main() {}
