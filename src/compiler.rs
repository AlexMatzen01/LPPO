use std::collections::HashMap;

use crate::ast::{Procedure, Program, Statement, Expr};
use crate::error::{LppoError, LppoResult};
use crate::runtime::{Operation, ProcedureBody};

/// Converts an LPPO program into an executable representation.
pub fn compile_program(program: Program) -> LppoResult<HashMap<String, ProcedureBody>> {
    let mut procedures = HashMap::new();
    for proc in program.procedures {
        if procedures.contains_key(&proc.name) {
            return Err(LppoError::DuplicateProcedure(proc.name));
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
    ProcedureBody { params: procedure.params.clone(), operations }
}

fn compile_statement(statement: Statement) -> Operation {
    match statement {
        Statement::Emit(exprs) => Operation::Emit(exprs),
        Statement::Parallel(branches) => Operation::Parallel(branches),
        Statement::Call(name, args) => Operation::Call(name, args),
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
        Statement::For { var, iterable, body } => {
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
        Statement::Let(name, expr) => Operation::Let(name, expr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_emit_and_parallel() {
        let program = Program {
            procedures: vec![Procedure {
        name: "main".to_string(),
        params: vec![],
        statements: vec![
                        Statement::Emit(vec![Expr::Str("hello".into())]),
                        Statement::Parallel(vec![Expr::Str("one".into()), Expr::Str("two".into())]),
                ],
            }],
        };
        let compiled = compile_program(program).expect("compile");
        assert!(compiled.contains_key("main"));
    }
}
