//! Expression -> C code.
//!
//! Translates `crate::ast::Expr` nodes into C expressions (as strings).
//! All values are `int64_t` at the C level:
//!   - Int64   -> i64
//!   - Bool    -> i64 (0 or 1)
//!   - Float   -> i64 (f64 bit pattern)
//!   - Str     -> i64 (pointer to a `static const char*`)
//!   - Arr<T>  -> i64 (pointer to the runtime array block)
//!
//! The generated C source provides a few `static` helpers used here:
//! `lppo_f64_to_i64`, `lppo_i64_to_f64`, `lppo_arr`, `lppo_norm_idx`.

use crate::ast::{BinaryOp, Expr, TypeAnnotation, UnaryOp};
use crate::codegen_c::{
    builtin_func_name, TAG_ARRAY, TAG_BOOL, TAG_BYTE, TAG_CHAR, TAG_ENUM, TAG_FLOAT, TAG_FN,
    TAG_DYN, TAG_INT, TAG_JSON, TAG_MAP, TAG_SET, TAG_STR, TAG_STRUCT, TAG_TUPLE,
};
use std::collections::HashMap;

/// Codegen an expression to a pure C expression string (no side effects).
pub fn codegen_expr(
    expr: &Expr,
    locals: &HashMap<String, String>,
    var_tags: &HashMap<String, i64>,
    string_indices: &HashMap<String, usize>,
    proc_return_types: &HashMap<String, TypeAnnotation>,
) -> Result<String, String> {
    match expr {
        Expr::Int(i) => Ok(format!("(int64_t){}{}", i, if *i == i64::MIN { "" } else { "LL" })),
        Expr::Return(value) => match value {
            Some(v) => codegen_expr(v, locals, var_tags, string_indices, proc_return_types),
            None => Ok("(int64_t)0".to_string()),
        },
        Expr::Char(c) => Ok(format!("(int64_t){}", *c as i64)),
        Expr::Byte(b) => Ok(format!("(int64_t){}", *b as i64)),
        Expr::Tuple(items) => {
            let mut parts = vec![format!("(int64_t){}", items.len())];
            for item in items {
                parts.push(format!("(int64_t){}", expr_tag(item, var_tags, proc_return_types)));
                parts.push(codegen_expr(
                    item,
                    locals,
                    var_tags,
                    string_indices,
                    proc_return_types,
                )?);
            }
            Ok(format!("lppo_tuple({})", parts.join(", ")))
        }
        Expr::EnumVariant {
            enum_name,
            variant,
            args,
        } => {
            let mut parts = vec![
                format!("(int64_t)(intptr_t)\"{}\"", escape_c(enum_name)),
                format!("(int64_t)(intptr_t)\"{}\"", escape_c(&enum_variant_runtime_name(enum_name, variant))),
                format!("(int64_t){}", args.len()),
            ];
            for arg in args {
                parts.push(format!("(int64_t){}", expr_tag(arg, var_tags, proc_return_types)));
                parts.push(codegen_expr(
                    arg,
                    locals,
                    var_tags,
                    string_indices,
                    proc_return_types,
                )?);
            }
            Ok(format!("lppo_enum({})", parts.join(", ")))
        }
        Expr::Bool(b) => Ok(if *b { "(int64_t)1" } else { "(int64_t)0" }.to_string()),
        Expr::Str(s) => {
            let idx = string_indices
                .get(s)
                .ok_or_else(|| format!("string not in pool: {:?}", s))?;
            Ok(format!("((int64_t)(intptr_t)str_{})", idx))
        }
        Expr::Float(s) => Ok(format!("lppo_f64_to_i64((double){})", s)),
        Expr::Var(name) => lookup_local(name, locals),
        Expr::QualifiedVar(qn) => lookup_local(&qn_string(qn), locals),
        Expr::BinaryOp { left, op, right } => {
            let l = codegen_expr(left, locals, var_tags, string_indices, proc_return_types)?;
            let r = codegen_expr(right, locals, var_tags, string_indices, proc_return_types)?;
            if *op == BinaryOp::Pow {
                return Ok(format!("lppo_rt_pow({}, {})", l, r));
            }
            if is_float_expr(left) || is_float_expr(right) {
                let lf = format!("lppo_i64_to_f64({})", l);
                let rf = format!("lppo_i64_to_f64({})", r);
                let arith = match op {
                    BinaryOp::Add => Some("+"),
                    BinaryOp::Sub => Some("-"),
                    BinaryOp::Mul => Some("*"),
                    BinaryOp::Div => Some("/"),
                    _ => None,
                };
                if let Some(opc) = arith {
                    return Ok(format!("lppo_f64_to_i64({} {} {})", lf, opc, rf));
                }
                let cmp = match op {
                    BinaryOp::Eq => Some("=="),
                    BinaryOp::Ne => Some("!="),
                    BinaryOp::Lt => Some("<"),
                    BinaryOp::Le => Some("<="),
                    BinaryOp::Gt => Some(">"),
                    BinaryOp::Ge => Some(">="),
                    _ => None,
                };
                if let Some(cmpc) = cmp {
                    return Ok(format!("((int64_t)({} {} {}))", lf, cmpc, rf));
                }
            }
            let string_cmp = matches!(
                op,
                BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
            ) && (expr_tag(left, var_tags, proc_return_types) == TAG_STR || expr_tag(right, var_tags, proc_return_types) == TAG_STR);
            if string_cmp {
                let cmp = format!("lppo_rt_str_cmp({}, {})", l, r);
                return Ok(match op {
                    BinaryOp::Eq => format!("lppo_rt_str_eq({}, {})", l, r),
                    BinaryOp::Ne => format!("(!lppo_rt_str_eq({}, {}))", l, r),
                    BinaryOp::Lt => format!("({} < 0)", cmp),
                    BinaryOp::Le => format!("({} <= 0)", cmp),
                    BinaryOp::Gt => format!("({} > 0)", cmp),
                    _ => format!("({} >= 0)", cmp),
                });
            }
            let opc = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Mod => "%",
                BinaryOp::Pow => unreachable!(),
                BinaryOp::Eq => "==",
                BinaryOp::Ne => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
                BinaryOp::BitAnd => "&",
                BinaryOp::BitOr => "|",
                BinaryOp::BitXor => "^",
            };
            Ok(format!("({} {} {})", l, opc, r))
        }
        Expr::UnaryOp { op, operand } => {
            let v = codegen_expr(operand, locals, var_tags, string_indices, proc_return_types)?;
            match op {
                UnaryOp::Neg => Ok(format!("(-{})", v)),
                UnaryOp::Not => Ok(format!("(({}) ? 0 : 1)", v)),
                UnaryOp::BitNot => Ok(format!("(~{})", v)),
            }
        }
        Expr::Array(elements) => {
            if elements.is_empty() {
                return Ok("((int64_t)(intptr_t)lppo_rt_array_make(0))".to_string());
            }
            let mut parts = vec![format!("(int64_t){}", elements.len())];
            for el in elements {
                parts.push(format!("(int64_t){}", expr_tag(el, var_tags, proc_return_types)));
                parts.push(codegen_expr(
                    el,
                    locals,
                    var_tags,
                    string_indices,
                    proc_return_types,
                )?);
            }
            Ok(format!("lppo_arr({})", parts.join(", ")))
        }
        Expr::Index { target, index } => {
            let t = codegen_expr(target, locals, var_tags, string_indices, proc_return_types)?;
            let i = codegen_expr(index, locals, var_tags, string_indices, proc_return_types)?;
            Ok(format!(
                "lppo_rt_array_get({}, lppo_norm_idx({}, lppo_rt_array_len({})))",
                t, i, t
            ))
        }
        Expr::MethodCall {
            target,
            method,
            args,
        } => {
            let t = codegen_expr(target, locals, var_tags, string_indices, proc_return_types)?;
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(codegen_expr(
                    arg,
                    locals,
                    var_tags,
                    string_indices,
                    proc_return_types,
                )?);
            }
            match method.as_str() {
                "push" => {
                    if arg_vals.len() != 1 {
                        return Err(format!("push expects 1 argument, got {}", arg_vals.len()));
                    }
                    let tag = expr_tag(&args[0], var_tags, proc_return_types);
                    Ok(format!(
                        "lppo_rt_array_push_tag({}, {}, {})",
                        t, tag, arg_vals[0]
                    ))
                }
                "pop" => Ok(format!("lppo_rt_array_pop({})", t)),
                "len" => Ok(format!("lppo_rt_array_len({})", t)),
                "iter" => Ok(t),
                _ => Err(format!(
                    "method '{}' not yet supported in C codegen",
                    method
                )),
            }
        }
        Expr::Ref { target, .. } | Expr::Borrow { target, .. } => {
            codegen_expr(target, locals, var_tags, string_indices, proc_return_types)
        }
        Expr::Deref(inner) => {
            // References are value-like in this runtime, so deref is identity.
            codegen_expr(inner, locals, var_tags, string_indices, proc_return_types)
        }
        Expr::FieldAccess { target, field } => {
            let t = codegen_expr(target, locals, var_tags, string_indices, proc_return_types)?;
            Ok(format!(
                "lppo_rt_struct_get({}, (int64_t)(intptr_t)\"{}\" )",
                t,
                escape_c(field)
            ))
        }
        Expr::SizeOf(ty) => Ok(format!("(int64_t){}", size_of_type(ty))),
        Expr::AlignOf(ty) => Ok(format!(
            "(int64_t){}",
            match ty {
                TypeAnnotation::Bool => 1,
                _ => 8,
            }
        )),
        Expr::Cast { expr, to_type } => {
            let v = codegen_expr(expr, locals, var_tags, string_indices, proc_return_types)?;
            match to_type {
                TypeAnnotation::Bool => Ok(format!("((({}) != 0) ? 1 : 0)", v)),
                TypeAnnotation::Int64 => Ok(format!("((int64_t)({}))", v)),
                TypeAnnotation::Float => Ok(format!("lppo_i64_to_f64_bits({})", v)),
                TypeAnnotation::Str => Ok(v),
                other => Err(format!(
                    "cast to {:?} not yet supported in C codegen",
                    other
                )),
            }
        }
        Expr::Call { func, args } => {
            let (call_name, is_user) = resolve_callee(func, proc_return_types)?;
            let raw = match &**func {
                Expr::Var(n) => n.clone(),
                Expr::QualifiedVar(qn) => qn_string(qn),
                _ => String::new(),
            };
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(codegen_expr(
                    arg,
                    locals,
                    var_tags,
                    string_indices,
                    proc_return_types,
                )?);
            }

            if matches!(raw.as_str(), "print" | "println" | "eprintln") {
                let mut body = String::from("({ ");
                for (i, v) in arg_vals.iter().enumerate() {
                    if i > 0 {
                        body.push_str("lppo_emit_sep(); ");
                    }
                    let tag = expr_tag(&args[i], var_tags, proc_return_types);
                    body.push_str(&format!("lppo_rt_print_value({}, {}); ", tag, v));
                }
                if raw != "print" {
                    body.push_str("lppo_rt_print_newline(); ");
                }
                body.push_str("(int64_t)0; })");
                return Ok(body);
            }

            if call_name == "lppo_rt_array_make" && arg_vals.is_empty() {
                return Ok("((int64_t)(intptr_t)lppo_rt_array_make(0))".to_string());
            }
            let tag_of = |e: &Expr| expr_tag(e, var_tags, proc_return_types);
            match (call_name.as_str(), arg_vals.len()) {
                ("lppo_rt_map_insert", 3) => {
                    return Ok(format!(
                        "lppo_rt_map_insert({}, {}, {}, {}, {})",
                        arg_vals[0],
                        tag_of(&args[1]),
                        arg_vals[1],
                        tag_of(&args[2]),
                        arg_vals[2]
                    ));
                }
                ("lppo_rt_map_get" | "lppo_rt_map_remove", 2) => {
                    return Ok(format!(
                        "{}({}, {}, {})",
                        call_name, arg_vals[0], tag_of(&args[1]), arg_vals[1]
                    ));
                }
                ("lppo_rt_set_insert" | "lppo_rt_set_contains" | "lppo_rt_set_remove", 2) => {
                    return Ok(format!(
                        "{}({}, {}, {})",
                        call_name, arg_vals[0], tag_of(&args[1]), arg_vals[1]
                    ));
                }
                _ => {}
            }

            if call_name == "lppo_rt_json_stringify" && arg_vals.len() == 1 {
                let tag = expr_tag(&args[0], var_tags, proc_return_types);
                return Ok(format!("lppo_rt_json_stringify({}, {})", tag, arg_vals[0]));
            }
            if call_name == "lppo_rt_inline_asm" && arg_vals.len() == 2 {
                return Ok(format!(
                    "lppo_rt_inline_asm({}, {})",
                    arg_vals[0], arg_vals[1]
                ));
            }

            let arg_str = arg_vals.join(", ");
            if is_user {
                let is_void = proc_return_types
                    .get(&raw)
                    .map(|t| matches!(t, TypeAnnotation::Void))
                    .unwrap_or(false);
                if is_void {
                    return Ok(format!("({{ {}({}); (int64_t)0; }})", call_name, arg_str));
                }
            }
            Ok(format!("{}({})", call_name, arg_str))
        }
        Expr::Closure { .. } => Err("closures not yet supported in C codegen".to_string()),
        Expr::StructInit { name, fields } => {
            let mut parts = vec![
                format!("(int64_t)(intptr_t)\"{}\"", escape_c(name)),
                format!("(int64_t){}", fields.len()),
            ];
            for (field_name, value) in fields {
                parts.push(format!("(int64_t)(intptr_t)\"{}\"", escape_c(field_name)));
                parts.push(format!("(int64_t){}", expr_tag(value, var_tags, proc_return_types)));
                parts.push(codegen_expr(
                    value,
                    locals,
                    var_tags,
                    string_indices,
                    proc_return_types,
                )?);
            }
            Ok(format!("lppo_struct({})", parts.join(", ")))
        }
        Expr::Range { .. } => Err("range expressions not yet supported in C codegen".to_string()),
        Expr::MatchExpr { .. } => {
            Err("match expressions not yet supported in C codegen".to_string())
        }
        Expr::Await(inner) | Expr::Join(inner) => {
            codegen_expr(inner, locals, var_tags, string_indices, proc_return_types)
        }
        Expr::Async { body } | Expr::Spawn { body } => {
            // v1: async/spawn blocks run inline and yield their value.
            codegen_expr(body, locals, var_tags, string_indices, proc_return_types)
        }
        Expr::Handle { .. } => Err("effect handling not yet supported in C codegen".to_string()),
    }
}

