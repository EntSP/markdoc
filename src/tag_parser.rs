use crate::types::*;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashMap;
use std::ops::Range;

// ────────────────────────────────────────────────────────────────────────────
// Structural tag parsing — `{% name attrs %}`, `{% /name %}`, `{% name /%}`,
// and the heading-id sugar `{% #id %}`. Replaces the previous regex-strip
// approach so that tags become first-class AST nodes.
// ────────────────────────────────────────────────────────────────────────────

/// A single `{% ... %}` occurrence parsed from source.
#[derive(Debug, Clone)]
pub struct ParsedTag {
    pub kind: TagKind,
    pub source_range: Range<usize>,
}

#[derive(Debug, Clone)]
pub enum TagKind {
    /// `{% name attrs %}`
    Open { name: String, attrs: TagAttrs },
    /// `{% /name %}`
    Close { name: String },
    /// `{% name attrs /%}`
    SelfClose { name: String, attrs: TagAttrs },
    /// `{% #id %}` — heading-id sugar.
    HeadingId { id: String },
}

/// Tag attributes split into literal values and unresolved expressions.
///
/// `primary` is the unkeyed value for tags like `{% if $cfg.foo %}` (where
/// the entire post-name body is one expression instead of `key=value` pairs).
#[derive(Debug, Clone, Default)]
pub struct TagAttrs {
    pub primary: Option<AttrValue>,
    pub named: IndexMap<String, AttrValue>,
}

/// A single attribute value: a literal scalar or the raw source of an
/// expression that needs an evaluation context to resolve.
#[derive(Debug, Clone)]
pub enum AttrValue {
    Literal(Scalar),
    /// Raw source for `$var.path` or `funcname(args)`.
    Expression(String),
}

/// Sentinel chars used to mark tag positions in the rewritten source that's
/// fed to the markdown tokenizer. Private Use Area code points so they
/// can't collide with real content.
const SENTINEL_OPEN: char = '\u{E000}';
const SENTINEL_CLOSE: char = '\u{E001}';

/// Find all `{% ... %}` occurrences in `content`, return:
/// 1. A rewritten string where each occurrence is replaced by a sentinel
///    `\u{E000}NN\u{E001}` (`NN` = decimal index into the returned vec).
/// 2. The parsed tags. Items the tag parser cannot recognise are left
///    in the rewritten string verbatim (the inline `{% $var %}` form, for
///    example, is handled separately by `replace_variables`).
pub fn segment_with_tags(content: &str) -> (String, Vec<ParsedTag>) {
    let re = tag_regex();
    let mut rewritten = String::with_capacity(content.len());
    let mut tags: Vec<ParsedTag> = Vec::new();
    let mut last = 0usize;

    for m in re.find_iter(content) {
        rewritten.push_str(&content[last..m.start()]);
        if let Some(parsed) = parse_one_tag(m.as_str(), m.start()..m.end()) {
            let idx = tags.len();
            tags.push(parsed);
            rewritten.push(SENTINEL_OPEN);
            rewritten.push_str(&idx.to_string());
            rewritten.push(SENTINEL_CLOSE);
        } else {
            // Not a recognisable structural tag — leave verbatim so that
            // `replace_variables` (or just text rendering) can pick it up.
            rewritten.push_str(m.as_str());
        }
        last = m.end();
    }
    rewritten.push_str(&content[last..]);
    (rewritten, tags)
}

fn tag_regex() -> Regex {
    // Match `{% ... %}` where the body does not itself contain `%}`.
    // (We keep the body simple — Markdoc tag bodies don't legitimately
    // contain `%}`; if one ever does, the surrounding match will fail and
    // the tag will fall through to text.)
    Regex::new(r"\{%[^%]*(?:%[^}][^%]*)*%\}").unwrap()
}

