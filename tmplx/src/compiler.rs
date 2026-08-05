//! Tmplx Compile-Time API (The Orchestrator).
//!
//! This module is the heart of the engine at **Compile-Time**. It is designed to be
//! invoked exactly once during `cargo build` via your `build.rs` script.
//!
//! # What it does
//! It loads all `.html` files in your template directory, analyzes them for security threats
//! (like infinite recursion or path traversal), and compiles them into **Zero-Allocation Rust Macros**.

use crate::build_logic::{classifier, generator, tokenizer, truncation, validator};

/// Compiles a standard Tmplx workspace into Rust macros.
///
/// This is your main entry point for compiling templates. You must call this from your `build.rs`.
///
/// # The 🛡️ 4-Pass Architecture
///
/// Under the hood, this function orchestrates a pipeline engineered for clarity and auditability:
/// 1. **Pass 1 (Tokenizer)**: Slices your raw HTML into safe lexical `Tokens` without copying memory.
/// 2. **Pass 2 (Validator)**: Semantically validates syntax and balances structures (`if`, `for`).
/// 3. **Pass 3 (Truncation)**: Applies meticulous whitespace stripping (`-` operators).
/// 4. **Pass 4 (Generator)**: Outputs pure, pre-sized Rust instructions directly into `template_gen.rs`.
///
/// # Panics (Security)
/// This function will intentionally panic and fail the build if it detects:
/// - Path traversal (`../`) in includes/extends.
/// - Unbounded recursion loops (`extends/include` depth > 10).
/// - Unbalanced tags (e.g., an `{% if %}` missing its `{% endif %}`).
///
/// # Arguments
/// - `template_dir`: Directory containing your standard `.html` files.
/// - `out_dir`     : Target directory for the generated Rust file.
pub fn build_workspace(template_dir: &std::path::Path, out_dir: &std::path::Path) {
    build_workspace_with_options(template_dir, out_dir, false)
}

/// Compiles a Tmplx workspace in **Strict Mode (Maximum Security)**.
///
/// Strict mode enforces military-grade XSS protection by completely **disabling raw outputs**.
///
/// If your project handles highly sensitive user data and you never need to inject pre-rendered HTML,
/// use this function instead of `build_workspace`.
///
/// # Behavior
/// - It behaves perfectly like [`build_workspace`].
/// - **BUT**: It will aggressively `panic!()` and fail the build if it encounters the `{%= %}` syntax in any template.
pub fn build_workspace_strict(template_dir: &std::path::Path, out_dir: &std::path::Path) {
    build_workspace_with_options(template_dir, out_dir, true)
}

struct CompileLimits {
    included_files: usize,
    max_included_files: usize,
    total_template_size: usize,
    max_template_size: usize,
}

struct SourceArena {
    sources: std::cell::UnsafeCell<Vec<Box<str>>>,
}

