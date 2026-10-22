# dioxus-effect-self-loop

Detects `use_effect` closures that write a `Signal` they also read.

| | |
|---|---|
| Lint | `dioxus_effect_self_loop` |
| Default level | `Warn` |
| Scope | any crate that calls `use_effect` (type information, no path filter) |

## What it catches

```rust
use_effect(move || {
    let current = value();          // read
    if current > 10 {
        value.set(10);              // write: self-loop
    }
});
```

Reads are: calling the signal (`value()`), `.read()`, `.cloned()`, `.peek()`.
Writes are: `.set()`, `.write()`, `.with_mut()`. The walk descends into
`spawn(async move { ... })` nested inside the effect closure.

## Why

Dioxus re-runs an effect when a signal read during the effect changes. Writing
that same signal from the effect closes the loop: the effect schedules itself,
which is the classic "effect that writes a signal it reads" anti-pattern (G1 in
`web_spec.json`). Derive the value with `use_memo`, or move the write to an
event handler / explicit action.

## Good

```rust
let doubled = use_memo(move || count() * 2);
```

or:

```rust
use_effect(move || {
    // read a signal, write a different one
    let current = source();
    mirror.set(current);
});
```

## Known problems

- Signals are tracked by resolving the receiver to a local or a `static`. A
  write through a re-borrowed field (`self.state`) is not tracked.
- Intentional one-shot "reset if invalid" effects are reported too. Allow the
  lint at the call site and state the reason.

## CI usage

```sh
cargo +nightly-2026-08-20 dylint \
  --path /home/hathaway/projects/rust-specs \
  --pattern 'lints/dioxus-effect-self-loop' \
  --all \
  --workspace
```