/// Render a qualified name (`a::b::c`) without relying on a `Display` impl.
pub fn qn_string(qn: &crate::ast::QualifiedName) -> String {
    if qn.path.is_empty() {
        qn.name.clone()
    } else {
        format!("{}::{}", qn.path.join("::"), qn.name)
    }
}

fn lookup_local(name: &str, locals: &HashMap<String, String>) -> Result<String, String> {
    locals
        .get(name)
        .cloned()
        .ok_or_else(|| format!("undefined variable: {}", name))
}

/// True when an expression is statically known to produce a Float.
pub fn is_float_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Float(_) => true,
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            operand,
        } => is_float_expr(operand),
        Expr::BinaryOp { left, right, op } => {
            !matches!(
                op,
                BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
            ) && (is_float_expr(left) || is_float_expr(right))
        }
        Expr::Cast { to_type, .. } => matches!(to_type, TypeAnnotation::Float),
        _ => false,
    }
}

/// Resolve a callee expression into a C function name.
/// Returns `(c_name, is_user_procedure)`.
pub fn resolve_callee(
    func: &Expr,
    proc_return_types: &HashMap<String, TypeAnnotation>,
) -> Result<(String, bool), String> {
    let raw = match func {
        Expr::Var(n) => n.clone(),
        Expr::QualifiedVar(qn) => qn_string(qn),
        _ => return Err(format!("call to non-function expression: {:?}", func)),
    };
    if proc_return_types.contains_key(&raw) {
        return Ok((crate::codegen_c::c_fn_name(&raw), true));
    }
    let mapped = builtin_func_name(&raw);
    Ok((mapped.to_string(), false))
}

