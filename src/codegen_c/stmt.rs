//! Statement / Operation -> C code.
//!
//! Translates `crate::runtime::Operation` variants into plain C statements.
//! All LPPO values are `int64_t` at the C level, so loops and branches map
//! directly onto C `if` / `while` / `for (;;)` constructs.

use crate::ast::{Expr, TypeAnnotation};
use crate::codegen_c::expr::{codegen_expr, expr_tag};
use crate::codegen_c::{
    builtin_func_name, resolve_callee, ClosureInfo, TAG_ARRAY, TAG_BOOL, TAG_BYTE, TAG_CHAR,
    TAG_DYN, TAG_ENUM, TAG_FLOAT, TAG_INT, TAG_JSON, TAG_SET, TAG_STR, TAG_STRUCT,
    TAG_MAP,
};
use crate::runtime::Operation;
use std::collections::HashMap;

/// Codegen state shared across a single procedure body.
pub struct Ctx<'a> {
    /// LPPO source name -> C variable name.
    pub locals: HashMap<String, String>,
    /// Print tag for each local (TAG_INT, TAG_STR, TAG_ARRAY, ...).
    pub var_tags: HashMap<String, i64>,
    pub label_counter: i64,
    pub string_indices: &'a HashMap<String, usize>,
    pub proc_return_types: &'a HashMap<String, TypeAnnotation>,
    pub return_type: &'a str,
    /// Array variables whose elements are strings.
    pub str_arrays: HashMap<String, bool>,
    /// Discovered closures (index, params, captures, body) during this procedure.
    pub closures: &'a mut Vec<ClosureInfo>,
    /// Next closure index to assign (shared across all procedures).
    pub closure_counter: &'a mut i64,
}

impl<'a> Ctx<'a> {
    pub fn next_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        format!("{}{}", prefix, self.label_counter)
    }

    fn expr(&mut self, expr: &Expr) -> Result<String, String> {
        codegen_expr(
            expr,
            &self.locals,
            &self.var_tags,
            self.string_indices,
            self.proc_return_types,
            self.closures,
            self.closure_counter,
        )
    }
}

/// Map a LPPO identifier to a safe C identifier.
pub fn c_ident(name: &str) -> String {
    let mut out = String::from("v_");
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.len() > 1 && out.as_bytes()[2].is_ascii_digit() {
        out.insert_str(2, "_");
    }
    out
}

