use crate::ast::Node;
use crate::frontmatter;
use crate::tag_parser::{self, AttrValue, TagKind};
use crate::tokenizer::{Token, TokenEvent, TokenType, Tokenizer};
use crate::types::*;
use std::collections::HashMap;

pub fn parse(content: &str, args: Option<ParserArgs>) -> Result<Node> {
    // Extract frontmatter.
    let (frontmatter_data, content_without_fm) = frontmatter::extract_frontmatter(content)?;

    // Build the variable map used for inline `{% $var %}` substitution.
    let mut variables = HashMap::new();
    if let Some(fm) = &frontmatter_data {
        let mut markdoc = HashMap::new();
        markdoc.insert("frontmatter".to_string(), Scalar::Object(fm.clone()));
        variables.insert("markdoc".to_string(), Scalar::Object(markdoc));
    }

    // 1. Substitute inline variables (`{% $var.path %}`) at text level.
    let content_with_vars = tag_parser::replace_variables(&content_without_fm, &variables);

    // 2. Extract structural tags (`{% name attrs %}`, `{% /name %}`, etc.)
    //    into a side-table; replace each occurrence with a sentinel so the
    //    markdown tokenizer leaves them as opaque text. The tokenizer
    //    re-merges tag events back into the token stream.
    let (content_with_sentinels, parsed_tags) = tag_parser::segment_with_tags(&content_with_vars);

    let tokenizer = Tokenizer::new();
    let tokens = tokenizer.tokenize_with_tags(&content_with_sentinels, &parsed_tags);
    let mut doc = parse_tokens(tokens, args)?;

    if let Some(fm) = frontmatter_data {
        doc.attributes
            .insert("frontmatter".to_string(), Scalar::Object(fm));
    }

    Ok(doc)
}

fn parse_tokens(tokens: Vec<Token>, args: Option<ParserArgs>) -> Result<Node> {
    let args = args.unwrap_or(ParserArgs {
        file: None,
        slots: false,
        location: true,
    });

    let mut doc = Node::new(NodeType::Document, HashMap::new(), Vec::new(), None);
    let mut stack: Vec<Node> = vec![doc.clone()];
    let line_num = 1;

    for token in tokens {
        match token.event {
            TokenEvent::Start(token_type) => {
                let node = create_node_from_token(&token_type, line_num, &args);
                stack.push(node);
            }
            TokenEvent::End(_token_type) => {
                if stack.len() > 1 {
                    let node = stack.pop().unwrap();

                    if let Some(parent) = stack.last_mut() {
                        if args.slots && is_slot_node(&node) {
                            if let Some(Scalar::String(slot_name)) = node.attributes.get("primary")
                            {
                                parent.slots.insert(slot_name.clone(), node);
                            } else {
                                parent.push(node);
                            }
                        } else {
                            parent.push(node);
                        }
                    }
                }
            }
            TokenEvent::Text(text) => {
                if let Some(parent) = stack.last_mut() {
                    let mut attrs = HashMap::new();
                    attrs.insert("content".to_string(), Scalar::String(text));
                    let text_node = Node::new(NodeType::Text, attrs, Vec::new(), None);
                    parent.push(text_node);
                }
            }
            TokenEvent::Code(code) => {
                if let Some(parent) = stack.last_mut() {
                    let mut attrs = HashMap::new();
                    attrs.insert("content".to_string(), Scalar::String(code));
                    let code_node = Node::new(NodeType::Code, attrs, Vec::new(), None);
                    parent.push(code_node);
                }
            }
            TokenEvent::Html(_) => {
                // TODO Skip HTML for now
            }
            TokenEvent::Tag(kind) => {
                handle_tag_event(kind, &mut stack, line_num, &args);
            }
        }

        // Track line numbers (simplified)
        if let Some((_, _end_pos)) = token.position {
            // TODO This is a simplification - in a real implementation,
            // we'd need to track actual line numbers from the source
        }
    }

    // Pop remaining nodes and add to parent
    while stack.len() > 1 {
        let node = stack.pop().unwrap();
        if let Some(parent) = stack.last_mut() {
            parent.push(node);
        }
    }

    doc = stack.pop().unwrap();
    Ok(doc)
}

