#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_hir;
extern crate rustc_span;

use clippy_utils::diagnostics::span_lint;
use dioxus_lint_scope::file_allowed;
use dioxus_rsx::{AttributeName, BodyNode};
use dioxus_rsx_scan::{byte_range, parse_rsx_calls, token_byte_range, walk_nodes};
use quote::ToTokens;
use rustc_hir::def_id::LOCAL_CRATE;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{BytePos, FileName, SourceFile, Span};
use syn::visit::Visit;
use syn::{Item, LitStr, Macro, Meta};

dylint_linting::declare_late_lint! {
    /// Source text must stay in English outside test code: comments, string literals and
    /// identifiers.
    ///
    /// Two rules live in one lint, and they are enforced differently. Comments — including doc
    /// comments inside `#[cfg(test)]` blocks — are always English. String literals and identifiers
    /// may carry Chinese, but only inside test scope: a fixture that asserts on Chinese input, a
    /// build script or an integration test can legitimately hold the string. A comment is never
    /// exempt, so a Chinese comment above a test function is reported the same way as one in a
    /// `src/` module.
    ///
    /// ### Why is this bad?
    ///
    /// The project convention is that comments are English and that user-facing copy lives in the
    /// i18n tables, so a Chinese literal in normal code is either copy that cannot be translated or
    /// a comment nobody maintains in two languages. Chinese inside test scope is different: the
    /// string is the fixture.
    ///
    /// ### Known problems
    ///
    /// Source-text based, so it cannot tell whether a string literal is user-visible; the same is
    /// true of the comment rule, which is deliberate. CJK is detected with the same three ranges
    /// (`U+3400..U+4DBF`, `U+4E00..U+9FFF`, `U+F900..U+FAFF`) that `dioxus-rsx-cjk-text` uses, so
    /// kana or Hangul pass through. Chinese inside an rsx element body is left to
    /// `dioxus-rsx-cjk-text` and is not reported twice; `data-*` rsx attributes, the arguments of
    /// `include_str!` / `include_bytes!`, `#[cfg(test)]` / `#[test]` items (`cfg(all(test, ..))`
    /// groupings included), test files and i18n / data paths are also exempt for literals and
    /// identifiers only. Chinese inside the inline-code spans of a doc comment is still reported,
    /// because it is part of the comment. Because the rule scans the whole workspace, its first run
    /// reports the existing debt: gate it behind screening before turning it into `-D`.
    ///
    /// ### Example
    ///
    /// ```rust
    /// // 保存按钮的渲染分支
    /// const SAVE: &str = "保存";
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// // Render branch for the save button.
    /// const SAVE: &str = t!("save.label");
    /// ```
    ///
    /// Inside a test Chinese is allowed for strings, not for comments:
    ///
    /// ```rust
    /// #[test]
    /// fn renders_the_save_label() {
    ///     let copy = "保存";
    ///     // 断言文案与 locales/zh.yaml 一致
    ///     assert_eq!(copy, "保存");
    /// }
    /// ```
    pub CJK_IN_CODE,
    Warn,
    "Chinese text appears in a comment, string literal or identifier outside test code"
}

/// Where the text sits in the source: the message and the exemption policy differ per kind.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TextKind {
    Comment,
    Literal,
    Char,
    Ident,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Region {
    kind: TextKind,
    /// Byte offsets into the file: the lexer works on raw text, so its positions are byte offsets.
    lo: usize,
    hi: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Hit {
    lo: usize,
    hi: usize,
    msg: &'static str,
}

impl Region {
    fn msg(&self) -> &'static str {
        match self.kind {
            TextKind::Comment => {
                "Chinese text in a comment; comments (doc comments included) must be in English"
            }
            TextKind::Literal => {
                "Chinese text in a string literal; keep non-test string content in English (a test may use Chinese)"
            }
            TextKind::Char => {
                "Chinese text in a character literal; keep non-test literals in English"
            }
            TextKind::Ident => "Chinese identifier; identifiers must be in English",
        }
    }
}

