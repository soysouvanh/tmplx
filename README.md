# Tmplx Workspace

[English](README.md) | [Français](README.fr.md)

A _Code-Gen-First_ HTML template engine for Rust, guaranteeing **zero dynamic memory allocation** at runtime and strict compile-time validation.

## Philosophy

Unlike traditional engines (Tera, Askama, Handlebars) that parse templates at runtime or interpolate strings, **Tmplx compiles your HTML mockups into very low-level native Rust functions (`output.push_str`)**.

- **Zero markup allocation**: Static parts are hardcoded into the Rust binary.
- **Zero runtime parsing**: Structural validity is guaranteed by `build.rs`.
- **Absolute typing**: Injected variables (`view_data`) are semantically checked at compile-time by Rust macros (Duck-Typing).

## Workspace Architecture

- `tmplx/` : Core crate containing the build engine logic (`build_logic/`) and generic traits.
- `tmplx-test/` : Integration test crate, validating the complete generation pipeline with `templates/`.
