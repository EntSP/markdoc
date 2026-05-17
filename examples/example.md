# Markdoc Rust Example

This is a **Markdoc** document written in _Markdown_.

## Features

- Parse Markdown with pulldown-cmark
- Transform to AST
- Validate content
- Render to HTML
- Generate PDFs (server-side)

## Code Example

```rust
use markdoc_core::{parse, transform};

let content = "# Hello World";
let node = parse(content, None)?;
let rendered = transform(&node, &config)?;
```

## Links

Check out the [Markdoc website](https://markdoc.dev) for more information.

---

Built with ♡ using Rust.