fn create_node_from_token(token_type: &TokenType, line: usize, args: &ParserArgs) -> Node {
    let (node_type, attributes) = match token_type {
        TokenType::Paragraph => (NodeType::Paragraph, HashMap::new()),
        TokenType::Heading(level) => {
            let mut attrs = HashMap::new();
            attrs.insert("level".to_string(), Scalar::Number(*level as f64));
            (NodeType::Heading, attrs)
        }
        TokenType::BlockQuote => (NodeType::Blockquote, HashMap::new()),
        TokenType::CodeBlock(lang) => {
            let mut attrs = HashMap::new();
            if let Some(language) = lang {
                attrs.insert("language".to_string(), Scalar::String(language.clone()));
            }
            (NodeType::Fence, attrs)
        }
        TokenType::List(ordered, start) => {
            let mut attrs = HashMap::new();
            attrs.insert("ordered".to_string(), Scalar::Boolean(*ordered));
            if let Some(start_num) = start {
                attrs.insert("start".to_string(), Scalar::Number(*start_num as f64));
            }
            (NodeType::List, attrs)
        }
        TokenType::Item => (NodeType::Item, HashMap::new()),
        TokenType::Table => (NodeType::Table, HashMap::new()),
        TokenType::TableHead => (NodeType::Thead, HashMap::new()),
        TokenType::TableRow => (NodeType::Tr, HashMap::new()),
        TokenType::TableCell => (NodeType::Td, HashMap::new()),
        TokenType::Emphasis => (NodeType::Em, HashMap::new()),
        TokenType::Strong => (NodeType::Strong, HashMap::new()),
        TokenType::Strikethrough => (NodeType::Strikethrough, HashMap::new()),
        TokenType::Link(url, title) => {
            let mut attrs = HashMap::new();
            attrs.insert("href".to_string(), Scalar::String(url.clone()));
            if !title.is_empty() {
                attrs.insert("title".to_string(), Scalar::String(title.clone()));
            }
            (NodeType::Link, attrs)
        }
        TokenType::Image(url, title) => {
            let mut attrs = HashMap::new();
            attrs.insert("src".to_string(), Scalar::String(url.clone()));
            if !title.is_empty() {
                attrs.insert("title".to_string(), Scalar::String(title.clone()));
            }
            (NodeType::Image, attrs)
        }
        TokenType::Rule => (NodeType::Hr, HashMap::new()),
        TokenType::LineBreak => (NodeType::Hardbreak, HashMap::new()),
        TokenType::SoftBreak => (NodeType::Softbreak, HashMap::new()),
    };

    let mut node = Node::new(node_type, attributes, Vec::new(), None);

    if args.location {
        node.lines = vec![line];
        node.location = Some(Location {
            file: args.file.clone(),
            start: LocationEdge {
                line,
                character: None,
            },
            end: LocationEdge {
                line,
                character: None,
            },
        });
    }

    node
}

fn is_slot_node(node: &Node) -> bool {
    node.tag.as_deref() == Some("slot")
}

/// Convert a parsed tag event into stack manipulation:
///   - Open       → push a new `Tag` node
///   - Close      → pop nodes until the matching `Tag` node is closed and
///     attached to its parent (closing markdown nodes that were
///     still open are attached too, in source order)
///   - SelfClose  → push then immediately pop
///   - HeadingId  → store the id on the nearest enclosing heading
fn handle_tag_event(kind: TagKind, stack: &mut Vec<Node>, line: usize, args: &ParserArgs) {
    match kind {
        TagKind::Open { name, attrs } => {
            stack.push(make_tag_node(&name, attrs, line, args));
        }
        TagKind::SelfClose { name, attrs } => {
            stack.push(make_tag_node(&name, attrs, line, args));
            close_to_tag(&name, stack);
        }
        TagKind::Close { name } => {
            close_to_tag(&name, stack);
        }
        TagKind::HeadingId { id } => {
            // Walk the stack backwards looking for the nearest open heading.
            for node in stack.iter_mut().rev() {
                if matches!(node.node_type, NodeType::Heading) {
                    node.attributes.insert("id".to_string(), Scalar::String(id));
                    return;
                }
            }
            // No enclosing heading — silently drop. (The Markdoc spec
            // restricts this sugar to heading contexts.)
        }
    }
}

/// Build a `Tag` node from parsed attributes, splitting them into the
/// `attributes` map (literal scalars) and the `expressions` map (raw
/// expression source for variables and function calls).
fn make_tag_node(name: &str, attrs: tag_parser::TagAttrs, line: usize, args: &ParserArgs) -> Node {
    let mut attributes: HashMap<String, Scalar> = HashMap::new();
    let mut expressions: HashMap<String, String> = HashMap::new();

    if let Some(primary) = attrs.primary {
        match primary {
            AttrValue::Literal(s) => {
                attributes.insert("primary".to_string(), s);
            }
            AttrValue::Expression(src) => {
                expressions.insert("primary".to_string(), src);
            }
        }
    }
    for (key, value) in attrs.named {
        match value {
            AttrValue::Literal(s) => {
                attributes.insert(key, s);
            }
            AttrValue::Expression(src) => {
                expressions.insert(key, src);
            }
        }
    }

    let mut node = Node::new(
        NodeType::Tag,
        attributes,
        Vec::new(),
        Some(name.to_string()),
    );
    node.expressions = expressions;
    if args.location {
        node.lines = vec![line];
        node.location = Some(Location {
            file: args.file.clone(),
            start: LocationEdge {
                line,
                character: None,
            },
            end: LocationEdge {
                line,
                character: None,
            },
        });
    }
    node
}

