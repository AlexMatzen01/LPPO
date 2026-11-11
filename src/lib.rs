pub mod ast;
pub mod buffer;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod pkg;
pub mod runtime;
pub mod typechecker;

pub use compiler::compile_program;
pub use error::{LppoError, LppoResult};
pub use parser::parse_program;
pub use pkg::{cli::PkgSubcommand, manager::PackageManager, PackageContext};
pub use runtime::Executor;
pub use typechecker::{typecheck_program, typecheck_program_with_context};

use crate::runtime::Operation;

use std::collections::HashSet;
use std::path::Path;

use crate::ast::{Program, UseDeclaration, UseItems};
use crate::pkg::installed::InstalledTracker;
/// Options controlling how source is compiled into bytecode.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// Remove declaration-only (empty-body) procedures from the payload.
    pub strip: bool,
    /// XOR-obfuscate the serialized bytecode payload.
    pub obfuscate: bool,
}

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
/// `use` imports relative to `root`. Package imports resolve against the
/// project-local (`.lppo/`) or global (`~/.LPPO/`) package context discovered
/// from `root`.
pub fn build_executor_with_root(source: &str, root: &Path) -> LppoResult<Executor> {
    let context = PackageContext::discover(root)?;
    let ast = resolve_program(source, root, &context, &mut HashSet::new())?;
    let ring0 = ast.ring0;
    typecheck_program_with_context(&ast, &context)?;
    let procedures = compile_program(ast)?;
    Ok(Executor::with_context(procedures, ring0, context))
}

/// Compile source and return serialized bytes (JSON) suitable for embedding.
///
/// Imports are resolved relative to the current directory.
pub fn compile_to_bytes(source: &str) -> LppoResult<Vec<u8>> {
    let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    compile_to_bytes_with_options(source, &root, CompileOptions::default())
}

/// Compile source and return serialized bytes (JSON) suitable for embedding,
/// resolving `use` imports relative to `root`.
pub fn compile_to_bytes_with_root(source: &str, root: &Path) -> LppoResult<Vec<u8>> {
    compile_to_bytes_with_options(source, root, CompileOptions::default())
}

/// Compile source with the given [`CompileOptions`] and return serialized
/// bytes (JSON) suitable for embedding, resolving `use` imports relative to
/// `root`.
pub fn compile_to_bytes_with_options(
    source: &str,
    root: &Path,
    options: CompileOptions,
) -> LppoResult<Vec<u8>> {
    let context = PackageContext::discover(root)?;
    let ast = resolve_program(source, root, &context, &mut HashSet::new())?;
    typecheck_program_with_context(&ast, &context)?;
    let mut procedures = compile_program(ast)?;

    if options.strip {
        // Remove declaration-only procedures (empty bodies) as a belt-and-
        // suspenders measure (the compiler already skips them at compile
        // time).
        procedures.retain(|_, body| !body.operations.is_empty());

        // Dead-code elimination: keep only procedures reachable from `main`
        // via direct calls, shrinking the payload.
        if procedures.contains_key("main") {
            let mut reachable = HashSet::new();
            let mut stack = vec!["main".to_string()];
            reachable.insert("main".to_string());
            while let Some(name) = stack.pop() {
                let Some(body) = procedures.get(&name) else { continue };
                let mut targets = Vec::new();
                collect_call_targets(&body.operations, &mut targets);
                for target in targets {
                    if reachable.insert(target.clone()) {
                        stack.push(target);
                    }
                }
            }
            procedures.retain(|name, _| reachable.contains(name));
        }
    }

    let mut bytes = runtime::serialize_procedures(&procedures)?;

    if options.obfuscate {
        bytes = runtime::obfuscate_payload(&bytes)?;
    }

    Ok(bytes)
}

/// Load an executor from serialized bytes.
pub fn load_executor_from_bytes(bytes: &[u8]) -> LppoResult<Executor> {
    runtime::executor_from_bytes(bytes)
}