/// Emit a single Operation as C statements into `out`.
pub fn codegen_operation(
    op: &Operation,
    ctx: &mut Ctx<'_>,
    out: &mut String,
) -> Result<(), String> {
    match op {
        Operation::Nop => Ok(()),

        Operation::Let(name, expr) => {
            let cname = match ctx.locals.get(name) {
                Some(c) => c.clone(),
                None => {
                    let c = c_ident(name);
                    ctx.locals.insert(name.clone(), c.clone());
                    out.push_str(&format!("  int64_t {} = 0;\n", c));
                    c
                }
            };
            let val = ctx.expr(expr)?;
            out.push_str(&format!("  {} = {};\n", cname, val));
            let tag = infer_tag(expr, ctx);
            ctx.var_tags.insert(name.clone(), tag);
            let is_str_arr = is_str_array_expr(expr, ctx);
            ctx.str_arrays.insert(name.clone(), is_str_arr);
            Ok(())
        }

        Operation::Emit(exprs) => {
            for (i, expr) in exprs.iter().enumerate() {
                if i > 0 {
                    out.push_str("  lppo_emit_sep();\n");
                }
                emit_print(ctx, expr, out)?;
            }
            out.push_str("  lppo_rt_print_newline();\n");
            Ok(())
        }

        Operation::Parallel(exprs) => {
            for (i, expr) in exprs.iter().enumerate() {
                if i > 0 {
                    out.push_str("  lppo_emit_sep();\n");
                }
                emit_print(ctx, expr, out)?;
            }
            out.push_str("  lppo_rt_print_newline();\n");
            Ok(())
        }

        Operation::Call(name, args) => {
            let raw = if let Some(n) = name.strip_prefix("self::") {
                n.to_string()
            } else {
                name.clone()
            };
            if matches!(raw.as_str(), "print" | "println" | "eprintln") {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str("  lppo_emit_sep();\n");
                    }
                    emit_print(ctx, arg, out)?;
                }
                if raw != "print" {
                    out.push_str("  lppo_rt_print_newline();\n");
                }
                return Ok(());
            }
            let mut arg_vals = Vec::new();
            for arg in args {
                arg_vals.push(ctx.expr(arg)?);
            }
            let arg_str = arg_vals.join(", ");
            let is_user = ctx.proc_return_types.contains_key(&raw);
            let cname = if is_user {
                crate::codegen_c::c_fn_name(&raw)
            } else {
                builtin_func_name(&raw).to_string()
            };
            if is_user {
                out.push_str(&format!("  {}({});\n", cname, arg_str));
            } else {
                out.push_str(&format!("  (void){}({});\n", cname, arg_str));
            }
            Ok(())
        }

        Operation::ExprStatement(expr) => {
            let val = ctx.expr(expr)?;
            out.push_str(&format!("  (void)({});\n", val));
            Ok(())
        }

        Operation::Return(opt_expr) => {
            if ctx.return_type == "void" {
                if let Some(e) = opt_expr {
                    let _ = ctx.expr(e)?;
                }
                out.push_str("  return;\n");
            } else {
                let val = match opt_expr {
                    Some(e) => ctx.expr(e)?,
                    None => "0".to_string(),
                };
                out.push_str(&format!("  return {};\n", val));
            }
            Ok(())
        }

        Operation::Input { var, typ, prompt } => {
            let cname = ctx.ensure_local(var, out);
            if let Some(p) = prompt {
                let pv = ctx.expr(p)?;
                out.push_str(&format!(
                    "  lppo_rt_print_value({}, {});\n",
                    TAG_STR, pv
                ));
                out.push_str("  lppo_rt_print_value(3, (int64_t)(intptr_t)\" \");\n");
                out.push_str("  lppo_rt_flush_stdout();\n");
            }
            let tag = match typ {
                TypeAnnotation::Bool => 2,
                TypeAnnotation::Str => 3,
                TypeAnnotation::Arr(_) => TAG_ARRAY,
                _ => TAG_INT,
            };
            out.push_str(&format!("  {} = lppo_rt_input({}, 0);\n", cname, tag));
            let vtag = match typ {
                TypeAnnotation::Str => TAG_STR,
                TypeAnnotation::Bool => TAG_BOOL,
                TypeAnnotation::Arr(_) => TAG_ARRAY,
                _ => TAG_INT,
            };
            if let TypeAnnotation::Arr(inner) = typ {
                if matches!(**inner, TypeAnnotation::Str) {
                    ctx.str_arrays.insert(var.clone(), true);
                }
            }
            ctx.var_tags.insert(var.clone(), vtag);
            Ok(())
        }

        Operation::Alloc { var, location: _, size } => {
            let cname = ctx.ensure_local(var, out);
            let size_val = ctx.expr(size)?;
            out.push_str(&format!("  {} = lppo_rt_alloc({});\n", cname, size_val));
            Ok(())
        }

        Operation::Dealloc(var) => {
            if let Some(cname) = ctx.locals.get(var).cloned() {
                out.push_str(&format!("  lppo_rt_dealloc({});\n", cname));
            }
            Ok(())
        }

        Operation::IndexedAssignment { target, index, value } => {
            let t = ctx.locals.get(target).cloned().unwrap_or_else(|| target.clone());
            let i = ctx.expr(index)?;
            let v = ctx.expr(value)?;
            let vt = expr_tag(value, &ctx.var_tags, &ctx.proc_return_types);
            out.push_str(&format!(
                "  {{ int64_t _t = {t}; int64_t _i = lppo_norm_idx({i}, lppo_rt_array_len(_t)); \
                 lppo_rt_array_set_tag(_t, _i, {vt}, {v}); }}\n",
                t = t, i = i, vt = vt, v = v
            ));
            Ok(())
        }

        Operation::If {
            cond,
            then_ops,
            elifs,
            else_ops,
        } => {
            let cond_val = ctx.expr(cond)?;
            out.push_str(&format!("  if ({}) {{\n", cond_val));
            for op in then_ops {
                codegen_operation(op, ctx, out)?;
            }
            out.push_str("  }");
            for (econd, eops) in elifs {
                let ecv = ctx.expr(econd)?;
                out.push_str(&format!(" else if ({}) {{\n", ecv));
                for op in eops {
                    codegen_operation(op, ctx, out)?;
                }
                out.push_str("  }");
            }
            if let Some(eops) = else_ops {
                out.push_str(" else {\n");
                for op in eops {
                    codegen_operation(op, ctx, out)?;
                }
                out.push_str("  }");
            }
            out.push_str("\n");
            Ok(())
        }

        Operation::While { cond, ops } => {
            let cond_val = ctx.expr(cond)?;
            out.push_str(&format!("  while ({}) {{\n", cond_val));
            for op in ops {
                codegen_operation(op, ctx, out)?;
            }
            out.push_str("  }\n");
            Ok(())
        }

        Operation::For { var, iterable, ops } => {
            let arr = ctx.next_label("arr");
            let idx = ctx.next_label("idx");
            let len = ctx.next_label("len");
            let iter_val = ctx.expr(iterable)?;
            out.push_str("  {\n");
            out.push_str(&format!("    int64_t {arr} = {};\n", iter_val));
            out.push_str(&format!(
                "    int64_t {len} = lppo_rt_array_len({arr});\n",
                arr = arr,
                len = len
            ));
            out.push_str(&format!(
                "    for (int64_t {idx} = 0; {idx} < {len}; {idx}++) {{\n",
                idx = idx,
                len = len
            ));
            let elem_tag = iterable_elem_tag(iterable, ctx);
            let cname = ctx.ensure_local(var, out);
            out.push_str(&format!(
                "      {} = lppo_rt_array_get({}, {});\n",
                cname, arr, idx
            ));
            ctx.var_tags.insert(var.clone(), elem_tag);
            for op in ops {
                codegen_operation(op, ctx, out)?;
            }
            out.push_str("    }\n  }\n");
            Ok(())
        }

        Operation::Loop { ops } => {
            out.push_str("  for (;;) {\n");
            for op in ops {
                codegen_operation(op, ctx, out)?;
            }
            out.push_str("  }\n");
            Ok(())
        }

        Operation::Break => {
            out.push_str("  break;\n");
            Ok(())
        }
        Operation::Continue => {
            out.push_str("  continue;\n");
            Ok(())
        }

        Operation::Scope { ops } | Operation::Unsafe { ops } => {
            out.push_str("  {\n");
            for op in ops {
                codegen_operation(op, ctx, out)?;
            }
            out.push_str("  }\n");
            Ok(())
        }

        Operation::Spawn { ops } => {
            // v1: spawn runs the block sequentially (no thread runtime yet).
            out.push_str("  {\n");
            for op in ops {
                codegen_operation(op, ctx, out)?;
            }
            out.push_str("  }\n");
            Ok(())
        }

        Operation::Pause {
            prompt,
            assign_to,
            is_async: _,
            ..
        } => {
            if let Some(p) = prompt {
                let pv = ctx.expr(p)?;
                out.push_str(&format!(
                    "  lppo_rt_print_value({}, {});\n",
                    TAG_STR, pv
                ));
                out.push_str("  lppo_rt_print_value(3, (int64_t)(intptr_t)\" \");\n");
                out.push_str("  lppo_rt_flush_stdout();\n");
            }
            let tmp = ctx.next_label("pause");
            out.push_str(&format!("  int64_t {} = lppo_rt_input(3, 0);\n", tmp));
            if let Some(name) = assign_to {
                let cname = ctx.ensure_local(name, out);
                out.push_str(&format!("  {} = {};\n", cname, tmp));
                ctx.var_tags.insert(name.clone(), TAG_STR);
            }
            Ok(())
        }
    }
}

