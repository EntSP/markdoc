//! Resolve CommonMark footnotes into the canonical `{% footnote %}` tag.
//!
//! pulldown-cmark parses `text[^id]` references and `[^id]: body` definitions
//! (footnotes are enabled in the tokenizer), which the parser preserves as
//! `footnote-ref` / `footnote-def` tag nodes. This pass matches each
//! reference to its definition, rewrites the reference into a `footnote` tag
//! whose children are the definition's body, and drops the definition blocks.
//!
//! The upshot: writers may use either the `{% footnote %}` tag or CommonMark
//! `[^id]` syntax, and both reach every renderer that understands
//! `{% footnote %}` — no renderer changes needed. Runs between `parse` (or
//! partial expansion) and `transform`, like [`crate::resolve_crossrefs`].

use crate::ast::Node;
use crate::types::*;
use std::collections::HashMap;

/// Rewrite CommonMark footnote reference / definition nodes into
/// `{% footnote %}` tags. A reference with no matching definition is dropped;
/// documents with no footnotes are returned structurally unchanged.
pub fn resolve_footnotes(node: &Node) -> Node {
    let mut defs: HashMap<String, Vec<Node>> = HashMap::new();
    collect_defs(node, &mut defs);
    // The top document node is never a footnote node, so `rewrite` returns
    // exactly one node here.
    rewrite(node, &defs)
        .into_iter()
        .next()
        .unwrap_or_else(|| node.clone())
}

/// The `name` label stored on a `footnote-ref` / `footnote-def` node.
fn footnote_name(node: &Node) -> Option<String> {
    match node.attributes.get("name") {
        Some(Scalar::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Index every `footnote-def` body by its label.
fn collect_defs(node: &Node, defs: &mut HashMap<String, Vec<Node>>) {
    if node.tag.as_deref() == Some("footnote-def")
        && let Some(name) = footnote_name(node)
    {
        defs.insert(name, node.children.clone());
    }
    for child in &node.children {
        collect_defs(child, defs);
    }
}

/// Walk `node`, returning its replacement(s): definitions vanish, references
/// become `footnote` tags carrying the matched body, everything else is
/// cloned with its children recursively rewritten.
fn rewrite(node: &Node, defs: &HashMap<String, Vec<Node>>) -> Vec<Node> {
    match node.tag.as_deref() {
        Some("footnote-def") => Vec::new(),
        Some("footnote-ref") => {
            let body = footnote_name(node)
                .and_then(|n| defs.get(&n).cloned())
                .unwrap_or_default();
            let mut fnote = Node::new(NodeType::Tag, HashMap::new(), body, Some("footnote".into()));
            fnote.inline = true;
            vec![fnote]
        }
        _ => {
            let mut n = node.clone();
            n.children = node
                .children
                .iter()
                .flat_map(|c| rewrite(c, defs))
                .collect();
            vec![n]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn find_tag<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
        if node.tag.as_deref() == Some(name) {
            return Some(node);
        }
        node.children.iter().find_map(|c| find_tag(c, name))
    }

    fn count_tag(node: &Node, name: &str) -> usize {
        let here = usize::from(node.tag.as_deref() == Some(name));
        here + node
            .children
            .iter()
            .map(|c| count_tag(c, name))
            .sum::<usize>()
    }

    fn text_of(node: &Node) -> String {
        let mut s = String::new();
        if let Some(Scalar::String(t)) = node.attributes.get("content") {
            s.push_str(t);
        }
        for c in &node.children {
            s.push_str(&text_of(c));
        }
        s
    }

    #[test]
    fn commonmark_footnote_becomes_footnote_tag() {
        let doc = parse("Text with a ref.[^a]\n\n[^a]: The footnote body.\n", None).unwrap();
        // Before resolution: a ref node and a def node exist.
        assert_eq!(count_tag(&doc, "footnote-ref"), 1);
        assert_eq!(count_tag(&doc, "footnote-def"), 1);

        let resolved = resolve_footnotes(&doc);
        // After: the ref is a `footnote` tag, the def is gone.
        assert_eq!(count_tag(&resolved, "footnote-ref"), 0);
        assert_eq!(count_tag(&resolved, "footnote-def"), 0);
        let fnote = find_tag(&resolved, "footnote").expect("footnote tag present");
        assert!(text_of(fnote).contains("The footnote body."));
    }

    #[test]
    fn dangling_reference_stays_literal() {
        // pulldown only treats `[^id]` as a footnote when a matching
        // definition exists, so a lone reference is left as plain text and
        // never becomes a footnote node.
        let doc = parse("A ref with no def.[^missing]\n", None).unwrap();
        assert_eq!(count_tag(&doc, "footnote-ref"), 0);
        let resolved = resolve_footnotes(&doc);
        assert_eq!(count_tag(&resolved, "footnote"), 0);
    }

    #[test]
    fn no_footnotes_is_a_noop() {
        let doc = parse("# Title\n\nJust prose, no footnotes.\n", None).unwrap();
        let resolved = resolve_footnotes(&doc);
        assert_eq!(count_tag(&resolved, "footnote"), 0);
        assert_eq!(count_tag(&resolved, "footnote-ref"), 0);
    }
}
