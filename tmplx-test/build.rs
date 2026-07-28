// build.rs — Cargo watch contract (§5.2) + the four passes of
// §5.3 (tokenization, validation, truncation, resolution + generation)
// + manifest (§4.2). Writes `$OUT_DIR/template_gen.rs`.

#[path = "../tmplx/build_logic/classifier.rs"]
mod classifier;
#[path = "../tmplx/build_logic/generator.rs"]
mod generator;

#[path = "../tmplx/build_logic/tokenizer.rs"]
mod tokenizer;
#[path = "../tmplx/build_logic/truncation.rs"]
mod truncation;
#[path = "../tmplx/build_logic/validator.rs"]
mod validator;

fn main() {
    println!("cargo:rerun-if-changed=templates");

    let template_dir = std::path::Path::new("templates");
    let mut entries = scan_templates(template_dir);
    entries.sort();

    let mut final_code = String::new();
    let mut function_names = std::collections::HashSet::new();

    for html_path in entries {
        println!("cargo:rerun-if-changed={}", html_path.display());

        let source = std::fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("reading {:?} failed: {}", html_path, e));
        let leaked_source: &str = Box::leak(source.into_boxed_str());

        let tokens_extended = apply_extends(leaked_source, html_path.parent().unwrap(), 0);
        let tokens_stripped = strip_template_blocks(tokens_extended);
        let mut tokens = expand_includes_on_tokens(tokens_stripped, html_path.parent().unwrap(), 0);

        validator::validate_pairing(&tokens);
        truncation::apply_truncation(&mut tokens);

        let fname = html_path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace("-", "_");
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

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not defined by Cargo");
    let output_path = std::path::Path::new(&out_dir).join("template_gen.rs");
    std::fs::write(&output_path, &final_code)
        .unwrap_or_else(|e| panic!("writing to {output_path:?} failed: {e}"));

    // --- Extra generation, from synthetic
    // templates (not templates/mockup.html), to provide
    // "actually compiles and runs" coverage to §12 cases that the
    // reference mockup doesn't exercise: case 10 (nested `{% for %}`
    // via a field, item.sub_list) and case 12 (`{% if !ident %}`).
    // Written to a SEPARATE file: template_gen.rs (§10.3 golden test)
    // is untouched.
    let extra_template = "{%- for g in groups %}{%- for m in g.members %}[{%= m.name %}]{%- endfor %}{%- endfor %}{% if !is_active %}(INACTIVE){% endif %}";
    let mut tokens_extra = tokenizer::tokenize(extra_template);
    validator::validate_pairing(&tokens_extra);
    truncation::apply_truncation(&mut tokens_extra);
    let code_extra = generator::generate(&tokens_extra, "render_extra_12", "extra_template");
    let extra_path = std::path::Path::new(&out_dir).join("extra_tests_gen.rs");
    std::fs::write(&extra_path, &code_extra)
        .unwrap_or_else(|e| panic!("writing {extra_path:?} failed: {e}"));
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
                && !path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("askama")
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
    depth: usize,
) -> Vec<tokenizer::Token<'a>> {
    if depth > 10 {
        panic!("tmplx build error: inheritance too deep");
    }

    let tokens = tokenizer::tokenize(source);
    validator::validate_pairing(&tokens);

    let mut extends_path = None;
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
            break;
        }
    }

    if let Some(path) = extends_path {
        let mut child_blocks: std::collections::HashMap<&str, Vec<tokenizer::Token<'a>>> =
            std::collections::HashMap::new();
        let mut current_block_name = None;
        let mut current_block_tokens = Vec::new();
        let mut block_stack = Vec::new();

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
                        if block_stack.iter().filter(|&&b| b).count() == 1 {
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
                        if let Some(true) = block_stack.pop()
                            && block_stack.iter().filter(|&&b| b).count() == 0
                        {
                            if let Some(name) = current_block_name.take() {
                                child_blocks
                                    .insert(name, std::mem::take(&mut current_block_tokens));
                            }
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            if current_block_name.is_some() {
                current_block_tokens.push(tok.clone());
            }
        }

        let parent_path = base_dir.join(path);
        println!("cargo:rerun-if-changed={}", parent_path.display());
        let parent_source = std::fs::read_to_string(&parent_path)
            .unwrap_or_else(|e| panic!("Inheritance error from {:?} : {}", parent_path, e));
        let leaked_parent: &'a str = Box::leak(parent_source.into_boxed_str());

        let parent_tokens = apply_extends(leaked_parent, parent_path.parent().unwrap(), depth + 1);

        let mut final_tokens = Vec::new();
        let mut skip_depth = 0;
        let mut parent_stack = Vec::new();

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
                            if parent_stack.iter().filter(|&&b| b).count() == 0 {
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
    depth: usize,
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
                let included_path = base_dir.join(path);
                println!("cargo:rerun-if-changed={}", included_path.display());
                let inc_source = std::fs::read_to_string(&included_path).unwrap_or_else(|e| {
                    panic!(
                        "Include error from {:?} at line {} : {}",
                        included_path, line, e
                    )
                });
                let leaked: &'a str = Box::leak(inc_source.into_boxed_str());
                let inc_tokens_extended = apply_extends(leaked, included_path.parent().unwrap(), 0);
                let inc_tokens_stripped = strip_template_blocks(inc_tokens_extended);
                let mut inc_tokens = expand_includes_on_tokens(
                    inc_tokens_stripped,
                    included_path.parent().unwrap(),
                    depth + 1,
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
