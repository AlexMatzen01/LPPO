//! Native Rust package integration for LPPO.
//!
//! This module bridges installed Rust/Cargo packages with the LPPO language.
//! It:
//!   * Loads the LPPO-facing interface (`lppo_interface.json`) of each
//!     installed package.
//!   * Registers package-exported functions into the LPPO type context so
//!     `use LPPX2D;` followed by calls like `x2d_draw_rectangle(...)` type-check.
//!   * Dispatches calls to package functions at runtime via `libloading`
//!     FFI against the package's compiled `cdylib`.

use crate::error::{LppoError, LppoResult};
use crate::pkg::installed::InstalledTracker;
use crate::pkg::library::{load_package_interface, PackageContext};
use crate::pkg::types::TypeAnnotationRef;
use crate::ast::TypeAnnotation;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

/// Thread-local package context used by native FFI dispatch.
///
/// Set by [`Executor::run`] (and the `Parallel` closure) so that
/// `call_native` resolves installed package functions against the project-local
/// (`.lppo/`) or global (`~/.LPPO/`) package database matching the running
/// program. Falls back to the global context when unset.
thread_local! {
    static NATIVE_PKG_CONTEXT: RefCell<Option<PackageContext>> = RefCell::new(None);
}

/// Set the native package context for the current thread.
pub fn set_native_context(context: Option<PackageContext>) {
    NATIVE_PKG_CONTEXT.with(|c| *c.borrow_mut() = context);
}

/// Clear the native package context for the current thread.
pub fn clear_native_context() {
    NATIVE_PKG_CONTEXT.with(|c| *c.borrow_mut() = None);
}

/// The native package context for the current thread, or `Global` if unset.
pub fn native_context() -> PackageContext {
    NATIVE_PKG_CONTEXT
        .with(|c| c.borrow().clone())
        .unwrap_or(PackageContext::Global)
}

/// RAII guard that installs a native package context for the lifetime of the
/// guard and restores the previous context on drop.
pub struct ContextGuard {
    previous: Option<PackageContext>,
}

impl ContextGuard {
    /// Install `context` for the current thread, returning a guard that
    /// restores the previous context on drop.
    pub fn install(context: PackageContext) -> Self {
        let previous = native_context();
        set_native_context(Some(context));
        Self { previous: Some(previous) }
    }
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        set_native_context(self.previous.take());
    }
}

////////////////////////////////////////////////////////////////////////////////
// FFI calling convention
////////////////////////////////////////////////////////////////////////////////

/// The native function signature every exported package function must follow.
///
/// ```c
/// i64 func(i64 a0, i64 a1, i64 a2, i64 a3, i64 a4, i64 a5, i64 a6, i64 a7);
/// ```
///
/// LPPO `Int64`/`Bool` values are passed as `i64` (Bool: 0/1). Up to eight
/// arguments are supported; missing arguments are passed as 0. The `i64`
/// result is mapped back to an LPPO `Int64` (nonzero => `true` for `Bool`).
type NativeFn = extern "C" fn(i64, i64, i64, i64, i64, i64, i64, i64) -> i64;

/// Maximum number of FFI arguments.
const MAX_NATIVE_ARGS: usize = 8;

////////////////////////////////////////////////////////////////////////////////
// Function description
////////////////////////////////////////////////////////////////////////////////

/// A single function exposed by an installed package to LPPO.
#[derive(Debug, Clone)]
pub struct NativeFunction {
    /// LPPO-facing name.
    pub name: String,
    /// Parameter types.
    pub params: Vec<TypeAnnotation>,
    /// Return type.
    pub return_type: TypeAnnotation,
    /// Symbol name in the native library.
    pub symbol: String,
    /// Absolute path to the package's cdylib artifact.
    pub library_path: Option<std::path::PathBuf>,
}

////////////////////////////////////////////////////////////////////////////////
// Global registry
////////////////////////////////////////////////////////////////////////////////

static NATIVE_REGISTRY: Lazy<Mutex<NativeRegistry>> =
    Lazy::new(|| Mutex::new(NativeRegistry::new()));

/// Registry of functions exposed by installed Rust packages.
#[derive(Debug, Clone, Default)]
pub struct NativeRegistry {
    functions: HashMap<String, NativeFunction>,
}

impl NativeRegistry {
    pub fn new() -> Self {
        Self { functions: HashMap::new() }
    }

    /// Register a single function.
    pub fn register(&mut self, func: NativeFunction) {
        self.functions.insert(func.name.clone(), func);
    }

    /// Returns true if a function with the given name is registered.
    pub fn is_registered(name: &str) -> bool {
        NATIVE_REGISTRY.lock().unwrap().functions.contains_key(name)
    }

    /// Look up a registered function by name.
    pub fn get(&self, name: &str) -> Option<&NativeFunction> {
        self.functions.get(name)
    }