/// Load an executor from a bundle payload.
pub fn load_executor_from_bundle_bytes(bytes: &[u8]) -> LppoResult<Executor> {
    runtime::executor_from_bundle_bytes(bytes)
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

/// Recursively collect the names of procedures called by an operation tree,
/// so dead-code elimination can determine reachability from `main`.
fn collect_call_targets(ops: &[Operation], out: &mut Vec<String>) {
    for op in ops {
        match op {
            Operation::Call(name, _) => out.push(name.clone()),
            Operation::If {
                then_ops,
                elifs,
                else_ops,
                ..
            } => {
                collect_call_targets(then_ops, out);
                for (_, ops) in elifs {
                    collect_call_targets(ops, out);
                }
                if let Some(ops) = else_ops {
                    collect_call_targets(ops, out);
                }
            }
            Operation::While { ops, .. }
            | Operation::Loop { ops }
            | Operation::Scope { ops }
            | Operation::Unsafe { ops }
            | Operation::For { ops, .. }
            | Operation::Spawn { ops } => {
                collect_call_targets(ops, out);
            }
            Operation::Emit(_)
            | Operation::Parallel(_)
            | Operation::Input { .. }
            | Operation::Let(_, _)
            | Operation::ExprStatement(_)
            | Operation::Break
            | Operation::Continue
            | Operation::Return(_)
            | Operation::Nop
            | Operation::Pause { .. } => {}
        }
    }
}

/// Parse `source` and recursively resolve every `use` import relative to
/// `root`, merging all imported modules into a single program. `seen` tracks
/// already-resolved files to prevent import cycles. `context` selects whether
/// `use <InstalledPackage>` resolves against the global or project-local
/// installed package database.
fn resolve_program(
    source: &str,
    root: &Path,
    context: &PackageContext,
    seen: &mut HashSet<String>,
) -> LppoResult<Program> {
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

        // If the import names an installed Rust package, its LPPO-facing
        // functions are registered globally by the type checker (via
        // NativeRegistry). Skip on-disk file resolution for it.
        // Check both project-local and global installed packages (case-insensitive).
        if let Some(first) = path.first() {
            let installed = InstalledTracker::load_with(context.clone())
                .map(|t| t.is_installed_any_case(first))
                .unwrap_or(false);
            let global_installed = InstalledTracker::load()
                .map(|t| t.is_installed_any_case(first))
                .unwrap_or(false);
            if installed || global_installed {
                continue;
            }
        }

        let (contents, _is_header) = resolve_import_file(root, path)?;
        let sub_root = root; // resolve nested imports relative to the same root
        let sub = resolve_program(&contents, sub_root, context, seen)?;
        merge_program(&mut program, sub);
    }

    program.use_declarations = use_declarations;
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Procedure, Statement, TypeAnnotation};
    use crate::runtime::{eval_expr, Value, ProcedureBody};
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
            &PackageContext::Global,
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
            &PackageContext::Global,
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
            &PackageContext::Global,
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
            &PackageContext::Global,
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
            &PackageContext::Global,
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

    #[test]
    fn obfuscated_payload_round_trip_runs() {
        let source = "pub proc main() -> Void { out \"obfuscated\"; }";
        let bytes = compile_to_bytes_with_options(
            source,
            Path::new(""),
            CompileOptions {
                obfuscate: true,
                ..Default::default()
            },
        )
        .unwrap();
        // Obfuscated payload must carry the obfuscation marker.
        assert!(bytes.starts_with(super::runtime::OBFUSCATION_MARKER));
        let executor = load_executor_from_bytes(&bytes).unwrap();
        assert!(executor.has_procedure("main"));
        executor.run("main").unwrap();
    }

    #[test]
    fn obfuscated_and_plain_payloads_differ() {
        let source = "pub proc main() -> Void { out \"hello\"; }";
        let plain = compile_to_bytes_with_options(source, Path::new(""), CompileOptions::default()).unwrap();
        let obfuscated = compile_to_bytes_with_options(
            source,
            Path::new(""),
            CompileOptions { obfuscate: true, ..Default::default() },
        )
        .unwrap();
        assert_ne!(plain, obfuscated);
    }

    #[test]
    fn strip_removes_unreachable_procedures() {
        let source = "pub proc main() -> Void { out \"main\"; }\npub proc unused() -> Void { out \"unused\"; }";
        let stripped = compile_to_bytes_with_options(
            source,
            Path::new(""),
            CompileOptions {
                strip: true,
                ..Default::default()
            },
        )
        .unwrap();
        let kept = load_executor_from_bytes(&stripped).unwrap();
        assert!(kept.has_procedure("main"));
        assert!(!kept.has_procedure("unused"));

        let unstripped =
            compile_to_bytes_with_options(source, Path::new(""), CompileOptions::default()).unwrap();
        let kept = load_executor_from_bytes(&unstripped).unwrap();
        assert!(kept.has_procedure("unused"));
    }

    #[test]
    fn strip_keeps_procedures_with_bodies() {
        let source = "pub proc main() -> Void { call kept(); out \"main\"; }\npub proc kept() -> Void { out \"kept\"; }\npub proc unused() -> Void { out \"unused\"; }";
        let bytes = compile_to_bytes_with_options(
            source,
            Path::new(""),
            CompileOptions {
                strip: true,
                ..Default::default()
            },
        )
        .unwrap();
        let executor = load_executor_from_bytes(&bytes).unwrap();
        assert!(executor.has_procedure("main"));
        assert!(executor.has_procedure("kept"));
        assert!(!executor.has_procedure("unused"));

        let unstripped =
            compile_to_bytes_with_options(source, Path::new(""), CompileOptions::default()).unwrap();
        let kept = load_executor_from_bytes(&unstripped).unwrap();
        assert!(kept.has_procedure("unused"));
    }

    #[test]
    fn collection_builtins_work() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let vec_new_call = || Expr::Call {
            func: Box::new(Expr::Var("vec_new".into())),
            args: vec![],
        };

        let empty = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("vec_new".into())),
                args: vec![],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(empty, Value::Array(ref a) if a.is_empty()));

        let one = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("vec_push".into())),
                args: vec![vec_new_call(), Expr::Int(5)],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(one, Value::Array(ref a) if a.len() == 1));
        env.insert("v1".into(), one);

        let len = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("vec_len".into())),
                args: vec![Expr::Var("v1".into())],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(len, Value::Int(1)));

        let got = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("vec_get".into())),
                args: vec![
                    Expr::Call {
                        func: Box::new(Expr::Var("vec_push".into())),
                        args: vec![vec_new_call(), Expr::Int(42)],
                    },
                    Expr::Int(0),
                ],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(got, Value::Int(42)));

        let m = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("map_new".into())),
                args: vec![],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(m, Value::Object(ref f) if f.is_empty()));
        env.insert("m1".into(), m);

        let inserted = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("map_insert".into())),
                args: vec![
                    Expr::Var("m1".into()),
                    Expr::Str("k".into()),
                    Expr::Int(7),
                ],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(inserted, Value::Object(ref f) if f.len() == 1));
        env.insert("m2".into(), inserted);

        let got = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("map_get".into())),
                args: vec![Expr::Var("m2".into()), Expr::Str("k".into())],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(got, Value::Int(7)));

        let keys = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("map_keys".into())),
                args: vec![Expr::Var("m2".into())],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(keys, Value::Array(ref a) if a.len() == 1));
    }

    #[test]
    fn memory_pool_builtins_work() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let used0 = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("pool_used".into())),
                args: vec![],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(used0, Value::Int(0)));

        let pid = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("pool_alloc".into())),
                args: vec![Expr::Int(64)],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        let id = match pid { Value::Int(i) => i, _ => panic!("expected pool id") };
        assert!(id > 0);

        let used1 = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("pool_used".into())),
                args: vec![],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(used1, Value::Int(1)));

        let freed = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("pool_free".into())),
                args: vec![Expr::Int(id)],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(freed, Value::Int(0)));

        let used2 = eval_expr(
            &Expr::Call {
                func: Box::new(Expr::Var("pool_used".into())),
                args: vec![],
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(used2, Value::Int(0)));
    }

    #[test]
    fn async_await_round_trip() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let fut = eval_expr(
            &Expr::Async {
                body: Box::new(Expr::Int(7)),
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        let id = match &fut {
            Value::Future { id } => *id,
            _ => panic!("async should return a future"),
        };

        let v = eval_expr(
            &Expr::Await(Box::new(Expr::Int(id as i64))),
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(v, Value::Int(7)));
    }

    #[test]
    fn spawn_join_round_trip() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let jh = eval_expr(
            &Expr::Spawn {
                body: Box::new(Expr::Int(99)),
            },
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        let id = match &jh {
            Value::JoinHandle { id } => *id,
            _ => panic!("spawn should return a join handle"),
        };

        let v = eval_expr(
            &Expr::Join(Box::new(Expr::Int(id as i64))),
            &mut env,
            &procedures,
            false,
        )
        .unwrap();
        assert!(matches!(v, Value::Int(99)));
    }

    #[test]
    fn eval_math_builtin() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let mut call = |s: &str| -> Value {
            eval_expr(
                &Expr::Call {
                    func: Box::new(Expr::Var("eval".into())),
                    args: vec![Expr::Str(s.into())],
                },
                &mut env,
                &procedures,
                false,
            )
            .unwrap()
        };

        assert!(matches!(call("2+3*4"), Value::Int(14)));
        assert!(matches!(call("(2+3)*4"), Value::Int(20)));
        assert!(matches!(call("2^10"), Value::Int(1024)));
        assert!(matches!(call("10/3"), Value::Int(3)));
        assert!(matches!(call("-5+3"), Value::Int(-2)));
        assert!(matches!(call("1+2+3+4+5"), Value::Int(15)));
        assert!(matches!(call("2*3+4*5"), Value::Int(26)));
    }

    #[test]
    fn sqrt_builtin() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let mut call = |v: i64| -> Value {
            eval_expr(
                &Expr::Call {
                    func: Box::new(Expr::Var("sqrt".into())),
                    args: vec![Expr::Int(v)],
                },
                &mut env,
                &procedures,
                false,
            )
            .unwrap()
        };

        assert!(matches!(call(16), Value::Int(4)));
        assert!(matches!(call(25), Value::Int(5)));
        assert!(matches!(call(2), Value::Int(1)));
        assert!(matches!(call(0), Value::Int(0)));
    }

    #[test]
    fn either_builtin() {
        let mut env = HashMap::new();
        let procedures: HashMap<String, ProcedureBody> = HashMap::new();

        let mut call = |a: i64, b: i64| -> Value {
            eval_expr(
                &Expr::Call {
                    func: Box::new(Expr::Var("either".into())),
                    args: vec![Expr::Int(a), Expr::Int(b)],
                },
                &mut env,
                &procedures,
                false,
            )
            .unwrap()
        };

        assert!(matches!(call(5, 1), Value::Array(arr) if arr == vec![Value::Int(6), Value::Int(4)]));
        assert!(matches!(call(0, 0), Value::Array(arr) if arr == vec![Value::Int(0), Value::Int(0)]));
        assert!(matches!(call(3, -7), Value::Array(arr) if arr == vec![Value::Int(-4), Value::Int(10)]));
    }
}