/// One file being scanned: enough to answer "is this test scope" and "is this copy data?".
struct Ctx<'a> {
    crate_name: &'a str,
    path: &'a str,
}

impl<'tcx> LateLintPass<'tcx> for CjkInCode {
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        let crate_name = cx.tcx.crate_name(LOCAL_CRATE);
        let crate_name = crate_name.as_str();
        let source_map = cx.sess().source_map();
        for file in source_map.files().iter() {
            if !file_allowed(file) {
                continue;
            }
            let Some(text) = file.src.as_deref().map(|text| text.as_str()) else {
                // Virtual files (macro expansions) have no source of their own; the file that
                // defines the macro is scanned in its own pass.
                continue;
            };
            let FileName::Real(name) = &file.name else {
                continue;
            };
            let Some(path) = name.local_path() else {
                continue;
            };
            if path.extension().map_or(true, |extension| extension != "rs") {
                continue;
            }
            let Some(path) = path.to_str() else {
                // A non-UTF-8 path cannot match the i18n / data directories below, and refusing to
                // exempt is the conservative reading.
                continue;
            };
            let ctx = Ctx { crate_name, path };
            for hit in scan(text, &ctx) {
                span_lint(
                    cx,
                    CJK_IN_CODE,
                    span_from_bytes(file, text, hit.lo, hit.hi),
                    hit.msg,
                );
            }
        }
    }
}

impl Default for CjkInCode {
    fn default() -> Self {
        CjkInCode
    }
}

/// Find every Chinese span this rule reports, in source order.
///
/// The scan is pure source text: comments exist only in the raw file, so nothing here can be
/// derived from the HIR or the token stream. It is also the seam the unit tests exercise.
fn scan(text: &str, ctx: &Ctx<'_>) -> Vec<Hit> {
    if !has_cjk(text) {
        return Vec::new();
    }
    let exempt = exempt_ranges(text);
    let file_exempts_literals = is_data_source(ctx) || is_test_path(ctx.path);
    let mut hits = Vec::new();
    for region in lex(text) {
        let region_text = &text[region.lo..region.hi];
        if !has_cjk(region_text) {
            continue;
        }
        // Comments are never exempt: a Chinese comment in a test file is still a Chinese comment.
        let exempt_here = !matches!(region.kind, TextKind::Comment)
            && (file_exempts_literals || overlaps(&exempt, region.lo, region.hi));
        if exempt_here {
            continue;
        }
        hits.push(Hit {
            lo: region.lo,
            hi: region.hi,
            msg: region.msg(),
        });
    }
    hits
}