fn size_of_type(ty: &TypeAnnotation) -> i64 {
    match ty {
        TypeAnnotation::Int64 | TypeAnnotation::Float => 8,
        TypeAnnotation::Bool => 1,
        TypeAnnotation::Str | TypeAnnotation::Arr(_) => 8,
        TypeAnnotation::Void => 0,
        _ => 8,
    }
}

/// Type tag passed to `lppo_rt_print_value` for this expression.
pub fn expr_tag(expr: &Expr, var_tags: &HashMap<String, i64>, proc_return_types: &HashMap<String, TypeAnnotation>) -> i64 {
    match expr {
        Expr::Float(_) => TAG_FLOAT,
        Expr::Bool(_) => TAG_BOOL,
        Expr::Char(_) => TAG_CHAR,
        Expr::Byte(_) => TAG_BYTE,
        Expr::Tuple(_) => TAG_DYN,
        Expr::EnumVariant { .. } => TAG_ENUM,
        Expr::StructInit { .. } => TAG_STRUCT,
        Expr::Array(_) => TAG_ARRAY,
        Expr::Str(_) => TAG_STR,
        Expr::Var(name) => var_tags.get(name).copied().unwrap_or(TAG_INT),
        Expr::QualifiedVar(qn) => var_tags
            .get(&qn_string(qn))
            .copied()
            .unwrap_or(TAG_INT),
        Expr::UnaryOp {
            op: UnaryOp::Not, ..
        } => TAG_BOOL,
        Expr::BinaryOp { op, .. } => match op {
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::And
            | BinaryOp::Or => TAG_BOOL,
            _ => {
                if is_float_expr(expr) {
                    TAG_FLOAT
                } else {
                    TAG_INT
                }
            }
        },
        Expr::Cast { to_type, .. } => match to_type {
            TypeAnnotation::Bool => TAG_BOOL,
            TypeAnnotation::Float => TAG_FLOAT,
            TypeAnnotation::Str => TAG_STR,
            _ => TAG_INT,
        },
        Expr::Call { func, .. } => call_tag(func, var_tags, proc_return_types),
        Expr::MethodCall { method, .. } => match method.as_str() {
            "push" | "iter" | "map" | "filter" => TAG_ARRAY,
            _ => TAG_INT,
        },
        _ => TAG_INT,
    }
}