impl SourceArena {
    fn new() -> Self {
        Self {
            sources: std::cell::UnsafeCell::new(Vec::new()),
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn add(&self, source: String) -> &str {
        let boxed = source.into_boxed_str();
        let ptr = boxed.as_ref() as *const str;
        unsafe {
            (*self.sources.get()).push(boxed);
            &*ptr
        }
    }
}

fn build_workspace_with_options(
    template_dir: &std::path::Path,
    out_dir: &std::path::Path,
    deny_raw_output: bool,
) {
    println!("cargo:rerun-if-changed={}", template_dir.display());

    let mut entries = scan_templates(template_dir);
    entries.sort();

    let canonical_template_dir = template_dir
        .canonicalize()
        .unwrap_or_else(|e| panic!("tmplx build error: cannot canonicalize template root: {e}"));

    let mut limits = CompileLimits {
        included_files: 0,
        max_included_files: 10_000,
        total_template_size: 0,
        max_template_size: 104_857_600, // 100 MB limit
    };

    let mut final_code = String::new();
    let mut function_names = std::collections::HashSet::new();

    let arena = SourceArena::new();

    for html_path in entries {
        println!("cargo:rerun-if-changed={}", html_path.display());

        let source = std::fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("reading {:?} failed: {}", html_path, e));

        limits.total_template_size += source.len();
        if limits.total_template_size > limits.max_template_size {
            panic!("tmplx build error: total template source size exceeds maximum limit");
        }

        let leaked_source = arena.add(source);

        let tokens_extended = apply_extends(
            leaked_source,
            html_path.parent().unwrap(),
            &canonical_template_dir,
            0,
            &mut limits,
            &arena,
        );
        let tokens_stripped = strip_template_blocks(tokens_extended);
        let mut tokens = expand_includes_on_tokens(
            tokens_stripped,
            html_path.parent().unwrap(),
            &canonical_template_dir,
            0,
            &mut limits,
            &arena,
        );

        validator::validate_pairing(&tokens);
        truncation::apply_truncation(&mut tokens);

        if deny_raw_output {
            for tok in &tokens {
                if let tokenizer::Token::Tagged {
                    content,
                    delimiter,
                    line,
                    ..
                } = tok
                    && let classifier::ClassifiedTag::RawOutput { .. } =
                        classifier::classify(content, *delimiter, *line)
                {
                    panic!(
                        "tmplx build error line {line}: raw output ({{%= %}}) is forbidden in strict mode."
                    );
                }
            }
        }

        let fname = html_path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("-", "_")
            .replace(".", "_");
        let function_name = format!("render_{}", fname);

        if !function_names.insert(function_name.clone()) {
            panic!(
                "tmplx build error: function '{}' is already used.",
                function_name
            );
        }

        let generated =
            generator::generate(&tokens, &function_name, &html_path.display().to_string());
        final_code.push_str(&generated);
        final_code.push('\n');
    }

    let output_path = out_dir.join("template_gen.rs");
    std::fs::write(&output_path, &final_code)
        .unwrap_or_else(|e| panic!("writing to {output_path:?} failed: {e}"));
}

fn scan_templates(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut entries = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                entries.extend(scan_templates(&path));
            } else if let Some(ext) = path.extension()
                && ext == "html"
            {
                entries.push(path);
            }
        }
    }
    entries
}

