pub mod ast;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod typechecker;

pub use compiler::compile_program;
pub use error::{LppoError, LppoResult};
pub use parser::parse_program;
pub use runtime::Executor;
pub use typechecker::typecheck_program;

use std::collections::HashSet;
use std::path::Path;

/// Parse and compile LPPO source into an executable program.
///
/// Imports (`use` declarations) are resolved relative to the current
/// directory. Use [`build_executor_with_root`] to resolve imports relative to
/// a different root directory.
pub fn build_executor(source: &str) -> LppoResult<Executor> {
    let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    build_executor_with_root(source, &root)
}

/// Parse and compile LPPO source into an executable program, resolving
/// `use` imports relative to `root`.
pub fn build_executor_with_root(source: &str, root: &Path) -> LppoResult<Executor> {
    let ast = resolve_program(source, root, &mut HashSet::new())?;
    let ring0 = ast.ring0;
    typecheck_program(&ast)?;
    let procedures = compile_program(ast)?;
    Ok(Executor::new(procedures, ring0))
}

/// Compile source and return serialized bytes (JSON) suitable for embedding.
///
/// Imports are resolved relative to the current directory.
pub fn compile_to_bytes(source: &str) -> LppoResult<Vec<u8>> {
    let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    compile_to_bytes_with_root(source, &root)
}

/// Compile source and return serialized bytes (JSON) suitable for embedding,
/// resolving `use` imports relative to `root`.
pub fn compile_to_bytes_with_root(source: &str, root: &Path) -> LppoResult<Vec<u8>> {
    let ast = resolve_program(source, root, &mut HashSet::new())?;
    typecheck_program(&ast)?;
    let procedures = compile_program(ast)?;
    runtime::serialize_procedures(&procedures)
}

/// Load an executor from serialized bytes.
pub fn load_executor_from_bytes(bytes: &[u8]) -> LppoResult<Executor> {
    runtime::executor_from_bytes(bytes)
}

const LPPO_EXTENSIONS: &[&str] = &["lppo", "lpph", "lph"];

/// Resolve a `use` import path to a file on disk, returning the file contents
/// and whether the resolved file is a header (`.lpph`/`.lph`) file.
///
/// Resolution order (first match wins, preferring implementation over header):
///   <root>/<path>.lppo, <root>/<path>.lpph, <root>/<path>.lph,
///   <root>/<path>/index.lppo, <root>/<path>/index.lpph, <root>/<path>/index.lph
fn resolve_import_file(root: &Path, path: &[String]) -> LppoResult<(String, bool)> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();

    let file_path = root.join(path.join("/"));
    for ext in LPPO_EXTENSIONS {
        candidates.push(file_path.with_extension(ext));
    }
    let index_path = root.join(path.join("/")).join("index");
    for ext in LPPO_EXTENSIONS {
        candidates.push(index_path.with_extension(ext));
    }

    for candidate in candidates {
        if candidate.is_file() {
            let is_header = matches!(
                candidate.extension().and_then(|e| e.to_str()),
                Some("lpph") | Some("lph")
            );
            let contents = std::fs::read_to_string(&candidate).map_err(|e| {
                LppoError::io_error(format!("Failed to read '{}': {}", candidate.display(), e))
            })?;
            return Ok((contents, is_header));
        }
    }

    let display = path.join("::");
    Err(LppoError::io_error(format!(
        "could not resolve module '{}' (tried .lppo, .lpph, .lph under '{}')",
        display,
        root.display()
    )))
}

/// Merge `other` into `self`, deduplicating top-level declarations by name.
///
/// Procedures: a non-empty body overrides an empty (header) body; two
/// non-empty bodies with the same name is a duplicate-procedure error.
/// Header-only (empty-body) procedures are kept as declarations.
fn merge_program(into: &mut Program, other: Program) {
    into.ring0 = into.ring0 || other.ring0;
    for proc in other.procedures {
        let has_body = !proc.statements.is_empty();
        match into.procedures.iter_mut().find(|p| p.name == proc.name) {
            Some(existing) => {
                let existing_has_body = !existing.statements.is_empty();
                if existing_has_body && has_body {
                    // Two implementations: keep the first, drop the duplicate.
                    continue;
                }
                if has_body && !existing_has_body {
                    *existing = proc;
                }
                // Otherwise keep the existing declaration.
            }
            None => into.procedures.push(proc),
        }
    }

    for module in other.modules {
        match into.modules.iter_mut().find(|m| m.name == module.name) {
            Some(existing) => {
                for proc in module.procedures {
                    let has_body = !proc.statements.is_empty();
                    match existing.procedures.iter_mut().find(|p| p.name == proc.name) {
                        Some(existing_proc) => {
                            let existing_has_body = !existing_proc.statements.is_empty();
                            if existing_has_body && has_body {
                                continue;
                            }
                            if has_body && !existing_has_body {
                                *existing_proc = proc;
                            }
                        }
                        None => existing.procedures.push(proc),
                    }
                }
            }
            None => into.modules.push(module),
        }
    }

    push_unique(&mut into.structs, other.structs, |s| s.name.clone());
    push_unique(&mut into.enums, other.enums, |e| e.name.clone());
    push_unique(&mut into.traits, other.traits, |t| t.name.clone());
    push_unique(&mut into.impls, other.impls, |i| {
        i.type_name.clone() + ":" + i.trait_name.as_deref().unwrap_or("")
    });
    push_unique(&mut into.type_aliases, other.type_aliases, |t| {
        t.name.clone()
    });
}