/// Classify the source into runs of comment, literal or identifier. Anything the rule does not
/// care about (whitespace, brackets, operators) is skipped.
fn lex(text: &str) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let current = char_at(text, i);
        let next = char_at(text, i + current.len_utf8());

        if let Some((lo, hi)) = literal_region(text, current, next, i) {
            regions.push(Region {
                kind: TextKind::Literal,
                lo,
                hi,
            });
            i = hi;
            continue;
        }
        if current == '/' && next == '/' {
            // `//`, `///` and `//!` are all comments; the closing newline stays out of the span.
            let mut hi = i;
            while hi < text.len() && char_at(text, hi) != '\n' {
                hi += char_at(text, hi).len_utf8();
            }
            regions.push(Region {
                kind: TextKind::Comment,
                lo: i,
                hi,
            });
            i = hi;
            continue;
        }
        if current == '/' && next == '*' {
            // Block comments nest in Rust, unlike in C.
            match block_comment_end(text, i + 1) {
                Some(hi) => {
                    let lo = i;
                    i = hi;
                    regions.push(Region {
                        kind: TextKind::Comment,
                        lo,
                        hi,
                    });
                }
                None => {
                    let lo = i;
                    regions.push(Region {
                        kind: TextKind::Comment,
                        lo,
                        hi: text.len(),
                    });
                    i = text.len();
                }
            }
            continue;
        }
        if current == '\'' {
            // `'a` / `'static` / `for<'a>` are lifetimes, not character literals: an identifier run
            // right after the quote that is not closed by another quote makes it a lifetime.
            if char_at(text, i + 1).is_alphanumeric() || char_at(text, i + 1) == '_' {
                let end = ident_end(text, i + 1);
                if char_at(text, end) != '\'' {
                    i = end;
                    continue;
                }
            }
            match literal_end(text, i + 1, '\'') {
                Some(hi) => {
                    let lo = i;
                    i = hi;
                    regions.push(Region {
                        kind: TextKind::Char,
                        lo,
                        hi,
                    });
                }
                // An unterminated character literal is unbalanced source; treat the quote as
                // punctuation rather than scanning to the end of the file.
                None => i += 1,
            }
            continue;
        }
        if current.is_alphanumeric() || current == '_' {
            let end = ident_end(text, i);
            regions.push(Region {
                kind: TextKind::Ident,
                lo: i,
                hi: end,
            });
            i = end;
            continue;
        }
        i += current.len_utf8();
    }
    regions
}

/// Delimit a string literal: `"..."`, `b"..."`, `r"..."`, `r#"..."#`, `br##"..."##`.
///
/// Returns the byte range including the prefix and the closing delimiter, or `None` when the
/// literal is unterminated.
fn literal_region(text: &str, current: char, _next: char, i: usize) -> Option<(usize, usize)> {
    // Everything the rule scans reduces to one case: either an ordinary string/char opening (`"`),
    // a C string (`c"..."`), a byte string (`b"..."`) or a raw string (`r"..."`,
    // `br#...#`, `cr"..."`, …). Detect the prefix letters (b/c/r in any of the orders rustc allows),
    // any `#`s, and then the opening quote.
    if current == '"' {
        return literal_end(text, i + 1, '"').map(|hi| (i, hi));
    }
    if !matches!(current, 'b' | 'c' | 'r') {
        return None;
    }
    let mut j = i;
    let mut saw_r = false;
    while j < text.len() && matches!(char_at(text, j), 'b' | 'c' | 'r') {
        if char_at(text, j) == 'r' {
            saw_r = true;
        }
        j += 1;
    }
    // `j` now sits on `#`s or directly on the opening quote.
    let hashes_start = j;
    while j < text.len() && char_at(text, j) == '#' {
        j += 1;
    }
    let hashes = j - hashes_start;
    if j >= text.len() || char_at(text, j) != '"' {
        return None; // a real identifier such as `bravo` or `result`, not a prefix
    }
    if saw_r {
        // Raw strings tolerate newlines and have no escapes, so `"` + hashes is the only terminator.
        raw_literal_end(text, j + 1, hashes).map(|hi| (i, hi))
    } else {
        // `b"..."` / `c"..."` behave like ordinary strings (with escapes and one closing quote).
        literal_end(text, j + 1, '"').map(|hi| (i, hi))
    }
}

/// Index right after the closing delimiter that opened this literal.
///
/// The delimiter is passed in: a string ends on `"` and a character literal on `'`, and sharing one
/// rule would end `"don't 中文"` at the apostrophe — the rest would then lex as code and report
/// Chinese as an identifier. A single byte after `\` is skipped; an invalid escape such as
/// `\u{1F600}` simply leaves its following characters to be walked one at a time until the closing
/// delimiter or a newline, so the literal is still consumed.
fn literal_end(text: &str, i: usize, delimiter: char) -> Option<usize> {
    let mut idx = i;
    while idx < text.len() {
        let current = char_at(text, idx);
        match current {
            '\\' => idx += 1 + char_at(text, idx + 1).len_utf8(),
            c if c == delimiter => return Some(idx + 1),
            '\n' => return None,
            _ => idx += current.len_utf8(),
        }
    }
    None
}
fn raw_literal_end(text: &str, i: usize, hashes: usize) -> Option<usize> {
    let close = format!("{}{}", '"', "#".repeat(hashes));
    text.get(i..)
        .and_then(|rest| rest.find(&close))
        .map(|offset| i + offset + close.len())
}

