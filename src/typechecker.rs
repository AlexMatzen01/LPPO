/// Type checking for LPPO programs
/// Validates types across expressions, statements, and procedure calls
use std::collections::HashMap;

use crate::ast::{
    Expr, BinaryOp, UnaryOp, TypeAnnotation, Statement, Procedure, Program, 
    QualifiedName, StructDef, EnumDef
};
use crate::error::{LppoError, LppoResult};

use crate::ast::OwnershipKind;

/// Ownership state for tracking moves and borrows
#[derive(Debug, Clone, PartialEq)]
enum OwnershipState {
    Owned,
    Moved,
    #[allow(dead_code)] // TODO: Implement borrow tracking
    Borrowed { mutable: bool },
}

/// Type context for tracking variable and procedure types
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// Variables in current scope with their types
    variables: HashMap<String, TypeAnnotation>,
    /// Ownership state of variables
    ownership: HashMap<String, (OwnershipKind, OwnershipState)>,
    /// Procedure signatures: name -> (param_types, return_type)
    procedures: HashMap<String, (Vec<TypeAnnotation>, TypeAnnotation)>,
    /// Struct definitions
    structs: HashMap<String, StructDef>,
    /// Enum definitions
    enums: HashMap<String, EnumDef>,
    /// Parent scope for nested contexts
    parent: Option<Box<TypeContext>>,
}

impl TypeContext {
    pub fn new() -> Self {
        TypeContext {
            variables: HashMap::new(),
            ownership: HashMap::new(),
            procedures: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            parent: None,
        }
    }

    /// Create a child scope
    pub fn child(&self) -> Self {
        TypeContext {
            variables: HashMap::new(),
            ownership: HashMap::new(),
            procedures: self.procedures.clone(),
            structs: self.structs.clone(),
            enums: self.enums.clone(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Add a variable to current scope with ownership tracking
    pub fn add_variable(&mut self, name: String, typ: TypeAnnotation) {
        self.add_variable_with_ownership(name, typ, OwnershipKind::Own);
    }

    /// Add a variable with explicit ownership
    pub fn add_variable_with_ownership(&mut self, name: String, typ: TypeAnnotation, ownership_kind: OwnershipKind) {
        self.variables.insert(name.clone(), typ);
        self.ownership.insert(name, (ownership_kind, OwnershipState::Owned));
    }

    /// Look up a variable type
    pub fn get_variable(&self, name: &str) -> Option<&TypeAnnotation> {
        self.variables.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.get_variable(name))
        })
    }

    /// Check if a variable has been moved
    pub fn is_moved(&self, name: &str) -> bool {
        if let Some((_, state)) = self.ownership.get(name) {
            matches!(state, OwnershipState::Moved)
        } else {
            self.parent.as_ref().map_or(false, |p| p.is_moved(name))
        }
    }

    /// Mark a variable as moved
    pub fn mark_moved(&mut self, name: &str) -> LppoResult<()> {
        if let Some((kind, state)) = self.ownership.get_mut(name) {
            if matches!(state, OwnershipState::Moved) {
                return Err(LppoError::MoveAfterMove {
                    variable: name.to_string(),
                    location: None,
                });
            }
            // Only owned values can be moved
            if !matches!(kind, OwnershipKind::Own) {
                return Err(LppoError::OwnershipError {
                    message: format!("cannot move non-owned value '{}'", name),
                    location: None,
                });
            }
            *state = OwnershipState::Moved;
            Ok(())
        } else {
            Err(LppoError::UnknownVariable {
                name: name.to_string(),
                location: None,
                suggestion: Some("declare the variable before using it".to_string()),
            })
        }
    }

    /// Check ownership state when accessing a variable
    pub fn check_variable_access(&self, name: &str) -> LppoResult<()> {
        if self.is_moved(name) {
            return Err(LppoError::MoveAfterMove {
                variable: name.to_string(),
                location: None,
            });
        }
        Ok(())
    }

