//! Markdoc list-syntax `{% table %}` support.
//!
//! markdoc segments `{% %}` tags and hands the rest to pulldown-cmark, which
//! mangles a list-table's `---` row separators (into thematic breaks / setext
//! headings) and `*` cells. So we intercept `{% table %}` blocks *before*
//! tokenization: [`extract_list_tables`] parses each block's raw inner text
//! into a proper `Table` node and leaves a placeholder sentinel in its place,
//! which [`splice_list_tables`] swaps back once the surrounding document has
//! been built.
//!
//! Rows are separated by top-level `---`; each `* item` in a row is a cell,
//! parsed recursively as Markdown so cells may hold rich content (paragraphs,
//! code, lists). The first row is the header unless the table starts with
//! `---`.

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::ast::Node;
use crate::types::{NodeType, Scalar};

/// Placeholder sentinels for an extracted table, distinct from the tag
/// sentinels in `tag_parser`: `\u{E002}<index>\u{E003}`.
const TABLE_OPEN: char = '\u{E002}';
const TABLE_CLOSE: char = '\u{E003}';

/// One parsed table cell: its content (the children of its `Td`/`Th`) plus
/// any `colspan` / `rowspan` / `align` annotations stripped from its source.
struct Cell {
    content: Vec<Node>,
    attrs: HashMap<String, Scalar>,
}
/// One row: its cells, in column order.
type Row = Vec<Cell>;

fn open_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // `{% table %}` / `{% table attrs %}` — an opening tag (not `/table`).
    RE.get_or_init(|| Regex::new(r"\{%\s*table\b[^%]*%\}").unwrap())
}

fn close_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{%\s*/\s*table\s*%\}").unwrap())
}

/// Does a `{% table %}` block use the Markdoc list-syntax (its first
/// non-blank line is a `*` cell or a `---` separator) rather than wrapping a
/// pipe table (which starts with `|`)? Only list-syntax blocks are parsed
/// here; pipe-table wrappers fall through to the normal tag pipeline.
fn is_list_syntax(inner: &str) -> bool {
    for line in inner.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        return t == "*" || t == "---" || t.starts_with("* ") || t.starts_with("*\t");
    }
    false
}

/// Replace each `{% table %}…{% /table %}` block with a placeholder sentinel
/// and return the parsed `Table` nodes (indexed by the sentinel). The inner
/// text is parsed before pulldown-cmark sees it, so `---`/`*` survive.
pub fn extract_list_tables(content: &str) -> (String, Vec<Node>) {
    let mut out = String::with_capacity(content.len());
    let mut tables: Vec<Node> = Vec::new();
    let mut pos = 0;
    while let Some(m) = open_re().find(&content[pos..]) {
        let open_start = pos + m.start();
        let open_end = pos + m.end();
        match close_re().find(&content[open_end..]) {
            Some(c) => {
                let close_end = open_end + c.end();
                let inner = &content[open_end..open_end + c.start()];
                if is_list_syntax(inner) {
                    out.push_str(&content[pos..open_start]);
                    let idx = tables.len();
                    tables.push(parse_list_table(inner));
                    out.push(TABLE_OPEN);
                    out.push_str(&idx.to_string());
                    out.push(TABLE_CLOSE);
                } else {
                    // Not list-syntax (e.g. a pipe table wrapped in
                    // `{% table … %}` for styling) — leave it verbatim for
                    // the normal tag pipeline + pipe-table renderer.
                    out.push_str(&content[pos..close_end]);
                }
                pos = close_end;
            }
            None => {
                // Unbalanced open — leave it verbatim and stop scanning.
                out.push_str(&content[pos..open_end]);
                pos = open_end;
            }
        }
    }
    out.push_str(&content[pos..]);
    (out, tables)
}

/// Walk `node` and replace each placeholder (a paragraph or text node holding
/// a `\u{E002}N\u{E003}` sentinel) with `tables[N]`.
pub fn splice_list_tables(node: &mut Node, tables: &[Node]) {
    let mut i = 0;
    while i < node.children.len() {
        if let Some(idx) = placeholder_index(&node.children[i]) {
            if let Some(table) = tables.get(idx) {
                node.children[i] = table.clone();
            }
        } else {
            splice_list_tables(&mut node.children[i], tables);
        }
        i += 1;
    }
}

