//! LPPO C codegen backend module entry point.
//!
//! Translates compiled LPPO bytecode (`ProcedureBody`) into C source code,
//! which is then compiled by the system C compiler (`cc`) into a native
//! executable.
//!
//! Pipeline:
//!   ProcedureBody (bytecode)  -->  C source (.c)  -->  .o object  -->  linked executable
//!
//! All values are represented as `int64_t` in C:
//!   - Int64   -> i64
//!   - Bool    -> i64 (0 or 1)
//!   - Float   -> i64 (bitcast)
//!   - Str     -> i64 (ptrtoint of char* global)
//!   - Arr<T>  -> i64 (ptrtoint of heap array struct)
//!   - Void    -> ()

mod expr;
mod proc;
mod stmt;
mod target;

pub use expr::*;
pub use proc::*;
pub use stmt::*;
pub use target::*;

use crate::ast::TypeAnnotation;
use crate::runtime::{ProcedureBody};
use std::collections::HashMap;

/// Top-level entry: compile all procedures into C source code string.
pub fn compile_program_to_c(
    procedures: &HashMap<String, ProcedureBody>,
) -> Result<String, String> {
    generate_ir_text(procedures)
}

/// Generate complete C source code text from all procedures.
fn generate_ir_text(
    procedures: &HashMap<String, ProcedureBody>,
) -> Result<String, String> {
    let mut out = String::new();

    // C headers and runtime declarations
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdarg.h>\n\n");

    // Declare all runtime built-in functions.
    declare_builtins_c(&mut out);
    emit_c_helpers(&mut out);

    // Build string pool and index map.
    let mut string_pool: Vec<String> = Vec::new();
    collect_strings(procedures, &mut string_pool);
    let string_indices: std::collections::HashMap<String, usize> = string_pool
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    // Emit string globals.
    for (idx, s) in string_pool.iter().enumerate() {
        let bytes = s.as_bytes();
        let mut escaped = String::with_capacity(bytes.len() + 2);
        for b in bytes {
            match b {
                b'\\' => escaped.push_str("\\\\"),
                b'"' => escaped.push_str("\\\""),
                b'\n' => escaped.push_str("\\n"),
                b'\r' => escaped.push_str("\\r"),
                b'\t' => escaped.push_str("\\t"),
                b if *b >= 0x20 && *b < 0x7f => escaped.push(*b as char),
                _ => escaped.push_str(&format!("\\{:02X}", b)),
            }
        }
        out.push_str(&format!(
            "static const char* str_{} = \"{}\";\n",
            idx, escaped
        ));
    }
    out.push_str("\n");

    // Build procedure return types map.
    let proc_return_types: HashMap<String, TypeAnnotation> = procedures
        .iter()
        .map(|(name, body)| (name.clone(), body.return_type.clone().unwrap_or(TypeAnnotation::Void)))
        .collect();

    // Forward declarations so procedures can be called in any order.
    for (name, body) in procedures {
        let c_name = if name == "main" {
            "lppo_user_main".to_string()
        } else {
            c_fn_name(name)
        };
        let ret = match body.return_type {
            Some(TypeAnnotation::Void) | None => "void",
            _ => "int64_t",
        };
        let params = if body.params.is_empty() {
            "void".to_string()
        } else {
            body.params
                .iter()
                .map(|p| format!("int64_t {}", crate::codegen_c::stmt::c_ident(p)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        out.push_str(&format!("static {} {}({});\n", ret, c_name, params));
    }
    out.push_str("\n");

    // Emit each procedure body as a C function.
    for (name, body) in procedures {
        let c_name = if name == "main" {
            "lppo_user_main".to_string()
        } else {
            c_fn_name(name)
        };
        out.push_str(&compile_procedure_ir(
            &c_name,
            body,
            &string_indices,
            &proc_return_types,
        )?);
        out.push_str("\n");
    }

    // Emit synthetic `main` wrapper (a no-op one when the module has no `main`).
    if let Some(main_body) = procedures.get("main") {
        out.push_str("int main(void) {\n");
        match main_body.return_type {
            Some(crate::ast::TypeAnnotation::Void) | None => {
                out.push_str("    lppo_user_main();\n");
                out.push_str("    return 0;\n");
            }
            _ => {
                out.push_str("    int64_t r = lppo_user_main();\n");
                out.push_str("    return (int)r;\n");
            }
        }
        out.push_str("}\n");
    } else {
        out.push_str("int main(void) { return 0; }\n");
    }

    Ok(out)
}

/// Collect all string literals from procedures into the string pool.
fn collect_strings(
    procedures: &HashMap<String, ProcedureBody>,
    pool: &mut Vec<String>,
) {
    for body in procedures.values() {
        collect_strings_in_ops(&body.operations, pool);
    }
}

fn collect_strings_in_ops(ops: &[crate::runtime::Operation], pool: &mut Vec<String>) {
    use crate::runtime::Operation;
    for op in ops {
        match op {
            Operation::Emit(exprs) | Operation::Parallel(exprs) => {
                for e in exprs {
                    collect_strings_in_expr(e, pool);
                }
            }
            Operation::Let(_, e) => collect_strings_in_expr(e, pool),
            Operation::Return(Some(e)) => collect_strings_in_expr(e, pool),
            Operation::ExprStatement(e) => collect_strings_in_expr(e, pool),
            Operation::Call(_, args) => {
                for a in args {
                    collect_strings_in_expr(a, pool);
                }
            }
            Operation::Input { prompt, .. } => {
                if let Some(p) = prompt {
                    collect_strings_in_expr(p, pool);
                }
            }
            Operation::Alloc { size, .. } => collect_strings_in_expr(size, pool),
            Operation::If { cond, then_ops, elifs, else_ops } => {
                collect_strings_in_expr(cond, pool);
                collect_strings_in_ops(then_ops, pool);
                for (ec, eops) in elifs {
                    collect_strings_in_expr(ec, pool);
                    collect_strings_in_ops(eops, pool);
                }
                if let Some(eops) = else_ops {
                    collect_strings_in_ops(eops, pool);
                }
            }
            Operation::While { cond, ops } => {
                collect_strings_in_expr(cond, pool);
                collect_strings_in_ops(ops, pool);
            }
            Operation::For { iterable, ops, .. } => {
                collect_strings_in_expr(iterable, pool);
                collect_strings_in_ops(ops, pool);
            }
            Operation::Loop { ops }
            | Operation::Scope { ops }
            | Operation::Unsafe { ops }
            | Operation::Spawn { ops } => collect_strings_in_ops(ops, pool),
            Operation::Pause { prompt, timeout_ms, until, on_mismatch, on_timeout, .. } => {
                for e in [prompt, timeout_ms, until, on_mismatch, on_timeout].into_iter().flatten() {
                    collect_strings_in_expr(e, pool);
                }
            }
            _ => {}
        }
    }
}

fn collect_strings_in_expr(expr: &crate::ast::Expr, pool: &mut Vec<String>) {
    use crate::ast::Expr;
    match expr {
        Expr::Str(s) => {
            if !pool.iter().any(|p| p == s) {
                pool.push(s.clone());
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_strings_in_expr(left, pool);
            collect_strings_in_expr(right, pool);
        }
        Expr::UnaryOp { operand, .. } => collect_strings_in_expr(operand, pool),
        Expr::Cast { expr, .. } => collect_strings_in_expr(expr, pool),
        Expr::Ref { target, .. } | Expr::Borrow { target, .. } => {
            collect_strings_in_expr(target, pool)
        }
        Expr::Deref(inner) => collect_strings_in_expr(inner, pool),
        Expr::Index { target, index } => {
            collect_strings_in_expr(target, pool);
            collect_strings_in_expr(index, pool);
        }
        Expr::MethodCall { target, args, .. } => {
            collect_strings_in_expr(target, pool);
            for a in args {
                collect_strings_in_expr(a, pool);
            }
        }
        Expr::Call { func, args } => {
            collect_strings_in_expr(func, pool);
            for a in args {
                collect_strings_in_expr(a, pool);
            }
        }
        Expr::Array(items) => {
            for a in items {
                collect_strings_in_expr(a, pool);
            }
        }
        Expr::Tuple(items) => {
            for a in items {
                collect_strings_in_expr(a, pool);
            }
        }
        Expr::EnumVariant { args, .. } => {
            for a in args {
                collect_strings_in_expr(a, pool);
            }
        }
        Expr::Range { start, end, .. } => {
            collect_strings_in_expr(start, pool);
            collect_strings_in_expr(end, pool);
        }
        Expr::StructInit { fields, .. } => {
            for (_, e) in fields {
                collect_strings_in_expr(e, pool);
            }
        }
        _ => {}
    }
}

/// Mangle an LPPO procedure name into a valid C function name.
pub fn c_fn_name(name: &str) -> String {
    let mut out = String::from("lppo_fn_");
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

/// Emit `static` helpers used by the generated expression code.
fn emit_c_helpers(out: &mut String) {
    out.push_str("static void lppo_emit_sep(void) { fputc(' ', stdout); fflush(stdout); }\n");
    out.push_str("static void lppo_rt_flush_stdout(void) { fflush(stdout); }\n");
    out.push_str("static int64_t lppo_f64_to_i64(double d) { int64_t r; memcpy(&r, &d, sizeof r); return r; }\n");
    out.push_str("static double lppo_i64_to_f64(int64_t v) { double d; memcpy(&d, &v, sizeof d); return d; }\n");
    out.push_str("static int64_t lppo_i64_to_f64_bits(int64_t v) { return lppo_f64_to_i64((double)v); }\n");
    out.push_str("static int64_t lppo_norm_idx(int64_t i, int64_t n) { return i < 0 ? n + i : i; }\n");
    out.push_str(
        "static int64_t lppo_arr(int64_t n, ...) {\n\
         \x20   int64_t a = (int64_t)(intptr_t)lppo_rt_array_make(n);\n\
         \x20   va_list ap; va_start(ap, n);\n\
         \x20   for (int64_t i = 0; i < n; i++) {\n\
         \x20       int64_t tag = va_arg(ap, int64_t);\n\
         \x20       int64_t val = va_arg(ap, int64_t);\n\
         \x20       lppo_rt_array_set_tag(a, i, tag, val);\n\
         \x20   }\n\
         \x20   va_end(ap);\n\
         \x20   return a;\n\
         }\n",
    );
    out.push_str(
        "static int64_t lppo_tuple(int64_t n, ...) {\n\
         \x20   int64_t t = (int64_t)(intptr_t)lppo_rt_tuple_make(n);\n\
         \x20   va_list ap; va_start(ap, n);\n\
         \x20   for (int64_t i = 0; i < n; i++) {\n\
         \x20       int64_t tag = va_arg(ap, int64_t);\n\
         \x20       int64_t val = va_arg(ap, int64_t);\n\
         \x20       lppo_rt_object_set(t, i, tag, val);\n\
         \x20   }\n\
         \x20   va_end(ap);\n\
         \x20   return t;\n\
         }\n",
    );
    out.push_str(
        "static int64_t lppo_enum(int64_t name, int64_t variant, int64_t n, ...) {\n\
         \x20   int64_t e = (int64_t)(intptr_t)lppo_rt_enum_make(name, variant, n);\n\
         \x20   va_list ap; va_start(ap, n);\n\
         \x20   for (int64_t i = 0; i < n; i++) {\n\
         \x20       int64_t tag = va_arg(ap, int64_t);\n\
         \x20       int64_t val = va_arg(ap, int64_t);\n\
         \x20       lppo_rt_object_set(e, i, tag, val);\n\
         \x20   }\n\
         \x20   va_end(ap);\n\
         \x20   return e;\n\
         }\n",
    );
    out.push_str(
        "static int64_t lppo_struct(int64_t name, int64_t n, ...) {\n\
         \x20   int64_t s = (int64_t)(intptr_t)lppo_rt_struct_make(name, n);\n\
         \x20   va_list ap; va_start(ap, n);\n\
         \x20   for (int64_t i = 0; i < n; i++) {\n\
         \x20       int64_t fname = va_arg(ap, int64_t);\n\
         \x20       int64_t tag = va_arg(ap, int64_t);\n\
         \x20       int64_t val = va_arg(ap, int64_t);\n\
         \x20       lppo_rt_struct_set(s, i, fname, tag, val);\n\
         \x20   }\n\
         \x20   va_end(ap);\n\
         \x20   return s;\n\
         }\n",
    );
    out.push_str("\n");
}

/// Declare all runtime built-in functions as C declarations.
fn declare_builtins_c(out: &mut String) {
    let builtins: &[(&str, &[&str], &str)] = &[
        ("lppo_rt_print_value", &["int64_t", "int64_t"], "void"),
        ("lppo_rt_print_newline", &[], "void"),
        ("lppo_rt_tui_init", &[], "void"),
        ("lppo_rt_tui_end", &[], "void"),
        ("lppo_rt_tui_clear", &[], "void"),
        ("lppo_rt_tui_move", &["int64_t", "int64_t"], "void"),
        ("lppo_rt_tui_put", &["int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_tui_puts", &["int64_t", "int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_tui_cls", &["int64_t", "int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_tui_box", &["int64_t", "int64_t", "int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_tui_title", &["int64_t"], "void"),
        ("lppo_rt_tui_style", &["int64_t"], "void"),
        ("lppo_rt_tui_reset", &[], "void"),
        ("lppo_rt_tui_progress", &["int64_t", "int64_t", "int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_tui_size", &[], "int64_t"),
        ("lppo_rt_tui_fg", &["int64_t"], "void"),
        ("lppo_rt_tui_bold", &["int64_t"], "void"),
        ("lppo_rt_tui_underline", &["int64_t"], "void"),
        ("lppo_rt_tui_hide_cursor", &[], "void"),
        ("lppo_rt_tui_show_cursor", &[], "void"),
        ("lppo_rt_tui_refresh", &[], "void"),
        ("lppo_rt_input", &["int64_t", "char*"], "int64_t"),
        ("lppo_rt_alloc", &["int64_t"], "int64_t"),
        ("lppo_rt_dealloc", &["int64_t"], "void"),
        ("lppo_rt_array_make", &["int64_t"], "int64_t"),
        ("lppo_rt_array_get", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_array_set", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_array_set_tag", &["int64_t", "int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_array_get_tag", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_array_push", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_array_push_tag", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_array_pop", &["int64_t"], "int64_t"),
        ("lppo_rt_array_len", &["int64_t"], "int64_t"),
        ("lppo_rt_abs", &["int64_t"], "int64_t"),
        ("lppo_rt_min", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_max", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_pow", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_sqrt", &["int64_t"], "int64_t"),
        ("lppo_rt_str_concat", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_int_to_str", &["int64_t"], "int64_t"),
        ("lppo_rt_str_to_int", &["int64_t"], "int64_t"),
        ("lppo_rt_str_eq", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_str_cmp", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_str_len", &["int64_t"], "int64_t"),
        ("lppo_rt_str_upper", &["int64_t"], "int64_t"),
        ("lppo_rt_str_lower", &["int64_t"], "int64_t"),
        ("lppo_rt_str_trim", &["int64_t"], "int64_t"),
        ("lppo_rt_str_split", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_eval", &["int64_t"], "int64_t"),
        ("lppo_rt_either", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_random", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_json_parse", &["int64_t"], "int64_t"),
        ("lppo_rt_json_stringify", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_system", &["int64_t"], "int64_t"),
        ("lppo_rt_color", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_read_file", &["int64_t"], "int64_t"),
        ("lppo_rt_write_file", &["int64_t", "int64_t"], "int32_t"),
        ("lppo_rt_append_file", &["int64_t", "int64_t"], "int32_t"),
        ("lppo_rt_file_exists", &["int64_t"], "int32_t"),
        ("lppo_rt_delete_file", &["int64_t"], "int32_t"),
        ("lppo_rt_pool_alloc", &["int64_t"], "int64_t"),
        ("lppo_rt_pool_free", &["int64_t"], "int64_t"),
        ("lppo_rt_pool_used", &[], "int64_t"),
        ("lppo_rt_panic", &["int64_t"], "void"),
        ("lppo_rt_volatile_read", &["int64_t"], "int64_t"),
        ("lppo_rt_volatile_write", &["int64_t", "int64_t"], "void"),
        ("lppo_rt_addr_of", &["int64_t"], "int64_t"),
        ("lppo_rt_ptr_read", &["int64_t"], "int64_t"),
        ("lppo_rt_ptr_write", &["int64_t", "int64_t"], "void"),
        ("lppo_rt_ptr_offset", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_inline_asm", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_map_new", &[], "int64_t"),
        ("lppo_rt_map_insert", &["int64_t", "int64_t", "int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_map_get", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_map_get_tag", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_map_len", &["int64_t"], "int64_t"),
        ("lppo_rt_map_remove", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_map_keys", &["int64_t"], "int64_t"),
        ("lppo_rt_map_values", &["int64_t"], "int64_t"),
        ("lppo_rt_tuple_make", &["int64_t"], "int64_t"),
        ("lppo_rt_tuple_get", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_tuple_get_tag", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_enum_make", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_enum_tag", &["int64_t"], "int64_t"),
        ("lppo_rt_enum_variant", &["int64_t"], "int64_t"),
        ("lppo_rt_enum_get", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_enum_get_tag", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_object_set", &["int64_t", "int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_struct_make", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_struct_set", &["int64_t", "int64_t", "int64_t", "int64_t", "int64_t"], "void"),
        ("lppo_rt_struct_get", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_struct_get_tag", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_set_new", &[], "int64_t"),
        ("lppo_rt_set_insert", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_set_contains", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_set_remove", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_set_len", &["int64_t"], "int64_t"),
        ("lppo_rt_set_to_array", &["int64_t"], "int64_t"),
        ("lppo_rt_fn_make", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_fn_index", &["int64_t"], "int64_t"),
        ("lppo_rt_fn_capture", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_splat", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_add", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_sub", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_mul", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_div", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_extract", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_insert", &["int64_t", "int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_shuffle", &["int64_t", "int64_t"], "int64_t"),
        ("lppo_rt_simd_reduce", &["int64_t", "int64_t"], "int64_t"),
    ];

    for (name, param_types, ret_type) in builtins {
        let param_str = if param_types.is_empty() {
            "void".to_string()
        } else {
            param_types.join(", ")
        };
        out.push_str(&format!("extern {} {}({});\n", ret_type, name, param_str));
    }
    out.push_str("\n");
}

/// Return the C type string for a procedure return type.
#[allow(dead_code)]
fn call_return_type(
    name: &str,
    proc_return_types: &HashMap<String, TypeAnnotation>,
) -> &'static str {
    // User procedure return types take priority
    if let Some(ta) = proc_return_types.get(name) {
        match ta {
            TypeAnnotation::Void => "void",
            _ => "int64_t",
        }
    } else {
        // Built-in functions all return i64
        "int64_t"
    }
}

/// Return the C argument types for a built-in function call.
/// All args are int64_t (pointers are ptrtoint'd to i64).
#[allow(dead_code)]
fn call_arg_types(name: &str) -> Vec<&'static str> {
    // All args are int64_t (strings are ptrtoint'd to i64).
    // Pointer args are inttoptr'd in codegen.
    match name {
        "lppo_rt_print_value" => vec!["int64_t", "int64_t"],
        "lppo_rt_print_newline" => vec![],
        "lppo_rt_input" => vec!["int64_t", "int64_t"],
        "lppo_rt_alloc" => vec!["int64_t"],
        "lppo_rt_dealloc" => vec!["int64_t"],
        "lppo_rt_array_make" => vec!["int64_t"],
        "lppo_rt_array_get" => vec!["int64_t", "int64_t"],
        "lppo_rt_array_set" => vec!["int64_t", "int64_t", "int64_t"],
        "lppo_rt_array_push" => vec!["int64_t", "int64_t"],
        "lppo_rt_array_pop" => vec!["int64_t"],
        "lppo_rt_array_len" => vec!["int64_t"],
        "lppo_rt_abs" => vec!["int64_t"],
        "lppo_rt_min" => vec!["int64_t", "int64_t"],
        "lppo_rt_max" => vec!["int64_t", "int64_t"],
        "lppo_rt_pow" => vec!["int64_t", "int64_t"],
        "lppo_rt_sqrt" => vec!["int64_t"],
        "lppo_rt_str_concat" => vec!["int64_t", "int64_t"],
        "lppo_rt_int_to_str" => vec!["int64_t"],
        "lppo_rt_str_to_int" => vec!["int64_t"],
        "lppo_rt_str_len" => vec!["int64_t"],
        "lppo_rt_str_upper" => vec!["int64_t"],
        "lppo_rt_str_lower" => vec!["int64_t"],
        "lppo_rt_str_trim" => vec!["int64_t"],
        "lppo_rt_str_split" => vec!["int64_t", "int64_t"],
        "lppo_rt_eval" => vec!["int64_t"],
        "lppo_rt_either" => vec!["int64_t", "int64_t"],
        "lppo_rt_random" => vec!["int64_t", "int64_t"],
        "lppo_rt_json_parse" => vec!["int64_t"],
        "lppo_rt_json_stringify" => vec!["int64_t", "int64_t"],
        "lppo_rt_system" => vec!["int64_t"],
        "lppo_rt_color" => vec!["int64_t", "int64_t"],
        "lppo_rt_read_file" => vec!["int64_t"],
        "lppo_rt_write_file" => vec!["int64_t", "int64_t"],
        "lppo_rt_append_file" => vec!["int64_t", "int64_t"],
        "lppo_rt_file_exists" => vec!["int64_t"],
        "lppo_rt_delete_file" => vec!["int64_t"],
        "lppo_rt_pool_alloc" => vec!["int64_t"],
        "lppo_rt_pool_free" => vec!["int64_t"],
        "lppo_rt_pool_used" => vec![],
        "lppo_rt_panic" => vec!["int64_t"],
        "lppo_rt_volatile_read" => vec!["int64_t"],
        "lppo_rt_volatile_write" => vec!["int64_t", "int64_t"],
        "lppo_rt_addr_of" => vec!["int64_t"],
        "lppo_rt_ptr_read" => vec!["int64_t"],
        "lppo_rt_ptr_write" => vec!["int64_t", "int64_t"],
        "lppo_rt_ptr_offset" => vec!["int64_t", "int64_t"],
        _ => vec![],
    }
}

/// Map LPPO source-level built-in names to runtime extern names.
fn builtin_func_name(name: &str) -> &str {
    match name {
        "println" | "print" | "eprintln" => "lppo_rt_print_value",
        "abs" => "lppo_rt_abs",
        "min" => "lppo_rt_min",
        "max" => "lppo_rt_max",
        "pow" => "lppo_rt_pow",
        "sqrt" => "lppo_rt_sqrt",
        "str_concat" => "lppo_rt_str_concat",
        "int_to_str" => "lppo_rt_int_to_str",
        "str_to_int" => "lppo_rt_str_to_int",
        "str_len" => "lppo_rt_str_len",
        "str_upper" => "lppo_rt_str_upper",
        "str_lower" => "lppo_rt_str_lower",
        "str_trim" => "lppo_rt_str_trim",
        "str_split" => "lppo_rt_str_split",
        "eval" => "lppo_rt_eval",
        "either" => "lppo_rt_either",
        "random" => "lppo_rt_random",
        "json_parse" => "lppo_rt_json_parse",
        "json_stringify" => "lppo_rt_json_stringify",
        "system" => "lppo_rt_system",
        "color" => "lppo_rt_color",
        "read_file" => "lppo_rt_read_file",
        "write_file" => "lppo_rt_write_file",
        "append_file" => "lppo_rt_append_file",
        "file_exists" => "lppo_rt_file_exists",
        "delete_file" => "lppo_rt_delete_file",
        "pool_alloc" => "lppo_rt_pool_alloc",
        "pool_free" => "lppo_rt_pool_free",
        "pool_used" => "lppo_rt_pool_used",
        "vec_new" => "lppo_rt_array_make",
        "map_new" => "lppo_rt_map_new",
        "map_insert" => "lppo_rt_map_insert",
        "map_get" => "lppo_rt_map_get",
        "map_len" => "lppo_rt_map_len",
        "map_remove" => "lppo_rt_map_remove",
        "map_keys" => "lppo_rt_map_keys",
        "map_values" => "lppo_rt_map_values",
        "set_new" => "lppo_rt_set_new",
        "set_insert" => "lppo_rt_set_insert",
        "set_contains" => "lppo_rt_set_contains",
        "set_remove" => "lppo_rt_set_remove",
        "set_len" => "lppo_rt_set_len",
        "set_to_array" | "set_items" => "lppo_rt_set_to_array",
        "tuple_new" => "lppo_rt_tuple_make",
        "tuple_get" => "lppo_rt_tuple_get",
        "enum_tag" => "lppo_rt_enum_tag",
        "enum_variant" => "lppo_rt_enum_variant",
        "enum_get" => "lppo_rt_enum_get",
        "struct_field" => "lppo_rt_struct_get",
        "vec_push" => "lppo_rt_array_push",
        "vec_pop" => "lppo_rt_array_pop",
        "vec_len" => "lppo_rt_array_len",
        "vec_get" => "lppo_rt_array_get",
        "vec_set" => "lppo_rt_array_set",
        "simd_splat" => "lppo_rt_simd_splat",
        "simd_add" => "lppo_rt_simd_add",
        "simd_sub" => "lppo_rt_simd_sub",
        "simd_mul" => "lppo_rt_simd_mul",
        "simd_div" => "lppo_rt_simd_div",
        "simd_extract" => "lppo_rt_simd_extract",
        "simd_insert" => "lppo_rt_simd_insert",
        "simd_shuffle" => "lppo_rt_simd_shuffle",
        "simd_reduce" => "lppo_rt_simd_reduce",
        "tui_init" => "lppo_rt_tui_init",
        "tui_end" => "lppo_rt_tui_end",
        "tui_clear" => "lppo_rt_tui_clear",
        "tui_move" => "lppo_rt_tui_move",
        "tui_put" => "lppo_rt_tui_put",
        "tui_puts" => "lppo_rt_tui_puts",
        "tui_cls" => "lppo_rt_tui_cls",
        "tui_box" => "lppo_rt_tui_box",
        "tui_title" => "lppo_rt_tui_title",
        "tui_style" => "lppo_rt_tui_style",
        "tui_reset" => "lppo_rt_tui_reset",
        "tui_progress" => "lppo_rt_tui_progress",
        "tui_size" => "lppo_rt_tui_size",
        "tui_fg" => "lppo_rt_tui_fg",
        "tui_bold" => "lppo_rt_tui_bold",
        "tui_underline" => "lppo_rt_tui_underline",
        "tui_hide_cursor" => "lppo_rt_tui_hide_cursor",
        "tui_show_cursor" => "lppo_rt_tui_show_cursor",
        "tui_refresh" => "lppo_rt_tui_refresh",
        "volatile_read" => "lppo_rt_volatile_read",
        "volatile_write" => "lppo_rt_volatile_write",
        "inline_asm" => "lppo_rt_inline_asm",
        "addr_of" => "lppo_rt_addr_of",
        "ptr_read" => "lppo_rt_ptr_read",
        "ptr_write" => "lppo_rt_ptr_write",
        "ptr_offset" => "lppo_rt_ptr_offset",
        _ => name,
    }
}

/// Type tag constants (match `lppo_rt_print_value` ABI).
const TAG_INT: i64 = 0;
const TAG_FLOAT: i64 = 1;
const TAG_BOOL: i64 = 2;
const TAG_STR: i64 = 3;
const TAG_ARRAY: i64 = 4;
const TAG_CHAR: i64 = 5;
const TAG_BYTE: i64 = 6;
const TAG_TUPLE: i64 = 7;
const TAG_MAP: i64 = 8;
const TAG_SET: i64 = 9;
const TAG_STRUCT: i64 = 10;
const TAG_ENUM: i64 = 11;
const TAG_FN: i64 = 12;
const TAG_JSON: i64 = 13;
const TAG_DYN: i64 = 14;
