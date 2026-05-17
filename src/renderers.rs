use crate::types::*;

pub mod html;

pub fn render_to_html(node: &RenderableTreeNode) -> String {
    html::render(node)
}