/// If `node` is a placeholder (a paragraph wrapping a single placeholder text,
/// or a placeholder text node), return its table index.
fn placeholder_index(node: &Node) -> Option<usize> {
    let text = match &node.node_type {
        NodeType::Paragraph if node.children.len() == 1 => text_content(&node.children[0])?,
        NodeType::Text => text_content(node)?,
        _ => return None,
    };
    let t = text.trim();
    let inner = t.strip_prefix(TABLE_OPEN)?.strip_suffix(TABLE_CLOSE)?;
    inner.parse::<usize>().ok()
}

fn text_content(node: &Node) -> Option<&str> {
    if matches!(node.node_type, NodeType::Text)
        && let Some(Scalar::String(s)) = node.attributes.get("content")
    {
        Some(s)
    } else {
        None
    }
}

/// Parse a `{% table %}` block's inner text into a `Table` node.
fn parse_list_table(inner: &str) -> Node {
    let raw_blocks = split_top_level_rows(inner);
    let leading_blank = raw_blocks
        .first()
        .map(|b| b.trim().is_empty())
        .unwrap_or(true);
    let rows: Vec<Row> = raw_blocks
        .iter()
        .filter(|b| !b.trim().is_empty())
        .map(|b| parse_row(b))
        .collect();

    let mut children = Vec::new();
    // The first non-blank row is the header unless the table began with `---`.
    let body_start = if leading_blank || rows.is_empty() {
        0
    } else {
        1
    };
    // Column alignment comes from `{% align %}` on the header cells; it feeds
    // the table's `align` attribute, which the renderer already honours.
    let mut table_attrs = HashMap::new();
    if body_start == 1 {
        if let Some(aligns) = column_aligns(&rows[0]) {
            table_attrs.insert("align".to_string(), aligns);
        }
        let head_cells = make_cells(&rows[0], true);
        let tr = Node::new(NodeType::Tr, HashMap::new(), head_cells, None);
        children.push(Node::new(NodeType::Thead, HashMap::new(), vec![tr], None));
    }
    let body_rows: Vec<Node> = rows[body_start..]
        .iter()
        .map(|cells| {
            let tds = make_cells(cells, false);
            Node::new(NodeType::Tr, HashMap::new(), tds, None)
        })
        .collect();
    if !body_rows.is_empty() {
        children.push(Node::new(NodeType::Tbody, HashMap::new(), body_rows, None));
    }

    Node::new(NodeType::Table, table_attrs, children, None)
}

/// Build the table `align` attribute (an array of `left`/`center`/`right`/``)
/// from the header row's per-cell `align`. `None` when no column is aligned.
fn column_aligns(header: &[Cell]) -> Option<Scalar> {
    let names: Vec<Scalar> = header
        .iter()
        .map(|c| {
            let name = match c.attrs.get("align") {
                Some(Scalar::String(s)) => s.clone(),
                _ => String::new(),
            };
            Scalar::String(name)
        })
        .collect();
    let any = names
        .iter()
        .any(|s| matches!(s, Scalar::String(x) if !x.is_empty()));
    any.then_some(Scalar::Array(names))
}

fn annotation_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\{%\s*(colspan|rowspan|align)\s*=\s*("[^"]*"|[^\s%}]+)\s*%\}"#).unwrap()
    })
}

/// Strip `{% colspan=N %}` / `{% rowspan=N %}` / `{% align="…" %}` annotations
/// from a row block, returning the cleaned text and `(cell_index, key, value)`
/// for each — the cell index being the `*` item the annotation trailed.
fn strip_annotations(block: &str) -> (String, Vec<(usize, String, Scalar)>) {
    let re = annotation_re();
    let mut found = Vec::new();
    let mut clean = String::with_capacity(block.len());
    let mut last = 0;
    for caps in re.captures_iter(block) {
        let m = caps.get(0).unwrap();
        if let Some(idx) = cell_index_at(block, m.start()) {
            let key = caps[1].to_string();
            let value = parse_attr_value(&key, &caps[2]);
            found.push((idx, key, value));
        }
        clean.push_str(&block[last..m.start()]);
        last = m.end();
    }
    clean.push_str(&block[last..]);
    (clean, found)
}

