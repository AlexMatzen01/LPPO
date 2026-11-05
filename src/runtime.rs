use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

// Global registry for background task handles scheduled by async pauses.
static BACKGROUND_HANDLES: Lazy<Mutex<Vec<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(Vec::new()));

// Simple LCG-based pseudo-random number generator seeded from system time.
static RNG: Lazy<Mutex<SimpleRng>> = Lazy::new(|| {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        .wrapping_add(0x9e3779b97f4a7c15);
    Mutex::new(SimpleRng::new(seed))
});

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn range(&mut self, min: i64, max: i64) -> i64 {
        if min == max {
            return min;
        }
        let span = max.abs_diff(min);
        let roll = (self.next_u64() as u128 % (span as u128 + 1)) as u64;
        if max >= min {
            min.wrapping_add(roll as i64)
        } else {
            min.wrapping_sub(roll as i64)
        }
    }
}

fn random_range(min: i64, max: i64) -> i64 {
    let mut rng = RNG.lock().unwrap();
    rng.range(min, max)
}

use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::error::{LppoError, LppoResult};
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
    While { cond: Expr, ops: Vec<Operation> },
    /// For-in loop
    For {
        var: String,
        iterable: Expr,
        ops: Vec<Operation>,
    },
    /// Infinite loop
    Loop { ops: Vec<Operation> },
    /// Break from loop
    Break,
    /// Continue loop
    Continue,
    /// Return from procedure
    Return(Option<Expr>),
    /// Explicit scope block
    Scope { ops: Vec<Operation> },
    /// Unsafe block (treated same as scope at runtime)
    Unsafe { ops: Vec<Operation> },
    /// No operation (placeholder)
    Nop,
    /// Pause execution: optionally show prompt, wait until condition or timeout
    Pause {
        prompt: Option<Expr>,
        timeout_ms: Option<Expr>,
        until: Option<Expr>,
        assign_to: Option<String>,
        on_mismatch: Option<Expr>,
        on_timeout: Option<Expr>,
        is_async: bool,
    },
    /// Expression statement - evaluate for side effects
    ExprStatement(Expr),
}

/// Runtime value types used during evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
    Closure {
        params: Vec<String>,
        body: Expr,
        captured_env: HashMap<String, Value>,
    },
}

