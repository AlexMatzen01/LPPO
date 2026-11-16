use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, MemoryLocation, Procedure, Program, QualifiedName, Statement};
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
}

impl CompileCtx {
    fn new() -> Self {
        CompileCtx {
            heap_stack: vec![HashSet::new()],
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
        Statement::Match { expr: _, arms: _ } => {
            // Match expressions will be compiled to a series of if-else chains
            // For now, just placeholder - will implement pattern matching later
            Operation::Nop
        }
        Statement::StructInit {
            name: _,
            typ: _,
            fields: _,
        } => {
            // Struct initialization - placeholder for now
            Operation::Nop
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