/// The 0-based index of the `*` cell `byte_pos` falls in: the number of
/// top-level (column-0) `*` markers starting at or before it, minus one.
fn cell_index_at(block: &str, byte_pos: usize) -> Option<usize> {
    let mut count = 0usize;
    let mut offset = 0usize;
    for line in block.split_inclusive('\n') {
        if offset >= byte_pos {
            break;
        }
        if line.starts_with('*') {
            count += 1;
        }
        offset += line.len();
    }
    count.checked_sub(1)
}

/// Parse an annotation value: `colspan`/`rowspan` as a number, `align` (and
/// anything else) as a string, with surrounding quotes removed.
fn parse_attr_value(key: &str, raw: &str) -> Scalar {
    let v = raw.trim().trim_matches('"');
    match key {
        "colspan" | "rowspan" => v
            .parse::<f64>()
            .map(Scalar::Number)
            .unwrap_or_else(|_| Scalar::String(v.to_string())),
        _ => Scalar::String(v.to_string()),
    }
}

/// Build `Th`/`Td` cell nodes from a row's parsed cells, carrying any
/// `colspan` / `rowspan` / `align` attributes.
fn make_cells(cells: &[Cell], header: bool) -> Vec<Node> {
    let nt = if header { NodeType::Th } else { NodeType::Td };
    cells
        .iter()
        .map(|c| Node::new(nt.clone(), c.attrs.clone(), c.content.clone(), None))
        .collect()
}

/// Parse one row block (a Markdown bulleted list) into its cells. Cell
/// annotations (`{% colspan=N %}` etc.) are stripped first — markdoc would
/// otherwise parse them as malformed tags that swallow the following cells —
/// then re-attached to the cell they trailed.
fn parse_row(block: &str) -> Row {
    let (clean, annotations) = strip_annotations(block);
    let Ok(doc) = crate::parse(&clean, None) else {
        return Vec::new();
    };
    let Some(list) = find_list(&doc) else {
        return Vec::new();
    };
    let mut cells: Row = list
        .children
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Item))
        .map(|item| Cell {
            content: item.children.clone(),
            attrs: HashMap::new(),
        })
        .collect();
    for (idx, key, value) in annotations {
        if let Some(cell) = cells.get_mut(idx) {
            cell.attrs.insert(key, value);
        }
    }
    cells
}

/// Find the first `List` node anywhere in the tree.
fn find_list(node: &Node) -> Option<&Node> {
    if matches!(node.node_type, NodeType::List) {
        return Some(node);
    }
    node.children.iter().find_map(find_list)
}