fn pattern_matches(
    pattern: &crate::ast::Pattern,
    value: &Value,
    env: &mut HashMap<String, Value>,
    procedures: &HashMap<String, ProcedureBody>,
) -> LppoResult<bool> {
    use crate::ast::Pattern;

    match pattern {
        Pattern::Wildcard => Ok(true),
        Pattern::Literal(expr) => {
            // Compare the value with the literal
            let lit_val = eval_expr(expr, env, procedures, false)?;
            Ok(values_equal(&lit_val, value))
        }
        Pattern::Variable(name) => {
            // Bind the variable to the value
            env.insert(name.clone(), value.clone());
            Ok(true)
        }
        Pattern::EnumVariant {
            variant, fields, ..
        } => {
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
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => a == b,
        (Value::Object(a), Value::Object(b)) => a == b,
        _ => false,
    }
}

pub(crate) fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    procedures: &HashMap<String, ProcedureBody>,
    ring0: bool,
) -> LppoResult<Value> {
    match expr {
        Expr::Int(i) => Ok(Value::Int(*i)),
        Expr::Float(f) => {
            let parsed = f
                .parse::<f64>()
                .map_err(|_| LppoError::type_error("invalid float literal".into()))?;
            Ok(Value::Float(parsed))
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
            let l = eval_expr(left, env, procedures, ring0)?;
            let r = eval_expr(right, env, procedures, ring0)?;
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
                // float arithmetic
                (Value::Float(a), Value::Float(b), BinaryOp::Add) => Ok(Value::Float(a + b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Sub) => Ok(Value::Float(a - b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Mul) => Ok(Value::Float(a * b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Div) => {
                    if b == 0.0 {
                        Err(LppoError::type_error("division by zero".into()))
                    } else {
                        Ok(Value::Float(a / b))
                    }
                }
                (Value::Float(a), Value::Float(b), BinaryOp::Eq) => Ok(Value::Bool(a == b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Ne) => Ok(Value::Bool(a != b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Lt) => Ok(Value::Bool(a < b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Le) => Ok(Value::Bool(a <= b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Gt) => Ok(Value::Bool(a > b)),
                (Value::Float(a), Value::Float(b), BinaryOp::Ge) => Ok(Value::Bool(a >= b)),
                // string concat with +
                (Value::Str(a), Value::Str(b), BinaryOp::Add) => {
                    Ok(Value::Str(format!("{}{}", a, b)))
                }
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
                _ => Err(LppoError::type_error(
                    "type mismatch in binary operation".into(),
                )),
            }
        }
        Expr::UnaryOp { op, operand } => {
            let val = eval_expr(operand, env, procedures, ring0)?;
            match (op, val) {
                (UnaryOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                (UnaryOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                (UnaryOp::BitNot, Value::Int(i)) => Ok(Value::Int(!i)),
                _ => Err(LppoError::type_error(
                    "type mismatch in unary operation".into(),
                )),
            }
        }
        Expr::Array(elements) => {
            let mut values = Vec::new();
            for elem in elements {
                values.push(eval_expr(elem, env, procedures, ring0)?);
            }
            Ok(Value::Array(values))
        }
        Expr::Index { target, index } => {
            let target_val = eval_expr(target, env, procedures, ring0)?;
            let index_val = eval_expr(index, env, procedures, ring0)?;
            match (target_val, index_val) {
                (Value::Array(arr), Value::Int(idx)) => {
                    let i = if idx < 0 {
                        (arr.len() as i64 + idx) as usize
                    } else {
                        idx as usize
                    };
                    arr.get(i).cloned().ok_or_else(|| {
                        LppoError::type_error(format!("index {} out of bounds", idx))
                    })
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
                        .ok_or_else(|| {
                            LppoError::type_error(format!("index {} out of bounds", idx))
                        })
                }
                _ => Err(LppoError::type_error(
                    "indexing requires array/string and integer".into(),
                )),
            }
        }
        Expr::MethodCall {
            target,
            method,
            args,
        } => {
            let target_val = eval_expr(target, env, procedures, ring0)?;
            match (target_val.clone(), method.as_str()) {
                (Value::Array(mut arr), "push") => {
                    for arg in args {
                        arr.push(eval_expr(arg, env, procedures, ring0)?);
                    }
                    Ok(Value::Array(arr))
                }
                (Value::Array(mut arr), "pop") => arr
                    .pop()
                    .ok_or_else(|| LppoError::type_error("pop on empty array".into())),
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
                    let closure_val = eval_expr(&args[0], env, procedures, ring0)?;

                    let mut result = Vec::new();
                    for item in arr {
                        match closure_val.clone() {
                            Value::Closure {
                                params,
                                body,
                                captured_env,
                            } => {
                                if params.len() != 1 {
                                    return Err(LppoError::type_error(
                                        "map closure must take 1 parameter".into(),
                                    ));
                                }
                                let mut closure_env = captured_env;
                                closure_env.insert(params[0].clone(), item);
                                let mapped_val =
                                    eval_expr(&body, &mut closure_env, procedures, ring0)?;
                                result.push(mapped_val);
                            }
                            _ => {
                                return Err(LppoError::type_error(
                                    "map argument must be a closure".into(),
                                ))
                            }
                        }
                    }
                    Ok(Value::Array(result))
                }
                (Value::Array(arr), "filter") => {
                    // filter keeps elements where closure returns true
                    if args.len() != 1 {
                        return Err(LppoError::type_error("filter requires 1 argument".into()));
                    }
                    let closure_val = eval_expr(&args[0], env, procedures, ring0)?;

                    let mut result = Vec::new();
                    for item in arr {
                        match closure_val.clone() {
                            Value::Closure {
                                params,
                                body,
                                captured_env,
                            } => {
                                if params.len() != 1 {
                                    return Err(LppoError::type_error(
                                        "filter closure must take 1 parameter".into(),
                                    ));
                                }
                                let mut closure_env = captured_env;
                                closure_env.insert(params[0].clone(), item.clone());
                                let keep = eval_expr(&body, &mut closure_env, procedures, ring0)?;
                                if is_truthy(&keep) {
                                    result.push(item);
                                }
                            }
                            _ => {
                                return Err(LppoError::type_error(
                                    "filter argument must be a closure".into(),
                                ))
                            }
                        }
                    }
                    Ok(Value::Array(result))
                }
                (Value::Array(arr), "collect") => {
                    // collect just returns the array as-is (already collected)
                    Ok(Value::Array(arr))
                }
                (Value::Array(arr), "first") => arr
                    .first()
                    .cloned()
                    .ok_or_else(|| LppoError::type_error("first on empty array".into())),
                (Value::Str(s), "len") => Ok(Value::Int(s.len() as i64)),
                (Value::Str(s), "upper") => Ok(Value::Str(s.to_uppercase())),
                (Value::Str(s), "lower") => Ok(Value::Str(s.to_lowercase())),
                (Value::Str(s), "split") => {
                    if args.len() != 1 {
                        return Err(LppoError::type_error("split requires 1 argument".into()));
                    }
                    let sep = eval_expr(&args[0], env, procedures, ring0)?;
                    if let Value::Str(sep_str) = sep {
                        let parts: Vec<Value> = s
                            .split(&sep_str)
                            .map(|p| Value::Str(p.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    } else {
                        Err(LppoError::type_error(
                            "split separator must be string".into(),
                        ))
                    }
                }
                _ => Err(LppoError::type_error(format!("unknown method: {}", method))),
            }
        }
        Expr::StructInit { name: _, fields } => {
            let mut values = Vec::new();
            for (field_name, expr) in fields {
                let val = eval_expr(expr, env, procedures, ring0)?;
                values.push((field_name.clone(), val));
            }
            Ok(Value::Object(values))
        }
        Expr::FieldAccess { target, field } => {
            let target_val = eval_expr(target, env, procedures, ring0)?;
            if let Value::Object(fields) = &target_val {
                for (name, value) in fields {
                    if name == field {
                        return Ok(value.clone());
                    }
                }
                return Err(LppoError::type_error(format!(
                    "object has no field '{}'",
                    field
                )));
            }
            Err(LppoError::type_error(format!(
                "field access not supported on this value: {}",
                field
            )))
        }
        Expr::Ref {
            target,
            lifetime: _,
            is_mut: _,
        } => {
            // In our simplified runtime, references are just the value itself
            // Full reference semantics would require borrow checking
            eval_expr(target, env, procedures, ring0)
        }
        Expr::Deref(target) => {
            // Dereference is also just evaluating the target
            eval_expr(target, env, procedures, ring0)
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
            let val = eval_expr(expr, env, procedures, ring0)?;
            match (val, to_type) {
                (Value::Int(i), crate::ast::TypeAnnotation::Bool) => Ok(Value::Bool(i != 0)),
                (Value::Int(i), crate::ast::TypeAnnotation::Str) => Ok(Value::Str(i.to_string())),
                (Value::Bool(b), crate::ast::TypeAnnotation::Int64) => {
                    Ok(Value::Int(if b { 1 } else { 0 }))
                }
                (Value::Str(s), crate::ast::TypeAnnotation::Int64) => s
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| LppoError::type_error("cannot cast string to Int64".into())),
                (v, _) => Ok(v), // passthrough for same type
            }
        }
        Expr::Borrow {
            target,
            lifetime: _,
        } => {
            // Simplified: borrow is just like a reference
            eval_expr(target, env, procedures, ring0)
        }
        Expr::Closure {
            params,
            return_type: _,
            body,
        } => {
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
                arg_vals.push(eval_expr(arg, env, procedures, ring0)?);
            }

            // Evaluate the function expression
            // Fast-path builtins when func is a simple identifier
            if let Expr::Var(name) = func.as_ref() {
                match name.as_str() {
                    "println" => {
                        let parts: Vec<String> = arg_vals.iter().map(value_to_string).collect();
                        println!("{}", parts.join(" "));
                        return Ok(Value::Int(0));
                    }
                    "print" => {
                        let parts: Vec<String> = arg_vals.iter().map(value_to_string).collect();
                        print!("{}", parts.join(" "));
                        return Ok(Value::Int(0));
                    }
                    "eprintln" => {
                        let parts: Vec<String> = arg_vals.iter().map(value_to_string).collect();
                        eprintln!("{}", parts.join(" "));
                        return Ok(Value::Int(0));
                    }
                    "read_file" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "read_file requires 1 argument (path)".into(),
                            ));
                        }
                        if let Value::Str(path) = &arg_vals[0] {
                            match std::fs::read_to_string(path) {
                                Ok(content) => return Ok(Value::Str(content)),
                                Err(e) => {
                                    return Err(LppoError::io_error(format!(
                                        "Failed to read file '{}': {}",
                                        path, e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "read_file path must be a string".into(),
                            ));
                        }
                    }
                    "write_file" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "write_file requires 2 arguments (path, content)".into(),
                            ));
                        }
                        if let (Value::Str(path), Value::Str(content)) =
                            (&arg_vals[0], &arg_vals[1])
                        {
                            match std::fs::write(path, content) {
                                Ok(_) => return Ok(Value::Bool(true)),
                                Err(e) => {
                                    return Err(LppoError::io_error(format!(
                                        "Failed to write file '{}': {}",
                                        path, e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "write_file path and content must be strings".into(),
                            ));
                        }
                    }
                    "append_file" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "append_file requires 2 arguments (path, content)".into(),
                            ));
                        }
                        if let (Value::Str(path), Value::Str(content)) =
                            (&arg_vals[0], &arg_vals[1])
                        {
                            match std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(path)
                            {
                                Ok(mut file) => {
                                    use std::io::Write;
                                    match file.write_all(content.as_bytes()) {
                                        Ok(_) => return Ok(Value::Bool(true)),
                                        Err(e) => {
                                            return Err(LppoError::io_error(format!(
                                                "Failed to append to file '{}': {}",
                                                path, e
                                            )))
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(LppoError::io_error(format!(
                                        "Failed to open file '{}': {}",
                                        path, e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "append_file path and content must be strings".into(),
                            ));
                        }
                    }
                    "file_exists" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "file_exists requires 1 argument (path)".into(),
                            ));
                        }
                        if let Value::Str(path) = &arg_vals[0] {
                            let exists = std::path::Path::new(path).exists();
                            return Ok(Value::Bool(exists));
                        } else {
                            return Err(LppoError::type_error(
                                "file_exists path must be a string".into(),
                            ));
                        }
                    }
                    "delete_file" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "delete_file requires 1 argument (path)".into(),
                            ));
                        }
                        if let Value::Str(path) = &arg_vals[0] {
                            match std::fs::remove_file(path) {
                                Ok(_) => return Ok(Value::Bool(true)),
                                Err(e) => {
                                    return Err(LppoError::io_error(format!(
                                        "Failed to delete file '{}': {}",
                                        path, e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "delete_file path must be a string".into(),
                            ));
                        }
                    }
                    "str_concat" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "str_concat requires 2 arguments".into(),
                            ));
                        }
                        if let (Value::Str(s1), Value::Str(s2)) = (&arg_vals[0], &arg_vals[1]) {
                            return Ok(Value::Str(format!("{}{}", s1, s2)));
                        } else {
                            return Err(LppoError::type_error(
                                "str_concat arguments must be strings".into(),
                            ));
                        }
                    }
                    "int_to_str" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "int_to_str requires 1 argument".into(),
                            ));
                        }
                        if let Value::Int(i) = arg_vals[0] {
                            return Ok(Value::Str(i.to_string()));
                        } else {
                            return Err(LppoError::type_error(
                                "int_to_str argument must be an integer".into(),
                            ));
                        }
                    }
                    "str_to_int" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "str_to_int requires 1 argument".into(),
                            ));
                        }
                        if let Value::Str(s) = &arg_vals[0] {
                            match s.parse::<i64>() {
                                Ok(i) => return Ok(Value::Int(i)),
                                Err(_) => {
                                    return Err(LppoError::type_error(format!(
                                        "Failed to parse '{}' as integer",
                                        s
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "str_to_int argument must be a string".into(),
                            ));
                        }
                    }
                    "str_len" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "str_len requires 1 argument".into(),
                            ));
                        }
                        if let Value::Str(s) = &arg_vals[0] {
                            return Ok(Value::Int(s.len() as i64));
                        } else {
                            return Err(LppoError::type_error(
                                "str_len argument must be a string".into(),
                            ));
                        }
                    }
                    "str_upper" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "str_upper requires 1 argument".into(),
                            ));
                        }
                        if let Value::Str(s) = &arg_vals[0] {
                            return Ok(Value::Str(s.to_uppercase()));
                        } else {
                            return Err(LppoError::type_error(
                                "str_upper argument must be a string".into(),
                            ));
                        }
                    }
                    "str_lower" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "str_lower requires 1 argument".into(),
                            ));
                        }
                        if let Value::Str(s) = &arg_vals[0] {
                            return Ok(Value::Str(s.to_lowercase()));
                        } else {
                            return Err(LppoError::type_error(
                                "str_lower argument must be a string".into(),
                            ));
                        }
                    }
                    "str_trim" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "str_trim requires 1 argument".into(),
                            ));
                        }
                        if let Value::Str(s) = &arg_vals[0] {
                            return Ok(Value::Str(s.trim().to_string()));
                        } else {
                            return Err(LppoError::type_error(
                                "str_trim argument must be a string".into(),
                            ));
                        }
                    }
                    "str_split" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "str_split requires 2 arguments (string, delimiter)".into(),
                            ));
                        }
                        if let (Value::Str(s), Value::Str(delim)) = (&arg_vals[0], &arg_vals[1]) {
                            let parts: Vec<Value> = s
                                .split(delim.as_str())
                                .map(|p| Value::Str(p.to_string()))
                                .collect();
                            return Ok(Value::Array(parts));
                        } else {
                            return Err(LppoError::type_error(
                                "str_split arguments must be strings".into(),
                            ));
                        }
                    }
                    "abs" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error("abs requires 1 argument".into()));
                        }
                        if let Value::Int(i) = arg_vals[0] {
                            return Ok(Value::Int(i.abs()));
                        } else {
                            return Err(LppoError::type_error(
                                "abs argument must be an integer".into(),
                            ));
                        }
                    }
                    "min" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error("min requires 2 arguments".into()));
                        }
                        if let (Value::Int(a), Value::Int(b)) = (&arg_vals[0], &arg_vals[1]) {
                            return Ok(Value::Int((*a).min(*b)));
                        } else {
                            return Err(LppoError::type_error(
                                "min arguments must be integers".into(),
                            ));
                        }
                    }
                    "max" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error("max requires 2 arguments".into()));
                        }
                        if let (Value::Int(a), Value::Int(b)) = (&arg_vals[0], &arg_vals[1]) {
                            return Ok(Value::Int((*a).max(*b)));
                        } else {
                            return Err(LppoError::type_error(
                                "max arguments must be integers".into(),
                            ));
                        }
                    }
                    "pow" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "pow requires 2 arguments (base, exponent)".into(),
                            ));
                        }
                        if let (Value::Int(base), Value::Int(exp)) = (&arg_vals[0], &arg_vals[1]) {
                            if *exp < 0 {
                                return Err(LppoError::type_error(
                                    "pow exponent must be non-negative".into(),
                                ));
                            }
                            if *exp > u32::MAX as i64 {
                                return Err(LppoError::type_error(
                                    "pow exponent too large (must be <= 4294967295)".into(),
                                ));
                            }
                            // Safe to cast since we've checked bounds
                            match base.checked_pow(*exp as u32) {
                                Some(result) => return Ok(Value::Int(result)),
                                None => {
                                    return Err(LppoError::type_error("pow result overflow".into()))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "pow arguments must be integers".into(),
                            ));
                        }
                    }
                    "random" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "random requires 2 arguments (min, max)".into(),
                            ));
                        }
                        if let (Value::Int(min), Value::Int(max)) = (&arg_vals[0], &arg_vals[1]) {
                            return Ok(Value::Int(random_range(*min, *max)));
                        } else {
                            return Err(LppoError::type_error(
                                "random arguments must be integers".into(),
                            ));
                        }
                    }
                    "json_parse" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "json_parse requires 1 argument (json string)".into(),
                            ));
                        }
                        if let Value::Str(s) = &arg_vals[0] {
                            match serde_json::from_str::<serde_json::Value>(s) {
                                Ok(v) => return Ok(json_to_value(&v)),
                                Err(e) => {
                                    return Err(LppoError::type_error(format!(
                                        "JSON parse error: {}",
                                        e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "json_parse argument must be a string".into(),
                            ));
                        }
                    }
                    "json_stringify" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "json_stringify requires 1 argument (value)".into(),
                            ));
                        }
                        return Ok(Value::Str(
                            serde_json::to_string(&value_to_json(&arg_vals[0]))
                                .map_err(|e| LppoError::type_error(e.to_string()))?,
                        ));
                    }
                    "system" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "system requires 1 argument (command)".into(),
                            ));
                        }
                        if let Value::Str(cmd) = &arg_vals[0] {
                            match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
                                Ok(output) => {
                                    let out = String::from_utf8_lossy(&output.stdout).to_string();
                                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                                    let combined = if err.is_empty() {
                                        out
                                    } else {
                                        format!("{}{}", out, err)
                                    };
                                    return Ok(Value::Str(combined));
                                }
                                Err(e) => {
                                    return Err(LppoError::type_error(format!(
                                        "Failed to run command: {}",
                                        e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "system argument must be a string".into(),
                            ));
                        }
                    }
                    "color" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "color requires 2 arguments (text, color)".into(),
                            ));
                        }
                        if let (Value::Str(text), Value::Str(color)) = (&arg_vals[0], &arg_vals[1])
                        {
                            return Ok(Value::Str(ansi_color(text, color)));
                        } else {
                            return Err(LppoError::type_error(
                                "color arguments must be strings".into(),
                            ));
                        }
                    }
                    "tui_init" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_init takes no arguments".into(),
                            ));
                        }
                        print!("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_end" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error("tui_end takes no arguments".into()));
                        }
                        print!("\x1b[2J\x1b[H\x1b[?25h\x1b[?1049l");
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_clear" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_clear takes no arguments".into(),
                            ));
                        }
                        print!("\x1b[2J\x1b[H");
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_move" => {
                        if arg_vals.len() != 2 {
                            return Err(LppoError::type_error(
                                "tui_move requires 2 arguments (x, y)".into(),
                            ));
                        }
                        if let (Value::Int(x), Value::Int(y)) = (&arg_vals[0], &arg_vals[1]) {
                            print!("\x1b[{};{}H", y, x);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_move arguments must be integers".into(),
                            ));
                        }
                    }
                    "tui_put" => {
                        if arg_vals.len() != 3 {
                            return Err(LppoError::type_error(
                                "tui_put requires 3 arguments (text, x, y)".into(),
                            ));
                        }
                        if let (Value::Str(text), Value::Int(x), Value::Int(y)) =
                            (&arg_vals[0], &arg_vals[1], &arg_vals[2])
                        {
                            print!("\x1b[{};{}H{}", y, x, text);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_put arguments must be (string, int, int)".into(),
                            ));
                        }
                    }
                    "tui_puts" => {
                        if arg_vals.len() != 4 {
                            return Err(LppoError::type_error(
                                "tui_puts requires 4 arguments (text, x, y, color)".into(),
                            ));
                        }
                        if let (Value::Str(text), Value::Int(x), Value::Int(y), Value::Str(color)) =
                            (&arg_vals[0], &arg_vals[1], &arg_vals[2], &arg_vals[3])
                        {
                            print!("\x1b[{};{}H{}{}\x1b[0m", y, x, color_prefix(color), text);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_puts arguments must be (string, int, int, string)".into(),
                            ));
                        }
                    }
                    "tui_cls" => {
                        if arg_vals.len() != 4 {
                            return Err(LppoError::type_error(
                                "tui_cls requires 4 arguments (x, y, w, h)".into(),
                            ));
                        }
                        if let (Value::Int(x), Value::Int(y), Value::Int(w), Value::Int(h)) =
                            (&arg_vals[0], &arg_vals[1], &arg_vals[2], &arg_vals[3])
                        {
                            let mut buf = String::new();
                            for row in 0..*h {
                                buf.push_str(&format!(
                                    "\x1b[{};{}H{}",
                                    y + row,
                                    x,
                                    " ".repeat(*w as usize)
                                ));
                            }
                            print!("{}", buf);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_cls arguments must be integers".into(),
                            ));
                        }
                    }
                    "tui_box" => {
                        if arg_vals.len() != 5 {
                            return Err(LppoError::type_error(
                                "tui_box requires 5 arguments (x, y, w, h, title)".into(),
                            ));
                        }
                        if let (
                            Value::Int(x),
                            Value::Int(y),
                            Value::Int(w),
                            Value::Int(h),
                            Value::Str(title),
                        ) = (
                            &arg_vals[0],
                            &arg_vals[1],
                            &arg_vals[2],
                            &arg_vals[3],
                            &arg_vals[4],
                        ) {
                            let w = *w as usize;
                            let h = *h as usize;
                            let x = *x;
                            let y = *y;
                            if w < 2 || h < 2 {
                                return Err(LppoError::type_error(
                                    "tui_box width and height must be at least 2".into(),
                                ));
                            }
                            let top = format!("┌{}┐", "─".repeat(w.saturating_sub(2)));
                            let bottom = format!("└{}┘", "─".repeat(w.saturating_sub(2)));
                            let side = "│";
                            let mut buf = String::new();
                            buf.push_str(&format!("\x1b[{};{}H{}", y, x, top));
                            for row in 1..(h.saturating_sub(1)) {
                                let fill = " ".repeat(w.saturating_sub(2));
                                buf.push_str(&format!(
                                    "\x1b[{};{}H{}{}{}",
                                    y + (row as i64),
                                    x,
                                    side,
                                    fill,
                                    side
                                ));
                            }
                            buf.push_str(&format!(
                                "\x1b[{};{}H{}",
                                y + (h.saturating_sub(1) as i64),
                                x,
                                bottom
                            ));
                            if !title.is_empty() && w > title.len() + 2 {
                                let t = &title[..title.len().min(w.saturating_sub(4))];
                                buf.push_str(&format!(
                                    "\x1b[{};{}H {} {}",
                                    y,
                                    x + 1,
                                    t,
                                    " ".repeat(w.saturating_sub(t.len() + 3))
                                ));
                            }
                            print!("{}", buf);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_box arguments must be (int, int, int, int, string)".into(),
                            ));
                        }
                    }
                    "tui_label" => {
                        if arg_vals.len() != 3 {
                            return Err(LppoError::type_error(
                                "tui_label requires 3 arguments (text, x, y)".into(),
                            ));
                        }
                        if let (Value::Str(text), Value::Int(x), Value::Int(y)) =
                            (&arg_vals[0], &arg_vals[1], &arg_vals[2])
                        {
                            print!("\x1b[{};{}H{}", y, x, text);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_label arguments must be (string, int, int)".into(),
                            ));
                        }
                    }
                    "tui_title" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_title requires 1 argument (title)".into(),
                            ));
                        }
                        if let Value::Str(title) = &arg_vals[0] {
                            print!("\x1b]0;{}\x07", title);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_title argument must be a string".into(),
                            ));
                        }
                    }
                    "tui_fg" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_fg requires 1 argument (color)".into(),
                            ));
                        }
                        if let Value::Str(color) = &arg_vals[0] {
                            print!("{}", color_prefix(color));
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_fg argument must be a string".into(),
                            ));
                        }
                    }
                    "tui_bg" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_bg requires 1 argument (color)".into(),
                            ));
                        }
                        if let Value::Str(color) = &arg_vals[0] {
                            let code = match color.as_str() {
                                "black" => "40",
                                "red" => "41",
                                "green" => "42",
                                "yellow" => "43",
                                "blue" => "44",
                                "magenta" => "45",
                                "cyan" => "46",
                                "white" => "47",
                                "bright_black" => "100",
                                "bright_red" => "101",
                                "bright_green" => "102",
                                "bright_yellow" => "103",
                                "bright_blue" => "104",
                                "bright_magenta" => "105",
                                "bright_cyan" => "106",
                                "bright_white" => "107",
                                _ => {
                                    if color.chars().all(|c| c.is_ascii_digit() || c == ';')
                                        && !color.is_empty()
                                    {
                                        color.as_str()
                                    } else {
                                        "0"
                                    }
                                }
                            };
                            if code != "0" {
                                print!("\x1b[{}m", code);
                                let _ = std::io::stdout().flush();
                            }
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_bg argument must be a string".into(),
                            ));
                        }
                    }
                    "tui_bold" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_bold requires 1 argument (on/off)".into(),
                            ));
                        }
                        if let Value::Int(v) = arg_vals[0] {
                            if v != 0 {
                                print!("\x1b[1m");
                            } else {
                                print!("\x1b[22m");
                            }
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_bold argument must be an integer".into(),
                            ));
                        }
                    }
                    "tui_underline" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_underline requires 1 argument (on/off)".into(),
                            ));
                        }
                        if let Value::Int(v) = arg_vals[0] {
                            if v != 0 {
                                print!("\x1b[4m");
                            } else {
                                print!("\x1b[24m");
                            }
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_underline argument must be an integer".into(),
                            ));
                        }
                    }
                    "tui_reverse" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_reverse requires 1 argument (on/off)".into(),
                            ));
                        }
                        if let Value::Int(v) = arg_vals[0] {
                            if v != 0 {
                                print!("\x1b[7m");
                            } else {
                                print!("\x1b[27m");
                            }
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_reverse argument must be an integer".into(),
                            ));
                        }
                    }
                    "tui_style" => {
                        if arg_vals.len() != 1 {
                            return Err(LppoError::type_error(
                                "tui_style requires 1 argument (style string)".into(),
                            ));
                        }
                        if let Value::Str(style) = &arg_vals[0] {
                            for part in style.split(&[',', ' '][..]).filter(|s| !s.is_empty()) {
                                print!("{}", color_prefix(part));
                            }
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_style argument must be a string".into(),
                            ));
                        }
                    }
                    "tui_reset" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_reset takes no arguments".into(),
                            ));
                        }
                        print!("\x1b[0m");
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_hide_cursor" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_hide_cursor takes no arguments".into(),
                            ));
                        }
                        print!("\x1b[?25l");
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_show_cursor" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_show_cursor takes no arguments".into(),
                            ));
                        }
                        print!("\x1b[?25h");
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_refresh" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_refresh takes no arguments".into(),
                            ));
                        }
                        let _ = std::io::stdout().flush();
                        return Ok(Value::Int(0));
                    }
                    "tui_input" => {
                        if arg_vals.len() != 4 {
                            return Err(LppoError::type_error(
                                "tui_input requires 4 arguments (prompt, x, y, w)".into(),
                            ));
                        }
                        if let (Value::Str(prompt), Value::Int(x), Value::Int(y), Value::Int(_w)) =
                            (&arg_vals[0], &arg_vals[1], &arg_vals[2], &arg_vals[3])
                        {
                            print!("\x1b[{};{}H{}", y, x, prompt);
                            let _ = std::io::stdout().flush();
                            let mut line = String::new();
                            match std::io::stdin().read_line(&mut line) {
                                Ok(_) => return Ok(Value::Str(line.trim_end().to_string())),
                                Err(e) => {
                                    return Err(LppoError::io_error(format!(
                                        "tui_input failed: {}",
                                        e
                                    )))
                                }
                            }
                        } else {
                            return Err(LppoError::type_error(
                                "tui_input arguments must be (string, int, int, int)".into(),
                            ));
                        }
                    }
                    "tui_getch" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_getch takes no arguments".into(),
                            ));
                        }
                        let _ = std::io::stdout().flush();
                        use std::io::Read;
                        let mut b = [0u8; 1];
                        match std::io::stdin().read(&mut b) {
                            Ok(1) => {
                                return Ok(Value::Str(String::from_utf8_lossy(&b).to_string()))
                            }
                            _ => return Ok(Value::Str(String::new())),
                        }
                    }
                    "tui_poll" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_poll takes no arguments".into(),
                            ));
                        }
                        let _ = std::io::stdout().flush();
                        use std::io::Read;
                        let mut line = String::new();
                        match std::io::stdin().read_line(&mut line) {
                            Ok(_) => return Ok(Value::Str(line.trim_end().to_string())),
                            Err(e) => {
                                return Err(LppoError::io_error(format!("tui_poll failed: {}", e)))
                            }
                        }
                    }
                    "tui_progress" => {
                        if arg_vals.len() != 5 {
                            return Err(LppoError::type_error(
                                "tui_progress requires 5 arguments (value, max, x, y, w)".into(),
                            ));
                        }
                        if let (
                            Value::Int(value),
                            Value::Int(max),
                            Value::Int(x),
                            Value::Int(y),
                            Value::Int(w),
                        ) = (
                            &arg_vals[0],
                            &arg_vals[1],
                            &arg_vals[2],
                            &arg_vals[3],
                            &arg_vals[4],
                        ) {
                            let max_v = if *max == 0 { 1 } else { *max };
                            let filled = ((*value as f64).clamp(0.0, max_v as f64) / max_v as f64
                                * *w as f64) as usize;
                            let w = *w as usize;
                            let bar = format!(
                                "{}{}",
                                "█".repeat(filled.min(w)),
                                "░".repeat(w.saturating_sub(filled))
                            );
                            print!("\x1b[{};{}H{}", y, x, bar);
                            let _ = std::io::stdout().flush();
                            return Ok(Value::Int(0));
                        } else {
                            return Err(LppoError::type_error(
                                "tui_progress arguments must be integers".into(),
                            ));
                        }
                    }
                    "tui_size" => {
                        if arg_vals.len() != 0 {
                            return Err(LppoError::type_error(
                                "tui_size takes no arguments".into(),
                            ));
                        }
                        let (rows, cols) = terminal_size();
                        return Ok(Value::Array(vec![Value::Int(rows), Value::Int(cols)]));
                    }
                    // Ring-0 (kernel privilege) built-ins. The host runtime runs in
                    // user mode, so these always fail here. The `ring0` flag is
                    // enforced at type-check time; this is a defense-in-depth guard.
                    "outb" | "outw" | "outl" | "inb" | "inw" | "inl" | "rdmsr" | "wrmsr"
                    | "cpuid" | "cli" | "sti" | "hlt" => {
                        if !ring0 {
                            return Err(LppoError::type_error(format!(
                                "ring-0 operation '{}' requires `ring 0` to be declared first",
                                name
                            )));
                        }
                        return Err(LppoError::type_error(format!(
                            "ring-0 operation '{}' not available in user mode (host)",
                            name
                        )));
                    }
                    _ => {}
                }
            }

            let func_val = eval_expr(func, env, procedures, ring0);

            // Handle the different cases
            match func_val {
                Ok(Value::Closure {
                    params,
                    body,
                    captured_env,
                }) => {
                    // Calling a closure value
                    if params.len() != arg_vals.len() {
                        return Err(LppoError::type_error(format!(
                            "closure argument count mismatch: expected {} got {}",
                            params.len(),
                            arg_vals.len()
                        )));
                    }

                    // Create environment from captured environment plus parameters
                    let mut closure_env = captured_env;
                    for (param, val) in params.iter().zip(arg_vals.into_iter()) {
                        closure_env.insert(param.clone(), val);
                    }

                    // Evaluate closure body
                    eval_expr(&body, &mut closure_env, procedures, ring0)
                }
                Err(_) => {
                    // If func doesn't evaluate to a value, try as a procedure name.
                    // Execute the full procedure body and return its value so that
                    // procedures can be used in expressions (e.g. `let x = foo(5);`).
                    if let Expr::Var(proc_name) = func.as_ref() {
                        // Look up the procedure
                        if let Some(proc_body) = procedures.get(proc_name) {
                            // Check arity
                            if proc_body.params.len() != arg_vals.len() {
                                return Err(LppoError::type_error(format!(
                                    "argument count mismatch for '{}': expected {} got {}",
                                    proc_name,
                                    proc_body.params.len(),
                                    arg_vals.len()
                                )));
                            }

                            // Create a new environment for the procedure call
                            let mut proc_env = HashMap::new();
                            for (param, val) in proc_body.params.iter().zip(arg_vals.into_iter()) {
                                proc_env.insert(param.clone(), val);
                            }

                            // Execute the full procedure body and capture the return value.
                            let exec = Executor::new(procedures.clone(), ring0);
                            let ret = exec.execute_procedure_with_env(proc_body, &mut proc_env)?;
                            return Ok(ret.unwrap_or(Value::Int(0)));
                        } else {
                            return Err(LppoError::unknown_procedure(proc_name.clone()));
                        }
                    } else {
                        return Err(LppoError::type_error(
                            "function calls require a procedure name or closure".into(),
                        ));
                    }
                }
                Ok(_) => Err(LppoError::type_error(
                    "can only call closures or procedures".into(),
                )),
            }
        }
        Expr::Range {
            start,
            end,
            inclusive,
        } => {
            // Evaluate range start and end
            let start_val = eval_expr(start, env, procedures, ring0)?;
            let end_val = eval_expr(end, env, procedures, ring0)?;

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
                _ => Err(LppoError::type_error(
                    "range expressions require integer bounds".into(),
                )),
            }
        }
        Expr::MatchExpr { expr, arms } => {
            // Evaluate the match expression
            let val = eval_expr(expr, env, procedures, ring0)?;

            // Try each arm in order
            for arm in arms {
                if pattern_matches(&arm.pattern, &val, env, procedures)? {
                    // Check guard if present
                    if let Some(ref guard) = arm.guard {
                        let guard_val = eval_expr(guard, env, procedures, ring0)?;
                        if !is_truthy(&guard_val) {
                            continue;
                        }
                    }

                    // Execute the arm body
                    return eval_expr(&arm.body, env, procedures, ring0);
                }
            }

            // No pattern matched
            Err(LppoError::type_error(
                "no pattern matched in match expression".into(),
            ))
        }
        Expr::Await(expr) => {
            // For now, simply evaluate the expression synchronously
            // In a real async runtime, this would suspend and resume
            eval_expr(expr, env, procedures, ring0)
        }
        Expr::Handle {
            expr,
            handler_type: _,
            handlers: _,
        } => {
            // For now, evaluate the expression and ignore effect handlers
            // A full implementation would set up effect handlers and call them on performance
            eval_expr(expr, env, procedures, ring0)
        }
    }
}

