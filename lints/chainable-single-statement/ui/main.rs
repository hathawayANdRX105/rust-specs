#![allow(dead_code)]

// Positive: if-let with a single bool expression statement.
fn if_let_single(v: Option<u32>) {
    if let Some(x) = v {
        x.checked_add(1).is_some();
    }
}

// Positive: if-let with a single Result expression statement.
fn if_let_result(v: Option<u32>) {
    if let Some(x) = v {
        x.checked_add(1).ok_or(());
    }
}

// Positive: Result pattern with a bool body (`is_err_and`).
fn if_let_err(res: Result<u32, u32>) {
    if let Err(e) = res {
        e > 3;
    }
}

// Positive: for with a single call.
fn for_single(xs: &[u32]) {
    for x in xs {
        println!("{x}");
    }
}

// Negative: the if-let body is `()` (needs `{ body; true }`).
fn if_let_unit(v: Option<u32>) {
    if let Some(x) = v {
        println!("{x}");
    }
}

// Negative: the if-let body is an assignment.
fn if_let_assign(v: Option<u32>) {
    let mut out = 0;
    if let Some(x) = v {
        out = x;
    }
}

// Negative: the pattern is not Some/Ok/Err.
enum E {
    A(u32),
    B,
}
fn if_let_other(e: E) {
    if let E::A(x) = e {
        x.checked_add(1).is_some();
    }
}

// Negative: an else branch is present.
fn if_let_else(v: Option<u32>) {
    if let Some(x) = v {
        x.checked_add(1).is_some();
    } else {
        false;
    }
}

// Negative: the body escapes with `return`.
fn if_let_return(v: Option<u32>) {
    if let Some(x) = v {
        return;
    }
}

// Negative: the body contains `?`.
fn if_let_try(v: Option<u32>) -> Option<()> {
    if let Some(x) = v {
        x.checked_add(1)?;
    }
    Some(())
}

// Negative: the for body is an assignment to a captured variable.
fn for_assign(params: &[u32]) {
    let mut q = 0;
    for p in params {
        q = *p;
    }
}

// Negative: the for body escapes with `break`.
fn for_break(xs: &[u32]) {
    for x in xs {
        break;
    }
}

// Negative: the for body is a nested `if`.
fn for_if(xs: &[u32]) {
    for x in xs {
        if *x > 3 {
            println!("{x}");
        }
    }
}

fn main() {}