fn parse_one_tag(raw: &str, source_range: Range<usize>) -> Option<ParsedTag> {
    let inner = raw.strip_prefix("{%")?.strip_suffix("%}")?.trim();
    if inner.is_empty() {
        return None;
    }

    // {% #id %}
    if let Some(rest) = inner.strip_prefix('#') {
        let id = rest.trim();
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c == '-' || c == '_' || c.is_alphanumeric())
        {
            return None;
        }
        return Some(ParsedTag {
            kind: TagKind::HeadingId { id: id.to_string() },
            source_range,
        });
    }

    // {% /name %}
    if let Some(rest) = inner.strip_prefix('/') {
        let name = rest.trim();
        if name.is_empty() || !is_valid_name(name) {
            return None;
        }
        return Some(ParsedTag {
            kind: TagKind::Close {
                name: name.to_string(),
            },
            source_range,
        });
    }

    // Detect trailing `/` for self-close.
    let (body, self_close) = if let Some(stripped) = inner.strip_suffix('/') {
        (stripped.trim_end(), true)
    } else {
        (inner, false)
    };

    // Take the leading identifier as the tag name.
    let mut name_end = 0usize;
    for (i, c) in body.char_indices() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            name_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if name_end == 0 {
        return None;
    }
    let name = body[..name_end].to_string();
    let attrs = parse_attrs(body[name_end..].trim());

    Some(ParsedTag {
        kind: if self_close {
            TagKind::SelfClose { name, attrs }
        } else {
            TagKind::Open { name, attrs }
        },
        source_range,
    })
}

fn is_valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Parse the attributes portion of a tag.
///
/// Recognises:
///   - `key="string"` / `key='string'`
///   - `key=number`
///   - `key=true|false|null`
///   - `key=$var.path`
///   - `key=funcname(arg1, arg2)`
///   - bare leading expression (no key=): becomes `primary`. This is
///     needed for `{% if $cfg.foo %}` and similar.
fn parse_attrs(src: &str) -> TagAttrs {
    let mut attrs = TagAttrs::default();
    let bytes = src.as_bytes();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        // Skip whitespace.
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }

        let key_start = cursor;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric()
                || bytes[cursor] == b'_'
                || bytes[cursor] == b'-')
        {
            cursor += 1;
        }
        // No identifier at this position, OR identifier not followed by `=`
        // → the rest of `src` is the primary expression.
        let identifier_end = cursor;
        let has_eq = identifier_end < bytes.len() && bytes[identifier_end] == b'=';

        if identifier_end == key_start || !has_eq {
            // The rest of `src` is the primary expression. Route it
            // through `consume_value` so quoted-string primaries like
            // `{% tag "X" /%}` are unwrapped the same way keyed values
            // are — without this, the surrounding quotes leak into the
            // attribute and downstream consumers see `"X"` instead of `X`.
            let remainder = &src[key_start..];
            let trimmed_start = remainder
                .bytes()
                .take_while(|b| b.is_ascii_whitespace())
                .count();
            let trimmed = &remainder[trimmed_start..];
            if !trimmed.is_empty() {
                let (value, _) = consume_value(remainder);
                attrs.primary = Some(value);
            }
            break;
        }

        let key = &src[key_start..identifier_end];
        cursor = identifier_end + 1; // past the `=`
        let (value, consumed) = consume_value(&src[cursor..]);
        attrs.named.insert(key.to_string(), value);
        cursor += consumed;
    }

    attrs
}