    /// Add a procedure signature
    pub fn add_procedure(&mut self, name: String, params: Vec<TypeAnnotation>, return_type: TypeAnnotation) {
        self.procedures.insert(name, (params, return_type));
    }

    /// Look up a procedure signature
    pub fn get_procedure(&self, name: &str) -> Option<&(Vec<TypeAnnotation>, TypeAnnotation)> {
        self.procedures.get(name)
    }

    /// Add a struct definition
    pub fn add_struct(&mut self, name: String, def: StructDef) {
        self.structs.insert(name, def);
    }

    /// Add an enum definition
    pub fn add_enum(&mut self, name: String, def: EnumDef) {
        self.enums.insert(name, def);
    }
}

/// Type check a complete program
pub fn typecheck_program(program: &Program) -> LppoResult<()> {
    let mut ctx = TypeContext::new();

    // First pass: register all procedures, structs, and enums
    for proc in &program.procedures {
        let param_types = proc.params.iter().map(|p| p.typ.clone()).collect();
        ctx.add_procedure(proc.name.clone(), param_types, proc.return_type.clone());
    }

    for module in &program.modules {
        for proc in &module.procedures {
            let qualified_name = format!("{}::{}", module.name, proc.name);
            let param_types = proc.params.iter().map(|p| p.typ.clone()).collect();
            ctx.add_procedure(qualified_name, param_types, proc.return_type.clone());
        }
    }

    for struct_def in &program.structs {
        ctx.add_struct(struct_def.name.clone(), struct_def.clone());
    }

    for enum_def in &program.enums {
        ctx.add_enum(enum_def.name.clone(), enum_def.clone());
    }

    // Second pass: type check all procedure bodies
    for proc in &program.procedures {
        typecheck_procedure(proc, &mut ctx)?;
    }

    for module in &program.modules {
        for proc in &module.procedures {
            typecheck_procedure(proc, &mut ctx)?;
        }
    }

    Ok(())
}

/// Type check a procedure
fn typecheck_procedure(proc: &Procedure, ctx: &mut TypeContext) -> LppoResult<()> {
    // Create child context for procedure scope
    let mut proc_ctx = ctx.child();

    // Add parameters to scope
    for param in &proc.params {
        proc_ctx.add_variable(param.name.clone(), param.typ.clone());
    }

    // Type check all statements
    let mut return_type: Option<TypeAnnotation> = None;
    for stmt in &proc.statements {
        if let Some(ret_type) = typecheck_statement(stmt, &mut proc_ctx)? {
            return_type = Some(ret_type);
        }
    }

    // Verify return type matches declaration
    if proc.return_type != TypeAnnotation::Void {
        if let Some(ret_type) = return_type {
            if !types_compatible(&proc.return_type, &ret_type) {
                return Err(LppoError::TypeError {
                    message: format!(
                        "procedure '{}' declared to return {:?} but returns {:?}",
                        proc.name, proc.return_type, ret_type
                    ),
                    expected: Some(format!("{:?}", proc.return_type)),
                    found: Some(format!("{:?}", ret_type)),
                    location: None,
                    suggestion: Some("ensure all return statements match the declared return type".to_string()),
                });
            }
        }
    }

    Ok(())
}