/// Executes compiled LPPO procedures.
#[derive(Debug, Default, Clone)]
pub struct Executor {
    procedures: HashMap<String, ProcedureBody>,
    ring0: bool,
}

impl Executor {
    /// Create a new executor from compiled procedures.
    pub fn new(procedures: HashMap<String, ProcedureBody>, ring0: bool) -> Self {
        Self { procedures, ring0 }
    }

    /// Run the specified entry procedure.
    pub fn run(&self, entry: &str) -> LppoResult<()> {
        let procedure = self
            .procedures
            .get(entry)
            .ok_or_else(|| LppoError::unknown_procedure(entry.to_string()))?;
        let mut env = HashMap::new();
        let _ = self.execute_procedure_with_env(procedure, &mut env)?;
        // Wait for any scheduled background tasks to finish before returning
        // This ensures async pauses get a chance to complete their continuations.
        let mut handles = BACKGROUND_HANDLES.lock().unwrap();
        while let Some(h) = handles.pop() {
            let _ = h.join();
        }
        Ok(())
    }

    fn execute_procedure_with_env(
        &self,
        procedure: &ProcedureBody,
        env: &mut HashMap<String, Value>,
    ) -> LppoResult<Option<Value>> {
        match self.execute_ops(&procedure.operations, env)? {
            ControlFlow::Return(v) => Ok(v),
            _ => Ok(None),
        }
    }