/// The byte index just past the end of a nested block comment whose first `*` sits at `i`.
fn block_comment_end(text: &str, i: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut idx = i;
    while idx < text.len() {
        let current = char_at(text, idx);
        let next = char_at(text, idx + current.len_utf8());
        if current == '*' && next == '/' {
            if depth == 1 {
                return Some(idx + 2);
            }
            depth -= 1;
            idx += 2;
        } else if current == '/' && next == '*' {
            depth += 1;
            idx += 2;
        } else {
            idx += current.len_utf8();
        }
    }
    None
}

/// The character at a byte offset, or `'\0'` past the end. `'\0'` matches no literal delimiter and
/// no identifier character, so a short read is always a no-op.
fn char_at(text: &str, index: usize) -> char {
    text.get(index..)
        .and_then(|rest| rest.chars().next())
        .unwrap_or('\0')
}

/// End of a run of identifier characters starting at `index`.
fn ident_end(text: &str, index: usize) -> usize {
    let mut idx = index;
    while idx < text.len() {
        let current = char_at(text, idx);
        if current.is_alphanumeric() || current == '_' {
            idx += current.len_utf8();
        } else {
            break;
        }
    }
    idx
}

/// Byte spans that are owned by another rule or that are not copy at all.
///
/// - Chinese inside an rsx element body is reported by `dioxus-rsx-cjk-text`; duplicating it here
///   would print the same line twice from two lints.
/// - rsx `data-*` attribute values are machine-readable markers, not copy.
/// - `include_str!` / `include_bytes!` arguments name a file; the file itself is not scanned.
/// - `#[cfg(test)]` / `#[test]` items are test scope, where Chinese strings are allowed.
fn exempt_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();

    let mut rsx: Vec<(usize, usize)> = Vec::new();
    for call in parse_rsx_calls(text) {
        walk_nodes(&call.body, &mut |node| match node {
            BodyNode::Text(node) => rsx.push(byte_range(text, node.input.span())),
            BodyNode::Element(element) => {
                for attribute in &element.raw_attributes {
                    if attribute_is_data(&attribute.name) {
                        rsx.push(byte_range(text, attribute.value.span()));
                    }
                }
            }
            _ => {}
        });
    }
    ranges.extend(rsx);

    /// Every item on the way down, so a `#[cfg(test)]` module exempts the functions inside it — not
    /// only the module's own strings.
    struct Scopes<'a> {
        text: &'a str,
        ranges: Vec<(usize, usize)>,
        scoped: Vec<bool>,
    }

    impl<'ast> Visit<'ast> for Scopes<'_> {
        fn visit_item(&mut self, item: &'ast Item) {
            let (lo, hi) = token_byte_range(self.text, item);
            let scoped = item_attrs(item).iter().any(is_test_meta)
                || self.scoped.last().copied().unwrap_or(false);
            if scoped {
                self.ranges.push((lo, hi));
            }
            self.scoped.push(scoped);
            syn::visit::visit_item(self, item);
            self.scoped.pop();
        }

        fn visit_macro(&mut self, macro_call: &'ast Macro) {
            if is_include_macro(&macro_call.path)
                && let Ok(path) = syn::parse2::<LitStr>(macro_call.tokens.clone())
            {
                self.ranges.push(byte_range(self.text, path.span()));
            }
            syn::visit::visit_macro(self, macro_call);
        }
    }

    if let Ok(file) = syn::parse_file(text) {
        let mut scopes = Scopes {
            text,
            ranges: Vec::new(),
            scoped: Vec::new(),
        };
        scopes.visit_file(&file);
        ranges.extend(scopes.ranges);
    }
    ranges
}

