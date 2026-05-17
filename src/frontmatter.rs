use crate::types::*;
use std::collections::HashMap;

/// Extract and parse YAML frontmatter from markdown content
pub fn extract_frontmatter(content: &str) -> Result<(Option<HashMap<String, Scalar>>, String)> {
    let lines: Vec<&str> = content.lines().collect();

    // Check if content starts with frontmatter delimiter
    if lines.is_empty() || lines[0].trim() != "---" {
        return Ok((None, content.to_string()));
    }

    // Find closing delimiter
    let mut end_line = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_line = Some(i);
            break;
        }
    }

    let Some(end) = end_line else {
        // No closing delimiter found, treat as regular content
        return Ok((None, content.to_string()));
    };

    // Extract frontmatter content
    let frontmatter_content = lines[1..end].join("\n");

    // Parse YAML
    let frontmatter: HashMap<String, serde_yaml::Value> =
        serde_yaml::from_str(&frontmatter_content)
            .map_err(|e| MarkdocError::ParseError(format!("Invalid YAML frontmatter: {}", e)))?;

    // Convert to Scalar
    let frontmatter_scalar = frontmatter
        .into_iter()
        .map(|(k, v)| (k, yaml_to_scalar(v)))
        .collect();

    // Get remaining content
    let remaining_content = lines[(end + 1)..].join("\n");

    Ok((Some(frontmatter_scalar), remaining_content))
}

fn yaml_to_scalar(value: serde_yaml::Value) -> Scalar {
    match value {
        serde_yaml::Value::Null => Scalar::Null,
        serde_yaml::Value::Bool(b) => Scalar::Boolean(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Scalar::Number(i as f64)
            } else if let Some(f) = n.as_f64() {
                Scalar::Number(f)
            } else {
                Scalar::Null
            }
        }
        serde_yaml::Value::String(s) => Scalar::String(s),
        serde_yaml::Value::Sequence(seq) => {
            Scalar::Array(seq.into_iter().map(yaml_to_scalar).collect())
        }
        serde_yaml::Value::Mapping(map) => Scalar::Object(
            map.into_iter()
                .filter_map(|(k, v)| {
                    if let serde_yaml::Value::String(key) = k {
                        Some((key, yaml_to_scalar(v)))
                    } else {
                        None
                    }
                })
                .collect(),
        ),
        _ => Scalar::Null,
    }
}
