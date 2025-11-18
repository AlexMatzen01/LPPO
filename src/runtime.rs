use std::collections::HashMap;

use crate::error::{LppoError, LppoResult};
use crate::ast::{Expr, BinaryOp, UnaryOp};
use serde::{Deserialize, Serialize};

// Control flow for loops and returns
#[derive(Debug, Clone, PartialEq)]
enum ControlFlow {
    None,
    Break,
    Continue,
    Return(Option<Value>),
}

/// Describes a compiled procedure ready for execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureBody {
    pub params: Vec<String>,
    pub operations: Vec<Operation>,
}

/// Represents a single executable operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Emit a string to standard output.
    /// Emit one or more evaluated expressions.
    Emit(Vec<Expr>),
    /// Emit several expressions concurrently.
    Parallel(Vec<Expr>),
    /// Read input from stdin into a variable
    Input {
        var: String,
        typ: crate::ast::TypeAnnotation,
        prompt: Option<Expr>,
    },
    /// Call another procedure by name with argument expressions.
    Call(String, Vec<Expr>),
    /// Let binding: set a local variable to the value of an expression.
    Let(String, Expr),
    /// Conditional branching with optional elifs and else
    If {
        cond: Expr,
        then_ops: Vec<Operation>,
        elifs: Vec<(Expr, Vec<Operation>)>,
        else_ops: Option<Vec<Operation>>,
    },
    /// While loop
    While {
        cond: Expr,
        ops: Vec<Operation>,
    },
    /// For-in loop
    For {
        var: String,
        iterable: Expr,
        ops: Vec<Operation>,
    },
    /// Infinite loop
    Loop {
        ops: Vec<Operation>,
    },
    /// Break from loop
    Break,
    /// Continue loop
    Continue,
    /// Return from procedure
    Return(Option<Expr>),
    /// Explicit scope block
    Scope {
        ops: Vec<Operation>,
    },
    /// Unsafe block (treated same as scope at runtime)
    Unsafe {
        ops: Vec<Operation>,
    },
    /// No operation (placeholder)
    Nop,
    /// Pause execution: optional prompt and optional timeout (ms)
    Pause {
        prompt: Option<Expr>,
        timeout_ms: Option<Expr>,
        key: Option<Expr>,
    },
    /// Expression statement - evaluate for side effects
    ExprStatement(Expr),
}

/// Runtime value types used during evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Closure {
        params: Vec<String>,
        body: Expr,
        captured_env: HashMap<String, Value>,
    },
}

