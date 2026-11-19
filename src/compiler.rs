use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryOp, Expr, MatchArm, MatchArmBody, MemoryLocation, Pattern, Procedure, Program,
    QualifiedName, Statement,
};
use crate::error::{LppoError, LppoResult};
use crate::runtime::{Operation, ProcedureBody};

/// Converts an LPPO program into an executable representation.
pub fn compile_program(program: Program) -> LppoResult<HashMap<String, ProcedureBody>> {
    let mut procedures = HashMap::new();

    // Compile module procedures with qualified names
    for module in program.modules {
        for proc in module.procedures {
            let qualified_name = format!("{}::{}", module.name, proc.name);
            if procedures.contains_key(&qualified_name) {
                return Err(LppoError::duplicate_procedure(qualified_name));
            }
            procedures.insert(qualified_name, compile_procedure(proc));
        }
    }

    // Compile top-level procedures
    for proc in program.procedures {
        // Skip header-only declarations (procedures without a body).
        if proc.statements.is_empty() {
            continue;
        }
        if procedures.contains_key(&proc.name) {
            return Err(LppoError::duplicate_procedure(proc.name));
        }
        procedures.insert(proc.name.clone(), compile_procedure(proc));
    }
    Ok(procedures)
}

fn compile_procedure(procedure: Procedure) -> ProcedureBody {
    let mut ctx = CompileCtx::new();
    let operations: Vec<Operation> = procedure
        .statements
        .into_iter()
        .map(|stmt| compile_statement(stmt, &mut ctx))
        .collect();
    // At end of procedure, deallocate any remaining heap-allocated variables
    let mut final_ops = operations;
    for var in ctx.current_heap_vars() {
        final_ops.push(Operation::Dealloc(var));
    }
    // Convert Parameters to just their names for runtime (types are compile-time only)
    let param_names: Vec<String> = procedure.params.into_iter().map(|p| p.name).collect();
    ProcedureBody {
        params: param_names,
        operations: final_ops,
        return_type: Some(procedure.return_type),
    }
}

fn qualified_name_to_string(qn: &QualifiedName) -> String {
    if qn.path.is_empty() {
        qn.name.clone()
    } else {
        format!("{}::{}", qn.path.join("::"), qn.name)
    }
}

/// Compilation context that tracks heap-allocated variables per scope level
/// for automatic deallocation at scope exit (no garbage collector needed).
struct CompileCtx {
    /// Stack of heap-allocated variable names per scope level
    heap_stack: Vec<HashSet<String>>,
    /// Counter for temporaries introduced by `match` lowering.
    match_counter: usize,
}

impl CompileCtx {
    fn new() -> Self {
        CompileCtx {
            heap_stack: vec![HashSet::new()],
            match_counter: 0,
        }
    }

    /// Enter a new scope level
    fn enter_scope(&mut self) {
        self.heap_stack.push(HashSet::new());
    }

    /// Exit the current scope level, returning heap variables to deallocate
    fn exit_scope(&mut self) -> Vec<String> {
        self.heap_stack.pop().unwrap_or_default().into_iter().collect()
    }

    /// Mark a variable as heap-allocated in the current scope
    fn mark_heap(&mut self, var: String) {
        self.heap_stack.last_mut().unwrap().insert(var);
    }

    /// Remove a variable from heap tracking (explicitly deallocated)
    fn unmark_heap(&mut self, var: &str) {
        if let Some(top) = self.heap_stack.last_mut() {
            top.remove(var);
        }
    }

    /// Get all heap-allocated variables in the current (top) scope
    fn current_heap_vars(&self) -> Vec<String> {
        self.heap_stack.last().cloned().unwrap_or_default().into_iter().collect()
    }
}