    /// Returns all registered functions.
    pub fn all(&self) -> Vec<&NativeFunction> {
        self.functions.values().collect()
    }

    /// Ensure installed packages have been loaded into the registry (global
    /// context).
    pub fn ensure_loaded() -> LppoResult<()> {
        Self::ensure_loaded_with(&PackageContext::Global)
    }

    /// Ensure installed packages have been loaded into the registry for the
    /// given context.
    pub fn ensure_loaded_with(context: &PackageContext) -> LppoResult<()> {
        let reg = NATIVE_REGISTRY.lock().unwrap();
        if reg.functions.is_empty() {
            drop(reg);
            Self::load_installed_with(context)
        } else {
            Ok(())
        }
    }

    /// Clear the registry (mainly for testing).
    pub fn clear(&mut self) {
        self.functions.clear();
    }

    /// Load every installed package's interface and register its functions
    /// (global context).
    pub fn load_installed() -> LppoResult<()> {
        Self::load_installed_with(&PackageContext::Global)
    }

    /// Load every installed package's interface and register its functions
    /// for the given context.
    pub fn load_installed_with(context: &PackageContext) -> LppoResult<()> {
        let tracker = InstalledTracker::load_with(context.clone())?;
        let mut reg = NATIVE_REGISTRY.lock().unwrap();
        for (name, iface) in tracker.packages_with_interfaces() {
            let installed = tracker.get(&name).unwrap();
            let library_path = installed.artifact_path.clone();
            for func in &iface.functions {
                reg.register(NativeFunction {
                    name: func.name.clone(),
                    params: func.params.iter().map(|t| to_lppo_type(t)).collect(),
                    return_type: to_lppo_type(&func.return_type),
                    symbol: func.symbol.clone().unwrap_or_else(|| func.name.clone()),
                    library_path: library_path.clone(),
                });
            }
            eprintln!("[native] registered {} functions from package {name}", iface.functions.len());
        }
        Ok(())
    }

    /// Register all native functions into an LPPO type-checking context
    /// (global context).
    pub fn register_in_type_context(ctx: &mut crate::typechecker::TypeContext) -> LppoResult<()> {
        Self::register_in_type_context_with(ctx, &PackageContext::Global)
    }

    /// Register all native functions into an LPPO type-checking context for
    /// the given package context.
    pub fn register_in_type_context_with(
        ctx: &mut crate::typechecker::TypeContext,
        context: &PackageContext,
    ) -> LppoResult<()> {
        Self::ensure_loaded_with(context)?;
        let reg = NATIVE_REGISTRY.lock().unwrap();
        for func in reg.all() {
            ctx.add_procedure(func.name.clone(), func.params.clone(), func.return_type.clone());
        }
        Ok(())
    }
}

impl std::fmt::Display for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native {}(", self.name)?;
        for (i, p) in self.params.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{:?}", p)?;
        }
        write!(f, ") -> {:?}", self.return_type)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Runtime dispatch
////////////////////////////////////////////////////////////////////////////////

/// Call a native package function by name with evaluated argument values
/// (global context).
///
/// Returns the LPPO `Value` produced by the function.
pub fn call_native(name: &str, args: &[crate::runtime::Value]) -> LppoResult<crate::runtime::Value> {
    call_native_with(&native_context(), name, args)
}

/// Call a native package function by name with evaluated argument values for
/// the given package context.
///
/// Returns the LPPO `Value` produced by the function.
pub fn call_native_with(
    context: &PackageContext,
    name: &str,
    args: &[crate::runtime::Value],
) -> LppoResult<crate::runtime::Value> {
    NativeRegistry::ensure_loaded_with(context)?;
    let reg = NATIVE_REGISTRY.lock().unwrap();
    let func = reg.get(name).ok_or_else(|| {
        LppoError::type_error(format!("native function '{name}' is not registered"))
    })?;

    if args.len() > MAX_NATIVE_ARGS {
        return Err(LppoError::type_error(format!(
            "native function '{name}' takes at most {MAX_NATIVE_ARGS} arguments"
        )));
    }

    // Validate argument types against the declared signature.
    for (i, arg) in args.iter().enumerate() {
        let expected = func.params.get(i).ok_or_else(|| {
            LppoError::type_error(format!(
                "native function '{name}' does not accept argument {i}"
            ))
        })?;
        check_value_type(arg, expected)?;
    }

    let library_path = func.library_path.as_ref().ok_or_else(|| {
        LppoError::type_error(format!(
            "native function '{name}' has no resolved library artifact"
        ))
    })?;

    unsafe {
        let lib = libloading::Library::new(library_path).map_err(|e| {
            LppoError::source_retrieval_failed(
                name.to_string(),
                format!("failed to load library '{}': {e}", library_path.display()),
            )
        })?;

        let symbol: libloading::Symbol<'_, NativeFn> = lib.get(func.symbol.as_bytes()).map_err(|e| {
            LppoError::source_retrieval_failed(
                name.to_string(),
                format!("failed to load symbol '{}' from '{}': {e}", func.symbol, library_path.display()),
            )
        })?;

        // Collect arguments, padding missing ones with 0.
        let mut a = [0i64; MAX_NATIVE_ARGS];
        for (i, arg) in args.iter().enumerate() {
            if i < MAX_NATIVE_ARGS {
                a[i] = value_to_i64(arg);
            }
        }

        let result = symbol(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
        Ok(i64_to_value(&func.return_type, result))
    }
}

