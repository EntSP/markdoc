use crate::schema::default_nodes;
use crate::types::*;
use std::collections::HashMap;

pub fn default_tags() -> HashMap<String, Schema> {
    let mut tags = HashMap::new();

    // ── if ──────────────────────────────────────────────────────────────
    // The predicate lives in `Node.expressions["primary"]` (see parser),
    // so the schema tracks `primary` rather than the older `condition` key.
    tags.insert(
        "if".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "primary".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![
                            ValidationType::Boolean,
                            ValidationType::String,
                            ValidationType::Number,
                            ValidationType::Object,
                            ValidationType::Array,
                        ]),
                        render: Some(SchemaRender::Bool(false)),
                        default: None,
                        required: false,
                        description: Some("Predicate to evaluate".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Conditional rendering tag".to_string()),
        },
    );

    // ── else ────────────────────────────────────────────────────────────
    // Self-closing branch separator inside a `{% if %}` body. Optional
    // primary expression turns it into an else-if; without one it is
    // the unconditional fallback.
    //
    //   {% if $a %} A {% else $b /%} B {% else /%} C {% /if %}
    //
    // The conditional evaluator splits the if's children at every
    // `{% else %}` and picks the first branch whose predicate is truthy.
    tags.insert(
        "else".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "primary".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![
                            ValidationType::Boolean,
                            ValidationType::String,
                            ValidationType::Number,
                            ValidationType::Object,
                            ValidationType::Array,
                        ]),
                        render: Some(SchemaRender::Bool(false)),
                        default: None,
                        required: false,
                        description: Some(
                            "Optional else-if predicate; bare `{% else /%}` is the fallback."
                                .into(),
                        ),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: false,
            description: Some("Branch separator inside `{% if %}`".to_string()),
        },
    );

    // ── partial ─────────────────────────────────────────────────────────
    tags.insert(
        "partial".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "file".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: Some(SchemaRender::Bool(false)),
                        default: None,
                        required: true,
                        description: Some("Path to partial file".to_string()),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: false,
            description: Some("Include a partial file".to_string()),
        },
    );

    // ── table ──────────────────────────────────────────────────────────
    tags.insert(
        "table".to_string(),
        Schema {
            render: Some("table".to_string()),
            children: None,
            attributes: None,
            self_closing: false,
            inline: false,
            description: Some("Table tag".to_string()),
        },
    );

    // ── callout ────────────────────────────────────────────────────────
    // Block callout with a `type` discriminator. Default `type` is `note`
    // so authors can write `{% callout %}...{% /callout %}` without
    // specifying severity. The validator accepts any string for `type`;
    // renderers decide how to map types to visual treatments.
    tags.insert(
        "callout".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "type".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: Some(Scalar::String("note".to_string())),
                        required: false,
                        description: Some(
                            "Severity: note, info, warning, caution, danger, success, notice"
                                .to_string(),
                        ),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Callout block".to_string()),
        },
    );

    // ── tag (anchor declaration) ────────────────────────────────────────
    // Declares a named anchor used by `{% tagref %}`. The Flux spec lets
    // the id appear under either `id=` or `tag=`; both are optional at
    // the schema level (cross-reference resolution validates that at
    // least one is present).
    tags.insert(
        "tag".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "id".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Anchor id".to_string()),
                    },
                );
                attrs.insert(
                    "tag".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Anchor id (alternate spelling)".to_string()),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: true,
            description: Some("Named anchor declaration".to_string()),
        },
    );

    // ── media ──────────────────────────────────────────────────────────
    // Structural reference to a media asset: image (bitmap/vector),
    // animated image, 3D model, audio, or video. The `src` URI carries
    // any transformation parameters (size, focus, effects) and is opaque
    // to markdoc — Arca handles all media processing/transformation;
    // renderers fetch ready-to-embed bytes from Arca using the URI as-is.
    //
    // Authors typically write self-closing form:
    //   {% media src="arca://abc-123?w=400&focus=0.5,0.3" alt="Front panel" /%}
    // Block form is also accepted, with body content treated as a caption:
    //   {% media src="..." %}**Figure 1**: front-panel layout{% /media %}
    tags.insert(
        "media".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "src".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: true,
                        description: Some(
                            "Media URI. May carry transformation params interpreted by Arca."
                                .to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "alt".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Accessibility / fallback text describing the media".to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "caption".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Visible caption rendered alongside the media".into()),
                    },
                );
                attrs.insert(
                    "kind".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Optional type hint: image | vector | animation | video | audio | model"
                                .into(),
                        ),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: false,
            description: Some(
                "Media reference (image, video, audio, 3D model, animation, etc.)".into(),
            ),
        },
    );

    // ── tagref (anchor reference) ───────────────────────────────────────
    tags.insert(
        "tagref".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "id".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Target anchor id".to_string()),
                    },
                );
                attrs.insert(
                    "tag".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Target anchor id (alternate spelling)".to_string()),
                    },
                );
                attrs.insert(
                    "doc".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Cross-document target — id of the Adeptus document that owns the anchor. Adeptus rewrites these into resolved links before publish; renderers that see one in raw source treat it as an unresolved placeholder."
                                .into(),
                        ),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: true,
            description: Some("Named anchor reference".to_string()),
        },
    );

    tags
}