fn compile_statement(
    statement: Statement,
    ctx: &mut CompileCtx,
) -> Operation {
    match statement {
        Statement::Emit(exprs) => Operation::Emit(exprs),
        Statement::Parallel(branches) => Operation::Parallel(branches),
        Statement::Input { var, typ, prompt } => {
            // Type is compile-time only, runtime will parse based on context
            Operation::Input { var, typ, prompt }
        }
        Statement::Call(qualified_name, args) => {
            let name = qualified_name_to_string(&qualified_name);
            Operation::Call(name, args)
        }
        Statement::If {
            cond,
            then_branch,
            elifs,
            else_branch,
        } => {
            // compile nested statements into operations
            let mut then_ops = Vec::new();
            for stmt in then_branch {
                then_ops.push(compile_statement(stmt, ctx));
            }
            let elif_ops = elifs
                .into_iter()
                .map(|(c, stmts)| {
                    let mut ops = Vec::new();
                    for stmt in stmts {
                        ops.push(compile_statement(stmt, ctx));
                    }
                    (c, ops)
                })
                .collect();
            let else_ops = else_branch.map(|stmts| {
                let mut ops = Vec::new();
                for stmt in stmts {
                    ops.push(compile_statement(stmt, ctx));
                }
                ops
            });
            Operation::If {
                cond,
                then_ops,
                elifs: elif_ops,
                else_ops,
            }
        }
        Statement::While { cond, body } => {
            let mut ops = Vec::new();
            for stmt in body {
                ops.push(compile_statement(stmt, ctx));
            }
            Operation::While { cond, ops }
        }
        Statement::For {
            var,
            typ: _,
            iterable,
            body,
        } => {
            // Type annotation is compile-time only, runtime doesn't track it
            let mut ops = Vec::new();
            for stmt in body {
                ops.push(compile_statement(stmt, ctx));
            }
            Operation::For { var, iterable, ops }
        }
        Statement::Loop { body } => {
            let mut ops = Vec::new();
            for stmt in body {
                ops.push(compile_statement(stmt, ctx));
            }
            Operation::Loop { ops }
        }
        Statement::Break => Operation::Break,
        Statement::Continue => Operation::Continue,
        Statement::Return(expr) => Operation::Return(expr),
        Statement::Let {
            name,
            typ: _,
            expr,
            is_mut: _,
            ownership: _,
        } => {
            // Type, mutability, and ownership are compile-time only
            // At runtime we just bind the value to the name
            Operation::Let(name, expr)
        }
        Statement::Alloc {
            var,
            typ: _,
            location,
            size,
        } => {
            // Track heap-allocated variables for automatic deallocation
            if matches!(location, MemoryLocation::Heap) {
                ctx.mark_heap(var.clone());
            }
            Operation::Alloc {
                var,
                location,
                size,
            }
        }
        Statement::Dealloc(var) => {
            // Explicit deallocation: remove from heap tracking
            ctx.unmark_heap(&var);
            Operation::Dealloc(var)
        }
        Statement::Move { source, target } => {
            // Move is semantically like let target = source
            Operation::Let(target, Expr::Var(source))
        }
        Statement::Copy { source, target } => {
            // Copy is also like let target = source (values are copied)
            Operation::Let(target, Expr::Var(source))
        }
        Statement::Scope { name: _, body } => {
            // Enter a new scope, compile body, then insert Dealloc for heap vars
            ctx.enter_scope();
            let mut ops = Vec::new();
            for stmt in body {
                ops.push(compile_statement(stmt, ctx));
            }
            // Insert Dealloc operations for heap-allocated variables at scope exit
            for var in ctx.exit_scope() {
                ops.push(Operation::Dealloc(var));
            }
            Operation::Scope { ops }
        }
        Statement::Unsafe { body } => {
            // Unsafe blocks are treated as normal blocks at runtime
            // but bypass borrow checking at compile time
            let mut ops = Vec::new();
            for stmt in body {
                ops.push(compile_statement(stmt, ctx));
            }
            Operation::Unsafe { ops }
        }
        Statement::Match { expr, arms } => compile_match(expr, arms, ctx),
        Statement::StructInit {
            name: _,
            typ: _,
            fields: _,
        } => {
            // Struct initialization - placeholder for now
            Operation::Nop
        }
        Statement::ExprStatement(Expr::MatchExpr { expr, arms }) => {
            compile_match(*expr, arms, ctx)
        }
        Statement::ExprStatement(expr) => {
            // Expression statement - evaluate expression for side effects
            Operation::ExprStatement(expr)
        }
        Statement::Assignment { target, value } => {
            // Assignment: var = expr is similar to let var = expr but updates existing var
            Operation::Let(target, value)
        }
        Statement::IndexedAssignment {
            target: _,
            index: _,
            value: _,
        } => {
            // Indexed assignment: arr[i] = expr
            // For now, treat as no-op since arrays are immutable in simplified runtime
            Operation::Nop
        }
        Statement::Pause {
            prompt,
            timeout_ms,
            until,
            assign_to,
            on_mismatch,
            on_timeout,
            is_async,
        } => Operation::Pause {
            prompt,
            timeout_ms,
            until,
            assign_to,
            on_mismatch,
            on_timeout,
            is_async,
        },
        Statement::Spawn { body } => {
            let mut ops = Vec::new();
            for stmt in body {
                ops.push(compile_statement(stmt, ctx));
            }
            Operation::Spawn { ops }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::TypeAnnotation;

    #[test]
    fn compiles_emit_and_parallel() {
        let program = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![Procedure {
                name: "main".to_string(),
                params: vec![],
                return_type: TypeAnnotation::Void,
                lifetimes: vec![],
                is_public: false,
                is_unsafe: false,
                statements: vec![
                    Statement::Emit(vec![Expr::Str("hello".into())]),
                    Statement::Parallel(vec![Expr::Str("one".into()), Expr::Str("two".into())]),
                ],
            }],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        let compiled = compile_program(program).expect("compile");
        assert!(compiled.contains_key("main"));
    }

    #[test]
    fn heap_alloc_is_deallocated_at_scope_exit() {
        let program = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![Procedure {
                name: "main".to_string(),
                params: vec![],
                return_type: TypeAnnotation::Void,
                lifetimes: vec![],
                is_public: false,
                is_unsafe: false,
                statements: vec![
                    Statement::Scope {
                        name: None,
                        body: vec![Statement::Alloc {
                            var: "buf".to_string(),
                            typ: TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
                            location: MemoryLocation::Heap,
                            size: Expr::Int(1024),
                        }],
                    },
                ],
            }],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        let compiled = compile_program(program).expect("compile");
        let main = compiled.get("main").unwrap();
        // Should have an Alloc followed by a Dealloc inside the scope
        let scope_ops: Vec<&Operation> = main
            .operations
            .iter()
            .flat_map(|op| match op {
                Operation::Scope { ops } => ops.iter().collect(),
                _ => Vec::new(),
            })
            .collect();
        assert!(scope_ops.iter().any(|op| matches!(op, Operation::Alloc { .. })));
        assert!(scope_ops.iter().any(|op| matches!(op, Operation::Dealloc(_))));
    }

    #[test]
    fn stack_alloc_is_not_deallocated() {
        let program = Program {
            ring0: false,
            use_declarations: vec![],
            type_aliases: vec![],
            modules: vec![],
            procedures: vec![Procedure {
                name: "main".to_string(),
                params: vec![],
                return_type: TypeAnnotation::Void,
                lifetimes: vec![],
                is_public: false,
                is_unsafe: false,
                statements: vec![
                    Statement::Scope {
                        name: None,
                        body: vec![Statement::Alloc {
                            var: "x".to_string(),
                            typ: TypeAnnotation::Int64,
                            location: MemoryLocation::Stack,
                            size: Expr::Int(0),
                        }],
                    },
                ],
            }],
            structs: vec![],
            enums: vec![],
            traits: vec![],
            impls: vec![],
        };
        let compiled = compile_program(program).expect("compile");
        let main = compiled.get("main").unwrap();
        // Stack alloc should NOT generate a Dealloc
        assert!(!main.operations.iter().any(|op| matches!(
            op,
            Operation::Dealloc(var) if var.as_str() == "x"
        )));
    }
}

// ---------------------------------------------------------------------------
// match lowering
// ---------------------------------------------------------------------------

fn builtin_call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        func: Box::new(Expr::Var(name.to_string())),
        args,
    }
}

