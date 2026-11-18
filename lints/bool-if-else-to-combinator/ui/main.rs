// edition:2024
// run-rustfix
// rustfix-only-machine-applicable
#![allow(dead_code)]

// `Some` in `then`, `None` in `else`; the payload is a literal, so the
// suggestion is the eager `then_some`.
fn some_then(ready: bool) -> Option<u32> {
    if ready {
        Some(1)
    } else {
        None
    }
}

// Branches swapped (`None` in `then`): the condition has to be negated.
fn none_then(ready: bool) -> Option<u32> {
    if ready {
        None
    } else {
        Some(2)
    }
}

// A negated condition over the swapped shape folds the two negations away.
fn negated_cond(ready: bool) -> Option<u32> {
    if !ready {
        None
    } else {
        Some(3)
    }
}

// `Ok`/`Err` with a unit error: the value travels through `then_some`, the
// error through `ok_or`.
fn ok_err(ready: bool) -> Result<u32, ()> {
    if ready {
        Ok(1)
    } else {
        Err(())
    }
}

// Swapped `Err`/`Ok`: same chain, negated condition.
fn err_ok(ready: bool) -> Result<u32, i32> {
    if ready {
        Err(7)
    } else {
        Ok(2)
    }
}

// `Ok(())` carries no value, so the chain ends at the error alone.
fn unit_ok(ready: bool) -> Result<(), i32> {
    if ready {
        Ok(())
    } else {
        Err(7)
    }
}

// A lazy error payload behind a unit `Ok`.
fn unit_ok_lazy(ready: bool) -> Result<(), String> {
    if ready {
        Ok(())
    } else {
        Err(String::from("boom"))
    }
}

// Swapped `Err`/`Ok` with a unit value: negated condition.
fn unit_ok_swapped(ready: bool) -> Result<(), i32> {
    if ready {
        Err(9)
    } else {
        Ok(())
    }
}

// An expensive payload is not evaluated eagerly: the lazy `then` keeps the old
// timing instead of building the `String` on every call.
fn lazy_value(ready: bool) -> Option<String> {
    if ready {
        Some(String::from("value"))
    } else {
        None
    }
}

// An expensive error is built only when the condition is false, so the chain
// ends in `ok_or_else`.
fn lazy_err(ready: bool) -> Result<u32, String> {
    if ready {
        Ok(1)
    } else {
        Err(String::from("boom"))
    }
}

// An expensive value uses the lazy `then`, and the cheap error still `ok_or`s.
fn lazy_result_value(ready: bool) -> Result<String, i32> {
    if ready {
        Ok(String::from("value"))
    } else {
        Err(7)
    }
}

// A compound condition keeps its parentheses: `ready && flag.then_some(1)`
// would parse the wrong way.
fn compound_cond(ready: bool, flag: bool) -> Option<u32> {
    if ready && flag {
        Some(1)
    } else {
        None
    }
}

// The negated form of a compound condition needs the `!` outside the
// parentheses as well.
fn compound_negated(ready: bool, flag: bool) -> Option<u32> {
    if ready || flag {
        None
    } else {
        Some(2)
    }
}

// Any `bool` expression works as the condition, not just a local.
fn compute() -> bool {
    true
}

fn computed_cond() -> Option<u32> {
    if compute() {
        Some(1)
    } else {
        None
    }
}

// The payload is unsized here (`&Vec<u32>` -> `&[u32]`); `then_some` takes it
// as a generic argument where that coercion would not apply, so the lint
// reports without a suggestion.
fn coerced(ready: bool, values: &Vec<u32>) -> Option<&[u32]> {
    if ready {
        Some(values)
    } else {
        None
    }
}

// A plain binary value choice: not an `Option`/`Result`, repo default, no hit.
fn plain_value(ready: bool) -> u32 {
    if ready {
        1
    } else {
        2
    }
}

// A lone `is_some` condition belongs to `is-check-needs-chain`, whose rewrite
// keeps the receiver (`o.map(..)`).
fn predicate(o: Option<u32>) -> Option<u32> {
    if o.is_some() {
        Some(1)
    } else {
        None
    }
}

// An `else if` chain is not a single if/else; each leaf is checked on its own,
// and no leaf here is a wrapping pair.
fn else_if_chain(a: bool, b: bool) -> Option<u32> {
    if a {
        Some(1)
    } else if b {
        Some(2)
    } else {
        Some(3)
    }
}

// A branch with statements cannot move into a combinator argument.
fn with_statement(ready: bool) -> Option<u32> {
    if ready {
        let n = 1;
        Some(n)
    } else {
        None
    }
}

// `if let` conditions are not `bool`; those belong to
// `chainable-single-statement`.
fn if_let(o: Option<u32>) -> Option<u32> {
    if let Some(v) = o {
        Some(v)
    } else {
        None
    }
}

async fn read_u32(v: u32) -> u32 {
    v
}

// A payload that awaits cannot move behind a method call: `then` takes a
// closure and `.await` is not allowed inside one — no report.
async fn awaited_payload(flag: bool) -> Option<u32> {
    if flag {
        Some(read_u32(1).await)
    } else {
        None
    }
}

// A payload that returns from the enclosing function cannot move into a
// closure either — no report.
fn returning_payload(flag: bool) -> Option<u32> {
    if flag {
        None
    } else {
        Some({
            if flag {
                return None;
            }
            2
        })
    }
}

// Locally defined `Some`/`None` variants are not `Option`'s; the lint resolves
// constructors, it does not match on names.
enum Local {
    Some(u32),
    None,
}

fn local_enum(flag: bool) -> Local {
    if flag {
        Local::Some(1)
    } else {
        Local::None
    }
}

// Both branches wrap a value: there is no fallback to select.
fn same_branch(ready: bool) -> Option<u32> {
    if ready {
        Some(1)
    } else {
        Some(2)
    }
}

// No `else`: nothing to choose between.
fn no_else(ready: bool) {
    if ready {
        println!("ready");
    }
}

fn main() {}