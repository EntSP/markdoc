//! Partial inclusion for Markdoc documents.
//!
//! Walks a parsed `Node` tree and replaces every `{% partial file="..." /%}`
//! tag with the contents of the referenced document. Partials are loaded
//! through a [`PartialResolver`] (defaults: filesystem and in-memory). Cycles
//! in the include graph are detected and reported with the offending chain.
//!
//! This runs **between** `parse` and `transform`:
//! ```ignore
//! let doc = markdoc::parse(&source, None)?;
//! let resolver = markdoc::partials::FsPartialResolver::new("./docs");
//! let expanded = markdoc::partials::expand_partials(&doc, &resolver)?;
//! let rendered = markdoc::transform(&expanded, &Config::default())?;
//! ```
//!
//! Splicing is structural: the partial's parsed `Document` node is unwrapped
//! and its children are inserted at the position of the partial tag in the
//! including document. Frontmatter on a partial is dropped — partials inherit
//! their evaluation context from the including document.

use crate::ast::Node;
use crate::parser::parse;
use crate::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pluggable source for partial files.
///
/// `canonicalize` returns the identity used for cycle detection. The default
/// implementation returns the path unchanged, which is fine for in-memory
/// resolvers; filesystem resolvers should resolve to absolute paths so that
/// `./b.markdoc` and `b.markdoc` are recognised as the same file.
pub trait PartialResolver: Send + Sync {
    fn load(&self, path: &Path) -> Result<String>;

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        Ok(path.to_path_buf())
    }
}

/// Filesystem-backed resolver. All `file=` paths in `{% partial %}` tags
/// are interpreted relative to `root`.
pub struct FsPartialResolver {
    pub root: PathBuf,
}

impl FsPartialResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl PartialResolver for FsPartialResolver {
    fn load(&self, path: &Path) -> Result<String> {
        let full = self.root.join(path);
        std::fs::read_to_string(&full).map_err(|e| {
            MarkdocError::TransformError(format!(
                "Failed to read partial {:?}: {e}",
                full.display()
            ))
        })
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let full = self.root.join(path);
        std::fs::canonicalize(&full).map_err(|e| {
            MarkdocError::TransformError(format!(
                "Failed to canonicalize partial path {:?}: {e}",
                full.display()
            ))
        })
    }
}

/// In-memory resolver, primarily for tests but also useful for callers
/// that have partial sources already loaded (e.g. from a database).
#[derive(Debug, Default)]
pub struct InMemoryPartialResolver {
    pub files: HashMap<PathBuf, String>,
}

impl InMemoryPartialResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.insert(path.into(), content.into());
        self
    }
}

impl PartialResolver for InMemoryPartialResolver {
    fn load(&self, path: &Path) -> Result<String> {
        self.files.get(path).cloned().ok_or_else(|| {
            MarkdocError::TransformError(format!("Partial not found: {}", path.display()))
        })
    }
}

/// Walk `node` recursively, replacing every `{% partial file="..." /%}` tag
/// with the parsed contents of the referenced file. Detects cycles in the
/// include graph and reports the offending chain.
pub fn expand_partials(node: &Node, resolver: &dyn PartialResolver) -> Result<Node> {
    let mut chain: Vec<PathBuf> = Vec::new();
    let expanded = expand_node(node, resolver, &mut chain)?;
    // The recursion returns Vec<Node> to allow splicing at partial positions.
    // At the top level we expect exactly one Document node; if a top-level
    // partial somehow expanded to multiple siblings, wrap them in a
    // synthetic Document to preserve the contract of the function.
    if expanded.len() == 1 {
        Ok(expanded.into_iter().next().unwrap())
    } else {
        Ok(Node::new(
            NodeType::Document,
            HashMap::new(),
            expanded,
            None,
        ))
    }
}

fn expand_node(
    node: &Node,
    resolver: &dyn PartialResolver,
    chain: &mut Vec<PathBuf>,
) -> Result<Vec<Node>> {
    if is_partial_tag(node) {
        return expand_partial_tag(node, resolver, chain);
    }
    let mut new_children = Vec::with_capacity(node.children.len());
    for child in &node.children {
        new_children.extend(expand_node(child, resolver, chain)?);
    }
    let mut new_node = node.clone();
    new_node.children = new_children;
    Ok(vec![new_node])
}

