use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub attributes: HashMap<String, Scalar>,
    /// Raw source for attribute values that are unresolved expressions
    /// (variables like `$config.foo` or function calls like `equals($a, $b)`).
    /// Populated by the tag parser; consumed by the transformer once an
    /// evaluation context is available. The key `"primary"` is reserved for
    /// the unkeyed expression in tags like `{% if $cfg.foo %}`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub expressions: HashMap<String, String>,
    pub children: Vec<Node>,
    pub errors: Vec<ValidationError>,
    pub lines: Vec<usize>,
    pub tag: Option<String>,
    pub annotations: Vec<AttributeValue>,
    pub inline: bool,
    pub location: Option<Location>,
    pub slots: HashMap<String, Node>,
}

impl Node {
    pub fn new(
        node_type: NodeType,
        attributes: HashMap<String, Scalar>,
        children: Vec<Node>,
        tag: Option<String>,
    ) -> Self {
        Self {
            node_type,
            attributes,
            expressions: HashMap::new(),
            children,
            tag,
            errors: Vec::new(),
            lines: Vec::new(),
            annotations: Vec::new(),
            inline: false,
            location: None,
            slots: HashMap::new(),
        }
    }

    pub fn push(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn walk(&self) -> impl Iterator<Item = &Node> {
        let mut nodes = Vec::new();
        self.walk_recursive(&mut nodes);
        nodes.into_iter()
    }

    fn walk_recursive<'a>(&'a self, nodes: &mut Vec<&'a Node>) {
        for slot in self.slots.values() {
            nodes.push(slot);
            slot.walk_recursive(nodes);
        }
        for child in &self.children {
            nodes.push(child);
            child.walk_recursive(nodes);
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn resolve(&self, config: &Config) -> Result<Node> {
        let mut resolved = self.clone();

        let mut resolved_children = Vec::new();
        for child in &self.children {
            resolved_children.push(child.resolve(config)?);
        }
        resolved.children = resolved_children;

        let mut resolved_slots = HashMap::new();
        for (name, slot) in &self.slots {
            resolved_slots.insert(name.clone(), slot.resolve(config)?);
        }
        resolved.slots = resolved_slots;

        Ok(resolved)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub path: Vec<String>,
}

impl Variable {
    pub fn new(name: String) -> Self {
        let path = name.split('.').map(|s| s.to_string()).collect();
        Self { name, path }
    }

    pub fn resolve(&self, config: &Config) -> Option<Scalar> {
        let mut current = config.variables.get(&self.path[0])?;

        for key in &self.path[1..] {
            match current {
                Scalar::Object(map) => {
                    current = map.get(key)?;
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Scalar>,
}

impl Function {
    pub fn new(name: String, parameters: Vec<Scalar>) -> Self {
        Self { name, parameters }
    }
}