/// The test-scope attributes of `item`: `#[cfg(test)]`, `#[test]`, `#[tokio::test]`.
///
/// `syn::Item` has no `attrs()` accessor — the variants each carry their own — and ToTokens writes
/// an attribute list before an item's body, so the list is read back from the item's own token
/// stream.
fn item_attrs(item: &Item) -> Vec<Meta> {
    let mut iter = item.to_token_stream().into_iter();
    let mut attrs = Vec::new();
    loop {
        let Some(tokens) = next_attribute(&mut iter) else {
            break;
        };
        match syn::parse2::<Meta>(tokens) {
            Ok(meta) => attrs.push(meta),
            Err(_) => break,
        }
    }
    attrs
}

/// The contents of one attribute's brackets, or `None` once the stream is no longer a run of
/// attribute tokens.
fn next_attribute(
    iter: &mut impl Iterator<Item = proc_macro2::TokenTree>,
) -> Option<proc_macro2::TokenStream> {
    let Some(proc_macro2::TokenTree::Punct(punct)) = iter.next() else {
        return None;
    };
    if punct.as_char() != '#' {
        return None;
    }
    // `#![...]` is an inner attribute: the delimited group comes after the `!`.
    let group = match iter.next()? {
        proc_macro2::TokenTree::Group(group) => group,
        proc_macro2::TokenTree::Punct(bang) if bang.as_char() == '!' => match iter.next()? {
            proc_macro2::TokenTree::Group(group) => group,
            _ => return None,
        },
        _ => return None,
    };
    if group.delimiter() == proc_macro2::Delimiter::Bracket {
        Some(group.stream())
    } else {
        None
    }
}

/// `test`, `cfg(test)`, `cfg(all(test, feature = "x"))` and `cfg(any(test))`. `cfg(not(test))` is
/// not test scope.
/// `#[test]`, `#[tokio::test]` and `#[cfg(test)]` (possibly wrapped in `all`/`any`). `cfg(not(test))`
/// is not test scope.
fn is_test_meta(meta: &Meta) -> bool {
    if meta
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
    {
        return true;
    }
    if !meta.path().is_ident("cfg") {
        return false;
    }
    let Meta::List(list) = &meta else {
        return false;
    };
    // `to_string()` inserts spaces (`all ( test , .. )`), so compare compacted strings.
    let compact: String = list
        .tokens
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if compact.starts_with("not(") {
        return false;
    }
    has_bare_test(&compact, 0)
}

/// Does a cfg value contain a *positive* (un-negated) `test` condition?
fn has_bare_test(tokens: &str, depth: usize) -> bool {
    if depth > 8 || tokens.trim().is_empty() {
        return false;
    }
    if has_top_level_comma(tokens) {
        // `all(test, feature = "x")` is a number: `all(...)` is the ONLY element at depth 0,
        // but a bare `test` alongside another depth-0 group (`test, x`) should also count. OR
        // the comma pieces together.
        return split_top_level(tokens)
            .iter()
            .any(|part| has_bare_test(part.trim(), depth));
    }
    let trimmed = tokens.trim();
    if trimmed == "test" {
        return true;
    }
    // One wrapping group `all(...)` / `any(...)` (or a negation)...
    for prefix in ["all(", "any(", "not("] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            if let Some(inner) = strip_one_group(rest) {
                if prefix == "not(" {
                    return false; // a negated group never counts as test scope
                }
                return has_bare_test(inner, depth + 1);
            }
        }
    }
    false
}

/// `Some(inner)` when `group` is `name(` ... `)` with exactly one redundant pair.
fn strip_one_group(group: &str) -> Option<&str> {
    let group = group.trim();
    if !group.ends_with(')') {
        return None;
    }
    Some(&group[..group.len() - 1])
}

