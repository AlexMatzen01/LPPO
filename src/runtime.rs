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
}

/// Runtime value types used during evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>) -> LppoResult<Value> {
    match expr {
        Expr::Int(i) => Ok(Value::Int(*i)),
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
            let l = eval_expr(left, env)?;
            let r = eval_expr(right, env)?;
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
            let val = eval_expr(operand, env)?;
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
                values.push(eval_expr(elem, env)?);
            }
            Ok(Value::Array(values))
        }
        Expr::Index { target, index } => {
            let target_val = eval_expr(target, env)?;
            let index_val = eval_expr(index, env)?;
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
            let target_val = eval_expr(target, env)?;
            match (target_val, method.as_str()) {
                (Value::Array(mut arr), "push") => {
                    for arg in args {
                        arr.push(eval_expr(arg, env)?);
                    }
                    Ok(Value::Array(arr))
                }
                (Value::Array(mut arr), "pop") => {
                    arr.pop()
                        .ok_or_else(|| LppoError::type_error("pop on empty array".into()))
                }
                (Value::Array(arr), "len") => Ok(Value::Int(arr.len() as i64)),
                (Value::Str(s), "len") => Ok(Value::Int(s.len() as i64)),
                (Value::Str(s), "upper") => Ok(Value::Str(s.to_uppercase())),
                (Value::Str(s), "lower") => Ok(Value::Str(s.to_lowercase())),
                (Value::Str(s), "split") => {
                    if args.len() != 1 {
                        return Err(LppoError::type_error("split requires 1 argument".into()));
                    }
                    let sep = eval_expr(&args[0], env)?;
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
            eval_expr(target, env)
        }
        Expr::Deref(target) => {
            // Dereference is also just evaluating the target
            eval_expr(target, env)
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
            let val = eval_expr(expr, env)?;
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
            eval_expr(target, env)
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
                        let v = eval_expr(e, env)?;
                        parts.push(value_to_string(&v));
                    }
                    println!("{}", parts.join(" "));
                }
                Operation::Parallel(exprs) => {
                    std::thread::scope(|scope| {
                        for e in exprs {
                            let e = e.clone();
                            let mut snapshot = env.clone();
                            scope.spawn(move || {
                                // evaluate in snapshot environment
                                let v = eval_expr(&e, &mut snapshot).expect("eval in thread");
                                println!("{}", value_to_string(&v));
                            });
                        }
                    });
                }
                Operation::Input { var, typ, prompt } => {
                    use std::io::{self, Write};
                    
                    // Display prompt if provided
                    if let Some(prompt_expr) = prompt {
                        let prompt_val = eval_expr(prompt_expr, env)?;
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
                Operation::Call(name, args) => {
                    // Evaluate argument expressions in current env
                    let mut arg_vals = Vec::new();
                    for a in args {
                        arg_vals.push(eval_expr(a, env)?);
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
                    let v = eval_expr(expr, env)?;
                    env.insert(name.clone(), v);
                }
                Operation::If { cond, then_ops, elifs, else_ops } => {
                    let cond_v = eval_expr(cond, env)?;
                    if is_truthy(&cond_v) {
                        let cf = self.execute_ops(then_ops, env)?;
                        if cf != ControlFlow::None {
                            return Ok(cf);
                        }
                        continue;
                    }
                    let mut handled = false;
                    for (econd, estmts) in elifs {
                        let ev = eval_expr(econd, env)?;
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
                        let cond_v = eval_expr(cond, env)?;
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
                    let iter_v = eval_expr(iterable, env)?;
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
                        Some(eval_expr(e, env)?)
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
    }
}

// Helper to check truthiness
fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Str(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
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

    #[test]
    fn reports_missing_procedure() {
        let exec = Executor::new(HashMap::new());
        let err = exec.run("main").expect_err("missing main");
        assert!(matches!(err, LppoError::UnknownProcedure { name, .. } if name == "main"));
    }
}
