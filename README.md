# Tmplx workspace

[English](README.md) | [Français](README.fr.md)

A _Code-Gen-First_ HTML template engine for Rust, guaranteeing **zero dynamic memory allocation** at runtime and strict compile-time validation.

## Philosophy

Unlike traditional engines (Tera, Askama, Handlebars) that parse templates at runtime or interpolate strings, **Tmplx compiles your HTML mockups into very low-level native Rust functions (`output.push_str`)**.

- **Zero markup allocation**: Static parts are hardcoded into the Rust binary.
- **Zero runtime parsing**: Structural validity is guaranteed by `build.rs`.
- **Absolute typing**: Injected variables (`view_data`) are semantically checked at compile-time by Rust macros (Duck-Typing).

## Workspace architecture

Here is how the Tmplx ecosystem is organized:

```text
tmplx-workspace/
├── tmplx/                  # The core engine (published on crates.io)
│   ├── build_logic/        # Compilation logic (parsing, tokenization, code generation)
│   ├── src/                # Runtime code (macro definitions, security, duck-typing)
│   └── templates/          # Internal templates (mockups for system integration)
│
└── tmplx-test/             # The showcase project (Living documentation)
    ├── benches/            # Load testing algorithms and performance benchmarking
    ├── src/                # Executable examples and full integration tests
    └── templates/          # Template use-cases (inheritance, logic, local assignments)
        └── partials/       # Reusable web components demonstration
```

### Running the test suite (Living documentation)

To verify the absolute reliability of the engine and explore the advanced examples (modular architecture, local assignment, truncation limits), you can execute the integration tests:

```bash
cargo test -p tmplx-test
```