/// Consume a single attribute value starting at the beginning of `s`.
/// Returns the parsed value plus the number of bytes consumed (including
/// any leading whitespace).
fn consume_value(s: &str) -> (AttrValue, usize) {
    let leading_ws = s.bytes().take_while(|b| b.is_ascii_whitespace()).count();
    let s_trim = &s[leading_ws..];
    if s_trim.is_empty() {
        return (AttrValue::Literal(Scalar::Null), leading_ws);
    }

    // Quoted string.
    if let Some(rest) = s_trim.strip_prefix('"')
        && let Some(end) = rest.find('"')
    {
        let value = AttrValue::Literal(Scalar::String(rest[..end].to_string()));
        return (value, leading_ws + 1 + end + 1);
    }
    if let Some(rest) = s_trim.strip_prefix('\'')
        && let Some(end) = rest.find('\'')
    {
        let value = AttrValue::Literal(Scalar::String(rest[..end].to_string()));
        return (value, leading_ws + 1 + end + 1);
    }

    // Bare token: extends until whitespace at bracket-depth 0. Function
    // calls and array / object literals continue across balanced
    // ()/[]/{}, and whitespace inside a quoted string doesn't end the token.
    let mut depth: i32 = 0;
    let mut end = 0usize;
    let mut quote: Option<char> = None;
    for (i, c) in s_trim.char_indices() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            end = i + c.len_utf8();
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            _ if depth == 0 && c.is_ascii_whitespace() => break,
            _ => {}
        }
        end = i + c.len_utf8();
    }
    let token = &s_trim[..end];
    (parse_attr_value(token), leading_ws + end)
}