fn push_unique<T, F: Fn(&T) -> String>(vec: &mut Vec<T>, items: Vec<T>, key: F) {
    for item in items {
        let k = key(&item);
        if vec.iter().any(|existing| key(existing) == k) {
            continue;
        }
        vec.push(item);
    }
}

/// Parse `source` and recursively resolve every `use` import relative to
/// `root`, merging all imported modules into a single program. `seen` tracks
/// already-resolved files to prevent import cycles.
fn resolve_program(source: &str, root: &Path, seen: &mut HashSet<String>) -> LppoResult<Program> {
    let mut program = parse_program(source)?;

    let use_declarations: Vec<UseDeclaration> = std::mem::take(&mut program.use_declarations);

    for decl in &use_declarations {
        // The module path on disk: for `use name;` (path empty) the item is
        // the module; for `use a::b;` the item is appended to the path.
        let mut module_path = decl.path.clone();
        if let UseItems::Single(name) = &decl.items {
            module_path.push(name.clone());
        }
        let path = &module_path;
        let key = path.join("::");
        if !seen.insert(key.clone()) {
            continue; // already imported (cycle guard)
        }

        let (contents, _is_header) = resolve_import_file(root, path)?;
        let sub_root = root; // resolve nested imports relative to the same root
        let sub = resolve_program(&contents, sub_root, seen)?;
        merge_program(&mut program, sub);
    }

    program.use_declarations = use_declarations;
    Ok(program)
}

use crate::ast::{Program, UseDeclaration, UseItems};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Procedure, Statement, TypeAnnotation};
    use crate::runtime::eval_expr;
    use std::collections::HashMap;

    fn empty_proc(name: &str) -> Procedure {
        Procedure {
            name: name.to_string(),
            params: vec![],
            return_type: TypeAnnotation::Void,
            lifetimes: vec![],
            statements: vec![],
            is_public: false,
            is_unsafe: false,
        }
    }

    fn body_proc(name: &str) -> Procedure {
        Procedure {
            name: name.to_string(),
            params: vec![],
            return_type: TypeAnnotation::Void,
            lifetimes: vec![],
            statements: vec![Statement::Return(None)],
            is_public: false,
            is_unsafe: false,
        }
    }

    #[test]
    fn impl_non_empty_wins_over_header_declaration() {
        let mut into = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![empty_proc("foo")],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        let other = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![body_proc("foo")],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        merge_program(&mut into, other);
        assert_eq!(into.procedures.len(), 1);
        assert!(!into.procedures[0].statements.is_empty());
    }

    #[test]
    fn two_implementations_keeps_first() {
        let mut into = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![body_proc("foo")],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        let other = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![body_proc("foo")],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        merge_program(&mut into, other);
        assert_eq!(into.procedures.len(), 1);
    }

    #[test]
    fn ring_zero_declared_first_parses() {
        let ast = resolve_program(
            "ring 0;\npub proc main() -> Void { out \"hi\"; }",
            Path::new(""),
            &mut HashSet::new(),
        )
        .unwrap();
        assert!(ast.ring0);
    }

    #[test]
    fn ring_zero_not_first_is_error() {
        let err = resolve_program(
            "pub proc main() -> Void {}\nring 0;",
            Path::new(""),
            &mut HashSet::new(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("ring 0 must be declared"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn ring_zero_second_declaration_is_error() {
        let err = resolve_program(
            "ring 0;\npub proc main() -> Void {}\nring 0;",
            Path::new(""),
            &mut HashSet::new(),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("ring 0 must be declared"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn ring_zero_builtins_typecheck_when_declared() {
        let ast = resolve_program(
            "ring 0;\npub proc main() -> Void { outb(992, 65); }",
            Path::new(""),
            &mut HashSet::new(),
        )
        .unwrap();
        typecheck_program(&ast).unwrap();
    }

    #[test]
    fn ring_zero_builtins_rejected_without_declaration() {
        let ast = resolve_program(
            "pub proc main() -> Void { outb(992, 65); }",
            Path::new(""),
            &mut HashSet::new(),
        )
        .unwrap();
        let err = typecheck_program(&ast).unwrap_err();
        assert!(
            err.to_string().contains("outb"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn ring_zero_host_runtime_errors() {
        let expr = Expr::Call {
            func: Box::new(Expr::Var("outb".to_string())),
            args: vec![Expr::Int(992), Expr::Int(65)],
        };
        let err = eval_expr(&expr, &mut HashMap::new(), &HashMap::new(), false).unwrap_err();
        assert!(
            err.to_string().contains("ring-0"),
            "unexpected error: {}",
            err
        );
    }
}