impl<'a> Ctx<'a> {
    /// Look up or create the C variable backing a LPPO name.
    fn ensure_local(&mut self, name: &str, out: &mut String) -> String {
        if let Some(c) = self.locals.get(name) {
            return c.clone();
        }
        let c = c_ident(name);
        self.locals.insert(name.to_string(), c.clone());
        out.push_str(&format!("  int64_t {} = 0;\n", c));
        c
    }
}

/// Emit a tagged print of `expr`, resolving aggregate element tags at runtime.
fn emit_print(ctx: &mut Ctx<'_>, expr: &Expr, out: &mut String) -> Result<(), String> {
    match expr {
        Expr::Index { target, index } => {
            let a = ctx.next_label("px");
            let t = ctx.expr(target)?;
            let i = ctx.expr(index)?;
            out.push_str(&format!(
                "  {{ int64_t {a} = {t}; int64_t {a}_i = lppo_norm_idx({i}, lppo_rt_array_len({a})); \
                 lppo_rt_print_value(lppo_rt_array_get_tag({a}, {a}_i), lppo_rt_array_get({a}, {a}_i)); }}\n",
                a = a,
                t = t,
                i = i
            ));
            Ok(())
        }
        Expr::FieldAccess { target, field } => {
            let a = ctx.next_label("pf");
            let t = ctx.expr(target)?;
            let lit = format!("(int64_t)(intptr_t)\"{}\"", crate::codegen_c::expr::escape_c(field));
            out.push_str(&format!(
                "  {{ int64_t {a} = {t}; lppo_rt_print_value(lppo_rt_struct_get_tag({a}, {lit}), lppo_rt_struct_get({a}, {lit})); }}\n",
                a = a,
                t = t,
                lit = lit
            ));
            Ok(())
        }
        Expr::Call { func, args } => {
            let raw = match func.as_ref() {
                Expr::Var(n) => n.clone(),
                Expr::QualifiedVar(qn) => crate::codegen_c::expr::qn_string(qn),
                _ => String::new(),
            };
            match (raw.as_str(), args.len()) {
                ("map_get", 2) => {
                    let a = ctx.next_label("pm");
                    let m = ctx.expr(&args[0])?;
                    let k = ctx.expr(&args[1])?;
                    let kt = expr_tag(&args[1], &ctx.var_tags, &ctx.proc_return_types);
                    out.push_str(&format!(
                        "  {{ int64_t {a} = {m}; int64_t {a}_k = {k}; \
                         lppo_rt_print_value(lppo_rt_map_get_tag({a}, {kt}, {a}_k), lppo_rt_map_get({a}, {kt}, {a}_k)); }}\n",
                        a = a,
                        m = m,
                        k = k,
                        kt = kt
                    ));
                    return Ok(());
                }
                ("tuple_get", 2) => {
                    let a = ctx.next_label("pt");
                    let t = ctx.expr(&args[0])?;
                    let i = ctx.expr(&args[1])?;
                    out.push_str(&format!(
                        "  {{ int64_t {a} = {t}; int64_t {a}_i = lppo_norm_idx({i}, 8); \
                         lppo_rt_print_value(lppo_rt_tuple_get_tag({a}, {a}_i), lppo_rt_tuple_get({a}, {a}_i)); }}\n",
                        a = a,
                        t = t,
                        i = i
                    ));
                    return Ok(());
                }
                ("enum_get", 2) => {
                    let a = ctx.next_label("pe");
                    let t = ctx.expr(&args[0])?;
                    let i = ctx.expr(&args[1])?;
                    out.push_str(&format!(
                        "  {{ int64_t {a} = {t}; int64_t {a}_i = {i}; \
                         lppo_rt_print_value(lppo_rt_enum_get_tag({a}, {a}_i), lppo_rt_enum_get({a}, {a}_i)); }}\n",
                        a = a,
                        t = t,
                        i = i
                    ));
                    return Ok(());
                }
                _ => {}
            }
            let tag = expr_tag(expr, &ctx.var_tags, &ctx.proc_return_types);
            let val = ctx.expr(expr)?;
            out.push_str(&format!("  lppo_rt_print_value({}, {});\n", tag, val));
            Ok(())
        }
        _ => {
            let tag = expr_tag(expr, &ctx.var_tags, &ctx.proc_return_types);
            let val = ctx.expr(expr)?;
            out.push_str(&format!("  lppo_rt_print_value({}, {});\n", tag, val));
            Ok(())
        }
    }
}

