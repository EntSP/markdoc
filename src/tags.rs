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
    let media_schema = Schema {
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
                        // Optional because `id` is an alternative locator; a
                        // media reference must carry one of `src` or `id`.
                        required: false,
                        description: Some(
                            "Media URI (alternative to `id`). May carry transformation params interpreted by Arca."
                                .to_string(),
                        ),
                    },
                );
            attrs.insert(
                    "id".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Arca asset id (alternative to `src`). Scriptor rewrites it to a concrete `src` before render; locally, markdoc-pdf resolves it to a file named `<id>.<ext>` under the assets root."
                                .into(),
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
                "title".to_string(),
                SchemaAttribute {
                    attr_type: Some(vec![ValidationType::String]),
                    render: None,
                    default: None,
                    required: false,
                    description: Some(
                        "Advisory title (the HTML `title` / tooltip); markdown `![alt](url \"title\")` sets it. Renderers may use it as a caption / alt fallback."
                            .to_string(),
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
            attrs.insert(
                    "side".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "When placed inside a `{% float %}`, the side to float this image to: left or right"
                                .into(),
                        ),
                    },
                );
            attrs.insert(
                    "width".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number, ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Display width — a fraction \u{2264} 1 of the column, a length, or a \"NN%\" string"
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
    };
    // `img` is an alias so `{% img … /%}` and markdown images share the media
    // contract (both dispatch to the same renderer path).
    tags.insert("img".to_string(), {
        let mut s = media_schema.clone();
        s.description = Some("Image reference (alias of `media`)".to_string());
        s
    });
    tags.insert("media".to_string(), media_schema);

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

    // ── columns ─────────────────────────────────────────────────────────
    // Side-by-side layout: each child (a list item or a blank-line
    // separated block) becomes one column. A pure layout primitive —
    // renderers map it to their medium (parley table in PDF, flexbox/grid
    // on the web). No attribute is required.
    tags.insert(
        "columns".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "widths".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String, ValidationType::Array]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Relative column widths, e.g. \"2 1\" or [2, 1]; equal if omitted"
                                .to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "gap".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number, ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Space between columns, in points (default 16)".to_string(),
                        ),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Side-by-side columns".to_string()),
        },
    );

    // ── float ───────────────────────────────────────────────────────────
    // Float an image to one side with content wrapping around it. With
    // inline `{% media side=… /%}` markers in the body it becomes a
    // multi-image "magazine" wrap (several floats anchored where they
    // appear in the prose). Units are interpreted by each renderer
    // (points in PDF, CSS length on the web); page-break behaviour is a
    // renderer concern, not part of this contract.
    tags.insert(
        "float".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "side".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Which side the image floats to: left (default) or right".to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "width".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number, ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Image width — a fraction \u{2264} 1 of the column, a length, or a \"NN%\" string (default 40%)"
                                .to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "gap".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number, ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Space between the image and the wrapped content (default 14)"
                                .to_string(),
                        ),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Float an image with content wrapping around it".to_string()),
        },
    );

    // ── color / c ───────────────────────────────────────────────────────
    // Inline coloured text span. `c` is a shorthand alias.
    let color_schema = |alias: bool| Schema {
        render: None,
        children: None,
        attributes: Some({
            let mut attrs = HashMap::new();
            attrs.insert(
                "value".to_string(),
                SchemaAttribute {
                    attr_type: Some(vec![ValidationType::String]),
                    render: None,
                    default: None,
                    required: false,
                    description: Some(
                        "Colour: a #rgb / #rrggbb hex value or a named colour".to_string(),
                    ),
                },
            );
            attrs
        }),
        self_closing: false,
        inline: true,
        description: Some(if alias {
            "Inline coloured text span (alias of `color`)".to_string()
        } else {
            "Inline coloured text span".to_string()
        }),
    };
    tags.insert("color".to_string(), color_schema(false));
    tags.insert("c".to_string(), color_schema(true));

    // ── list ────────────────────────────────────────────────────────────
    // A list with a custom marker style, wrapping ordinary list items.
    tags.insert(
        "list".to_string(),
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
                        default: None,
                        required: false,
                        description: Some("Marker style: checkmark | dash | none".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("List with a custom marker style".to_string()),
        },
    );

    // ── caption ─────────────────────────────────────────────────────────
    // Caption text for an adjacent figure or table. Placed above (default)
    // or below its target; the media tag's own `caption` attribute is the
    // inline alternative for a single asset.
    tags.insert(
        "caption".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "position".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Placement relative to the figure/table: above (default) or below"
                                .to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "color".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Colour for the caption text".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Caption for an adjacent figure or table".to_string()),
        },
    );

    // ── input ───────────────────────────────────────────────────────────
    // A form input field. Attributes mirror the HTML input attributes so
    // they translate across renderers: the web maps them to a native
    // `<input>` with full constraint validation; PDF draws a print form box
    // (label + ruled field) since PDF/A forbids the JavaScript that dynamic
    // field validation would need. Author-time validation here only checks
    // the tag itself (attribute types); the field's own required/min/max
    // rules are enforced when a user fills the rendered form, not in markdoc.
    tags.insert(
        "input".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "name".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: true,
                        description: Some("Field identifier".to_string()),
                    },
                );
                attrs.insert(
                    "type".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some(
                            "Input type: text (default) | number | email | tel | url | date | checkbox"
                                .to_string(),
                        ),
                    },
                );
                attrs.insert(
                    "label".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Human-readable label shown beside the field".to_string()),
                    },
                );
                attrs.insert(
                    "required".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Boolean]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Whether a value must be supplied".to_string()),
                    },
                );
                attrs.insert(
                    "value".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String, ValidationType::Number]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Default / pre-filled value".to_string()),
                    },
                );
                attrs.insert(
                    "placeholder".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Placeholder hint shown in an empty field".to_string()),
                    },
                );
                attrs.insert(
                    "min".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number, ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Minimum value (number / date types)".to_string()),
                    },
                );
                attrs.insert(
                    "max".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number, ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Maximum value (number / date types)".to_string()),
                    },
                );
                attrs.insert(
                    "minlength".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Minimum length in characters (text types)".to_string()),
                    },
                );
                attrs.insert(
                    "maxlength".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Maximum length in characters (text types)".to_string()),
                    },
                );
                attrs.insert(
                    "pattern".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: None,
                        default: None,
                        required: false,
                        description: Some("Regular expression the value must match".to_string()),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: false,
            description: Some(
                "Form input field (HTML input on the web; a print form box in PDF)".to_string(),
            ),
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
        // `src` is optional now — a reference may instead carry `id`.
        assert!(!attrs.get("src").unwrap().required);
        assert!(!attrs.get("id").unwrap().required);
        // `img` is registered as an alias of `media`.
        assert!(cfg.tags.contains_key("img"));
        assert!(!attrs.get("alt").unwrap().required);
        assert!(!attrs.get("title").unwrap().required);
        assert!(!attrs.get("caption").unwrap().required);
        assert!(!attrs.get("kind").unwrap().required);
        // Gained for the `{% float %}` anchored/magazine mode.
        assert!(!attrs.get("side").unwrap().required);
        assert!(!attrs.get("width").unwrap().required);
    }

    #[test]
    fn default_config_has_layout_tags_registered() {
        let cfg = Config::default();
        for name in ["columns", "float", "color", "c", "list", "caption"] {
            assert!(cfg.tags.contains_key(name), "{name} tag registered");
        }
        // Float declares side/width/gap, all optional.
        let fattrs = cfg.tags["float"].attributes.as_ref().unwrap();
        for a in ["side", "width", "gap"] {
            assert!(!fattrs.get(a).unwrap().required, "float.{a} optional");
        }
        // columns is a block container, color is an inline span.
        assert!(!cfg.tags["columns"].inline);
        assert!(cfg.tags["color"].inline);
        assert!(
            cfg.tags["columns"]
                .attributes
                .as_ref()
                .unwrap()
                .contains_key("widths")
        );
    }

    #[test]
    fn layout_tags_validate_without_error() {
        // A representative document using the layout tags must not raise any
        // error-level validation issues against the default config.
        let src = "\
{% columns widths=\"2 1\" gap=16 %}\n* a\n* b\n{% /columns %}\n\n\
{% float side=\"left\" width=120 %}\n{% media src=\"x.png\" /%}\n\ntext\n{% /float %}\n\n\
Inline {% color value=\"#c026d3\" %}pink{% /color %} text.\n";
        let doc = parse(src, None).unwrap();
        let errors = crate::validator::validate(&doc, &Config::default());
        let hard: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e.level, crate::types::ValidationLevel::Error))
            .collect();
        assert!(hard.is_empty(), "unexpected validation errors: {hard:?}");
    }

    #[test]
    fn input_tag_schema_and_validation() {
        let cfg = Config::default();
        let input = cfg.tags.get("input").expect("input tag registered");
        let attrs = input.attributes.as_ref().unwrap();
        assert!(attrs["name"].required, "name is required");
        for a in [
            "type",
            "required",
            "min",
            "max",
            "minlength",
            "maxlength",
            "pattern",
        ] {
            assert!(!attrs[a].required, "input.{a} optional");
        }
        assert!(input.self_closing);

        // A well-formed field validates clean.
        let ok = parse(
            "{% input name=\"qty\" type=\"number\" required=true min=1 max=100 maxlength=3 /%}\n",
            None,
        )
        .unwrap();
        assert!(
            crate::validator::validate(&ok, &cfg)
                .iter()
                .all(|e| !matches!(e.level, crate::types::ValidationLevel::Error)),
            "valid input should not error"
        );

        // Missing the required `name` is flagged.
        let bad = parse("{% input type=\"text\" /%}\n", None).unwrap();
        assert!(
            crate::validator::validate(&bad, &cfg)
                .iter()
                .any(|e| e.id == "missing-attribute"),
            "missing name should be flagged"
        );
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
    fn media_accepts_id_instead_of_src() {
        // `src` is optional now — a reference may instead carry `id` — so
        // neither is flagged as missing. Holds for the `img` alias too.
        use crate::validator::validate;
        for src in [
            "{% media id=\"0a02bb82-1a68-4c5c-883f-406361e1235e\" /%}",
            "{% img id=\"0a02bb82-1a68-4c5c-883f-406361e1235e\" /%}",
        ] {
            let doc = parse(src, None).unwrap();
            let errors = validate(&doc, &Config::default());
            assert!(
                !errors.iter().any(|e| e.id == "missing-attribute"),
                "{src} should validate without a missing-attribute error, got: {errors:?}"
            );
        }
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