fn and_expr(left: Option<Expr>, right: Option<Expr>) -> Option<Expr> {
    match (left, right) {
        (Some(a), Some(b)) => Some(Expr::BinaryOp {
            left: Box::new(a),
            op: BinaryOp::And,
            right: Box::new(b),
        }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Compile `match expr { arms }` into a temp binding plus an if/elif chain.
fn compile_match(expr: Expr, arms: Vec<MatchArm>, ctx: &mut CompileCtx) -> Operation {
    ctx.match_counter += 1;
    let tmp = format!("__match{}", ctx.match_counter);

    let mut prologue = vec![Operation::Let(tmp.clone(), expr)];
    let mut branches: Vec<(Expr, Vec<Operation>)> = Vec::new();
    let mut fallback: Option<Vec<Operation>> = None;

    for arm in arms {
        let (cond, mut bindings) = pattern_test(&arm.pattern, &Expr::Var(tmp.clone()), ctx);
        let cond = match arm.guard {
            Some(guard) => and_expr(cond, Some(guard)),
            None => cond,
        };

        match arm.body {
            MatchArmBody::Emit(exprs) => bindings.push(Operation::Emit(exprs)),
            MatchArmBody::Expr(Expr::Return(value)) => {
                bindings.push(Operation::Return(value.map(|v| *v)))
            }
            MatchArmBody::Expr(e) => bindings.push(Operation::ExprStatement(e)),
        }

        match cond {
            Some(c) => branches.push((c, bindings)),
            None => {
                if fallback.is_none() {
                    fallback = Some(bindings);
                }
            }
        }
    }

    let inner = if branches.is_empty() {
        Operation::Scope {
            ops: fallback.unwrap_or_default(),
        }
    } else {
        let (first_cond, first_ops) = branches.remove(0);
        Operation::If {
            cond: first_cond,
            then_ops: first_ops,
            elifs: branches,
            else_ops: fallback,
        }
    };

    prologue.push(inner);
    Operation::Scope { ops: prologue }
}

/// Produce the condition and bindings for one pattern against `scrutinee`.
fn pattern_test(
    pattern: &Pattern,
    scrutinee: &Expr,
    ctx: &mut CompileCtx,
) -> (Option<Expr>, Vec<Operation>) {
    match pattern {
        Pattern::Wildcard => (None, Vec::new()),
        Pattern::Variable(name) => (None, vec![Operation::Let(name.clone(), scrutinee.clone())]),
        Pattern::Literal(lit) => (
            Some(Expr::BinaryOp {
                left: Box::new(scrutinee.clone()),
                op: BinaryOp::Eq,
                right: Box::new(lit.clone()),
            }),
            Vec::new(),
        ),
        Pattern::EnumVariant {
            variant, fields, ..
        } => {
            let cond = Expr::BinaryOp {
                left: Box::new(builtin_call(
                    "enum_tag",
                    vec![scrutinee.clone()],
                )),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Str(variant.clone())),
            };
            let (extra, mut ops) = sub_patterns(fields, scrutinee, "enum_get", ctx);
            (and_expr(Some(cond), extra), ops.drain(..).collect())
        }
        Pattern::Tuple(fields) => {
            let (cond, mut ops) = sub_patterns(fields, scrutinee, "tuple_get", ctx);
            (cond, ops.drain(..).collect())
        }
        Pattern::Struct { fields, .. } => {
            let mut cond: Option<Expr> = None;
            let mut ops = Vec::new();
            for (field, pat) in fields {
                let accessor = builtin_call(
                    "struct_field",
                    vec![scrutinee.clone(), Expr::Str(field.clone())],
                );
                let (c, mut b) = pattern_test(pat, &accessor, ctx);
                cond = and_expr(cond, c);
                ops.append(&mut b);
            }
            (cond, ops)
        }
        Pattern::Or(alts) => {
            let mut cond: Option<Expr> = None;
            let mut bindings: Vec<Operation> = Vec::new();
            for (i, alt) in alts.iter().enumerate() {
                let (c, mut b) = pattern_test(alt, scrutinee, ctx);
                if i == 0 {
                    bindings = b;
                }
                cond = match (cond, c) {
                    (Some(a), Some(b)) => Some(Expr::BinaryOp {
                        left: Box::new(a),
                        op: BinaryOp::Or,
                        right: Box::new(b),
                    }),
                    (Some(a), None) => Some(a),
                    (None, c) => c,
                };
            }
            (cond, bindings)
        }
    }
}

/// Destructure a tuple-like value (tuple/enum payload) into sub-patterns.
fn sub_patterns(
    fields: &[Pattern],
    scrutinee: &Expr,
    accessor: &str,
    ctx: &mut CompileCtx,
) -> (Option<Expr>, Vec<Operation>) {
    let mut cond: Option<Expr> = None;
    let mut ops = Vec::new();
    for (i, pat) in fields.iter().enumerate() {
        let accessor_expr = builtin_call(
            accessor,
            vec![scrutinee.clone(), Expr::Int(i as i64)],
        );
        let (c, mut b) = pattern_test(pat, &accessor_expr, ctx);
        cond = and_expr(cond, c);
        ops.append(&mut b);
    }
    (cond, ops)
}