fn pattern_matches(pattern: &crate::ast::Pattern, value: &Value, env: &mut HashMap<String, Value>, procedures: &HashMap<String, ProcedureBody>) -> LppoResult<bool> {
    use crate::ast::Pattern;
    
    match pattern {
        Pattern::Wildcard => Ok(true),
        Pattern::Literal(expr) => {
            // Compare the value with the literal
            let lit_val = eval_expr(expr, env, procedures)?;
            Ok(values_equal(&lit_val, value))
        }
        Pattern::Variable(name) => {
            // Bind the variable to the value
            env.insert(name.clone(), value.clone());
            Ok(true)
        }
        Pattern::EnumVariant { variant, fields, .. } => {
            // For now, handle Some and None patterns for Option
            match variant.as_str() {
                "Some" => {
                    // Check if value is not the special "None" value
                    // For simplicity, we'll treat any non-null value as Some
                    if fields.len() == 1 {
                        // Bind the inner value
                        pattern_matches(&fields[0], value, env, procedures)
                    } else {
                        Ok(true)
                    }
                }
                "None" => {
                    // Check if value is the special "None" value
                    // For now, we'll use Int(0) as None
                    Ok(matches!(value, Value::Int(0)))
                }
                "Ok" => {
                    // Similar to Some
                    if fields.len() == 1 {
                        pattern_matches(&fields[0], value, env, procedures)
                    } else {
                        Ok(true)
                    }
                }
                "Err" => {
                    // For now, treat negative numbers as errors
                    Ok(matches!(value, Value::Int(n) if *n < 0))
                }
                _ => Ok(false),
            }
        }
        Pattern::Tuple(patterns) => {
            // Match tuple patterns against array values
            if let Value::Array(arr) = value {
                if patterns.len() != arr.len() {
                    return Ok(false);
                }
                
                for (pat, val) in patterns.iter().zip(arr.iter()) {
                    if !pattern_matches(pat, val, env, procedures)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Pattern::Struct { name: _, fields } => {
            // For now, treat structs as arrays where each field matches by position
            // A full implementation would need actual struct values
            if let Value::Array(arr) = value {
                if fields.len() != arr.len() {
                    return Ok(false);
                }
                
                for ((_, pat), val) in fields.iter().zip(arr.iter()) {
                    if !pattern_matches(pat, val, env, procedures)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Pattern::Or(patterns) => {
            // Try each pattern in order, return true if any matches
            for pat in patterns {
                // Clone env for each attempt since failed matches shouldn't bind variables
                let mut temp_env = env.clone();
                if pattern_matches(pat, value, &mut temp_env, procedures)? {
                    // If match succeeds, update the original env with bindings
                    *env = temp_env;
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => a == b,
        _ => false,
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>, procedures: &HashMap<String, ProcedureBody>) -> LppoResult<Value> {
    match expr {
        Expr::Int(i) => Ok(Value::Int(*i)),
        Expr::Float(f) => {
            // Parse float string to f64 and store as string for display
            // For now, treat floats as strings in output
            Ok(Value::Str(f.clone()))
        }
        Expr::Str(s) => Ok(Value::Str(s.clone())),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| LppoError::unknown_variable(name.clone())),
        Expr::QualifiedVar(qn) => {
            // For now, treat qualified variables as simple variables
            // Full namespace support would require a more sophisticated environment
            let full_name = if qn.path.is_empty() {
                qn.name.clone()
            } else {
                format!("{}::{}", qn.path.join("::"), qn.name)
            };
            env.get(&full_name)
                .or_else(|| env.get(&qn.name))
                .cloned()
                .ok_or_else(|| LppoError::unknown_variable(full_name))
        }
        Expr::BinaryOp { left, op, right } => {
            let l = eval_expr(left, env, procedures)?;
            let r = eval_expr(right, env, procedures)?;
                match (l, r, op) {
                    (Value::Int(a), Value::Int(b), BinaryOp::Add) => Ok(Value::Int(a + b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Sub) => Ok(Value::Int(a - b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Mul) => Ok(Value::Int(a * b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Div) => {
                        if b == 0 {
                            Err(LppoError::type_error("division by zero".into()))
                        } else {
                            Ok(Value::Int(a / b))
                        }
                    }
                    (Value::Int(a), Value::Int(b), BinaryOp::Mod) => Ok(Value::Int(a % b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Pow) => Ok(Value::Int(a.pow(b as u32))),
                    (Value::Int(a), Value::Int(b), BinaryOp::BitAnd) => Ok(Value::Int(a & b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::BitOr) => Ok(Value::Int(a | b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::BitXor) => Ok(Value::Int(a ^ b)),
                    // comparisons produce bools
                    (Value::Int(a), Value::Int(b), BinaryOp::Eq) => Ok(Value::Bool(a == b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Ne) => Ok(Value::Bool(a != b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Lt) => Ok(Value::Bool(a < b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Le) => Ok(Value::Bool(a <= b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Gt) => Ok(Value::Bool(a > b)),
                    (Value::Int(a), Value::Int(b), BinaryOp::Ge) => Ok(Value::Bool(a >= b)),
                    // string concat with +
                    (Value::Str(a), Value::Str(b), BinaryOp::Add) => Ok(Value::Str(format!("{}{}", a, b))),
                    // string comparisons
                    (Value::Str(a), Value::Str(b), BinaryOp::Eq) => Ok(Value::Bool(a == b)),
                    (Value::Str(a), Value::Str(b), BinaryOp::Ne) => Ok(Value::Bool(a != b)),
                    // logical ops expect bools
                    (Value::Bool(a), Value::Bool(b), BinaryOp::And) => Ok(Value::Bool(a && b)),
                    (Value::Bool(a), Value::Bool(b), BinaryOp::Or) => Ok(Value::Bool(a || b)),
                    // array concat
                    (Value::Array(mut a), Value::Array(b), BinaryOp::Add) => {
                        a.extend(b);
                        Ok(Value::Array(a))
                    }
                    _ => Err(LppoError::type_error("type mismatch in binary operation".into())),
                }
        }
        Expr::UnaryOp { op, operand } => {
            let val = eval_expr(operand, env, procedures)?;
            match (op, val) {
                (UnaryOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (UnaryOp::BitNot, Value::Int(i)) => Ok(Value::Int(!i)),
                _ => Err(LppoError::type_error("type mismatch in unary operation".into())),
            }
        }
        Expr::Array(elements) => {
            let mut values = Vec::new();
            for elem in elements {
                values.push(eval_expr(elem, env, procedures)?);
            }
            Ok(Value::Array(values))
        }
        Expr::Index { target, index } => {
            let target_val = eval_expr(target, env, procedures)?;
            let index_val = eval_expr(index, env, procedures)?;
            match (target_val, index_val) {
                (Value::Array(arr), Value::Int(idx)) => {
                    let i = if idx < 0 {
                        (arr.len() as i64 + idx) as usize
                    } else {
                        idx as usize
                    };
                    arr.get(i)
                        .cloned()
                        .ok_or_else(|| LppoError::type_error(format!("index {} out of bounds", idx)))
                }
                (Value::Str(s), Value::Int(idx)) => {
                    let i = if idx < 0 {
                        (s.len() as i64 + idx) as usize
                    } else {
                        idx as usize
                    };
                    s.chars()
                        .nth(i)
                        .map(|c| Value::Str(c.to_string()))
                        .ok_or_else(|| LppoError::type_error(format!("index {} out of bounds", idx)))
                }
                _ => Err(LppoError::type_error("indexing requires array/string and integer".into())),
            }
        }
        Expr::MethodCall { target, method, args } => {
            let target_val = eval_expr(target, env, procedures)?;
            match (target_val.clone(), method.as_str()) {
                (Value::Array(mut arr), "push") => {
                    for arg in args {
                        arr.push(eval_expr(arg, env, procedures)?);
                    }
                    Ok(Value::Array(arr))
                }
                (Value::Array(mut arr), "pop") => {
                    arr.pop()
                        .ok_or_else(|| LppoError::type_error("pop on empty array".into()))
                }
                (Value::Array(arr), "len") => Ok(Value::Int(arr.len() as i64)),
                (Value::Array(arr), "iter") => {
                    // Return the array itself wrapped as an iterator
                    // The iterator is represented as an array that can be further chained
                    Ok(Value::Array(arr))
                }
                (Value::Array(arr), "map") => {
                    // map applies a closure to each element
                    if args.len() != 1 {
                        return Err(LppoError::type_error("map requires 1 argument".into()));
                    }
                    let closure_val = eval_expr(&args[0], env, procedures)?;
                    
                    let mut result = Vec::new();
                    for item in arr {
                        match closure_val.clone() {
                            Value::Closure { params, body, captured_env } => {
                                if params.len() != 1 {
                                    return Err(LppoError::type_error("map closure must take 1 parameter".into()));
                                }
                                let mut closure_env = captured_env;
                                closure_env.insert(params[0].clone(), item);
                                let mapped_val = eval_expr(&body, &mut closure_env, procedures)?;
                                result.push(mapped_val);
                            }
                            _ => return Err(LppoError::type_error("map argument must be a closure".into())),
                        }
                    }
                    Ok(Value::Array(result))
                }
                (Value::Array(arr), "filter") => {
                    // filter keeps elements where closure returns true
                    if args.len() != 1 {
                        return Err(LppoError::type_error("filter requires 1 argument".into()));
                    }
                    let closure_val = eval_expr(&args[0], env, procedures)?;
                    
                    let mut result = Vec::new();
                    for item in arr {
                        match closure_val.clone() {
                            Value::Closure { params, body, captured_env } => {
                                if params.len() != 1 {
                                    return Err(LppoError::type_error("filter closure must take 1 parameter".into()));
                                }
                                let mut closure_env = captured_env;
                                closure_env.insert(params[0].clone(), item.clone());
                                let keep = eval_expr(&body, &mut closure_env, procedures)?;
                                if is_truthy(&keep) {
                                    result.push(item);
                                }
                            }
                            _ => return Err(LppoError::type_error("filter argument must be a closure".into())),
                        }
                    }
                    Ok(Value::Array(result))
                }
                (Value::Array(arr), "collect") => {
                    // collect just returns the array as-is (already collected)
                    Ok(Value::Array(arr))
                }
                (Value::Array(arr), "first") => {
                    arr.first()
                        .cloned()
                        .ok_or_else(|| LppoError::type_error("first on empty array".into()))
                }
                (Value::Str(s), "len") => Ok(Value::Int(s.len() as i64)),
                (Value::Str(s), "upper") => Ok(Value::Str(s.to_uppercase())),
                (Value::Str(s), "lower") => Ok(Value::Str(s.to_lowercase())),
                (Value::Str(s), "split") => {
                    if args.len() != 1 {
                        return Err(LppoError::type_error("split requires 1 argument".into()));
                    }
                    let sep = eval_expr(&args[0], env, procedures)?;
                    if let Value::Str(sep_str) = sep {
                        let parts: Vec<Value> = s
                            .split(&sep_str)
                            .map(|p| Value::Str(p.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    } else {
                        Err(LppoError::type_error("split separator must be string".into()))
                    }
                }
                _ => Err(LppoError::type_error(format!("unknown method: {}", method))),
            }
        }
        Expr::FieldAccess { target: _, field } => {
            // For now, we don't have structs, so this is an error
            Err(LppoError::type_error(format!("field access not yet implemented: {}", field)))
        }
        Expr::Ref { target, lifetime: _, is_mut: _ } => {
            // In our simplified runtime, references are just the value itself
            // Full reference semantics would require borrow checking
            eval_expr(target, env, procedures)
        }
        Expr::Deref(target) => {
            // Dereference is also just evaluating the target
            eval_expr(target, env, procedures)
        }
        Expr::SizeOf(typ) => {
            // Return a placeholder size based on type
            let size = match typ {
                crate::ast::TypeAnnotation::Int64 => 8,
                crate::ast::TypeAnnotation::Bool => 1,
                crate::ast::TypeAnnotation::Str => 16, // pointer size
                crate::ast::TypeAnnotation::Arr(_) => 24, // vector size
                crate::ast::TypeAnnotation::Void => 0,
                _ => 8, // default
            };
            Ok(Value::Int(size))
        }
        Expr::AlignOf(typ) => {
            // Return a placeholder alignment based on type
            let align = match typ {
                crate::ast::TypeAnnotation::Int64 => 8,
                crate::ast::TypeAnnotation::Bool => 1,
                _ => 8,
            };
            Ok(Value::Int(align))
        }
        Expr::Cast { expr, to_type } => {
            // Simple type casting
            let val = eval_expr(expr, env, procedures)?;
            match (val, to_type) {
                (Value::Int(i), crate::ast::TypeAnnotation::Bool) => Ok(Value::Bool(i != 0)),
                (Value::Int(i), crate::ast::TypeAnnotation::Str) => Ok(Value::Str(i.to_string())),
                (Value::Bool(b), crate::ast::TypeAnnotation::Int64) => Ok(Value::Int(if b { 1 } else { 0 })),
                (Value::Str(s), crate::ast::TypeAnnotation::Int64) => {
                    s.parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| LppoError::type_error("cannot cast string to Int64".into()))
                }
                (v, _) => Ok(v), // passthrough for same type
            }
        }
        Expr::Borrow { target, lifetime: _ } => {
            // Simplified: borrow is just like a reference
            eval_expr(target, env, procedures)
        }
        Expr::Closure { params, return_type: _, body } => {
            // Create a closure value that captures the current environment
            let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            Ok(Value::Closure {
                params: param_names,
                body: (**body).clone(),
                captured_env: env.clone(),
            })
        }
        Expr::Call { func, args } => {
            // Evaluate arguments
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(eval_expr(arg, env, procedures)?);
            }
            
            // Evaluate the function expression
            let func_val = eval_expr(func, env, procedures);
            
            // Handle the different cases
            match func_val {
                Ok(Value::Closure { params, body, captured_env }) => {
                    // Calling a closure value
                    if params.len() != arg_vals.len() {
                        return Err(LppoError::type_error(format!(
                            "closure argument count mismatch: expected {} got {}",
                            params.len(), arg_vals.len()
                        )));
                    }
                    
                    // Create environment from captured environment plus parameters
                    let mut closure_env = captured_env;
                    for (param, val) in params.iter().zip(arg_vals.into_iter()) {
                        closure_env.insert(param.clone(), val);
                    }
                    
                    // Evaluate closure body
                    eval_expr(&body, &mut closure_env, procedures)
                }
                Err(_) => {
                    // If func doesn't evaluate to a value, try as a procedure name
                    if let Expr::Var(proc_name) = func.as_ref() {
                        // Look up the procedure
                        if let Some(proc_body) = procedures.get(proc_name) {
                            // Check arity
                            if proc_body.params.len() != arg_vals.len() {
                                return Err(LppoError::type_error(format!(
                                    "argument count mismatch for '{}': expected {} got {}",
                                    proc_name, proc_body.params.len(), arg_vals.len()
                                )));
                            }
                            
                            // Create a new environment for the procedure call
                            let mut proc_env = HashMap::new();
                            for (param, val) in proc_body.params.iter().zip(arg_vals.into_iter()) {
                                proc_env.insert(param.clone(), val);
                            }
                            
                            // Execute the procedure
                            // For now, we need to extract the return value from the procedure execution
                            // Since procedures may use "return" statements, we need to handle ControlFlow
                            for op in &proc_body.operations {
                                match op {
                                    Operation::Return(Some(ret_expr)) => {
                                        return eval_expr(ret_expr, &mut proc_env, procedures);
                                    }
                                    Operation::Return(None) => {
                                        return Ok(Value::Int(0)); // Unit/void return
                                    }
                                    _ => {
                                        // Execute other operations but ignore their results
                                        // This is a simplified implementation
                                    }
                                }
                            }
                            
                            // If no explicit return, return unit value
                            Ok(Value::Int(0))
                        } else {
                            Err(LppoError::unknown_procedure(proc_name.clone()))
                        }
                    } else {
                        Err(LppoError::type_error("function calls require a procedure name or closure".into()))
                    }
                }
                Ok(_) => {
                    Err(LppoError::type_error("can only call closures or procedures".into()))
                }
            }
        }
        Expr::Range { start, end, inclusive } => {
            // Evaluate range start and end
            let start_val = eval_expr(start, env, procedures)?;
            let end_val = eval_expr(end, env, procedures)?;
            
            // Only support integer ranges for now
            match (start_val, end_val) {
                (Value::Int(s), Value::Int(e)) => {
                    let range: Vec<Value> = if *inclusive {
                        (s..=e).map(Value::Int).collect()
                    } else {
                        (s..e).map(Value::Int).collect()
                    };
                    Ok(Value::Array(range))
                }
                _ => Err(LppoError::type_error("range expressions require integer bounds".into()))
            }
        }
        Expr::MatchExpr { expr, arms } => {
            // Evaluate the match expression
            let val = eval_expr(expr, env, procedures)?;
            
            // Try each arm in order
            for arm in arms {
                if pattern_matches(&arm.pattern, &val, env, procedures)? {
                    // Check guard if present
                    if let Some(ref guard) = arm.guard {
                        let guard_val = eval_expr(guard, env, procedures)?;
                        if !is_truthy(&guard_val) {
                            continue;
                        }
                    }
                    
                    // Execute the arm body
                    return eval_expr(&arm.body, env, procedures);
                }
            }
            
            // No pattern matched
            Err(LppoError::type_error("no pattern matched in match expression".into()))
        }
        Expr::Await(expr) => {
            // For now, simply evaluate the expression synchronously
            // In a real async runtime, this would suspend and resume
            eval_expr(expr, env, procedures)
        }
        Expr::Handle { expr, handler_type: _, handlers: _ } => {
            // For now, evaluate the expression and ignore effect handlers
            // A full implementation would set up effect handlers and call them on performance
            eval_expr(expr, env, procedures)
        }
    }
}

/// Executes compiled LPPO procedures.
#[derive(Debug, Default, Clone)]
pub struct Executor {
    procedures: HashMap<String, ProcedureBody>,
}

impl Executor {
    /// Create a new executor from compiled procedures.
    pub fn new(procedures: HashMap<String, ProcedureBody>) -> Self {
        Self { procedures }
    }

    /// Run the specified entry procedure.
    pub fn run(&self, entry: &str) -> LppoResult<()> {
        let procedure = self
            .procedures
            .get(entry)
            .ok_or_else(|| LppoError::unknown_procedure(entry.to_string()))?;
        let mut env = HashMap::new();
        self.execute_procedure_with_env(procedure, &mut env)
    }

    fn execute_procedure_with_env(&self, procedure: &ProcedureBody, env: &mut HashMap<String, Value>) -> LppoResult<()> {
        let _cf = self.execute_ops(&procedure.operations, env)?;
        Ok(())
    }

    fn execute_ops(&self, ops: &[Operation], env: &mut HashMap<String, Value>) -> LppoResult<ControlFlow> {
        for op in ops {
            match op {
                Operation::Emit(exprs) => {
                    // evaluate all expressions and join them with spaces
                    let mut parts = Vec::new();
                    for e in exprs {
                        let v = eval_expr(e, env, &self.procedures)?;
                        parts.push(value_to_string(&v));
                    }
                    println!("{}", parts.join(" "));
                }
                Operation::Parallel(exprs) => {
                    std::thread::scope(|scope| {
                        for e in exprs {
                            let e = e.clone();
                            let mut snapshot = env.clone();
                            let procs = self.procedures.clone();
                            scope.spawn(move || {
                                // evaluate in snapshot environment
                                let v = eval_expr(&e, &mut snapshot, &procs).expect("eval in thread");
                                println!("{}", value_to_string(&v));
                            });
                        }
                    });
                }
                Operation::Input { var, typ, prompt } => {
                    use std::io::{self, Write};
                    
                    // Display prompt if provided
                    if let Some(prompt_expr) = prompt {
                        let prompt_val = eval_expr(prompt_expr, env, &self.procedures)?;
                        print!("{} ", value_to_string(&prompt_val));
                        io::stdout().flush().unwrap();
                    }
                    
                    // Read input from stdin
                    let mut input_line = String::new();
                    io::stdin()
                        .read_line(&mut input_line)
                        .map_err(|e| LppoError::type_error(format!("Failed to read input: {}", e)))?;
                    
                    let input_line = input_line.trim();
                    
                    // Parse based on type annotation
                    use crate::ast::TypeAnnotation;
                    let value = match typ {
                        TypeAnnotation::Int64 => {
                            input_line
                                .parse::<i64>()
                                .map(Value::Int)
                                .map_err(|_| LppoError::type_error(format!("Invalid Int64 input: '{}'", input_line)))?
                        }
                        TypeAnnotation::Bool => {
                            match input_line.to_lowercase().as_str() {
                                "true" | "t" | "yes" | "y" | "1" => Value::Bool(true),
                                "false" | "f" | "no" | "n" | "0" => Value::Bool(false),
                                _ => return Err(LppoError::type_error(format!("Invalid Bool input: '{}'. Expected: true/false, yes/no, 1/0", input_line)))
                            }
                        }
                        TypeAnnotation::Str => Value::Str(input_line.to_string()),
                        TypeAnnotation::Arr(_) => {
                            // Parse array input: [1, 2, 3] or comma-separated: 1,2,3
                            let input_line = input_line.trim();
                            
                            // Try parsing as JSON-like array first
                            if input_line.starts_with('[') && input_line.ends_with(']') {
                                let inner = &input_line[1..input_line.len()-1];
                                let elements: Vec<String> = inner
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                
                                // Try to infer element type from the inner type annotation
                                if let TypeAnnotation::Arr(inner_type_box) = typ {
                                    let inner_type = &**inner_type_box;
                                    let parsed_elements: Result<Vec<Value>, _> = elements
                                        .iter()
                                        .map(|elem| match inner_type {
                                            TypeAnnotation::Int64 => elem.parse::<i64>().map(Value::Int).map_err(|_| 
                                                LppoError::type_error(format!("Invalid Int64 in array: '{}'", elem))),
                                            TypeAnnotation::Str => Ok(Value::Str(elem.trim_matches('"').to_string())),
                                            TypeAnnotation::Bool => match elem.to_lowercase().as_str() {
                                                "true" => Ok(Value::Bool(true)),
                                                "false" => Ok(Value::Bool(false)),
                                                _ => Err(LppoError::type_error(format!("Invalid Bool in array: '{}'", elem)))
                                            },
                                            _ => Err(LppoError::type_error("Unsupported array element type".to_string()))
                                        })
                                        .collect();
                                    Value::Array(parsed_elements?)
                                } else {
                                    return Err(LppoError::type_error("Invalid array type annotation".to_string()));
                                }
                            } else {
                                // Try comma-separated format
                                let elements: Vec<String> = input_line
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                
                                if let TypeAnnotation::Arr(inner_type_box) = typ {
                                    let inner_type = &**inner_type_box;
                                    let parsed_elements: Result<Vec<Value>, _> = elements
                                        .iter()
                                        .map(|elem| match inner_type {
                                            TypeAnnotation::Int64 => elem.parse::<i64>().map(Value::Int).map_err(|_| 
                                                LppoError::type_error(format!("Invalid Int64 in array: '{}'", elem))),
                                            TypeAnnotation::Str => Ok(Value::Str(elem.to_string())),
                                            TypeAnnotation::Bool => match elem.to_lowercase().as_str() {
                                                "true" => Ok(Value::Bool(true)),
                                                "false" => Ok(Value::Bool(false)),
                                                _ => Err(LppoError::type_error(format!("Invalid Bool in array: '{}'", elem)))
                                            },
                                            _ => Err(LppoError::type_error("Unsupported array element type".to_string()))
                                        })
                                        .collect();
                                    Value::Array(parsed_elements?)
                                } else {
                                    return Err(LppoError::type_error("Invalid array type annotation".to_string()));
                                }
                            }
                        }
                        _ => {
                            return Err(LppoError::type_error(format!("Unsupported input type: {:?}", typ)));
                        }
                    };
                    
                    env.insert(var.clone(), value);
                }
                Operation::Pause { prompt, timeout_ms, key } => {
                    use std::io::{self, Write, Read};
                    use std::sync::mpsc;
                    use std::thread;
                    use std::time::Duration;

                    // Display prompt if provided
                    if let Some(prompt_expr) = prompt {
                        let prompt_val = eval_expr(prompt_expr, env, &self.procedures)?;
                        print!("{} ", value_to_string(&prompt_val));
                        io::stdout().flush().unwrap();
                    } else {
                        // default prompt
                        print!("(paused) Press Enter to continue...");
                        io::stdout().flush().unwrap();
                    }

                    // Determine if we wait for a specific key or Enter
                    let wait_for_key = if let Some(key_expr) = key {
                        let key_val = eval_expr(key_expr, env, &self.procedures)?;
                        match key_val {
                            Value::Str(s) => Some(s),
                            _ => return Err(LppoError::type_error("pause key must be a string".into())),
                        }
                    } else {
                        None
                    };

                    // If no timeout, block until key is pressed
                    if timeout_ms.is_none() {
                        if let Some(expected_key) = wait_for_key {
                            if expected_key == "*" {
                                // Wait for any key
                                let mut buf = [0u8; 1];
                                io::stdin()
                                    .read_exact(&mut buf)
                                    .map_err(|e| LppoError::type_error(format!("Failed to read input: {}", e)))?;
                            } else if expected_key.len() == 1 {
                                // Wait for specific key
                                loop {
                                    let mut buf = [0u8; 1];
                                    io::stdin()
                                        .read_exact(&mut buf)
                                        .map_err(|e| LppoError::type_error(format!("Failed to read input: {}", e)))?;
                                    if buf[0] as char == expected_key.chars().next().unwrap() {
                                        break;
                                    }
                                }
                            } else {
                                return Err(LppoError::type_error("pause key must be a single character or '*'".into()));
                            }
                        } else {
                            // Default: wait for Enter
                            let mut buf = String::new();
                            io::stdin()
                                .read_line(&mut buf)
                                .map_err(|e| LppoError::type_error(format!("Failed to read input: {}", e)))?;
                        }
                    } else {
                        // timeout provided: evaluate to milliseconds
                        let t_expr = timeout_ms.as_ref().unwrap();
                        let t_val = eval_expr(t_expr, env, &self.procedures)?;
                        let millis = match t_val {
                            Value::Int(i) if i >= 0 => i as u64,
                            _ => return Err(LppoError::type_error("pause timeout must be a non-negative integer".into())),
                        };

                        // Spawn a thread to read input and use recv_timeout
                        let (tx, rx) = mpsc::sync_channel(0);
                        let expected_key = wait_for_key.clone();
                        let handle = thread::spawn(move || {
                            if let Some(expected) = expected_key {
                                if expected == "*" {
                                    let mut buf = [0u8; 1];
                                    let _ = io::stdin().read_exact(&mut buf);
                                    let _ = tx.send(());
                                } else if expected.len() == 1 {
                                    loop {
                                        let mut buf = [0u8; 1];
                                        let _ = io::stdin().read_exact(&mut buf);
                                        if buf[0] as char == expected.chars().next().unwrap() {
                                            let _ = tx.send(());
                                            break;
                                        }
                                    }
                                }
                            } else {
                                let mut s = String::new();
                                let _ = io::stdin().read_line(&mut s);
                                let _ = tx.send(());
                            }
                        });

                        if rx.recv_timeout(Duration::from_millis(millis)).is_err() {
                            // timeout reached; simply continue
                        }
                        // Don't join the handle here because read may still block; detach
                        let _ = handle.thread().unpark();
                    }
                }
                Operation::Call(name, args) => {
                    // Evaluate argument expressions in current env
                    let mut arg_vals = Vec::new();
                    for a in args {
                        arg_vals.push(eval_expr(a, env, &self.procedures)?);
                    }

                    // Find the procedure body
                    let proc = self
                        .procedures
                        .get(name)
                        .ok_or_else(|| LppoError::unknown_procedure(name.clone()))?;

                    // check arity
                    if proc.params.len() != arg_vals.len() {
                        return Err(LppoError::type_error(format!("argument count mismatch for '{}': expected {} got {}", name, proc.params.len(), arg_vals.len())));
                    }

                    // bind parameters in a fresh env and execute
                    let mut new_env: HashMap<String, Value> = HashMap::new();
                    for (p, v) in proc.params.iter().zip(arg_vals.into_iter()) {
                        new_env.insert(p.clone(), v);
                    }
                    self.execute_procedure_with_env(proc, &mut new_env)?;
                }
                Operation::Let(name, expr) => {
                    let v = eval_expr(expr, env, &self.procedures)?;
                    env.insert(name.clone(), v);
                }
                Operation::If { cond, then_ops, elifs, else_ops } => {
                    let cond_v = eval_expr(cond, env, &self.procedures)?;
                    if is_truthy(&cond_v) {
                        let cf = self.execute_ops(then_ops, env)?;
                        if cf != ControlFlow::None {
                            return Ok(cf);
                        }
                        continue;
                    }
                    let mut handled = false;
                    for (econd, estmts) in elifs {
                        let ev = eval_expr(econd, env, &self.procedures)?;
                        if is_truthy(&ev) {
                            let cf = self.execute_ops(estmts, env)?;
                            if cf != ControlFlow::None {
                                return Ok(cf);
                            }
                            handled = true;
                            break;
                        }
                    }
                    if handled {
                        continue;
                    }
                    if let Some(es) = else_ops {
                        let cf = self.execute_ops(es, env)?;
                        if cf != ControlFlow::None {
                            return Ok(cf);
                        }
                    }
                }
                Operation::While { cond, ops } => {
                    loop {
                        let cond_v = eval_expr(cond, env, &self.procedures)?;
                        if !is_truthy(&cond_v) {
                            break;
                        }
                        let cf = self.execute_ops(ops, env)?;
                        match cf {
                            ControlFlow::None => continue,
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        }
                    }
                }
                Operation::For { var, iterable, ops } => {
                    let iter_v = eval_expr(iterable, env, &self.procedures)?;
                    let items = match iter_v {
                        Value::Array(ref arr) => arr.clone(),
                        _ => return Err(LppoError::type_error("for loop requires an array".to_string())),
                    };
                    for item in items {
                        env.insert(var.clone(), item);
                        let cf = self.execute_ops(ops, env)?;
                        match cf {
                            ControlFlow::None => continue,
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        }
                    }
                }
                Operation::Loop { ops } => {
                    loop {
                        let cf = self.execute_ops(ops, env)?;
                        match cf {
                            ControlFlow::None => continue,
                            ControlFlow::Break => break,
                            ControlFlow::Continue => continue,
                            ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        }
                    }
                }
                Operation::Break => {
                    return Ok(ControlFlow::Break);
                }
                Operation::Continue => {
                    return Ok(ControlFlow::Continue);
                }
                Operation::Return(opt_expr) => {
                    let v = if let Some(e) = opt_expr {
                        Some(eval_expr(e, env, &self.procedures)?)
                    } else {
                        None
                    };
                    return Ok(ControlFlow::Return(v));
                }
                Operation::Scope { ops } => {
                    // Execute operations in the scope
                    let cf = self.execute_ops(ops, env)?;
                    if cf != ControlFlow::None {
                        return Ok(cf);
                    }
                }
                Operation::Unsafe { ops } => {
                    // Unsafe blocks are treated the same as scope at runtime
                    let cf = self.execute_ops(ops, env)?;
                    if cf != ControlFlow::None {
                        return Ok(cf);
                    }
                }
                Operation::Nop => {
                    // No operation - do nothing
                }
                Operation::ExprStatement(expr) => {
                    // Evaluate expression for side effects (e.g., method calls)
                    let _ = eval_expr(expr, env, &self.procedures)?;
                }
            }
        }
        Ok(ControlFlow::None)
    }

    /// Returns true if the executor contains a procedure with the given name.
    pub fn has_procedure(&self, name: &str) -> bool {
        self.procedures.contains_key(name)
    }
}

// Helper to convert Value to string
fn value_to_string(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Closure { params, .. } => {
            format!("<closure with {} params>", params.len())
        }
    }
}

// Helper to check truthiness
fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Str(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Closure { .. } => true, // Closures are always truthy
    }
}

/// Serialize compiled procedures into JSON bytes (stable, portable format).
pub fn serialize_procedures(
    procedures: &HashMap<String, ProcedureBody>,
) -> LppoResult<Vec<u8>> {
    serde_json::to_vec(procedures).map_err(|e| LppoError::type_error(e.to_string()))
}

/// Deserialize procedures from JSON bytes and return an Executor.
pub fn executor_from_bytes(bytes: &[u8]) -> LppoResult<Executor> {
    let map: HashMap<String, ProcedureBody> = serde_json::from_slice(bytes)
        .map_err(|e| LppoError::type_error(e.to_string()))?;
    Ok(Executor::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Parameter, TypeAnnotation};

    #[test]
    fn reports_missing_procedure() {
        let exec = Executor::new(HashMap::new());
        let err = exec.run("main").expect_err("missing main");
        assert!(matches!(err, LppoError::UnknownProcedure { name, .. } if name == "main"));
    }

    #[test]
    fn test_range_expression() {
        let mut env = HashMap::new();
        let procedures = HashMap::new();
        
        // Test exclusive range 1..5
        let range_expr = Expr::Range {
            start: Box::new(Expr::Int(1)),
            end: Box::new(Expr::Int(5)),
            inclusive: false,
        };
        let result = eval_expr(&range_expr, &mut env, &procedures).unwrap();
        assert_eq!(result, Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)]));
        
        // Test inclusive range 1..=5
        let range_expr_inclusive = Expr::Range {
            start: Box::new(Expr::Int(1)),
            end: Box::new(Expr::Int(5)),
            inclusive: true,
        };
        let result_inclusive = eval_expr(&range_expr_inclusive, &mut env, &procedures).unwrap();
        assert_eq!(result_inclusive, Value::Array(vec![
            Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4), Value::Int(5)
        ]));
    }

    #[test]
    fn test_closure_creation() {
        let mut env = HashMap::new();
        let procedures = HashMap::new();
        
        // Create a closure: |x: Int64| -> Int64 { return x * 2; }
        let closure_expr = Expr::Closure {
            params: vec![Parameter {
                name: "x".to_string(),
                typ: TypeAnnotation::Int64,
                lifetime: None,
            }],
            return_type: Some(TypeAnnotation::Int64),
            body: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Var("x".to_string())),
                op: crate::ast::BinaryOp::Mul,
                right: Box::new(Expr::Int(2)),
            }),
        };
        
        let result = eval_expr(&closure_expr, &mut env, &procedures).unwrap();
        match result {
            Value::Closure { params, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], "x");
            }
            _ => panic!("Expected closure value"),
        }
    }

    #[test]
    fn test_closure_call() {
        let mut env = HashMap::new();
        let procedures = HashMap::new();
        
        // Create a closure
        let closure = Value::Closure {
            params: vec!["x".to_string()],
            body: Expr::BinaryOp {
                left: Box::new(Expr::Var("x".to_string())),
                op: crate::ast::BinaryOp::Mul,
                right: Box::new(Expr::Int(2)),
            },
            captured_env: HashMap::new(),
        };
        
        // Store it in a variable
        env.insert("double".to_string(), closure);
        
        // Call it: double(5)
        let call_expr = Expr::Call {
            func: Box::new(Expr::Var("double".to_string())),
            args: vec![Expr::Int(5)],
        };
        
        let result = eval_expr(&call_expr, &mut env, &procedures).unwrap();
        assert_eq!(result, Value::Int(10));
    }

    #[test]
    fn test_tuple_pattern_matching() {
        let mut env = HashMap::new();
        let procedures = HashMap::new();
        
        // Test matching [1, 2] against tuple pattern
        let value = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let pattern = crate::ast::Pattern::Tuple(vec![
            crate::ast::Pattern::Literal(Expr::Int(1)),
            crate::ast::Pattern::Literal(Expr::Int(2)),
        ]);
        
        let matches = pattern_matches(&pattern, &value, &mut env, &procedures).unwrap();
        assert!(matches);
        
        // Test non-matching
        let value2 = Value::Array(vec![Value::Int(1), Value::Int(3)]);
        let matches2 = pattern_matches(&pattern, &value2, &mut env, &procedures).unwrap();
        assert!(!matches2);
    }

    #[test]
    fn test_or_pattern_matching() {
        let mut env = HashMap::new();
        let procedures = HashMap::new();
        
        // Test matching with or pattern: 1 | 2 | 3
        let value = Value::Int(2);
        let pattern = crate::ast::Pattern::Or(vec![
            crate::ast::Pattern::Literal(Expr::Int(1)),
            crate::ast::Pattern::Literal(Expr::Int(2)),
            crate::ast::Pattern::Literal(Expr::Int(3)),
        ]);
        
        let matches = pattern_matches(&pattern, &value, &mut env, &procedures).unwrap();
        assert!(matches);
        
        // Test non-matching
        let value2 = Value::Int(4);
        let matches2 = pattern_matches(&pattern, &value2, &mut env, &procedures).unwrap();
        assert!(!matches2);
    }
}