/// Element print tag for a `for` loop over this iterable.
fn iterable_elem_tag(expr: &Expr, ctx: &Ctx<'_>) -> i64 {
    if is_str_iterable(expr, ctx) {
        return TAG_STR;
    }
    match expr {
        Expr::Var(n) => match ctx.str_arrays.get(n) {
            Some(_) => TAG_INT,
            None => TAG_INT,
        },
        _ => TAG_INT,
    }
}

/// Is the iterable known to be an array of strings?
fn is_str_iterable(expr: &Expr, ctx: &Ctx<'_>) -> bool {
    match expr {
        Expr::Var(n) => ctx.str_arrays.get(n).copied().unwrap_or(false),
        other => is_str_array_expr(other, ctx),
    }
}

/// Is this expression a call that yields a string array?
fn is_str_array_expr(expr: &Expr, ctx: &Ctx<'_>) -> bool {
    match expr {
        Expr::Call { func, .. } => resolve_callee(func, ctx.proc_return_types)
            .map(|(n, _)| n == "lppo_rt_str_split" || n == "lppo_rt_map_keys")
            .unwrap_or(false),
        Expr::Var(n) => ctx.str_arrays.get(n).copied().unwrap_or(false),
        _ => false,
    }
}

/// Would printing this expression print an array of strings?
fn is_str_array_value(expr: &Expr, ctx: &Ctx<'_>) -> bool {
    is_str_array_expr(expr, ctx)
}

