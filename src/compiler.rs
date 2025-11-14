use std::collections::HashMap;

use crate::ast::{Procedure, Program, Statement, Expr, QualifiedName};
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
        if procedures.contains_key(&proc.name) {
            return Err(LppoError::duplicate_procedure(proc.name));
        }
        procedures.insert(proc.name.clone(), compile_procedure(proc));
    }
    Ok(procedures)
}

fn compile_procedure(procedure: Procedure) -> ProcedureBody {
    let operations = procedure
        .statements
        .into_iter()
        .map(compile_statement)
        .collect();
    // Convert Parameters to just their names for runtime (types are compile-time only)
    let param_names: Vec<String> = procedure.params.into_iter().map(|p| p.name).collect();
    ProcedureBody { params: param_names, operations }
}

fn qualified_name_to_string(qn: &QualifiedName) -> String {
    if qn.path.is_empty() {
        qn.name.clone()
    } else {
        format!("{}::{}", qn.path.join("::"), qn.name)
    }
}

fn compile_statement(statement: Statement) -> Operation {
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
        Statement::If { cond, then_branch, elifs, else_branch } => {
            // compile nested statements into operations
            let then_ops = then_branch.into_iter().map(compile_statement).collect();
            let elif_ops = elifs.into_iter().map(|(c, stmts)| (c, stmts.into_iter().map(compile_statement).collect())).collect();
            let else_ops = else_branch.map(|stmts| stmts.into_iter().map(compile_statement).collect());
            Operation::If { cond, then_ops, elifs: elif_ops, else_ops }
        }
        Statement::While { cond, body } => {
            let ops = body.into_iter().map(compile_statement).collect();
            Operation::While { cond, ops }
        }
        Statement::For { var, typ: _, iterable, body } => {
            // Type annotation is compile-time only, runtime doesn't track it
            let ops = body.into_iter().map(compile_statement).collect();
            Operation::For { var, iterable, ops }
        }
        Statement::Loop { body } => {
            let ops = body.into_iter().map(compile_statement).collect();
            Operation::Loop { ops }
        }
        Statement::Break => Operation::Break,
        Statement::Continue => Operation::Continue,
        Statement::Return(expr) => Operation::Return(expr),
        Statement::Let { name, typ: _, expr, is_mut: _, ownership: _ } => {
            // Type, mutability, and ownership are compile-time only
            // At runtime we just bind the value to the name
            Operation::Let(name, expr)
        }
        Statement::Alloc { var, typ: _, location: _, size } => {
            // Create an array of the specified size, initialized with zeros
            // This is a simplified implementation
            let array_expr = match size {
                Expr::Int(n) => {
                    // If size is a constant, create that many zero elements
                    let elements = vec![Expr::Int(0); n as usize];
                    Expr::Array(elements)
                }
                _ => {
                    // For non-constant size, create empty array
                    // In a full implementation, this would be handled at runtime
                    Expr::Array(vec![])
                }
            };
            Operation::Let(var, array_expr)
        }
        Statement::Dealloc(var) => {
            // Dealloc is a no-op in our simplified runtime (we rely on Rust's GC)
            // But we can remove the variable from scope
            Operation::Let(var, Expr::Int(0)) // placeholder
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
            // Scope blocks just execute their body
            let ops = body.into_iter().map(compile_statement).collect();
            Operation::Scope { ops }
        }
        Statement::Unsafe { body } => {
            // Unsafe blocks are treated as normal blocks at runtime
            let ops = body.into_iter().map(compile_statement).collect();
            Operation::Unsafe { ops }
        }
        Statement::Match { expr: _, arms: _ } => {
            // Match expressions will be compiled to a series of if-else chains
            // For now, just placeholder - will implement pattern matching later
            Operation::Nop
        }
        Statement::StructInit { name: _, typ: _, fields: _ } => {
            // Struct initialization - placeholder for now
            Operation::Nop
        }
        Statement::Assignment { name, expr } => {
            // Assignment to existing variable - same as Let at runtime
            Operation::Let(name, expr)
        }
        Statement::IndexedAssignment { name, index, expr } => {
            // Indexed assignment needs special handling
            // For now, we'll create a pseudo-operation
            // In a full implementation, this would modify the array in place
            Operation::IndexedAssignment { name, index, expr }
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
}