/// Hand-written `Default` for `Config` so a default-constructed config
/// already knows about the standard markdown nodes and built-in tags.
/// Lives in this module (rather than `types.rs`) to avoid an import
/// cycle: `tags.rs` → `types.rs` → `tags.rs`.
impl Default for Config {
    fn default() -> Self {
        Self {
            nodes: default_nodes(),
            tags: default_tags(),
            variables: HashMap::new(),
            functions: HashMap::new(),
            partials: HashMap::new(),
        }
    }
}

pub fn truthy(value: &Scalar) -> bool {
    match value {
        Scalar::Null => false,
        Scalar::Boolean(b) => *b,
        Scalar::Number(n) => *n != 0.0,
        Scalar::String(s) => !s.is_empty(),
        Scalar::Array(a) => !a.is_empty(),
        Scalar::Object(o) => !o.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use crate::transformer::transform;
    use crate::types::RenderableTreeNode;

    fn find_tag<'a>(node: &'a RenderableTreeNode, name: &str) -> Option<&'a Tag> {
        match node {
            RenderableTreeNode::Tag(t) if t.name == name => Some(t),
            RenderableTreeNode::Tag(t) => t.children.iter().find_map(|c| find_tag(c, name)),
            _ => None,
        }
    }

    #[test]
    fn default_config_has_callout_tag_registered() {
        let cfg = Config::default();
        assert!(cfg.tags.contains_key("callout"));
        assert!(cfg.tags.contains_key("tag"));
        assert!(cfg.tags.contains_key("tagref"));
        // Standard markdown nodes are also registered now.
        assert!(cfg.nodes.contains_key("heading"));
        assert!(cfg.nodes.contains_key("paragraph"));
    }

    #[test]
    fn callout_without_explicit_type_gets_default_note() {
        let src = "{% callout %}body{% /callout %}";
        let doc = parse(src, None).unwrap();
        let rendered = transform(&doc, &Config::default()).unwrap();
        let callout = find_tag(&rendered, "callout").expect("callout in tree");
        assert_eq!(
            callout.attributes.get("type"),
            Some(&Scalar::String("note".to_string())),
            "expected default type=note, attrs were {:?}",
            callout.attributes
        );
    }

    #[test]
    fn callout_explicit_type_overrides_default() {
        let src = r#"{% callout type="warning" %}body{% /callout %}"#;
        let doc = parse(src, None).unwrap();
        let rendered = transform(&doc, &Config::default()).unwrap();
        let callout = find_tag(&rendered, "callout").unwrap();
        assert_eq!(
            callout.attributes.get("type"),
            Some(&Scalar::String("warning".to_string()))
        );
    }

    #[test]
    fn heading_level_attribute_is_dropped_under_default_config() {
        // The heading schema marks `level` as render: false. With the
        // populated default config, the renderable tree should not carry
        // level=N as an HTML attribute (it survives only as the element
        // name h1..h6).
        let src = "# Hello";
        let doc = parse(src, None).unwrap();
        let rendered = transform(&doc, &Config::default()).unwrap();
        let h1 = find_tag(&rendered, "h1").expect("h1 in tree");
        assert!(
            !h1.attributes.contains_key("level"),
            "level should be dropped, attrs were {:?}",
            h1.attributes
        );
    }

    #[test]
    fn truthy_classifies_scalars_as_expected() {
        assert!(!truthy(&Scalar::Null));
        assert!(!truthy(&Scalar::Boolean(false)));
        assert!(truthy(&Scalar::Boolean(true)));
        assert!(!truthy(&Scalar::Number(0.0)));
        assert!(truthy(&Scalar::Number(1.0)));
        assert!(!truthy(&Scalar::String(String::new())));
        assert!(truthy(&Scalar::String("x".into())));
        assert!(!truthy(&Scalar::Array(vec![])));
        assert!(truthy(&Scalar::Array(vec![Scalar::Null])));
    }

    #[test]
    fn default_config_has_media_tag_registered() {
        let cfg = Config::default();
        let media = cfg.tags.get("media").expect("media tag registered");
        let attrs = media.attributes.as_ref().unwrap();
        assert!(attrs.get("src").unwrap().required);
        assert!(!attrs.get("alt").unwrap().required);
        assert!(!attrs.get("caption").unwrap().required);
        assert!(!attrs.get("kind").unwrap().required);
    }

    #[test]
    fn media_tag_with_complex_uri_parses_intact() {
        // The src URI carries Arca transformation params (size, focus,
        // effects) verbatim. markdoc must not interpret or mutate them.
        let src = r#"{% media src="arca://abc-123?w=400&focus=0.5,0.3&blur-edge=8" alt="Front panel" kind="image" /%}"#;
        let doc = parse(src, None).unwrap();
        let rendered = transform(&doc, &Config::default()).unwrap();
        let media = find_tag(&rendered, "media").expect("media in tree");
        assert_eq!(
            media.attributes.get("src"),
            Some(&Scalar::String(
                "arca://abc-123?w=400&focus=0.5,0.3&blur-edge=8".into()
            ))
        );
        assert_eq!(
            media.attributes.get("alt"),
            Some(&Scalar::String("Front panel".into()))
        );
        assert_eq!(
            media.attributes.get("kind"),
            Some(&Scalar::String("image".into()))
        );
    }

    #[test]
    fn media_required_src_validated_when_missing() {
        // Validator should flag a `media` tag without `src`.
        use crate::validator::validate;
        let src = "{% media alt=\"oops no src\" /%}";
        let doc = parse(src, None).unwrap();
        let errors = validate(&doc, &Config::default());
        let has_required = errors
            .iter()
            .any(|e| e.id == "missing-attribute" && e.message.contains("src"));
        assert!(
            has_required,
            "expected missing-attribute error for src, got: {errors:?}"
        );
    }

    #[test]
    fn media_works_for_non_image_kinds() {
        // The same tag is used for all media types — only `kind` differs.
        for kind in &["image", "vector", "animation", "video", "audio", "model"] {
            let src = format!(
                r#"{{% media src="arca://x" kind="{kind}" /%}}"#,
                kind = kind
            );
            let doc = parse(&src, None).unwrap();
            let rendered = transform(&doc, &Config::default()).unwrap();
            let media = find_tag(&rendered, "media")
                .unwrap_or_else(|| panic!("media not found for kind={kind}"));
            assert_eq!(
                media.attributes.get("kind"),
                Some(&Scalar::String((*kind).into())),
                "kind={kind}"
            );
        }
    }
}
