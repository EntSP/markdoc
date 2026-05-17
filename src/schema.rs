use crate::types::*;
use std::collections::HashMap;

pub fn default_nodes() -> HashMap<String, Schema> {
    let mut nodes = HashMap::new();

    // Document
    nodes.insert(
        "document".to_string(),
        Schema {
            render: None,
            children: None,
            attributes: None,
            self_closing: false,
            inline: false,
            description: Some("Root document node".to_string()),
        },
    );

    // Heading
    nodes.insert(
        "heading".to_string(),
        Schema {
            render: None, // Will be dynamically set to h1-h6
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "level".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Number]),
                        render: Some(SchemaRender::Bool(false)),
                        default: Some(Scalar::Number(1.0)),
                        required: true,
                        description: Some("Heading level (1-6)".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Heading node".to_string()),
        },
    );

    // Paragraph
    nodes.insert(
        "paragraph".to_string(),
        Schema {
            render: Some("p".to_string()),
            children: None,
            attributes: None,
            self_closing: false,
            inline: false,
            description: Some("Paragraph node".to_string()),
        },
    );

    // Link
    nodes.insert(
        "link".to_string(),
        Schema {
            render: Some("a".to_string()),
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "href".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: Some(SchemaRender::Bool(true)),
                        default: None,
                        required: true,
                        description: Some("Link URL".to_string()),
                    },
                );
                attrs.insert(
                    "title".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: Some(SchemaRender::Bool(true)),
                        default: None,
                        required: false,
                        description: Some("Link title".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: true,
            description: Some("Link node".to_string()),
        },
    );

    // Image
    nodes.insert(
        "image".to_string(),
        Schema {
            render: Some("img".to_string()),
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "src".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: Some(SchemaRender::Bool(true)),
                        default: None,
                        required: true,
                        description: Some("Image source URL".to_string()),
                    },
                );
                attrs.insert(
                    "alt".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: Some(SchemaRender::Bool(true)),
                        default: None,
                        required: false,
                        description: Some("Alternative text".to_string()),
                    },
                );
                attrs
            }),
            self_closing: true,
            inline: true,
            description: Some("Image node".to_string()),
        },
    );

    // List
    nodes.insert(
        "list".to_string(),
        Schema {
            render: None, // Will be ol or ul
            children: Some(vec!["item".to_string()]),
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "ordered".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::Boolean]),
                        render: Some(SchemaRender::Bool(false)),
                        default: Some(Scalar::Boolean(false)),
                        required: false,
                        description: Some("Whether list is ordered".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("List node".to_string()),
        },
    );

    // Code block
    nodes.insert(
        "fence".to_string(),
        Schema {
            render: Some("pre".to_string()),
            children: None,
            attributes: Some({
                let mut attrs = HashMap::new();
                attrs.insert(
                    "language".to_string(),
                    SchemaAttribute {
                        attr_type: Some(vec![ValidationType::String]),
                        render: Some(SchemaRender::String("data-language".to_string())),
                        default: None,
                        required: false,
                        description: Some("Code language".to_string()),
                    },
                );
                attrs
            }),
            self_closing: false,
            inline: false,
            description: Some("Code fence node".to_string()),
        },
    );

    nodes
}