fn is_partial_tag(node: &Node) -> bool {
    matches!(node.node_type, NodeType::Tag) && node.tag.as_deref() == Some("partial")
}

fn expand_partial_tag(
    node: &Node,
    resolver: &dyn PartialResolver,
    chain: &mut Vec<PathBuf>,
) -> Result<Vec<Node>> {
    let file = node
        .attributes
        .get("file")
        .and_then(|v| match v {
            Scalar::String(s) => Some(s.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            MarkdocError::TransformError("{% partial %} requires a string `file` attribute".into())
        })?;

    let path = PathBuf::from(&file);
    let canon = resolver.canonicalize(&path)?;

    if chain.contains(&canon) {
        let mut display: Vec<String> = chain.iter().map(|p| p.display().to_string()).collect();
        display.push(canon.display().to_string());
        return Err(MarkdocError::TransformError(format!(
            "Partial inclusion cycle detected: {}",
            display.join(" -> ")
        )));
    }

    let source = resolver.load(&path)?;
    let subdoc = parse(&source, None)?;

    chain.push(canon);
    let expanded = expand_node(&subdoc, resolver, chain)?;
    chain.pop();

    // Splice the subdoc's CHILDREN (not the Document wrapper) at this
    // position so the partial seamlessly merges into the parent's
    // children list.
    let mut spliced = Vec::new();
    for n in expanded {
        if matches!(n.node_type, NodeType::Document) {
            spliced.extend(n.children);
        } else {
            spliced.push(n);
        }
    }
    Ok(spliced)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_text(node: &Node, needle: &str) -> usize {
        let mut n = 0;
        if matches!(node.node_type, NodeType::Text)
            && let Some(Scalar::String(s)) = node.attributes.get("content")
            && s.contains(needle)
        {
            n += 1;
        }
        for child in &node.children {
            n += count_text(child, needle);
        }
        n
    }

    fn has_text(node: &Node, needle: &str) -> bool {
        count_text(node, needle) > 0
    }

    fn has_tag(node: &Node, name: &str) -> bool {
        if matches!(node.node_type, NodeType::Tag) && node.tag.as_deref() == Some(name) {
            return true;
        }
        node.children.iter().any(|c| has_tag(c, name))
    }

    #[test]
    fn inlines_simple_partial() {
        let resolver = InMemoryPartialResolver::new().with("b.markdoc", "Hello from partial B");
        let src = "Before\n\n{% partial file=\"b.markdoc\" /%}\n\nAfter";
        let doc = parse(src, None).unwrap();
        let expanded = expand_partials(&doc, &resolver).unwrap();

        assert!(has_text(&expanded, "Hello from partial B"));
        assert!(has_text(&expanded, "Before"));
        assert!(has_text(&expanded, "After"));
        // Partial tag must no longer appear after expansion.
        assert!(!has_tag(&expanded, "partial"));
    }

    #[test]
    fn detects_cycle_self_reference() {
        let resolver = InMemoryPartialResolver::new()
            .with("a.markdoc", "Recursive: {% partial file=\"a.markdoc\" /%}");
        let src = "{% partial file=\"a.markdoc\" /%}";
        let doc = parse(src, None).unwrap();
        let err = expand_partials(&doc, &resolver).unwrap_err();
        match err {
            MarkdocError::TransformError(msg) => {
                assert!(msg.contains("cycle"), "got message: {msg}");
                assert!(msg.contains("a.markdoc"));
            }
            other => panic!("expected TransformError, got {other:?}"),
        }
    }

    #[test]
    fn detects_cycle_indirect() {
        let resolver = InMemoryPartialResolver::new()
            .with("a.markdoc", "A: {% partial file=\"b.markdoc\" /%}")
            .with("b.markdoc", "B: {% partial file=\"a.markdoc\" /%}");
        let src = "{% partial file=\"a.markdoc\" /%}";
        let doc = parse(src, None).unwrap();
        let err = expand_partials(&doc, &resolver).unwrap_err();
        match err {
            MarkdocError::TransformError(msg) => {
                assert!(msg.contains("cycle"));
                assert!(msg.contains("a.markdoc"));
                assert!(msg.contains("b.markdoc"));
            }
            other => panic!("expected TransformError, got {other:?}"),
        }
    }

    #[test]
    fn missing_partial_errors_with_path() {
        let resolver = InMemoryPartialResolver::new();
        let src = "{% partial file=\"missing.markdoc\" /%}";
        let doc = parse(src, None).unwrap();
        let err = expand_partials(&doc, &resolver).unwrap_err();
        match err {
            MarkdocError::TransformError(msg) => assert!(msg.contains("missing.markdoc")),
            other => panic!("expected TransformError, got {other:?}"),
        }
    }

    #[test]
    fn missing_file_attr_errors() {
        let resolver = InMemoryPartialResolver::new();
        let src = "{% partial /%}";
        let doc = parse(src, None).unwrap();
        let err = expand_partials(&doc, &resolver).unwrap_err();
        match err {
            MarkdocError::TransformError(msg) => assert!(msg.contains("file")),
            other => panic!("expected TransformError, got {other:?}"),
        }
    }

    #[test]
    fn nested_non_cyclic_chain_works() {
        // A → B → C, no cycles.
        let resolver = InMemoryPartialResolver::new()
            .with("a.markdoc", "A: {% partial file=\"b.markdoc\" /%}")
            .with("b.markdoc", "B: {% partial file=\"c.markdoc\" /%}")
            .with("c.markdoc", "C-content");
        let src = "Top: {% partial file=\"a.markdoc\" /%}";
        let doc = parse(src, None).unwrap();
        let expanded = expand_partials(&doc, &resolver).unwrap();

        assert!(has_text(&expanded, "Top:"));
        assert!(has_text(&expanded, "A:"));
        assert!(has_text(&expanded, "B:"));
        assert!(has_text(&expanded, "C-content"));
        assert!(!has_tag(&expanded, "partial"));
    }

    #[test]
    fn same_partial_twice_is_not_a_cycle() {
        let resolver = InMemoryPartialResolver::new().with("b.markdoc", "Repeated content");
        let src =
            "{% partial file=\"b.markdoc\" /%}\n\nMiddle\n\n{% partial file=\"b.markdoc\" /%}";
        let doc = parse(src, None).unwrap();
        let expanded = expand_partials(&doc, &resolver).unwrap();
        assert_eq!(
            count_text(&expanded, "Repeated content"),
            2,
            "expected 2 inclusions, got {}",
            count_text(&expanded, "Repeated content")
        );
    }

    #[test]
    fn partial_frontmatter_is_dropped() {
        // The partial's frontmatter (which would otherwise appear as a
        // `frontmatter` attribute on its Document node) must not leak into
        // the including document; we only splice the partial's children.
        let resolver = InMemoryPartialResolver::new().with(
            "with_fm.markdoc",
            "---\ntitle: Inner Title\n---\n\nBody from inner.",
        );
        let src = "---\ntitle: Outer Title\n---\n\n{% partial file=\"with_fm.markdoc\" /%}";
        let doc = parse(src, None).unwrap();
        let expanded = expand_partials(&doc, &resolver).unwrap();

        // Outer frontmatter survives on the top Document node.
        match expanded.attributes.get("frontmatter") {
            Some(Scalar::Object(fm)) => match fm.get("title") {
                Some(Scalar::String(s)) => assert_eq!(s, "Outer Title"),
                _ => panic!("outer title missing"),
            },
            _ => panic!("outer frontmatter missing"),
        }
        // Inner partial's body is present.
        assert!(has_text(&expanded, "Body from inner."));
        // No nested Document node was spliced in.
        for child in &expanded.children {
            assert!(
                !matches!(child.node_type, NodeType::Document),
                "Document node should not appear among children"
            );
        }
    }
}