/// Infer the print tag a bound value will have at runtime.
fn infer_tag(expr: &Expr, ctx: &Ctx<'_>) -> i64 {
    match expr {
        Expr::Str(_) => TAG_STR,
        Expr::Array(_) => TAG_ARRAY,
        Expr::Bool(_) => TAG_BOOL,
        Expr::Char(_) => TAG_CHAR,
        Expr::Byte(_) => TAG_BYTE,
        Expr::Tuple(_) => TAG_DYN,
        Expr::EnumVariant { .. } => TAG_ENUM,
        Expr::StructInit { .. } => TAG_STRUCT,
        Expr::Float(_) => crate::codegen_c::TAG_FLOAT,
        Expr::Var(_) | Expr::QualifiedVar(_) | Expr::Index { .. } => expr_tag(expr, &ctx.var_tags, &ctx.proc_return_types),
        Expr::MethodCall { method, .. } => match method.as_str() {
            "push" | "iter" => TAG_ARRAY,
            _ => TAG_INT,
        },
        Expr::Call { func, .. } => {
            if let Ok((name, is_user)) = resolve_callee(func, ctx.proc_return_types) {
                if is_user {
                    return match name.as_str() {
                        n => match ctx
                            .proc_return_types
                            .iter()
                            .find(|(k, _)| crate::codegen_c::c_fn_name(k) == n)
                            .map(|(_, t)| t.clone())
                        {
                            Some(TypeAnnotation::Str) => TAG_STR,
                            Some(TypeAnnotation::Arr(_)) => TAG_ARRAY,
                            Some(TypeAnnotation::Bool) => TAG_BOOL,
                            Some(TypeAnnotation::Char) => TAG_CHAR,
                            Some(TypeAnnotation::Byte) => TAG_BYTE,
                            Some(TypeAnnotation::Tuple(_)) => TAG_DYN,
                            Some(TypeAnnotation::Map(..)) => TAG_MAP,
                            Some(TypeAnnotation::Set(_)) => TAG_SET,
                            Some(TypeAnnotation::Option(_) | TypeAnnotation::Result(..)) => TAG_ENUM,
                            Some(TypeAnnotation::Custom(_)) => TAG_DYN,
                            Some(TypeAnnotation::Float | TypeAnnotation::Float32
                                | TypeAnnotation::Float64) => crate::codegen_c::TAG_FLOAT,
                            _ => TAG_INT,
                        },
                    };
                }
                return match name.as_str() {
                    "lppo_rt_json_parse" => TAG_JSON,
                    "lppo_rt_int_to_str" | "lppo_rt_str_concat" | "lppo_rt_str_upper"
                    | "lppo_rt_str_lower" | "lppo_rt_str_trim"
                    | "lppo_rt_json_stringify" | "lppo_rt_system" | "lppo_rt_color"
                    | "lppo_rt_read_file" => TAG_STR,
                    "lppo_rt_tuple_make" => TAG_DYN,
                    "lppo_rt_map_new" | "lppo_rt_map_insert" | "lppo_rt_map_remove" => TAG_MAP,
                    "lppo_rt_set_new" | "lppo_rt_set_insert" | "lppo_rt_set_remove" => TAG_SET,
                    "lppo_rt_either" | "lppo_rt_array_make" | "lppo_rt_array_push" | "lppo_rt_str_split"
                    | "lppo_rt_map_keys" | "lppo_rt_simd_splat" | "lppo_rt_simd_add"
                    | "lppo_rt_simd_sub" | "lppo_rt_simd_mul" | "lppo_rt_simd_div"
                    | "lppo_rt_simd_insert" | "lppo_rt_simd_shuffle" => TAG_ARRAY,
                    "lppo_rt_write_file" | "lppo_rt_append_file" | "lppo_rt_file_exists"
                    | "lppo_rt_delete_file" | "lppo_rt_pool_free" => TAG_BOOL,
                    _ => TAG_INT,
                };
            }
            TAG_INT
        }
        Expr::Cast { expr, .. } => infer_tag(expr, ctx),
        Expr::Ref { target, .. } | Expr::Borrow { target, .. } => infer_tag(target, ctx),
        _ => TAG_INT,
    }
}