fn has_top_level_comma(tokens: &str) -> bool {
    split_top_level(tokens).len() > 1
}

fn split_top_level(tokens: &str) -> Vec<&str> {
    let mut depth: usize = 0;
    let mut in_str = false;
    let mut start = 0;
    let mut out = Vec::new();
    for (i, c) in tokens.char_indices() {
        if in_str {
            if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth = depth.saturating_sub(1);
        } else if c == ',' && depth == 0 {
            out.push(&tokens[start..i]);
            start = i + 1;
        }
    }
    out.push(&tokens[start..]);
    out
}

fn is_include_macro(path: &syn::Path) -> bool {
    path.segments.last().is_some_and(|segment| {
        matches!(
            segment.ident.to_string().as_str(),
            "include_str" | "include_bytes"
        )
    })
}

/// `data-testid: "x"` — a custom attribute, because the name has a hyphen — and `data_testid: "x"`.
fn attribute_is_data(name: &AttributeName) -> bool {
    match name {
        AttributeName::Custom(lit) => lit.value().starts_with("data-"),
        AttributeName::BuiltIn(ident) => {
            let name = ident.to_string();
            name.starts_with("data-") || name.starts_with("data_")
        }
        AttributeName::Spread(_) => false,
    }
}

fn overlaps(ranges: &[(usize, usize)], lo: usize, hi: usize) -> bool {
    ranges.iter().any(|(start, end)| *start < hi && *end > lo)
}

fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let file = normalized.rsplit('/').next().unwrap_or(&normalized);
    normalized.contains("/tests/")
        || normalized.ends_with("/tests")
        || file.ends_with("_test.rs")
        || file.starts_with("test_")
}

/// i18n / copy-data crates: their strings are locale content, and their comments are still English.
fn is_data_crate(crate_name: &str) -> bool {
    const KEYS: [&str; 2] = ["i18n", "locale"];
    let normalized = crate_name.replace('-', "_");
    KEYS.iter().any(|key| normalized.contains(key))
}

/// Data directories hold corpora (prompts, translated tables, fixtures), not code copy.
fn is_data_path(path: &str) -> bool {
    ["/locales/", "/i18n/", "/assets/", "/data/", "/prompts/"]
        .iter()
        .any(|segment| path.contains(segment))
}

fn is_data_source(ctx: &Ctx<'_>) -> bool {
    is_data_crate(ctx.crate_name) || is_data_path(ctx.path)
}

/// A `SourceFile` span must stay inside the file: a byte offset past its end is what makes rustc
/// panic, and the panic then aborts the whole crate's lint pass.
fn span_from_bytes(file: &SourceFile, text: &str, lo: usize, hi: usize) -> Span {
    let total = text.len();
    let hi = hi.max(lo + 1).min(total);
    let lo = lo.min(hi);
    Span::with_root_ctxt(
        file.start_pos + BytePos(lo as u32),
        file.start_pos + BytePos(hi as u32),
    )
}

/// CJK ranges, the same ones `dioxus-rsx-cjk-text` uses.
fn has_cjk(text: &str) -> bool {
    text.chars().any(is_cjk)
}