/// Pop nodes off the stack and attach them to their parents until a `Tag`
/// node with the given name is itself popped and attached. If no such tag
/// is open, the call is a no-op (mismatched close — silently ignored for now;
/// future work: emit a ValidationError).
fn close_to_tag(name: &str, stack: &mut Vec<Node>) {
    // Find the position of the matching tag on the stack.
    let target = stack
        .iter()
        .rposition(|n| matches!(n.node_type, NodeType::Tag) && n.tag.as_deref() == Some(name));
    let Some(target) = target else { return };

    // Pop everything above the target into their parents.
    while stack.len() > target + 1 {
        let node = stack.pop().unwrap();
        if let Some(parent) = stack.last_mut() {
            parent.push(node);
        }
    }
    // Pop the target itself and attach to its parent.
    let tag_node = stack.pop().unwrap();
    if let Some(parent) = stack.last_mut() {
        parent.push(tag_node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Walk the tree and find the first node satisfying `pred`.
    fn find<'a>(root: &'a Node, pred: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
        if pred(root) {
            return Some(root);
        }
        for child in &root.children {
            if let Some(found) = find(child, pred) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn parses_simple_markdown_unchanged() {
        let src = "# Hello\n\nThis is **bold**.";
        let doc = parse(src, None).unwrap();
        let heading =
            find(&doc, &|n| matches!(n.node_type, NodeType::Heading)).expect("heading present");
        assert_eq!(heading.attributes.get("level"), Some(&Scalar::Number(1.0)));
        let strong =
            find(&doc, &|n| matches!(n.node_type, NodeType::Strong)).expect("strong present");
        let text =
            find(strong, &|n| matches!(n.node_type, NodeType::Text)).expect("text in strong");
        assert_eq!(
            text.attributes.get("content"),
            Some(&Scalar::String("bold".to_string()))
        );
    }

    #[test]
    fn parses_callout_as_tag_node() {
        let src = r#"{% callout type="warning" %}
**Warn**: be careful.
{% /callout %}"#;
        let doc = parse(src, None).unwrap();

        let callout = find(&doc, &|n| {
            matches!(n.node_type, NodeType::Tag) && n.tag.as_deref() == Some("callout")
        })
        .expect("callout tag node present");

        assert_eq!(
            callout.attributes.get("type"),
            Some(&Scalar::String("warning".to_string()))
        );
        // The callout should contain the `**Warn**: be careful.` content.
        let strong = find(callout, &|n| matches!(n.node_type, NodeType::Strong))
            .expect("bold inside callout");
        let warn = find(strong, &|n| matches!(n.node_type, NodeType::Text)).expect("warn text");
        assert_eq!(
            warn.attributes.get("content"),
            Some(&Scalar::String("Warn".to_string()))
        );
    }

    #[test]
    fn captures_heading_id_sugar() {
        let src = "# Overview {% #my-overview %}";
        let doc = parse(src, None).unwrap();
        let heading =
            find(&doc, &|n| matches!(n.node_type, NodeType::Heading)).expect("heading present");
        assert_eq!(
            heading.attributes.get("id"),
            Some(&Scalar::String("my-overview".to_string()))
        );
    }

    #[test]
    fn captures_unresolved_expressions_in_node_expressions_field() {
        let src = "{% if $config.show_section %}content{% /if %}";
        let doc = parse(src, None).unwrap();
        let if_node = find(&doc, &|n| {
            matches!(n.node_type, NodeType::Tag) && n.tag.as_deref() == Some("if")
        })
        .expect("if tag node");
        // The primary expression should land in `expressions["primary"]`,
        // not in `attributes` (because it's an unresolved expression).
        assert!(!if_node.attributes.contains_key("primary"));
        assert_eq!(
            if_node.expressions.get("primary").map(String::as_str),
            Some("$config.show_section")
        );
    }

    #[test]
    fn self_closing_partial_becomes_empty_tag_node() {
        let src = r#"{% partial file="x.markdoc" /%}"#;
        let doc = parse(src, None).unwrap();
        let partial = find(&doc, &|n| {
            matches!(n.node_type, NodeType::Tag) && n.tag.as_deref() == Some("partial")
        })
        .expect("partial tag");
        assert_eq!(
            partial.attributes.get("file"),
            Some(&Scalar::String("x.markdoc".to_string()))
        );
        assert!(partial.children.is_empty());
    }

    #[test]
    fn frontmatter_variable_interpolation_still_works() {
        let src = "---\ntitle: Hello\n---\n\n# {% $markdoc.frontmatter.title %}";
        let doc = parse(src, None).unwrap();
        let heading =
            find(&doc, &|n| matches!(n.node_type, NodeType::Heading)).expect("heading present");
        let text =
            find(heading, &|n| matches!(n.node_type, NodeType::Text)).expect("text in heading");
        assert_eq!(
            text.attributes.get("content"),
            Some(&Scalar::String("Hello".to_string()))
        );
    }
}