fn call_tag(func: &Expr, var_tags: &HashMap<String, i64>, proc_return_types: &HashMap<String, TypeAnnotation>) -> i64 {
    let raw = match func {
        Expr::Var(n) => n.clone(),
        Expr::QualifiedVar(qn) => qn_string(qn),
        _ => return TAG_INT,
    };
    // User-defined procedure: tag by its declared return type.
    if proc_return_types.contains_key(&raw) {
        return match proc_return_types[&raw] {
            TypeAnnotation::Str => TAG_STR,
            TypeAnnotation::Arr(_) => TAG_ARRAY,
            TypeAnnotation::Bool => TAG_BOOL,
            TypeAnnotation::Char => TAG_CHAR,
            TypeAnnotation::Byte => TAG_BYTE,
            TypeAnnotation::Tuple(_) | TypeAnnotation::Map(..) => TAG_DYN,
            TypeAnnotation::Custom(ref n) => if n == "Float64" || n == "F64" { TAG_FLOAT } else { TAG_DYN },
            _ => TAG_INT,
        };
    }
    match raw.as_str() {
        "json_parse" => TAG_JSON,
        "int_to_str" | "str_concat" | "str_upper" | "str_lower" | "str_trim"
        | "json_stringify" | "system" | "color" | "read_file" => TAG_STR,
        "either" => TAG_ARRAY,
        "write_file" | "append_file" | "file_exists" | "delete_file" => TAG_BOOL,
        "str_to_int" | "str_len" | "str_split" | "simd_extract" | "simd_reduce" => TAG_INT,
        "simd_splat" | "simd_add" | "simd_sub" | "simd_mul" | "simd_div" | "simd_insert"
        | "simd_shuffle" | "vec_new" | "vec_push" | "map_keys" | "map_values"
        | "set_to_array" => TAG_ARRAY,
        "tuple_new" | "tuple_get" => TAG_DYN,
        "map_new" | "map_insert" | "map_remove" => TAG_MAP,
        "set_new" | "set_insert" | "set_remove" => TAG_SET,
        "set_contains" => TAG_BOOL,
        _ => var_tags.get(&raw).copied().unwrap_or(TAG_INT),
    }
}

/// Escape a string for inclusion in a generated C string literal.
pub fn escape_c(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Option/Result are built-in enum-like values whose `enum_name` is not
/// printed; keep the variant as-is for everything else.
fn enum_variant_runtime_name(enum_name: &str, variant: &str) -> String {
    match enum_name {
        "Option" | "Result" => variant.to_string(),
        _ => variant.to_string(),
    }
}
