use crate::ast::Node;
use crate::types::*;

pub fn validate(node: &Node, config: &Config) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    validate_node(node, config, &mut errors);
    errors
}

fn validate_node(node: &Node, config: &Config, errors: &mut Vec<ValidationError>) {
    // Add existing node errors
    errors.extend(node.errors.clone());

    // Find schema
    let schema = find_schema(node, config);

    // Validate attributes
    if let Some(schema) = &schema {
        validate_attributes(node, schema, errors);
        validate_children(node, schema, errors);
    }

    // Recursively validate children
    for child in &node.children {
        validate_node(child, config, errors);
    }

    // Validate slots
    for slot in node.slots.values() {
        validate_node(slot, config, errors);
    }
}

fn find_schema(node: &Node, config: &Config) -> Option<Schema> {
    if let Some(tag) = &node.tag
        && let Some(schema) = config.tags.get(tag)
    {
        return Some(schema.clone());
    }

    let node_key = node.node_type.to_string();
    config.nodes.get(&node_key).cloned()
}

fn validate_attributes(node: &Node, schema: &Schema, errors: &mut Vec<ValidationError>) {
    if let Some(schema_attrs) = &schema.attributes {
        // Check required attributes
        for (attr_name, attr_schema) in schema_attrs {
            if attr_schema.required && !node.attributes.contains_key(attr_name) {
                errors.push(ValidationError {
                    id: "missing-attribute".to_string(),
                    level: ValidationLevel::Error,
                    message: format!(
                        "Required attribute '{}' is missing from '{}'",
                        attr_name,
                        node.tag.as_deref().unwrap_or(&node.node_type.to_string())
                    ),
                    location: node.location.clone(),
                });
            }
        }

        // Validate attribute types
        for (attr_name, attr_value) in &node.attributes {
            if let Some(attr_schema) = schema_attrs.get(attr_name)
                && let Some(valid_types) = &attr_schema.attr_type
                && !validate_type(attr_value, valid_types)
            {
                errors.push(ValidationError {
                    id: "invalid-attribute-type".to_string(),
                    level: ValidationLevel::Error,
                    message: format!(
                        "Attribute '{}' has invalid type for '{}'",
                        attr_name,
                        node.tag.as_deref().unwrap_or(&node.node_type.to_string())
                    ),
                    location: node.location.clone(),
                });
            }
        }
    }
}

fn validate_children(node: &Node, schema: &Schema, errors: &mut Vec<ValidationError>) {
    if let Some(allowed_children) = &schema.children {
        for child in &node.children {
            let child_type_string = child.node_type.to_string();
            let child_type = child.tag.as_deref().unwrap_or(&child_type_string);
            if !allowed_children.contains(&child_type.to_string()) {
                errors.push(ValidationError {
                    id: "invalid-child".to_string(),
                    level: ValidationLevel::Warning,
                    message: format!(
                        "Node '{}' is not allowed as child of '{}'",
                        child_type,
                        node.tag.as_deref().unwrap_or(&node.node_type.to_string())
                    ),
                    location: child.location.clone(),
                });
            }
        }
    }
}

fn validate_type(value: &Scalar, valid_types: &[ValidationType]) -> bool {
    for valid_type in valid_types {
        let matches = matches!(
            (value, valid_type),
            (Scalar::String(_), ValidationType::String)
                | (Scalar::Number(_), ValidationType::Number)
                | (Scalar::Boolean(_), ValidationType::Boolean)
                | (Scalar::Array(_), ValidationType::Array)
                | (Scalar::Object(_), ValidationType::Object)
                | (Scalar::Null, _)
        );

        if matches {
            return true;
        }
    }

    false
}
