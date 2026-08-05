//! # Tmplx: Zero-Allocation, Statically Compiled Template Engine
//!
//! **Tmplx** is an HTML template engine engineered for extreme performance (State of the Art)
//! and absolute security.
//!
//! Unlike traditional engines that read files and parse logic dynamically at runtime (causing memory overhead),
//! **Tmplx translates your HTML templates directly into highly optimized Rust macros during your project's compilation phase** (via `build.rs`).
//!
//! ## 🧠 Mental Model: How it works (Pedagogical Overview)
//!
//! 1. **Compile-Time (You write HTML)**
//!    You write standard HTML enriched with `{% %}` syntax.
//!    During `cargo build`, Tmplx reads your HTML files, parses them securely, blocks any vulnerabilities (like Path Traversal), and translates them into pure `Rust` code.
//!
//! 2. **Runtime (Your Server runs)**
//!    Your server imports the generated Rust. Rendering a template is exactly as fast as calling a pre-compiled `output.push_str("<html>...")` function.
//!    **Result: Zero bytes of heap memory allocated** dynamically.
//!
//! ## 🚀 Quickstart Example
//!
//! **1. Create your template (`templates/hello.html`)**
//! ```html
//! <h1>Hello {%= name %}</h1>
//! ```
//!
//! **2. Compile it (`build.rs`)**
//! ```rust,ignore
//! fn main() {
//!     let template_dir = std::path::Path::new("templates");
//!     let out_dir = std::path::Path::new(&std::env::var("OUT_DIR").unwrap());
//!     // This generates the `render_hello!` macro automatically.
//!     tmplx::compiler::build_workspace_strict(template_dir, out_dir);
//! }
//! ```
//!
//! **3. Run it (`src/main.rs`)**
//! ```rust,ignore
//! // Output buffer, pre-sized to exactly the right capacity.
//! let mut html_output = String::with_capacity(1024);
//! render_hello!(&mut html_output, &view_data);
//! // Boom! Rendering completed instantly without any hidden memory allocation.
//! ```
//!
//! ## 🛡️ Security by Default (Contextual Escaping)
//!
//! Cross-Site Scripting (XSS) is prevented by an explicit, context-aware syntax:
//! - `{%= your_var %}` : **Raw output**, explicitly requested by you. (Forbidden in strict mode).
//! - `{%%= your_var %}` : **HTML Escaping**, safe for generic text.
//! - `{%js= your_var %}` : **JavaScript Escaping**, safe inside `<script>` tags.
//! - `{%url= your_var %}` : **URL Encoding**, safe for `href="..."` attributes.
//!
//! *Pedagogical note: We enforce explicit escaping tags so you always remain conscious of the data's destination context.*
//!
//! ## 🏛️ Framework Architecture
//!
//! The codebase is cleanly separated to enforce the mental model:
//! - **Compile-Time ([`compiler`])**: The heavy lifting. Lexing, semantic validation, and macro generation.
//! - **Runtime ([`tmplx_runtime`])**: A tiny, ultra-optimized module deployed alongside your binary, exclusively containing zero-allocation XSS mitigations.

pub mod build_logic;
pub mod compiler;
pub mod tmplx_runtime;