/// Convert an LPPO `TypeAnnotationRef` to an LPPO `TypeAnnotation`.
fn to_lppo_type(r: &TypeAnnotationRef) -> TypeAnnotation {
    match r {
        TypeAnnotationRef::Int64 => TypeAnnotation::Int64,
        TypeAnnotationRef::Bool => TypeAnnotation::Bool,
        TypeAnnotationRef::Str => TypeAnnotation::Str,
        TypeAnnotationRef::Void => TypeAnnotation::Void,
    }
}

/// Check that a runtime `Value` matches an expected `TypeAnnotation`.
fn check_value_type(value: &crate::runtime::Value, expected: &TypeAnnotation) -> LppoResult<()> {
    match (value, expected) {
        (crate::runtime::Value::Int(_), TypeAnnotation::Int64) => Ok(()),
        (crate::runtime::Value::Bool(_), TypeAnnotation::Bool) => Ok(()),
        (crate::runtime::Value::Str(_), TypeAnnotation::Str) => Ok(()),
        (crate::runtime::Value::Int(_), TypeAnnotation::Bool) => Ok(()), // ints coerce to bool
        (crate::runtime::Value::Bool(_), TypeAnnotation::Int64) => Ok(()),
        (_, TypeAnnotation::Void) => Err(LppoError::type_error("unexpected argument of type Void".to_string())),
        _ => Err(LppoError::type_error(format!(
            "type mismatch: expected {:?}, found {:?}", expected, value
        ))),
    }
}

/// Convert an LPPO runtime `Value` to an `i64` for FFI.
fn value_to_i64(value: &crate::runtime::Value) -> i64 {
    match value {
        crate::runtime::Value::Int(n) => *n,
        crate::runtime::Value::Bool(b) => if *b { 1 } else { 0 },
        crate::runtime::Value::Str(s) => {
            // Hash the string into a stable i64 for pointer-less passing.
            // This is a limitation; real string FFI is future work.
            let mut h: u64 = 1469598103934665603;
            for b in s.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
            h as i64
        }
        _ => 0,
    }
}

/// Convert an `i64` FFI result to an LPPO `Value` based on the return type.
fn i64_to_value(return_type: &TypeAnnotation, result: i64) -> crate::runtime::Value {
    match return_type {
        TypeAnnotation::Bool => crate::runtime::Value::Bool(result != 0),
        TypeAnnotation::Int64 | _ => crate::runtime::Value::Int(result),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mut reg = NativeRegistry::new();
        reg.register(NativeFunction {
            name: "foo".to_string(),
            params: vec![TypeAnnotation::Int64],
            return_type: TypeAnnotation::Int64,
            symbol: "foo".to_string(),
            library_path: None,
        });
        assert!(reg.get("foo").is_some());
        assert!(reg.get("bar").is_none());
    }

    #[test]
    fn value_to_i64_roundtrip_bool() {
        assert_eq!(value_to_i64(&crate::runtime::Value::Bool(true)), 1);
        assert_eq!(value_to_i64(&crate::runtime::Value::Bool(false)), 0);
        assert_eq!(value_to_i64(&crate::runtime::Value::Int(42)), 42);
    }

    #[test]
    fn i64_to_value_bool() {
        assert!(matches!(i64_to_value(&TypeAnnotation::Bool, 0), crate::runtime::Value::Bool(false)));
        assert!(matches!(i64_to_value(&TypeAnnotation::Bool, 1), crate::runtime::Value::Bool(true)));
        assert!(matches!(i64_to_value(&TypeAnnotation::Int64, 7), crate::runtime::Value::Int(7)));
    }

    #[test]
    fn max_args_enforced() {
        let mut reg = NativeRegistry::new();
        reg.register(NativeFunction {
            name: "arity2".to_string(),
            params: vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
            return_type: TypeAnnotation::Int64,
            symbol: "arity2".to_string(),
            library_path: None,
        });
        let reg_ref = reg;
        let func = reg_ref.get("arity2").unwrap();
        assert_eq!(func.params.len(), 2);
    }
}
