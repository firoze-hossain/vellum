# Vellum Studio 📜

A modern, high-performance PDF reader, editor, and writer built in pure Rust.

## Features

- **Pure Rust Engine**: Built with zero external C/C++ or system rendering dependencies.
- **Conforming PDF 1.7 Processor**: Parses, modifies, and serializes ISO 32000-1 compliant PDF documents.
- **RFC 1950/1951 Deflate/Inflate Codec**: Fast in-memory stream compression and decompression.
- **In-Place Editing**: Modify existing text spans, insert new content, and manage document objects with undo/redo history.
- **Annotation & Markup**: Highlight, underline, strikethrough, freehand drawing, vector shapes, notes, and redactions.
- **Fast Search**: In-memory text extraction, bounding box hit computation, and jump navigation.
- **Clean User Interface**: Tabbed document view, interactive thumbnail sidebar, table of contents, and night reading mode.
- **NovaOS Integration**: Default document studio for NovaOS 2.0 with desktop, window manager, and shell support.

## Architecture

```
vellum/
├── crates/
│   ├── vellum-core/    # Parser, compressor, renderer, font metrics, serializer
│   ├── vellum-ui/      # Viewport, toolbars, sidebar, dialogs, drawing engine
│   └── vellum-app/     # CLI driver, PNG exporter, sample document generator
├── novaos/             # Bare-metal userland integration (VELLUM.ELF / PDF.ELF)
└── assets/             # Sample PDF assets
```

## Quickstart

```bash
# Build
cargo build --release

# Run unit and integration tests
cargo test --workspace

# Inspect document
./target/release/vellum --info assets/novaos_architecture.pdf

# Extract text
./target/release/vellum --extract-text assets/novaos_architecture.pdf

# Export page to image
./target/release/vellum --export-png assets/novaos_architecture.pdf /tmp/page.png
```

## License

MIT OR Apache-2.0
