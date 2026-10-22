#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_span;

use dioxus_rsx::{BodyNode, CallBody, TemplateBody};
use rustc_span::{BytePos, SourceFile, Span};
use syn::visit::Visit;

/// One finding produced from source text (line/column are 1-based line and
/// 0-based column, matching `proc_macro2`'s span-locations).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Finding {
    pub line: usize,
    pub column: usize,
    pub len: usize,
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

pub fn finding(span: proc_macro2::Span, len: usize, msg: &str) -> Finding {
    let start = span.start();
    Finding {
        line: start.line,
        column: start.column,
        len,
        msg: msg.to_string(),
    }
}

fn byte_offset(text: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (index, current) in text.split_inclusive('\n').enumerate() {
        if index + 1 == line {
            let line_len = current.trim_end_matches('\n').len();
            return offset + column.min(line_len);
        }
        offset += current.len();
    }
    offset
}

pub fn span_for(file: &SourceFile, text: &str, finding: &Finding) -> Span {
    let offset = byte_offset(text, finding.line, finding.column);
    let lo = file.start_pos + BytePos(offset as u32);
    let hi = lo + BytePos(finding.len.max(1) as u32);
    Span::with_root_ctxt(lo, hi)
}