fn apply_extends<'a>(
    source: &'a str,
    base_dir: &std::path::Path,
    template_root: &std::path::Path,
    depth: usize,
    limits: &mut CompileLimits,
    arena: &'a SourceArena,
) -> Vec<tokenizer::Token<'a>> {
    if depth > 10 {
        panic!("tmplx build error: inheritance too deep");
    }

    let tokens = tokenizer::tokenize(source);
    validator::validate_pairing(&tokens);

    let mut extends_path = None;
    let mut extends_line = 0;
    for tok in &tokens {
        if let tokenizer::Token::Tagged {
            content,
            delimiter: tokenizer::DelimiterType::Instruction,
            line,
            ..
        } = tok
            && let classifier::ClassifiedTag::Extends { path } =
                classifier::classify(content, tokenizer::DelimiterType::Instruction, *line)
        {
            extends_path = Some(path);
            extends_line = *line;
            break;
        }
    }

    if let Some(path) = extends_path {
        let mut child_blocks: std::collections::HashMap<&str, Vec<tokenizer::Token<'a>>> =
            std::collections::HashMap::new();
        let mut current_block_name = None;
        let mut current_block_tokens = Vec::new();
        let mut block_stack = Vec::new();
        let mut block_depth = 0;

        for tok in &tokens {
            if let tokenizer::Token::Tagged {
                content,
                delimiter: tokenizer::DelimiterType::Instruction,
                line,
                ..
            } = tok
            {
                let tag =
                    classifier::classify(content, tokenizer::DelimiterType::Instruction, *line);
                match tag {
                    classifier::ClassifiedTag::Block { name } => {
                        block_stack.push(true);
                        block_depth += 1;
                        if block_depth == 1 {
                            current_block_name = Some(name);
                            current_block_tokens = Vec::new();
                            continue;
                        }
                    }
                    classifier::ClassifiedTag::If { .. }
                    | classifier::ClassifiedTag::For { .. } => {
                        block_stack.push(false);
                    }
                    classifier::ClassifiedTag::EndBlock
                    | classifier::ClassifiedTag::EndIf
                    | classifier::ClassifiedTag::EndFor => {
                        if let Some(true) = block_stack.pop() {
                            block_depth -= 1;
                            if block_depth == 0 {
                                if let Some(name) = current_block_name.take() {
                                    child_blocks
                                        .insert(name, std::mem::take(&mut current_block_tokens));
                                }
                                continue;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if current_block_name.is_some() {
                current_block_tokens.push(tok.clone());
            }
        }

        let parent_path = resolve_template_path(base_dir, template_root, path, extends_line);
        println!("cargo:rerun-if-changed={}", parent_path.display());
        let parent_source = std::fs::read_to_string(&parent_path)
            .unwrap_or_else(|e| panic!("Inheritance error from {:?} : {}", parent_path, e));

        limits.total_template_size += parent_source.len();
        if limits.total_template_size > limits.max_template_size {
            panic!("tmplx build error: total template source size exceeds maximum limit");
        }

        let leaked_parent: &'a str = arena.add(parent_source);

        limits.included_files += 1;
        if limits.included_files > limits.max_included_files {
            panic!("tmplx build error: too many template inclusions");
        }

        let parent_tokens = apply_extends(
            leaked_parent,
            parent_path.parent().unwrap(),
            template_root,
            depth + 1,
            limits,
            arena,
        );

        let mut final_tokens = Vec::new();
        let mut skip_depth = 0;
        let mut parent_stack = Vec::new();
        let mut parent_depth = 0;

        for tok in parent_tokens {
            let mut skip_this_token = false;

            if let tokenizer::Token::Tagged {
                content,
                delimiter: tokenizer::DelimiterType::Instruction,
                line,
                ..
            } = tok
            {
                let tag =
                    classifier::classify(content, tokenizer::DelimiterType::Instruction, line);
                match tag {
                    classifier::ClassifiedTag::Block { name } => {
                        parent_stack.push(true);
                        parent_depth += 1;
                        if skip_depth == 0 {
                            final_tokens.push(tok.clone());
                            if let Some(override_tokens) = child_blocks.get(name) {
                                final_tokens.extend(override_tokens.iter().cloned());
                                skip_depth = 1;
                            }
                            skip_this_token = true;
                        } else {
                            skip_depth += 1;
                        }
                    }
                    classifier::ClassifiedTag::If { .. }
                    | classifier::ClassifiedTag::For { .. } => {
                        parent_stack.push(false);
                        if skip_depth > 0 {
                            skip_depth += 1;
                        }
                    }
                    classifier::ClassifiedTag::EndBlock
                    | classifier::ClassifiedTag::EndIf
                    | classifier::ClassifiedTag::EndFor => {
                        if let Some(true) = parent_stack.pop() {
                            parent_depth -= 1;
                            if parent_depth == 0 {
                                if skip_depth > 0 {
                                    skip_depth = 0;
                                }
                            } else if skip_depth > 0 {
                                skip_depth -= 1;
                            }
                        } else if skip_depth > 0 {
                            skip_depth -= 1;
                        }
                    }
                    _ => {}
                }
            }
            if !skip_this_token && skip_depth == 0 {
                final_tokens.push(tok);
            }
        }
        return final_tokens;
    }
    tokens
}

fn strip_template_blocks<'a>(tokens: Vec<tokenizer::Token<'a>>) -> Vec<tokenizer::Token<'a>> {
    let mut clean = Vec::new();
    let mut block_stack = Vec::new();
    for tok in tokens {
        let mut skip = false;
        if let tokenizer::Token::Tagged {
            content,
            delimiter: tokenizer::DelimiterType::Instruction,
            line,
            ..
        } = tok
        {
            let tag = classifier::classify(content, tokenizer::DelimiterType::Instruction, line);
            match tag {
                classifier::ClassifiedTag::Block { .. } => {
                    block_stack.push(true);
                    skip = true;
                }
                classifier::ClassifiedTag::If { .. } | classifier::ClassifiedTag::For { .. } => {
                    block_stack.push(false);
                }
                classifier::ClassifiedTag::EndBlock
                | classifier::ClassifiedTag::EndIf
                | classifier::ClassifiedTag::EndFor => {
                    if let Some(true) = block_stack.pop() {
                        skip = true;
                    }
                }
                classifier::ClassifiedTag::Extends { .. } => {
                    skip = true;
                }
                _ => {}
            }
        }
        if skip {
            let mut mutated_tok = tok.clone();
            if let tokenizer::Token::Tagged { delimiter, .. } = &mut mutated_tok {
                *delimiter = tokenizer::DelimiterType::Comment;
            }
            clean.push(mutated_tok);
        } else {
            clean.push(tok);
        }
    }
    clean
}

fn expand_includes_on_tokens<'a>(
    tokens: Vec<tokenizer::Token<'a>>,
    base_dir: &std::path::Path,
    template_root: &std::path::Path,
    depth: usize,
    limits: &mut CompileLimits,
    arena: &'a SourceArena,
) -> Vec<tokenizer::Token<'a>> {
    if depth > 10 {
        panic!("tmplx build error: include too deep");
    }
    let mut expanded = Vec::new();
    for tok in tokens {
        if let tokenizer::Token::Tagged {
            content,
            delimiter: tokenizer::DelimiterType::Instruction,
            line,
            ..
        } = &tok
        {
            let tag = classifier::classify(content, tokenizer::DelimiterType::Instruction, *line);
            if let classifier::ClassifiedTag::Include { path } = tag {
                let included_path = resolve_template_path(base_dir, template_root, path, *line);
                println!("cargo:rerun-if-changed={}", included_path.display());
                let inc_source = std::fs::read_to_string(&included_path).unwrap_or_else(|e| {
                    panic!(
                        "Include error from {:?} at line {} : {}",
                        included_path, line, e
                    )
                });

                limits.total_template_size += inc_source.len();
                if limits.total_template_size > limits.max_template_size {
                    panic!("tmplx build error: total template source size exceeds maximum limit");
                }

                let leaked: &'a str = arena.add(inc_source);

                limits.included_files += 1;
                if limits.included_files > limits.max_included_files {
                    panic!("tmplx build error: too many template inclusions");
                }

                let inc_tokens_extended = apply_extends(
                    leaked,
                    included_path.parent().unwrap(),
                    template_root,
                    0,
                    limits,
                    arena,
                );
                let inc_tokens_stripped = strip_template_blocks(inc_tokens_extended);
                let mut inc_tokens = expand_includes_on_tokens(
                    inc_tokens_stripped,
                    included_path.parent().unwrap(),
                    template_root,
                    depth + 1,
                    limits,
                    arena,
                );

                let mut mutated_tok = tok.clone();
                if let tokenizer::Token::Tagged { delimiter, .. } = &mut mutated_tok {
                    *delimiter = tokenizer::DelimiterType::Comment;
                }
                expanded.push(mutated_tok);

                expanded.append(&mut inc_tokens);
                continue;
            }
        }
        expanded.push(tok);
    }
    expanded
}

fn resolve_template_path(
    base_dir: &std::path::Path,
    template_root: &std::path::Path,
    raw_path: &str,
    line: usize,
) -> std::path::PathBuf {
    let raw_path = raw_path.trim_matches('"').trim_matches('\'');
    let requested = std::path::Path::new(raw_path);

    if requested.is_absolute() {
        panic!(
            "tmplx build error line {line}: absolute template path is forbidden: {:?}",
            raw_path
        );
    }

    if requested
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        panic!(
            "tmplx build error line {line}: parent directory traversal is forbidden in template path: {:?}",
            raw_path
        );
    }

    let joined = base_dir.join(requested);

    let canonical_path = joined.canonicalize().unwrap_or_else(|e| {
        panic!(
            "tmplx build error line {line}: cannot resolve template path {:?}: {e}",
            joined
        )
    });

    if !canonical_path.starts_with(template_root) {
        panic!(
            "tmplx build error line {line}: template path escapes the template root: {:?}",
            raw_path
        );
    }

    canonical_path
}
