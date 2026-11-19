//! Procedure body -> C function codegen.

use crate::ast::TypeAnnotation;
use crate::codegen_c::stmt::{c_ident, codegen_operation, Ctx};
use crate::runtime::Operation;
use std::collections::{HashMap, HashSet};

/// Compile a `ProcedureBody` into a C function definition.
pub fn compile_procedure_ir(
    name: &str,
    body: &crate::runtime::ProcedureBody,
    string_indices: &HashMap<String, usize>,
    proc_return_types: &HashMap<String, TypeAnnotation>,
) -> Result<String, String> {
    let ret_type = match body.return_type {
        Some(TypeAnnotation::Void) | None => "void",
        _ => "int64_t",
    };

    let mut out = String::new();

    // Signature
    let param_str = if body.params.is_empty() {
        "void".to_string()
    } else {
        body.params
            .iter()
            .map(|p| format!("int64_t {}", c_ident(p)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str(&format!("static {} {}({}) {{\n", ret_type, name, param_str));

    // Declare every local variable up-front so block scoping never hides one.
    let mut locals: HashMap<String, String> = HashMap::new();
    for p in &body.params {
        locals.insert(p.clone(), c_ident(p));
    }
    let mut declared: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for p in &body.params {
        seen.insert(p.clone());
    }

    let mut extra: Vec<String> = Vec::new();
    collect_locals(&body.operations, &mut extra);
    for v in extra {
        if locals.contains_key(&v) || !seen.insert(v.clone()) {
            continue;
        }
        locals.insert(v.clone(), c_ident(&v));
        declared.push(c_ident(&v));
    }
    for d in &declared {
        out.push_str(&format!("  int64_t {} = 0;\n", d));
    }

    let mut ctx = Ctx {
        locals,
        var_tags: HashMap::new(),
        label_counter: 0,
        string_indices,
        proc_return_types,
        return_type: ret_type,
        str_arrays: HashMap::new(),
    };

    for op in &body.operations {
        codegen_operation(op, &mut ctx, &mut out)?;
    }

    if ret_type != "void" {
        out.push_str("  return 0;\n");
    }
    out.push_str("}\n");
    Ok(out)
}

/// Collect every name that receives a binding somewhere in the body.
fn collect_locals(ops: &[Operation], out: &mut Vec<String>) {
    for op in ops {
        match op {
            Operation::Let(name, _) => out.push(name.clone()),
            Operation::Input { var, .. } => out.push(var.clone()),
            Operation::Alloc { var, .. } => out.push(var.clone()),
            Operation::For { var, ops, .. } => {
                out.push(var.clone());
                collect_locals(ops, out);
            }
            Operation::If {
                then_ops,
                elifs,
                else_ops,
                ..
            } => {
                collect_locals(then_ops, out);
                for (_, eops) in elifs {
                    collect_locals(eops, out);
                }
                if let Some(eops) = else_ops {
                    collect_locals(eops, out);
                }
            }
            Operation::While { ops, .. }
            | Operation::Loop { ops }
            | Operation::Scope { ops }
            | Operation::Unsafe { ops }
            | Operation::Spawn { ops } => collect_locals(ops, out),
            Operation::Pause { assign_to, .. } => {
                if let Some(n) = assign_to {
                    out.push(n.clone());
                }
            }
            _ => {}
        }
    }
}