fn parse_attr_value(token: &str) -> AttrValue {
    let token = token.trim();
    match token {
        "true" => return AttrValue::Literal(Scalar::Boolean(true)),
        "false" => return AttrValue::Literal(Scalar::Boolean(false)),
        "null" => return AttrValue::Literal(Scalar::Null),
        _ => {}
    }
    if let Ok(n) = token.parse::<f64>() {
        return AttrValue::Literal(Scalar::Number(n));
    }
    // Variables, function calls, and array / object literals are all
    // resolved by the expression evaluator at transform time.
    if token.starts_with('$')
        || token.starts_with('[')
        || token.starts_with('{')
        || token.contains('(')
    {
        AttrValue::Expression(token.to_string())
    } else {
        // Unquoted bare word → string literal.
        AttrValue::Literal(Scalar::String(token.to_string()))
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Inline variable interpolation — `{% $name.path %}`.
//
// Kept from the previous implementation; this runs as a text-level
// substitution before tokenization and is independent of structural tag
// handling. The two paths are disjoint because variable interpolation
// requires `{%` to be immediately followed by `$`.
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct VariableMatch {
    pub path: String,
    pub start: usize,
    pub end: usize,
}

pub fn extract_variables(content: &str) -> Vec<VariableMatch> {
    let mut matches = Vec::new();
    let var_regex = Regex::new(r"\{%\s*\$([a-zA-Z_][a-zA-Z0-9_.-]*)\s*%\}").unwrap();

    for cap in var_regex.captures_iter(content) {
        if let Some(var_path) = cap.get(1) {
            let full_match = cap.get(0).unwrap();
            matches.push(VariableMatch {
                path: var_path.as_str().to_string(),
                start: full_match.start(),
                end: full_match.end(),
            });
        }
    }
    matches
}

pub fn replace_variables(content: &str, variables: &HashMap<String, Scalar>) -> String {
    // Interpolate `{% <expression> %}` spans: variables (`$a.b[0]`),
    // function calls (`default($x, "y")`, `debug($x)`), and literals.
    // Anything that doesn't read as an interpolation — structural tags
    // (`{% if %}`, `{% /table %}`), annotations (`{% #id %}`), and
    // list-table cell annotations — is left untouched for the tag
    // machinery downstream.
    let ctx = crate::types::Context {
        variables: variables.clone(),
    };
    let mut result = String::new();
    let mut rest = content;
    while let Some(open) = rest.find("{%") {
        result.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        let Some(close_rel) = after_open.find("%}") else {
            // No closing delimiter — emit the remainder verbatim.
            result.push_str(&rest[open..]);
            return result;
        };
        let span_end = open + 2 + close_rel + 2;
        let inner = after_open[..close_rel].trim();
        if is_interpolation(inner)
            && let Ok(expr) = crate::expression::parse_expression(inner)
        {
            // On a successful evaluation, substitute the stringified value;
            // on an evaluation error (e.g. unknown function) drop the span
            // rather than leak `{% … %}` into the rendered text.
            if let Ok(value) = crate::expression::evaluate_default(&expr, &ctx) {
                result.push_str(&scalar_to_string(&value));
            }
        } else {
            // Not an interpolation — keep the literal span for the tag pass.
            result.push_str(&rest[open..span_end]);
        }
        rest = &rest[span_end..];
    }
    result.push_str(rest);
    result
}

/// True when a `{% … %}` body should be evaluated as an interpolation
/// (a `$variable…` reference or a `name(...)` function call) rather than
/// treated as a structural tag or annotation.
fn is_interpolation(inner: &str) -> bool {
    let inner = inner.trim_start();
    if inner.starts_with('$') {
        return true;
    }
    // `name(` — a function call. (Tags like `if $x` have a space, not `(`.)
    matches!(
        inner.split_once('('),
        Some((head, _))
            if !head.is_empty()
                && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    )
}

fn scalar_to_string(scalar: &Scalar) -> String {
    match scalar {
        Scalar::String(s) => s.clone(),
        Scalar::Number(n) => n.to_string(),
        Scalar::Boolean(b) => b.to_string(),
        Scalar::Null => String::new(),
        Scalar::Array(_) | Scalar::Object(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(s: &str) -> &str {
        // Helper: extract just the rewritten text inside the sentinel
        // boundaries, so tests can assert sentinel placement abstractly.
        s
    }

    #[test]
    fn extracts_variable() {
        let content = "Title: {% $markdoc.frontmatter.title %}";
        let vars = extract_variables(content);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].path, "markdoc.frontmatter.title");
    }

    #[test]
    fn parses_array_and_object_literal_attrs() {
        let (_, tags) = segment_with_tags(r#"{% point coords=[1, 2, 3] meta={id: "x"} /%}"#);
        assert_eq!(tags.len(), 1);
        let TagKind::SelfClose { name, attrs } = &tags[0].kind else {
            panic!("expected self-closing tag, got {:?}", tags[0].kind);
        };
        assert_eq!(name, "point");
        // Array / object literals are captured whole (whitespace inside the
        // brackets doesn't end the token) and routed to the expression
        // evaluator at transform time.
        match attrs.named.get("coords").unwrap() {
            AttrValue::Expression(s) => assert_eq!(s, "[1, 2, 3]"),
            v => panic!("expected expression, got {v:?}"),
        }
        match attrs.named.get("meta").unwrap() {
            AttrValue::Expression(s) => assert_eq!(s, r#"{id: "x"}"#),
            v => panic!("expected expression, got {v:?}"),
        }
    }

    #[test]
    fn parses_tag_with_string_attr() {
        let (rewritten, tags) = segment_with_tags(r#"{% callout type="warning" %}"#);
        assert_eq!(tags.len(), 1);
        assert!(matches!(tags[0].kind, TagKind::Open { .. }));
        if let TagKind::Open { name, attrs } = &tags[0].kind {
            assert_eq!(name, "callout");
            match attrs.named.get("type").unwrap() {
                AttrValue::Literal(Scalar::String(s)) => assert_eq!(s, "warning"),
                v => panic!("expected literal string, got {v:?}"),
            }
        }
        // Sentinel should be the only thing in `rewritten`.
        assert!(rewritten.starts_with('\u{E000}'));
        assert!(rewritten.ends_with('\u{E001}'));
    }

    #[test]
    fn parses_self_closing_tag() {
        let (_, tags) = segment_with_tags(r#"{% partial file="x.markdoc" /%}"#);
        assert_eq!(tags.len(), 1);
        assert!(matches!(tags[0].kind, TagKind::SelfClose { .. }));
        if let TagKind::SelfClose { name, attrs } = &tags[0].kind {
            assert_eq!(name, "partial");
            match attrs.named.get("file").unwrap() {
                AttrValue::Literal(Scalar::String(s)) => assert_eq!(s, "x.markdoc"),
                _ => panic!(),
            }
        }
    }

    #[test]
    fn parses_closing_tag() {
        let (_, tags) = segment_with_tags("{% /callout %}");
        assert_eq!(tags.len(), 1);
        match &tags[0].kind {
            TagKind::Close { name } => assert_eq!(name, "callout"),
            _ => panic!("expected close"),
        }
    }

    #[test]
    fn parses_heading_id() {
        let (_, tags) = segment_with_tags("# Title {% #my-id %}");
        assert_eq!(tags.len(), 1);
        match &tags[0].kind {
            TagKind::HeadingId { id } => assert_eq!(id, "my-id"),
            _ => panic!("expected heading id"),
        }
    }

    #[test]
    fn parses_if_with_primary_variable_expression() {
        let (_, tags) = segment_with_tags("{% if $config.foo %}");
        assert_eq!(tags.len(), 1);
        if let TagKind::Open { name, attrs } = &tags[0].kind {
            assert_eq!(name, "if");
            assert!(attrs.named.is_empty());
            match attrs.primary.as_ref().unwrap() {
                AttrValue::Expression(s) => assert_eq!(s, "$config.foo"),
                v => panic!("expected expression, got {v:?}"),
            }
        } else {
            panic!("expected open tag");
        }
    }

    #[test]
    fn parses_if_with_function_call_expression() {
        let (_, tags) = segment_with_tags(r#"{% if equals($a, "x") %}"#);
        assert_eq!(tags.len(), 1);
        if let TagKind::Open { attrs, .. } = &tags[0].kind {
            match attrs.primary.as_ref().unwrap() {
                AttrValue::Expression(s) => assert_eq!(s, r#"equals($a, "x")"#),
                v => panic!("expected expression, got {v:?}"),
            }
        }
    }

    #[test]
    fn parses_numeric_and_boolean_attrs() {
        let (_, tags) = segment_with_tags("{% mything count=42 active=true %}");
        if let TagKind::Open { attrs, .. } = &tags[0].kind {
            match attrs.named.get("count").unwrap() {
                AttrValue::Literal(Scalar::Number(n)) => assert_eq!(*n, 42.0),
                _ => panic!(),
            }
            match attrs.named.get("active").unwrap() {
                AttrValue::Literal(Scalar::Boolean(b)) => assert!(*b),
                _ => panic!(),
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn parses_variable_attr_value() {
        let (_, tags) = segment_with_tags("{% mytag value=$config.x %}");
        if let TagKind::Open { attrs, .. } = &tags[0].kind {
            match attrs.named.get("value").unwrap() {
                AttrValue::Expression(s) => assert_eq!(s, "$config.x"),
                _ => panic!(),
            }
        } else {
            panic!()
        }
    }

    #[test]
    fn ignores_inline_variable_interpolation() {
        // `{% $x %}` is the inline-variable form; it should NOT be parsed
        // as a structural tag (the leading char is `$`, not an identifier).
        let (rewritten, tags) = segment_with_tags("Hello {% $name %}");
        assert!(tags.is_empty());
        assert_eq!(rewritten, "Hello {% $name %}");
        // (replace_variables, run separately, will substitute it.)
    }

    #[test]
    fn segments_multiple_tags_in_paragraph() {
        let src = "before {% callout type=\"note\" %}inside{% /callout %} after";
        let (rewritten, tags) = segment_with_tags(src);
        assert_eq!(tags.len(), 2);
        assert!(matches!(tags[0].kind, TagKind::Open { .. }));
        assert!(matches!(tags[1].kind, TagKind::Close { .. }));
        // The two sentinels should appear in order in the rewrite.
        let i0 = rewritten.find('\u{E000}').unwrap();
        let i1 = rewritten.rfind('\u{E000}').unwrap();
        assert!(i0 < i1);
        let _ = open(&rewritten);
    }
}