fn is_cjk(current: char) -> bool {
    matches!(
        current as u32,
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF
    )
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Ctx<'static> {
        Ctx {
            crate_name: "sample_crate",
            path: "src/sample.rs",
        }
    }

    fn ctx(crate_name: &'static str, path: &'static str) -> Ctx<'static> {
        Ctx { crate_name, path }
    }

    #[test]
    fn clean_source_is_silent() {
        assert!(
            scan(
                "// Render the save branch.
fn save(label: &str) { println!(\"{label}\"); }
",
                &sample()
            )
            .is_empty()
        );
    }

    #[test]
    fn ascii_source_short_circuits() {
        assert!(scan("fn save(label: &str) { println!(\"{label}\"); }", &sample()).is_empty());
    }

    #[test]
    fn line_doc_and_nested_block_comments_are_flagged() {
        let source = r#"
// 行内注释
/// 文档注释
//! crate 注释
/* 块注释
   // 块里 // 不是注释
   /* 嵌套 */
*/
fn keep() {}
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 4, "{hits:?}");
        assert!(hits.iter().all(|hit| hit.msg.contains("comment")));
    }

    #[test]
    fn comments_in_test_scope_are_still_flagged() {
        let source = r#"
#[cfg(test)]
mod tests {
    // 断言文案与 locales/zh.yaml 一致
    #[test]
    fn renders_the_label() {
        let copy = "保存";
        assert_eq!(copy, "保存");
    }
}
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("comment"));
    }

    #[test]
    fn strings_in_cfg_test_item_are_allowed() {
        let source = r#"
#[cfg(test)]
mod tests {
    fn fixture() -> &'static str { "保存" }
    fn other() { let copy = format!("{copy} 按钮"); }
    pub fn public_api() -> u32 { 1 }
}
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn strings_in_test_attribute_function_are_allowed() {
        let source = r#"
fn main() {
    #[test]
    fn chinese_fixture() {
        let save: &str = "保存";
        assert_eq!(save, "保存");
    }
}
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn cfg_all_test_grouping_is_test_scope() {
        let source = r#"
#[cfg(all(test, feature = "fixtures"))]
mod tests {
    fn fixture() { let copy = "保存"; }
}
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn cfg_not_test_is_not_test_scope() {
        let source = r#"
#[cfg(not(test))]
fn production() { let copy = "保存"; }
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("string literal"));
    }

    #[test]
    fn strings_in_a_test_file_are_allowed() {
        assert!(
            scan(
                "fn f() { let copy = \"保存\"; }",
                &ctx("page_chat", "ui/page-chat/tests/save/copy.rs")
            )
            .is_empty()
        );
        assert!(
            scan(
                "fn f() { let copy = \"保存\"; }",
                &ctx("page_chat", "src/save_test.rs")
            )
            .is_empty()
        );
        assert!(
            scan(
                "fn f() { let copy = \"保存\"; }",
                &ctx("page_chat", "src/test_helpers.rs")
            )
            .is_empty()
        );
    }

    #[test]
    fn strings_outside_test_scope_are_flagged() {
        let source = r#"fn f() { let save = "保存按钮"; let _ = save; }
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("string literal"));
    }

    #[test]
    fn character_literals_are_flagged() {
        let hits = scan("fn f() { let c = '中'; let _ = c; }\n", &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("character"));
    }

    #[test]
    fn identifiers_are_flagged() {
        let hits = scan("fn 保存按钮(count: u32) -> u32 { count }\n", &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("identifier"));
    }

    #[test]
    fn lifetime_quotes_are_not_character_literals() {
        let source = r#"
fn render<'a>(value: &'a str) -> &'a str { value }
fn take<'t>(thing: &'static str, other: u32) -> u32 { let _ = thing; other }
fn main() { let _ = "ascii"; }
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn cjk_url_is_a_string_not_a_comment() {
        let source = "fn f() { let url = \"https://例子.com/路径\"; let _ = url; }\n";
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("string literal"));
    }

    #[test]
    fn slashes_inside_ascii_strings_are_not_comments() {
        let source = r#"fn f() { let url = "https://example.com/a"; let _ = url; }
// 这是注释
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("comment"));
    }

    #[test]
    fn raw_and_byte_strings_are_literals() {
        let source = "fn f() {\n    let raw = r#\"容器名\"#;\n    let byte = b\"标签\";\n}\n";
        assert_eq!(scan(source, &sample()).len(), 2);
    }

    #[test]
    fn escapes_do_not_end_a_literal() {
        let source = "fn f() { let s = \"中文\\\"引号\\\"; 完成\"; let _ = s; }\n";
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        // The escaped quotes are part of the same literal, so the two CJK runs are one finding.
        assert!(hits[0].msg.contains("string literal"));
    }

    #[test]
    fn apostrophe_in_a_string_does_not_end_it() {
        // An apostrophe splits `"中文 don't 中文"` in two, and the part after it would otherwise be
        // lexed as code — reported as an identifier, with a span that does not cover the literal.
        let source = "fn f() { let s = \"中文 don't 中文\"; let _ = s; }\n";
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("string literal"));
        assert_eq!(&source[hits[0].lo..hits[0].hi], "\"中文 don't 中文\"");
    }

    #[test]
    fn apostrophe_before_the_chinese_does_not_end_it() {
        let source = "fn f() { let s = \"don't 中文\"; let _ = s; }\n";
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("string literal"));
    }

    #[test]
    fn block_comment_is_closed_and_code_after_it_is_scanned() {
        let source = "/* 块注释\n */\nfn f() { let s = \"中文\"; }\n";
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert!(hits[0].msg.contains("comment"));
        assert!(hits[1].msg.contains("string literal"));
    }

    #[test]
    fn include_str_arguments_are_exempt() {
        let source = r#"fn f() { let s = include_str!("中文.txt"); let _ = s; }
"#;
        assert!(scan(source, &sample()).is_empty());
        let source = r#"fn f() { let s = include_bytes!("中文.txt"); let _ = s; }
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn include_str_argument_does_not_swallow_the_next_literal() {
        let source = r#"fn f() {
    let s = include_str!("a.txt");
    let copy = "保存";
    let _ = s;
}
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("string literal"));
    }

    #[test]
    fn rsx_text_nodes_belong_to_dioxus_rsx_cjk_text() {
        let source = r#"fn f() {
    rsx! { span { "保存" } }
    rsx! { div { "data-testid": "保存按钮" } }
}
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn rsx_data_attribute_value_is_exempt() {
        let source = r#"fn f() {
    rsx! { div { "data-testid": "保存-{id}" } }
}
"#;
        assert!(scan(source, &sample()).is_empty());
    }

    #[test]
    fn rsx_comment_next_to_text_is_still_a_comment() {
        let source = r#"fn f() {
    rsx! { span { "保存" } } // 保存按钮文案
}
"#;
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("comment"));
    }

    #[test]
    fn data_crates_exempt_strings_but_not_comments() {
        assert!(
            scan(
                "fn f() { let name = \"中文\"; }",
                &ctx("i18n", "src/lib.rs")
            )
            .is_empty()
        );
        let source = r#"// 语言切换
fn f() { let name = "中文"; }
"#;
        let hits = scan(source, &ctx("i18n", "src/lib.rs"));
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].msg.contains("comment"));
    }

    #[test]
    fn data_directories_exempt_strings() {
        assert!(
            scan(
                "fn f() { let n = \"中文\"; }",
                &ctx("page_chat", "src/locales/zh.rs")
            )
            .is_empty()
        );
        assert!(
            scan(
                "fn f() { let n = \"中文\"; }",
                &ctx("harness_tools", "src/prompts/assets/preset.rs")
            )
            .is_empty()
        );
        assert_eq!(
            scan(
                "fn f() { let n = \"中文\"; }",
                &ctx("page_chat", "src/zh.rs")
            )
            .len(),
            1
        );
    }

    #[test]
    fn findings_are_in_byte_order() {
        let hits = scan("fn f() { let a = 串; let b = \"串\"; }", &sample());
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert!(hits[0].lo < hits[1].lo, "{hits:?}");
        assert!(hits[0].hi <= hits[1].lo, "{hits:?}");
    }

    #[test]
    fn comment_span_covers_only_the_comment() {
        let source = "// 文案\nfn f() { }\n";
        let hits = scan(source, &sample());
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(&source[hits[0].lo..hits[0].hi], "// 文案");
    }
}
