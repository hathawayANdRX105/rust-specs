#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_span;

use dioxus_rsx::{BodyNode, CallBody, TemplateBody};
use rustc_span::{BytePos, SourceFile, Span};
use syn::visit::Visit;

/// One finding produced from source text.
///
/// Lines are 1-based and columns are 0-based, matching `proc_macro2`'s
/// `span-locations`. The end position is carried explicitly: deriving the span
/// end from a guessed byte length (`lo + len`) overshoots the real token —
/// `dioxus-rsx-cjk-text` used to pass the length of the *rendered* token
/// string, which pushed `hi` hundreds of bytes past the literal and made rustc
/// panic (`assertion failed: bpos >= mbc.pos + mbc.bytes`), aborting the whole
/// crate's lint pass.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Finding {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub msg: String,
}

/// Parse every `rsx!` macro invocation in a Rust source file.
pub fn parse_rsx_calls(text: &str) -> Vec<CallBody> {
    let Ok(file) = syn::parse_file(text) else {
        return Vec::new();
    };
    let mut visitor = RsxVisitor { calls: Vec::new() };
    visitor.visit_file(&file);
    visitor.calls
}

struct RsxVisitor {
    calls: Vec<CallBody>,
}

impl<'ast> Visit<'ast> for RsxVisitor {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let is_rsx = mac
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "rsx");
        if is_rsx && let Ok(body) = syn::parse2::<CallBody>(mac.tokens.clone()) {
            self.calls.push(body);
        }
        syn::visit::visit_macro(self, mac);
    }
}

/// Depth-first walk over every rsx node, including branches of `if`/`for`.
/// `for` is a container like any other; callers decide whether it is allowed.
pub fn walk_nodes(body: &TemplateBody, f: &mut impl FnMut(&BodyNode)) {
    for node in &body.roots {
        walk_node(node, f);
    }
}

fn walk_node(node: &BodyNode, f: &mut impl FnMut(&BodyNode)) {
    f(node);
    match node {
        BodyNode::Element(element) => {
            for child in &element.children {
                walk_node(child, f);
            }
        }
        BodyNode::Component(component) => walk_nodes(&component.children, f),
        BodyNode::ForLoop(looped) => walk_nodes(&looped.body, f),
        BodyNode::IfChain(chain) => {
            chain.for_each_branch(&mut |branch| walk_nodes(branch, f));
        }
        BodyNode::RawExpr(_) | BodyNode::Text(_) => {}
    }
}

pub fn finding(span: proc_macro2::Span, msg: String) -> Finding {
    let start = span.start();
    let end = span.end();
    Finding {
        line: start.line,
        column: start.column,
        end_line: end.line,
        end_column: end.column,
        msg,
    }
}

/// Byte offset of the start of `line` (1-based) plus `column` characters.
///
/// `proc_macro2::Span::start()` reports its column in **characters**, not bytes,
/// so it must be converted before it can index a `str`. Adding it directly
/// makes the offset land inside a multi-byte character on any line that holds
/// non-ASCII text (Chinese rsx bodies), and rustc then panics on the resulting
/// span (`assertion failed: bpos >= mbc.pos + mbc.bytes`), which aborts the
/// whole crate's lint pass and silently drops every later finding.
fn byte_offset(text: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (index, current) in text.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let line = current.trim_end_matches('\n');
            return offset + char_column_to_byte(line, column);
        }
        offset += current.len();
    }
    offset
}

/// Byte offset of the `column`-th (0-based) character of `line`, clamped to the
/// end of the line and always on a char boundary.
fn char_column_to_byte(line: &str, column: usize) -> usize {
    let mut byte_col = line.len();
    for (count, (byte_idx, _)) in line.char_indices().enumerate() {
        if count >= column {
            byte_col = byte_idx;
            break;
        }
    }
    byte_col
}

pub fn span_for(file: &SourceFile, text: &str, finding: &Finding) -> Span {
    // Both ends are clamped to the text we scanned: a span that runs past the
    // end of the file is what makes rustc assert, and a closed (empty) span is
    // not a diagnostic rustc can point at.
    let text_len = text.len();
    let lo_off = byte_offset(text, finding.line, finding.column).min(text_len);
    let hi_off = byte_offset(text, finding.end_line, finding.end_column)
        .max(lo_off + 1)
        .min(text_len.max(lo_off + 1));
    let lo = file.start_pos + BytePos(lo_off as u32);
    let hi = file.start_pos + BytePos(hi_off as u32);
    Span::with_root_ctxt(lo, hi)
}