/// Split a table's inner text into row blocks on top-level `---` lines
/// (ignoring `---` inside fenced code blocks).
fn split_top_level_rows(inner: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut cur = String::new();
    let mut in_fence = false;
    for line in inner.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
        }
        if !in_fence && line.trim() == "---" {
            blocks.push(std::mem::take(&mut cur));
            continue;
        }
        cur.push_str(line);
        cur.push('\n');
    }
    blocks.push(cur);
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_node<'a>(node: &'a Node, nt: &NodeType) -> Option<&'a Node> {
        if &node.node_type == nt {
            return Some(node);
        }
        node.children.iter().find_map(|c| find_node(c, nt))
    }

    fn count_nodes(node: &Node, nt: &NodeType) -> usize {
        let here = usize::from(&node.node_type == nt);
        here + node
            .children
            .iter()
            .map(|c| count_nodes(c, nt))
            .sum::<usize>()
    }

    #[test]
    fn basic_list_table_has_header_and_body() {
        let doc = crate::parse(
            "{% table %}\n* H1\n* H2\n---\n* a\n* b\n{% /table %}\n",
            None,
        )
        .unwrap();
        let table = find_node(&doc, &NodeType::Table).expect("table node");
        assert!(find_node(table, &NodeType::Thead).is_some(), "has a header");
        assert_eq!(count_nodes(table, &NodeType::Th), 2, "two header cells");
        assert_eq!(count_nodes(table, &NodeType::Td), 2, "two body cells");
    }

    #[test]
    fn header_less_list_table_has_no_thead() {
        let doc = crate::parse(
            "{% table %}\n---\n* a\n* b\n---\n* c\n* d\n{% /table %}\n",
            None,
        )
        .unwrap();
        let table = find_node(&doc, &NodeType::Table).expect("table node");
        assert!(find_node(table, &NodeType::Thead).is_none(), "no header");
        assert_eq!(count_nodes(table, &NodeType::Th), 0);
        assert_eq!(count_nodes(table, &NodeType::Td), 4, "four body cells");
    }

    fn find_tag<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
        if matches!(node.node_type, NodeType::Tag) && node.tag.as_deref() == Some(name) {
            return Some(node);
        }
        node.children.iter().find_map(|c| find_tag(c, name))
    }

    #[test]
    fn pipe_table_wrapper_is_not_intercepted() {
        // `{% table … %}` wrapping a PIPE table (a styling wrapper) must not be
        // parsed as a list-table — it stays a Tag with the pipe table inside.
        let src = "{% table borders=\"grid\" %}\n| A | B |\n|---|---|\n| x | y |\n{% /table %}\n";
        let doc = crate::parse(src, None).unwrap();
        assert!(
            find_tag(&doc, "table").is_some(),
            "the table styling wrapper survived as a tag"
        );
        let table = find_node(&doc, &NodeType::Table).expect("inner pipe table");
        assert!(
            count_nodes(table, &NodeType::Td) >= 2,
            "pipe table parsed normally (not mis-parsed as an empty list-table)"
        );
    }

    #[test]
    fn rich_cell_keeps_block_content() {
        // A cell holding a fenced code block must survive as a Fence node.
        let src = "{% table %}\n* H\n---\n*\n  ```\n  code\n  ```\n* plain\n{% /table %}\n";
        let doc = crate::parse(src, None).unwrap();
        let table = find_node(&doc, &NodeType::Table).expect("table node");
        assert!(
            find_node(table, &NodeType::Fence).is_some(),
            "code block in a cell is preserved, tree was {table:#?}"
        );
    }

    fn has_attr_number(node: &Node, nt: &NodeType, key: &str, val: f64) -> bool {
        (&node.node_type == nt
            && matches!(node.attributes.get(key), Some(Scalar::Number(n)) if (n - val).abs() < 1e-9))
            || node
                .children
                .iter()
                .any(|c| has_attr_number(c, nt, key, val))
    }

    #[test]
    fn header_align_sets_column_alignment() {
        // `{% align %}` on header cells becomes the table's column `align`.
        let src = "{% table %}\n* L\n* C {% align=\"center\" %}\n* R {% align=\"right\" %}\n---\n* a\n* b\n* c\n{% /table %}\n";
        let doc = crate::parse(src, None).unwrap();
        let table = find_node(&doc, &NodeType::Table).expect("table node");
        match table.attributes.get("align") {
            Some(Scalar::Array(a)) => {
                let names: Vec<&str> = a
                    .iter()
                    .filter_map(|s| match s {
                        Scalar::String(x) => Some(x.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(names, vec!["", "center", "right"]);
            }
            other => panic!("expected an align array, got {other:?}"),
        }
    }

    #[test]
    fn colspan_annotation_stored_on_cell() {
        // `{% colspan=2 %}` on a body cell is stored as a numeric attribute,
        // and does not swallow the following cells.
        let src = "{% table %}\n---\n* a {% colspan=2 %}\n* b\n{% /table %}\n";
        let doc = crate::parse(src, None).unwrap();
        let table = find_node(&doc, &NodeType::Table).expect("table node");
        assert!(
            has_attr_number(table, &NodeType::Td, "colspan", 2.0),
            "a td carries colspan=2, tree was {table:#?}"
        );
        assert_eq!(
            count_nodes(table, &NodeType::Td),
            2,
            "both cells survive (annotation didn't swallow the next cell)"
        );
    }
}