/// Type check a statement, returning the type if it's a return statement
fn typecheck_statement(stmt: &Statement, ctx: &mut TypeContext) -> LppoResult<Option<TypeAnnotation>> {
    match stmt {
        Statement::Let { name, typ, expr, ownership, .. } => {
            let expr_type = typecheck_expr(expr, ctx)?;
            if !types_compatible(typ, &expr_type) {
                return Err(LppoError::TypeError {
                    message: format!(
                        "variable '{}' declared as {:?} but assigned {:?}",
                        name, typ, expr_type
                    ),
                    expected: Some(format!("{:?}", typ)),
                    found: Some(format!("{:?}", expr_type)),
                    location: None,
                    suggestion: Some("ensure the expression type matches the variable type".to_string()),
                });
            }
            ctx.add_variable_with_ownership(name.clone(), typ.clone(), ownership.clone());
            Ok(None)
        }

        Statement::Assignment { target, value } => {
            let var_type = ctx.get_variable(target).ok_or_else(|| {
                LppoError::UnknownVariable {
                    name: target.clone(),
                    location: None,
                    suggestion: Some("declare the variable before assigning to it".to_string()),
                }
            })?;
            let value_type = typecheck_expr(value, ctx)?;
            if !types_compatible(var_type, &value_type) {
                return Err(LppoError::TypeError {
                    message: format!(
                        "cannot assign {:?} to variable '{}' of type {:?}",
                        value_type, target, var_type
                    ),
                    expected: Some(format!("{:?}", var_type)),
                    found: Some(format!("{:?}", value_type)),
                    location: None,
                    suggestion: None,
                });
            }
            Ok(None)
        }

        Statement::Return(Some(expr)) => {
            let ret_type = typecheck_expr(expr, ctx)?;
            Ok(Some(ret_type))
        }

        Statement::Return(None) => Ok(Some(TypeAnnotation::Void)),

        Statement::If { cond, then_branch, elifs, else_branch } => {
            let cond_type = typecheck_expr(cond, ctx)?;
            if !types_compatible(&TypeAnnotation::Bool, &cond_type) {
                return Err(LppoError::TypeError {
                    message: "if condition must be a boolean".to_string(),
                    expected: Some("Bool".to_string()),
                    found: Some(format!("{:?}", cond_type)),
                    location: None,
                    suggestion: Some("use a comparison operator like ==, !=, <, >, etc.".to_string()),
                });
            }

            // Type check branches in child scopes
            let mut branch_ctx = ctx.child();
            for stmt in then_branch {
                typecheck_statement(stmt, &mut branch_ctx)?;
            }

            for (elif_cond, elif_stmts) in elifs {
                let elif_cond_type = typecheck_expr(elif_cond, ctx)?;
                if !types_compatible(&TypeAnnotation::Bool, &elif_cond_type) {
                    return Err(LppoError::TypeError {
                        message: "elif condition must be a boolean".to_string(),
                        expected: Some("Bool".to_string()),
                        found: Some(format!("{:?}", elif_cond_type)),
                        location: None,
                        suggestion: None,
                    });
                }
                let mut elif_ctx = ctx.child();
                for stmt in elif_stmts {
                    typecheck_statement(stmt, &mut elif_ctx)?;
                }
            }

            if let Some(else_stmts) = else_branch {
                let mut else_ctx = ctx.child();
                for stmt in else_stmts {
                    typecheck_statement(stmt, &mut else_ctx)?;
                }
            }

            Ok(None)
        }

        Statement::While { cond, body } => {
            let cond_type = typecheck_expr(cond, ctx)?;
            if !types_compatible(&TypeAnnotation::Bool, &cond_type) {
                return Err(LppoError::TypeError {
                    message: "while condition must be a boolean".to_string(),
                    expected: Some("Bool".to_string()),
                    found: Some(format!("{:?}", cond_type)),
                    location: None,
                    suggestion: None,
                });
            }

            let mut loop_ctx = ctx.child();
            for stmt in body {
                typecheck_statement(stmt, &mut loop_ctx)?;
            }
            Ok(None)
        }

        Statement::For { var, typ, iterable, body } => {
            let iter_type = typecheck_expr(iterable, ctx)?;
            
            // Validate iterable is an array
            match &iter_type {
                TypeAnnotation::Arr(elem_type) => {
                    if !types_compatible(typ, elem_type) {
                        return Err(LppoError::TypeError {
                            message: format!(
                                "loop variable '{}' declared as {:?} but array contains {:?}",
                                var, typ, elem_type
                            ),
                            expected: Some(format!("{:?}", typ)),
                            found: Some(format!("{:?}", elem_type)),
                            location: None,
                            suggestion: Some("ensure loop variable type matches array element type".to_string()),
                        });
                    }
                }
                _ => {
                    return Err(LppoError::TypeError {
                        message: format!("cannot iterate over {:?}, expected array", iter_type),
                        expected: Some("Arr<T>".to_string()),
                        found: Some(format!("{:?}", iter_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }

            let mut loop_ctx = ctx.child();
            loop_ctx.add_variable(var.clone(), typ.clone());
            for stmt in body {
                typecheck_statement(stmt, &mut loop_ctx)?;
            }
            Ok(None)
        }

        Statement::Call(qualified_name, args) => {
            let proc_name = qualified_name_to_string(qualified_name);
            let (param_types, _) = ctx.get_procedure(&proc_name).ok_or_else(|| {
                LppoError::UnknownProcedure {
                    name: proc_name.clone(),
                    location: None,
                    suggestion: Some("check that the procedure is defined and spelled correctly".to_string()),
                }
            })?.clone();

            if args.len() != param_types.len() {
                return Err(LppoError::TypeError {
                    message: format!(
                        "procedure '{}' expects {} arguments but got {}",
                        proc_name,
                        param_types.len(),
                        args.len()
                    ),
                    expected: Some(format!("{} arguments", param_types.len())),
                    found: Some(format!("{} arguments", args.len())),
                    location: None,
                    suggestion: None,
                });
            }

            for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = typecheck_expr(arg, ctx)?;
                if !types_compatible(expected_type, &arg_type) {
                    return Err(LppoError::TypeError {
                        message: format!(
                            "argument {} to '{}' has type {:?} but {:?} expected",
                            i + 1,
                            proc_name,
                            arg_type,
                            expected_type
                        ),
                        expected: Some(format!("{:?}", expected_type)),
                        found: Some(format!("{:?}", arg_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            Ok(None)
        }

        Statement::Emit(exprs) => {
            for expr in exprs {
                typecheck_expr(expr, ctx)?;
            }
            Ok(None)
        }

        Statement::Parallel(exprs) => {
            for expr in exprs {
                typecheck_expr(expr, ctx)?;
            }
            Ok(None)
        }

        Statement::Input { var, typ, .. } => {
            ctx.add_variable(var.clone(), typ.clone());
            Ok(None)
        }

        Statement::Loop { body } => {
            let mut loop_ctx = ctx.child();
            for stmt in body {
                typecheck_statement(stmt, &mut loop_ctx)?;
            }
            Ok(None)
        }

        Statement::Scope { body, .. } => {
            let mut scope_ctx = ctx.child();
            for stmt in body {
                typecheck_statement(stmt, &mut scope_ctx)?;
            }
            Ok(None)
        }

        Statement::Unsafe { body } => {
            let mut unsafe_ctx = ctx.child();
            for stmt in body {
                typecheck_statement(stmt, &mut unsafe_ctx)?;
            }
            Ok(None)
        }

        Statement::Break | Statement::Continue => Ok(None),

        Statement::ExprStatement(expr) => {
            typecheck_expr(expr, ctx)?;
            Ok(None)
        }

        Statement::IndexedAssignment { target, index, value } => {
            let target_type = typecheck_expr(target, ctx)?;
            let index_type = typecheck_expr(index, ctx)?;
            let value_type = typecheck_expr(value, ctx)?;

            match &target_type {
                TypeAnnotation::Arr(elem_type) => {
                    if !types_compatible(&TypeAnnotation::Int64, &index_type) {
                        return Err(LppoError::TypeError {
                            message: "array index must be Int64".to_string(),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", index_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    if !types_compatible(elem_type, &value_type) {
                        return Err(LppoError::TypeError {
                            message: format!(
                                "cannot assign {:?} to array of {:?}",
                                value_type, elem_type
                            ),
                            expected: Some(format!("{:?}", elem_type)),
                            found: Some(format!("{:?}", value_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                }
                _ => {
                    return Err(LppoError::TypeError {
                        message: format!("cannot index into {:?}", target_type),
                        expected: Some("Arr<T>".to_string()),
                        found: Some(format!("{:?}", target_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            Ok(None)
        }

        Statement::Move { source, target } => {
            // Mark source as moved and create new variable with ownership
            ctx.mark_moved(source)?;
            let src_type = ctx.get_variable(source).ok_or_else(|| {
                LppoError::UnknownVariable {
                    name: source.clone(),
                    location: None,
                    suggestion: None,
                }
            })?.clone();
            ctx.add_variable_with_ownership(target.clone(), src_type, OwnershipKind::Own);
            Ok(None)
        }

        Statement::Copy { source, target } => {
            // Copy doesn't move ownership
            let src_type = ctx.get_variable(source).ok_or_else(|| {
                LppoError::UnknownVariable {
                    name: source.clone(),
                    location: None,
                    suggestion: None,
                }
            })?.clone();
            ctx.add_variable_with_ownership(target.clone(), src_type, OwnershipKind::Own);
            Ok(None)
        }

        // For now, treat these as no-ops in type checking, but validate pause operands
        Statement::Alloc { .. }
        | Statement::Dealloc(_)
        | Statement::Match { .. }
        | Statement::StructInit { .. } => Ok(None),

        Statement::Pause { prompt, timeout, key } => {
            if let Some(p) = prompt {
                let p_type = typecheck_expr(p, ctx)?;
                if !types_compatible(&TypeAnnotation::Str, &p_type) {
                    return Err(LppoError::TypeError {
                        message: "pause prompt must be a string".to_string(),
                        expected: Some("Str".to_string()),
                        found: Some(format!("{:?}", p_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            if let Some(t) = timeout {
                let t_type = typecheck_expr(t, ctx)?;
                if !types_compatible(&TypeAnnotation::Int64, &t_type) {
                    return Err(LppoError::TypeError {
                        message: "pause timeout must be an Int64 (milliseconds)".to_string(),
                        expected: Some("Int64".to_string()),
                        found: Some(format!("{:?}", t_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            if let Some(k) = key {
                let k_type = typecheck_expr(k, ctx)?;
                if !types_compatible(&TypeAnnotation::Str, &k_type) {
                    return Err(LppoError::TypeError {
                        message: "pause key must be a string".to_string(),
                        expected: Some("Str".to_string()),
                        found: Some(format!("{:?}", k_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            Ok(None)
        }
    }
}

/// Type check an expression and return its type
fn typecheck_expr(expr: &Expr, ctx: &TypeContext) -> LppoResult<TypeAnnotation> {
    match expr {
        Expr::Int(_) => Ok(TypeAnnotation::Int64),
        Expr::Bool(_) => Ok(TypeAnnotation::Bool),
        Expr::Str(_) => Ok(TypeAnnotation::Str),

        Expr::Var(name) => {
            // Check ownership state before access
            ctx.check_variable_access(name)?;
            ctx.get_variable(name)
                .cloned()
                .ok_or_else(|| LppoError::UnknownVariable {
                    name: name.clone(),
                    location: None,
                    suggestion: Some("declare the variable before using it".to_string()),
                })
        }

        Expr::Array(elements) => {
            if elements.is_empty() {
                // Empty array - can't infer type
                return Ok(TypeAnnotation::Arr(Box::new(TypeAnnotation::Void)));
            }

            let first_type = typecheck_expr(&elements[0], ctx)?;
            for elem in &elements[1..] {
                let elem_type = typecheck_expr(elem, ctx)?;
                if !types_compatible(&first_type, &elem_type) {
                    return Err(LppoError::TypeError {
                        message: format!(
                            "array elements have inconsistent types: {:?} and {:?}",
                            first_type, elem_type
                        ),
                        expected: Some(format!("{:?}", first_type)),
                        found: Some(format!("{:?}", elem_type)),
                        location: None,
                        suggestion: Some("ensure all array elements have the same type".to_string()),
                    });
                }
            }
            Ok(TypeAnnotation::Arr(Box::new(first_type)))
        }

        Expr::Index { target, index } => {
            let target_type = typecheck_expr(target, ctx)?;
            let index_type = typecheck_expr(index, ctx)?;

            if !types_compatible(&TypeAnnotation::Int64, &index_type) {
                return Err(LppoError::TypeError {
                    message: "array index must be Int64".to_string(),
                    expected: Some("Int64".to_string()),
                    found: Some(format!("{:?}", index_type)),
                    location: None,
                    suggestion: None,
                });
            }

            match target_type {
                TypeAnnotation::Arr(elem_type) => Ok(*elem_type),
                _ => Err(LppoError::TypeError {
                    message: format!("cannot index into {:?}", target_type),
                    expected: Some("Arr<T>".to_string()),
                    found: Some(format!("{:?}", target_type)),
                    location: None,
                    suggestion: None,
                }),
            }
        }

        Expr::BinaryOp { left, op, right } => {
            let left_type = typecheck_expr(left, ctx)?;
            let right_type = typecheck_expr(right, ctx)?;

            match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                    if !types_compatible(&TypeAnnotation::Int64, &left_type) {
                        return Err(LppoError::TypeError {
                            message: format!("arithmetic operator {:?} requires Int64 operands", op),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", left_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    if !types_compatible(&TypeAnnotation::Int64, &right_type) {
                        return Err(LppoError::TypeError {
                            message: format!("arithmetic operator {:?} requires Int64 operands", op),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", right_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Int64)
                }

                BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                    if !types_compatible(&left_type, &right_type) {
                        return Err(LppoError::TypeError {
                            message: format!("comparison requires same types, got {:?} and {:?}", left_type, right_type),
                            expected: Some(format!("{:?}", left_type)),
                            found: Some(format!("{:?}", right_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Bool)
                }

                BinaryOp::And | BinaryOp::Or => {
                    if !types_compatible(&TypeAnnotation::Bool, &left_type) {
                        return Err(LppoError::TypeError {
                            message: format!("logical operator {:?} requires Bool operands", op),
                            expected: Some("Bool".to_string()),
                            found: Some(format!("{:?}", left_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    if !types_compatible(&TypeAnnotation::Bool, &right_type) {
                        return Err(LppoError::TypeError {
                            message: format!("logical operator {:?} requires Bool operands", op),
                            expected: Some("Bool".to_string()),
                            found: Some(format!("{:?}", right_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Bool)
                }

                BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                    if !types_compatible(&TypeAnnotation::Int64, &left_type) {
                        return Err(LppoError::TypeError {
                            message: format!("bitwise operator {:?} requires Int64 operands", op),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", left_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    if !types_compatible(&TypeAnnotation::Int64, &right_type) {
                        return Err(LppoError::TypeError {
                            message: format!("bitwise operator {:?} requires Int64 operands", op),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", right_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Int64)
                }
            }
        }

        Expr::UnaryOp { op, operand } => {
            let operand_type = typecheck_expr(operand, ctx)?;

            match op {
                UnaryOp::Neg => {
                    if !types_compatible(&TypeAnnotation::Int64, &operand_type) {
                        return Err(LppoError::TypeError {
                            message: "negation requires Int64 operand".to_string(),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", operand_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Int64)
                }
                UnaryOp::Not => {
                    if !types_compatible(&TypeAnnotation::Bool, &operand_type) {
                        return Err(LppoError::TypeError {
                            message: "logical not requires Bool operand".to_string(),
                            expected: Some("Bool".to_string()),
                            found: Some(format!("{:?}", operand_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Bool)
                }
                UnaryOp::BitNot => {
                    if !types_compatible(&TypeAnnotation::Int64, &operand_type) {
                        return Err(LppoError::TypeError {
                            message: "bitwise not requires Int64 operand".to_string(),
                            expected: Some("Int64".to_string()),
                            found: Some(format!("{:?}", operand_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
                    Ok(TypeAnnotation::Int64)
                }
            }
        }

        Expr::Call { func, args } => {
            // For now, handle simple variable calls (not complex expressions)
            let proc_name = match func.as_ref() {
                Expr::Var(name) => name.clone(),
                Expr::QualifiedVar(qn) => qualified_name_to_string(qn),
                _ => {
                    // Complex call expressions not yet supported in type checking
                    return Ok(TypeAnnotation::Void);
                }
            };
            let (param_types, return_type) = ctx.get_procedure(&proc_name).ok_or_else(|| {
                LppoError::UnknownProcedure {
                    name: proc_name.clone(),
                    location: None,
                    suggestion: Some("check that the procedure is defined and spelled correctly".to_string()),
                }
            })?.clone();

            if args.len() != param_types.len() {
                return Err(LppoError::TypeError {
                    message: format!(
                        "procedure '{}' expects {} arguments but got {}",
                        proc_name,
                        param_types.len(),
                        args.len()
                    ),
                    expected: Some(format!("{} arguments", param_types.len())),
                    found: Some(format!("{} arguments", args.len())),
                    location: None,
                    suggestion: None,
                });
            }

            for (i, (arg, expected_type)) in args.iter().zip(param_types.iter()).enumerate() {
                let arg_type = typecheck_expr(arg, ctx)?;
                if !types_compatible(expected_type, &arg_type) {
                    return Err(LppoError::TypeError {
                        message: format!(
                            "argument {} to '{}' has type {:?} but {:?} expected",
                            i + 1,
                            proc_name,
                            arg_type,
                            expected_type
                        ),
                        expected: Some(format!("{:?}", expected_type)),
                        found: Some(format!("{:?}", arg_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            Ok(return_type)
        }

        // For unimplemented expression types, return a placeholder
        _ => Ok(TypeAnnotation::Void),
    }
}

/// Check if two types are compatible
fn types_compatible(expected: &TypeAnnotation, actual: &TypeAnnotation) -> bool {
    match (expected, actual) {
        (TypeAnnotation::Int64, TypeAnnotation::Int64) => true,
        (TypeAnnotation::Bool, TypeAnnotation::Bool) => true,
        (TypeAnnotation::Str, TypeAnnotation::Str) => true,
        (TypeAnnotation::Void, TypeAnnotation::Void) => true,
        (TypeAnnotation::Arr(e), TypeAnnotation::Arr(a)) => types_compatible(e, a),
        (TypeAnnotation::Custom(e), TypeAnnotation::Custom(a)) => e == a,
        (TypeAnnotation::Generic(e_name, e_params), TypeAnnotation::Generic(a_name, a_params)) => {
            e_name == a_name
                && e_params.len() == a_params.len()
                && e_params
                    .iter()
                    .zip(a_params.iter())
                    .all(|(e, a)| types_compatible(e, a))
        }
        (TypeAnnotation::Option(e), TypeAnnotation::Option(a)) => types_compatible(e, a),
        (TypeAnnotation::Result(e_ok, e_err), TypeAnnotation::Result(a_ok, a_err)) => {
            types_compatible(e_ok, a_ok) && types_compatible(e_err, a_err)
        }
        (TypeAnnotation::Tuple(e_types), TypeAnnotation::Tuple(a_types)) => {
            e_types.len() == a_types.len()
                && e_types
                    .iter()
                    .zip(a_types.iter())
                    .all(|(e, a)| types_compatible(e, a))
        }
        _ => false,
    }
}

/// Convert qualified name to string
fn qualified_name_to_string(qn: &QualifiedName) -> String {
    if qn.path.is_empty() {
        qn.name.clone()
    } else {
        format!("{}::{}", qn.path.join("::"), qn.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_compatible() {
        assert!(types_compatible(&TypeAnnotation::Int64, &TypeAnnotation::Int64));
        assert!(types_compatible(&TypeAnnotation::Bool, &TypeAnnotation::Bool));
        assert!(!types_compatible(&TypeAnnotation::Int64, &TypeAnnotation::Bool));
        
        let arr_int = TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64));
        let arr_int2 = TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64));
        assert!(types_compatible(&arr_int, &arr_int2));
    }

    #[test]
    fn test_expr_type_checking() {
        let mut ctx = TypeContext::new();
        ctx.add_variable("x".to_string(), TypeAnnotation::Int64);
        
        // Test integer literal
        assert_eq!(typecheck_expr(&Expr::Int(42), &ctx).unwrap(), TypeAnnotation::Int64);
        
        // Test boolean literal
        assert_eq!(typecheck_expr(&Expr::Bool(true), &ctx).unwrap(), TypeAnnotation::Bool);
        
        // Test variable lookup
        assert_eq!(typecheck_expr(&Expr::Var("x".to_string()), &ctx).unwrap(), TypeAnnotation::Int64);
        
        // Test unknown variable
        assert!(typecheck_expr(&Expr::Var("y".to_string()), &ctx).is_err());
    }

    #[test]
    fn test_binary_op_type_checking() {
        let ctx = TypeContext::new();
        
        // Test arithmetic: 1 + 2
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Int(2)),
        };
        assert_eq!(typecheck_expr(&expr, &ctx).unwrap(), TypeAnnotation::Int64);
        
        // Test comparison: 1 < 2
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Lt,
            right: Box::new(Expr::Int(2)),
        };
        assert_eq!(typecheck_expr(&expr, &ctx).unwrap(), TypeAnnotation::Bool);
        
        // Test logical: true and false
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Bool(true)),
            op: BinaryOp::And,
            right: Box::new(Expr::Bool(false)),
        };
        assert_eq!(typecheck_expr(&expr, &ctx).unwrap(), TypeAnnotation::Bool);
    }

    #[test]
    fn test_array_type_checking() {
        let ctx = TypeContext::new();
        
        // Test homogeneous array
        let expr = Expr::Array(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)]);
        let result = typecheck_expr(&expr, &ctx).unwrap();
        assert!(matches!(result, TypeAnnotation::Arr(_)));
        
        // Test heterogeneous array (should fail)
        let expr = Expr::Array(vec![Expr::Int(1), Expr::Bool(true)]);
        assert!(typecheck_expr(&expr, &ctx).is_err());
    }

    #[test]
    fn test_move_after_move() {
        use crate::ast::OwnershipKind;
        let mut ctx = TypeContext::new();
        ctx.add_variable_with_ownership("x".to_string(), TypeAnnotation::Int64, OwnershipKind::Own);
        
        // First move should succeed
        assert!(ctx.mark_moved("x").is_ok());
        
        // Second move should fail
        assert!(ctx.mark_moved("x").is_err());
        
        // Access after move should fail
        assert!(ctx.check_variable_access("x").is_err());
    }

    #[test]
    fn test_ownership_tracking() {
        use crate::ast::OwnershipKind;
        let mut ctx = TypeContext::new();
        
        // Add owned variable
        ctx.add_variable_with_ownership("x".to_string(), TypeAnnotation::Int64, OwnershipKind::Own);
        
        // Should be accessible
        assert!(ctx.check_variable_access("x").is_ok());
        assert!(!ctx.is_moved("x"));
        
        // After moving, should not be accessible
        ctx.mark_moved("x").unwrap();
        assert!(ctx.is_moved("x"));
        assert!(ctx.check_variable_access("x").is_err());
    }
}
