use markdoc::{parse, renderers::html, transform, types::Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example markdown content
    let markdown = r#"
# Hello World

This is a **Markdoc** example written in _Markdown_.

## Features

- Parse Markdown with pulldown-cmark
- Transform to renderable tree
- Validate content
- Render to HTML

## Code Example

```rust
fn main() {
    println!("Hello, Markdoc!");
}
```

[Visit Markdoc](https://markdoc.dev)
"#;

    // Parse the markdown
    println!("Parsing markdown...");
    let ast = parse(markdown, None)?;

    // Transform with default config
    println!("Transforming AST...");
    let config = Config::default();
    let rendered = transform(&ast, &config)?;

    // Render to HTML
    println!("Rendering to HTML...");
    let html = html::render(&rendered);

    println!("\n=== Generated HTML ===\n");
    println!("{}", html);

    Ok(())
}