    fn execute_ops(
        &self,
        ops: &[Operation],
        env: &mut HashMap<String, Value>,
    ) -> LppoResult<ControlFlow> {
        let len = ops.len();
        for i in 0..len {
            let op = &ops[i];
            match op {
                Operation::Emit(exprs) => {
                    // evaluate all expressions and join them with spaces
                    let mut parts = Vec::new();
                    for e in exprs {
                        let v = eval_expr(e, env, &self.procedures, self.ring0)?;
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
                                let v = eval_expr(&e, &mut snapshot, &procs, self.ring0)
                                    .expect("eval in thread");
                                println!("{}", value_to_string(&v));
                            });
                        }
                    });
                }
                Operation::Input { var, typ, prompt } => {
                    use std::io::{self, Write};

                    // Display prompt if provided
                    if let Some(prompt_expr) = prompt {
                        let prompt_val = eval_expr(prompt_expr, env, &self.procedures, self.ring0)?;
                        print!("{} ", value_to_string(&prompt_val));
                        io::stdout().flush().unwrap();
                    }

                    // Read input from stdin
                    let mut input_line = String::new();
                    io::stdin().read_line(&mut input_line).map_err(|e| {
                        LppoError::type_error(format!("Failed to read input: {}", e))
                    })?;

                    let input_line = input_line.trim();

                    // Parse based on type annotation
                    use crate::ast::TypeAnnotation;
                    let value = match typ {
                        TypeAnnotation::Int64 => {
                            input_line.parse::<i64>().map(Value::Int).map_err(|_| {
                                LppoError::type_error(format!(
                                    "Invalid Int64 input: '{}'",
                                    input_line
                                ))
                            })?
                        }
                        TypeAnnotation::Bool => {
                            match input_line.to_lowercase().as_str() {
                                "true" | "t" | "yes" | "y" | "1" => Value::Bool(true),
                                "false" | "f" | "no" | "n" | "0" => Value::Bool(false),
                                _ => return Err(LppoError::type_error(format!(
                                    "Invalid Bool input: '{}'. Expected: true/false, yes/no, 1/0",
                                    input_line
                                ))),
                            }
                        }
                        TypeAnnotation::Str => Value::Str(input_line.to_string()),
                        TypeAnnotation::Arr(_) => {
                            // Parse array input: [1, 2, 3] or comma-separated: 1,2,3
                            let input_line = input_line.trim();

                            // Try parsing as JSON-like array first
                            if input_line.starts_with('[') && input_line.ends_with(']') {
                                let inner = &input_line[1..input_line.len() - 1];
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
                                            TypeAnnotation::Int64 => {
                                                elem.parse::<i64>().map(Value::Int).map_err(|_| {
                                                    LppoError::type_error(format!(
                                                        "Invalid Int64 in array: '{}'",
                                                        elem
                                                    ))
                                                })
                                            }
                                            TypeAnnotation::Str => {
                                                Ok(Value::Str(elem.trim_matches('"').to_string()))
                                            }
                                            TypeAnnotation::Bool => {
                                                match elem.to_lowercase().as_str() {
                                                    "true" => Ok(Value::Bool(true)),
                                                    "false" => Ok(Value::Bool(false)),
                                                    _ => Err(LppoError::type_error(format!(
                                                        "Invalid Bool in array: '{}'",
                                                        elem
                                                    ))),
                                                }
                                            }
                                            _ => Err(LppoError::type_error(
                                                "Unsupported array element type".to_string(),
                                            )),
                                        })
                                        .collect();
                                    Value::Array(parsed_elements?)
                                } else {
                                    return Err(LppoError::type_error(
                                        "Invalid array type annotation".to_string(),
                                    ));
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
                                            TypeAnnotation::Int64 => {
                                                elem.parse::<i64>().map(Value::Int).map_err(|_| {
                                                    LppoError::type_error(format!(
                                                        "Invalid Int64 in array: '{}'",
                                                        elem
                                                    ))
                                                })
                                            }
                                            TypeAnnotation::Str => Ok(Value::Str(elem.to_string())),
                                            TypeAnnotation::Bool => {
                                                match elem.to_lowercase().as_str() {
                                                    "true" => Ok(Value::Bool(true)),
                                                    "false" => Ok(Value::Bool(false)),
                                                    _ => Err(LppoError::type_error(format!(
                                                        "Invalid Bool in array: '{}'",
                                                        elem
                                                    ))),
                                                }
                                            }
                                            _ => Err(LppoError::type_error(
                                                "Unsupported array element type".to_string(),
                                            )),
                                        })
                                        .collect();
                                    Value::Array(parsed_elements?)
                                } else {
                                    return Err(LppoError::type_error(
                                        "Invalid array type annotation".to_string(),
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(LppoError::type_error(format!(
                                "Unsupported input type: {:?}",
                                typ
                            )));
                        }
                    };

                    env.insert(var.clone(), value);
                }
                Operation::Pause {
                    prompt,
                    timeout_ms,
                    until,
                    assign_to,
                    on_mismatch,
                    on_timeout,
                    is_async,
                } => {
                    // Pause invocation
                    use std::io::{self, Write};
                    use std::sync::mpsc;
                    use std::thread;
                    use std::time::{Duration, Instant};

                    // Evaluate timeout if provided, else default to 1 second
                    let timeout_dur_opt: Option<Duration> = if let Some(t) = &timeout_ms {
                        match eval_expr(t, env, &self.procedures, self.ring0)? {
                            Value::Int(ms) if ms > 0 => Some(Duration::from_millis(ms as u64)),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    // Prepare until target if provided
                    let until_target_opt: Option<String> = if let Some(u) = &until {
                        match eval_expr(u, env, &self.procedures, self.ring0)? {
                            Value::Str(s) => Some(s),
                            Value::Int(i) => Some(i.to_string()),
                            Value::Bool(b) => Some(b.to_string()),
                            other => Some(value_to_string(&other)),
                        }
                    } else {
                        None
                    };

                    // If no arguments at all, just sleep for 1 second
                    if prompt.is_none()
                        && timeout_ms.is_none()
                        && until.is_none()
                        && assign_to.is_none()
                        && on_mismatch.is_none()
                        && on_timeout.is_none()
                    {
                        std::thread::sleep(Duration::from_secs(1));

                        continue;
                    }

                    // If only timeout is set and no until, handle either synchronously or asynchronously
                    if until_target_opt.is_none()
                        && timeout_dur_opt.is_some()
                        && assign_to.is_none()
                        && on_mismatch.is_none()
                        && on_timeout.is_none()
                    {
                        // Async scheduling: spawn a background thread to continue remaining ops after timeout
                        if *is_async {
                            // Capture remaining operations and environment, clone procedures
                            let remaining: Vec<Operation> = if i + 1 < len {
                                ops[i + 1..].to_vec()
                            } else {
                                vec![]
                            };
                            let procs_clone = self.procedures.clone();
                            let mut env_clone = env.clone();
                            let sleep_dur = timeout_dur_opt.unwrap();
                            let ring0 = self.ring0;
                            let handle = thread::spawn(move || {
                                std::thread::sleep(sleep_dur);
                                let exec = Executor::new(procs_clone, ring0);
                                let _ = exec.execute_ops(&remaining, &mut env_clone);
                            });
                            // Store handle so run() can join before exiting
                            BACKGROUND_HANDLES.lock().unwrap().push(handle);
                            // Stop executing this procedure now; remaining ops will run in background
                            return Ok(ControlFlow::None);
                        } else {
                            std::thread::sleep(timeout_dur_opt.unwrap());
                            continue;
                        }
                    }

                    // Otherwise, proceed with input-waiting logic
                    // Evaluate prompt if present
                    if let Some(p) = &prompt {
                        let pv = eval_expr(p, env, &self.procedures, self.ring0)?;
                        print!("{} ", value_to_string(&pv));
                        io::stdout().flush().unwrap();
                    }

                    // Channel reader thread: sends each read line to channel
                    let (tx, rx) = mpsc::channel::<String>();
                    thread::spawn(move || {
                        let stdin = io::stdin();
                        loop {
                            let mut line = String::new();
                            if stdin.read_line(&mut line).is_err() {
                                break;
                            }
                            // send trimmed line
                            let _ = tx.send(line);
                        }
                    });

                    let start = Instant::now();
                    let mut handled = false;

                    // Loop until conditions satisfied or timeout
                    loop {
                        // compute remaining timeout
                        let recv_res = if let Some(timeout_total) = timeout_dur_opt {
                            let elapsed = start.elapsed();
                            if elapsed >= timeout_total {
                                Err(mpsc::RecvTimeoutError::Timeout)
                            } else {
                                let remaining = timeout_total - elapsed;
                                rx.recv_timeout(remaining)
                            }
                        } else {
                            // wait indefinitely
                            match rx.recv() {
                                Ok(s) => Ok(s),
                                Err(_) => Err(mpsc::RecvTimeoutError::Disconnected),
                            }
                        };

                        match recv_res {
                            Ok(line) => {
                                let input = line.trim().to_string();

                                // assign input to variable if requested
                                if let Some(var) = &assign_to {
                                    env.insert(var.clone(), Value::Str(input.clone()));
                                }

                                // check until condition
                                if let Some(ref target) = until_target_opt {
                                    if &input == target {
                                        // matched, we're done
                                        handled = true;
                                        break;
                                    } else {
                                        // mismatch
                                        if let Some(expr) = &on_mismatch {
                                            // evaluate handler
                                            let _ =
                                                eval_expr(expr, env, &self.procedures, self.ring0)?;
                                        } else {
                                            return Err(LppoError::type_error(
                                                "pause: input did not match 'until' condition"
                                                    .into(),
                                            ));
                                        }
                                        // if no timeout, continue waiting; if timeout is set, keep looping until timeout
                                        if timeout_dur_opt.is_none() {
                                            continue;
                                        } else {
                                            // continue looping; remaining timeout governs
                                            continue;
                                        }
                                    }
                                } else {
                                    // no until specified: we're done after first input
                                    handled = true;
                                    break;
                                }
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => {
                                // timeout occurred
                                if let Some(expr) = &on_timeout {
                                    let _ = eval_expr(expr, env, &self.procedures, self.ring0)?;
                                } else {
                                    // default: just continue (no-op)
                                }
                                break;
                            }
                            Err(mpsc::RecvTimeoutError::Disconnected) => {
                                // stdin closed; break
                                break;
                            }
                        }
                    }

                    // If there was no input and no until, just return (user pressed Enter without typing)
                    if !handled {
                        // If we didn't handle but assign requested, ensure variable exists (assign empty)
                        if let Some(var_ref) = &assign_to {
                            env.insert(var_ref.clone(), Value::Str(String::new()));
                        }
                    }
                }
                Operation::Call(name, args) => {
                    // Evaluate argument expressions in current env
                    let mut arg_vals = Vec::new();
                    for a in args {
                        arg_vals.push(eval_expr(a, env, &self.procedures, self.ring0)?);
                    }

                    // Find the procedure body
                    let proc = self
                        .procedures
                        .get(name)
                        .ok_or_else(|| LppoError::unknown_procedure(name.clone()))?;

                    // check arity
                    if proc.params.len() != arg_vals.len() {
                        return Err(LppoError::type_error(format!(
                            "argument count mismatch for '{}': expected {} got {}",
                            name,
                            proc.params.len(),
                            arg_vals.len()
                        )));
                    }

                    // bind parameters in a fresh env and execute
                    let mut new_env: HashMap<String, Value> = HashMap::new();
                    for (p, v) in proc.params.iter().zip(arg_vals.into_iter()) {
                        new_env.insert(p.clone(), v);
                    }
                    self.execute_procedure_with_env(proc, &mut new_env)?;
                }
                Operation::Let(name, expr) => {
                    let v = eval_expr(expr, env, &self.procedures, self.ring0)?;
                    env.insert(name.clone(), v);
                }
                Operation::If {
                    cond,
                    then_ops,
                    elifs,
                    else_ops,
                } => {
                    let cond_v = eval_expr(cond, env, &self.procedures, self.ring0)?;
                    if is_truthy(&cond_v) {
                        let cf = self.execute_ops(then_ops, env)?;
                        if cf != ControlFlow::None {
                            return Ok(cf);
                        }
                        continue;
                    }
                    let mut handled = false;
                    for (econd, estmts) in elifs {
                        let ev = eval_expr(econd, env, &self.procedures, self.ring0)?;
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
                Operation::While { cond, ops } => loop {
                    let cond_v = eval_expr(cond, env, &self.procedures, self.ring0)?;
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
                },
                Operation::For { var, iterable, ops } => {
                    let iter_v = eval_expr(iterable, env, &self.procedures, self.ring0)?;
                    let items = match iter_v {
                        Value::Array(ref arr) => arr.clone(),
                        _ => {
                            return Err(LppoError::type_error(
                                "for loop requires an array".to_string(),
                            ))
                        }
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
                Operation::Loop { ops } => loop {
                    let cf = self.execute_ops(ops, env)?;
                    match cf {
                        ControlFlow::None => continue,
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    }
                },
                Operation::Break => {
                    return Ok(ControlFlow::Break);
                }
                Operation::Continue => {
                    return Ok(ControlFlow::Continue);
                }
                Operation::Return(opt_expr) => {
                    let v = if let Some(e) = opt_expr {
                        Some(eval_expr(e, env, &self.procedures, self.ring0)?)
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
                    let _ = eval_expr(expr, env, &self.procedures, self.ring0)?;
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

// Convert an LPPO runtime Value into a serde_json::Value (for JSON serialization).
fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Value::Float(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0)),
        ),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(value_to_json).collect()),
        Value::Object(fields) => {
            let mut map = serde_json::Map::new();
            for (k, val) in fields {
                map.insert(k.clone(), value_to_json(val));
            }
            serde_json::Value::Object(map)
        }
        Value::Closure { .. } => serde_json::Value::String("<closure>".to_string()),
    }
}

// Convert a serde_json::Value into an LPPO runtime Value (for JSON parsing).
fn json_to_value(jv: &serde_json::Value) -> Value {
    match jv {
        serde_json::Value::Null => Value::Str("null".to_string()),
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Str(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::Str(s.clone()),
        serde_json::Value::Array(arr) => Value::Array(arr.iter().map(json_to_value).collect()),
        serde_json::Value::Object(map) => {
            let mut fields = Vec::new();
            for (k, val) in map {
                fields.push((k.clone(), json_to_value(val)));
            }
            Value::Object(fields)
        }
    }
}

// Wrap text in ANSI color codes.
//
// The `color` argument can be:
//   * a single named attribute/color: "bold", "underline", "red", "blue", ...
//   * several combined at once, space or comma separated: "bold underline red",
//     "bold,underline,cyan"
//   * raw SGR parameters: "1", "1;31", "4;34"
//   * a raw ANSI escape: "\x1b[1;31m"
//
// Supported names: reset, bold, dim, italic, underline, blink, reverse, hidden,
// black, red, green, yellow, blue, magenta, cyan, white and the bright_* variants.
fn ansi_color(text: &str, color: &str) -> String {
    if let Some(code) = extract_ansi_code(color) {
        return format!("\x1b[{}m{}\x1b[0m", code, text);
    }
    if color.chars().all(|c| c.is_ascii_digit() || c == ';') && !color.is_empty() {
        return format!("\x1b[{}m{}\x1b[0m", color, text);
    }
    let codes: Vec<&str> = color
        .split(&[',', ' ', ';'][..])
        .filter(|s| !s.is_empty())
        .map(|s| s.trim())
        .filter_map(|name| match name {
            "reset" => Some("0"),
            "bold" => Some("1"),
            "dim" => Some("2"),
            "italic" => Some("3"),
            "underline" => Some("4"),
            "blink" => Some("5"),
            "reverse" => Some("7"),
            "hidden" => Some("8"),
            "black" => Some("30"),
            "red" => Some("31"),
            "green" => Some("32"),
            "yellow" => Some("33"),
            "blue" => Some("34"),
            "magenta" => Some("35"),
            "cyan" => Some("36"),
            "white" => Some("37"),
            "bright_black" => Some("90"),
            "bright_red" => Some("91"),
            "bright_green" => Some("92"),
            "bright_yellow" => Some("93"),
            "bright_blue" => Some("94"),
            "bright_magenta" => Some("95"),
            "bright_cyan" => Some("96"),
            "bright_white" => Some("97"),
            _ => None,
        })
        .collect();
    let code = codes.join(";");
    if code.is_empty() {
        format!("\x1b[0m{}\x1b[0m", text)
    } else {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    }
}

// Extract the SGR parameter string from a raw ANSI escape like "\x1b[1;31m".
fn extract_ansi_code(color: &str) -> Option<&str> {
    let esc = "\x1b[";
    let start = color.find(esc)?;
    let after = &color[start + esc.len()..];
    let end = after.find('m')?;
    let code = &after[..end];
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_digit() || c == ';') {
        return None;
    }
    Some(code)
}

// Return the ANSI SGR prefix for a color/style name (e.g. "\x1b[1;31m"),
// or an empty string when unknown. Supports combined names and raw codes.
fn color_prefix(color: &str) -> String {
    if let Some(code) = extract_ansi_code(color) {
        return format!("\x1b[{}m", code);
    }
    if color.chars().all(|c| c.is_ascii_digit() || c == ';') && !color.is_empty() {
        return format!("\x1b[{}m", color);
    }
    let code = match color {
        "reset" | "" => return String::new(),
        "bold" => "1",
        "dim" => "2",
        "italic" => "3",
        "underline" => "4",
        "blink" => "5",
        "reverse" => "7",
        "hidden" => "8",
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "bright_black" => "90",
        "bright_red" => "91",
        "bright_green" => "92",
        "bright_yellow" => "93",
        "bright_blue" => "94",
        "bright_magenta" => "95",
        "bright_cyan" => "96",
        "bright_white" => "97",
        _ => return String::new(),
    };
    format!("\x1b[{}m", code)
}

// Read terminal dimensions via tput(1), falling back to 24x80.
fn terminal_size() -> (i64, i64) {
    let mut rows = 24i64;
    let mut cols = 80i64;
    if let Ok(out) = std::process::Command::new("tput").arg("lines").output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(r) = s.trim().parse::<i64>() {
                if r > 0 {
                    rows = r;
                }
            }
        }
    }
    if let Ok(out) = std::process::Command::new("tput").arg("cols").output() {
        if let Ok(s) = String::from_utf8(out.stdout) {
            if let Ok(c) = s.trim().parse::<i64>() {
                if c > 0 {
                    cols = c;
                }
            }
        }
    }
    (rows, cols)
}

// Helper to convert Value to string
fn value_to_string(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_string).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(fields) => {
            let items: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, value_to_string(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
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
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Closure { .. } => true, // Closures are always truthy
    }
}

/// Serialize compiled procedures into JSON bytes (stable, portable format).
pub fn serialize_procedures(procedures: &HashMap<String, ProcedureBody>) -> LppoResult<Vec<u8>> {
    serde_json::to_vec(procedures).map_err(|e| LppoError::type_error(e.to_string()))
}

/// Deserialize procedures from JSON bytes and return an Executor.
pub fn executor_from_bytes(bytes: &[u8]) -> LppoResult<Executor> {
    let map: HashMap<String, ProcedureBody> =
        serde_json::from_slice(bytes).map_err(|e| LppoError::type_error(e.to_string()))?;
    Ok(Executor::new(map, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Parameter, TypeAnnotation};

    #[test]
    fn reports_missing_procedure() {
        let exec = Executor::new(HashMap::new(), false);
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
        let result = eval_expr(&range_expr, &mut env, &procedures, false).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4)
            ])
        );

        // Test inclusive range 1..=5
        let range_expr_inclusive = Expr::Range {
            start: Box::new(Expr::Int(1)),
            end: Box::new(Expr::Int(5)),
            inclusive: true,
        };
        let result_inclusive =
            eval_expr(&range_expr_inclusive, &mut env, &procedures, false).unwrap();
        assert_eq!(
            result_inclusive,
            Value::Array(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5)
            ])
        );
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

        let result = eval_expr(&closure_expr, &mut env, &procedures, false).unwrap();
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

        let result = eval_expr(&call_expr, &mut env, &procedures, false).unwrap();
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
