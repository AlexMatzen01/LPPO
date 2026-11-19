/// Type checking for LPPO programs
/// Validates types across expressions, statements, and procedure calls
use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryOp, EnumDef, Expr, Procedure, Program, QualifiedName, Statement, StructDef,
    TypeAnnotation, UnaryOp,
};
use crate::error::{LppoError, LppoResult};
use crate::pkg::library::PackageContext;
use crate::pkg::native::NativeRegistry;

use crate::ast::OwnershipKind;

/// Ownership state for tracking moves, borrows, and drops
#[derive(Debug, Clone, PartialEq)]
enum OwnershipState {
    Owned,
    Moved,
    Borrowed {
        mutable: bool,
    },
    Dropped,
}

/// Kind of borrow (shared/immutable or exclusive/mutable)
#[derive(Debug, Clone, PartialEq)]
enum BorrowKind {
    Imm,
    Mut,
}

/// Information about an active borrow
#[derive(Debug, Clone)]
struct BorrowInfo {
    kind: BorrowKind,
    scope_level: u32,
}

/// Type context for tracking variable and procedure types
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// Variables in current scope with their types
    variables: HashMap<String, TypeAnnotation>,
    /// Ownership state of variables
    ownership: HashMap<String, (OwnershipKind, OwnershipState)>,
    /// Active borrows: variable -> list of borrows
    borrows: HashMap<String, Vec<BorrowInfo>>,
    /// Procedure signatures: name -> (param_types, return_type)
    procedures: HashMap<String, (Vec<TypeAnnotation>, TypeAnnotation)>,
    /// Struct definitions
    structs: HashMap<String, StructDef>,
    /// Enum definitions
    enums: HashMap<String, EnumDef>,
    /// Parent scope for nested contexts
    parent: Option<Box<TypeContext>>,
    /// Current scope nesting level (for borrow lifetime tracking)
    scope_level: u32,
    /// Variables allocated on the heap (need explicit deallocation)
    heap_allocated: HashSet<String>,
    /// Whether we are inside an unsafe block (bypasses borrow checker)
    in_unsafe: bool,
    /// Variables declared at each scope level, for cleanup on scope exit
    scope_vars: HashMap<u32, Vec<String>>,
}

impl TypeContext {
    pub fn new() -> Self {
        TypeContext {
            variables: HashMap::new(),
            ownership: HashMap::new(),
            borrows: HashMap::new(),
            procedures: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            parent: None,
            scope_level: 0,
            heap_allocated: HashSet::new(),
            in_unsafe: false,
            scope_vars: HashMap::new(),
        }
    }

    /// Create a child scope
    pub fn child(&self) -> Self {
        TypeContext {
            variables: HashMap::new(),
            ownership: HashMap::new(),
            borrows: HashMap::new(),
            procedures: self.procedures.clone(),
            structs: self.structs.clone(),
            enums: self.enums.clone(),
            parent: Some(Box::new(self.clone())),
            scope_level: self.scope_level + 1,
            heap_allocated: self.heap_allocated.clone(),
            in_unsafe: self.in_unsafe,
            scope_vars: HashMap::new(),
        }
    }

    /// Enter a new scope level (increment scope counter)
    pub fn enter_scope(&mut self) {
        self.scope_level += 1;
    }

    /// Exit the current scope: end all borrows at this level and drop heap-allocated vars
    pub fn exit_scope(&mut self) {
        // End all borrows at the current scope level
        for borrows_list in self.borrows.values_mut() {
            borrows_list.retain(|b| b.scope_level < self.scope_level);
        }
        self.borrows.retain(|_, v| !v.is_empty());

        // Drop variables declared at this scope level
        if let Some(vars) = self.scope_vars.remove(&self.scope_level) {
            for var in &vars {
                let _ = self.mark_dropped(var);
                self.heap_allocated.remove(var);
                self.variables.remove(var);
                self.ownership.remove(var);
            }
        }

        self.scope_level -= 1;
    }

    /// Mark a variable as heap-allocated (needs explicit deallocation at drop)
    pub fn mark_heap_allocated(&mut self, name: String) {
        self.heap_allocated.insert(name);
    }

    /// Check if a variable is heap-allocated
    pub fn is_heap_allocated(&self, name: &str) -> bool {
        self.heap_allocated.contains(name)
            || self
                .parent
                .as_ref()
                .map_or(false, |p| p.is_heap_allocated(name))
    }

    /// Add a variable to current scope with ownership tracking
    pub fn add_variable(&mut self, name: String, typ: TypeAnnotation) {
        self.add_variable_with_ownership(name, typ, OwnershipKind::Own);
    }

    /// Add a variable with explicit ownership
    pub fn add_variable_with_ownership(
        &mut self,
        name: String,
        typ: TypeAnnotation,
        ownership_kind: OwnershipKind,
    ) {
        self.variables.insert(name.clone(), typ);
        self.ownership
            .insert(name.clone(), (ownership_kind, OwnershipState::Owned));
        self.scope_vars
            .entry(self.scope_level)
            .or_default()
            .push(name);
    }

    /// Look up a variable type
    pub fn get_variable(&self, name: &str) -> Option<&TypeAnnotation> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_variable(name)))
    }

    /// Check if a variable has been moved
    pub fn is_moved(&self, name: &str) -> bool {
        if let Some((_, state)) = self.ownership.get(name) {
            matches!(state, OwnershipState::Moved | OwnershipState::Dropped)
        } else {
            self.parent.as_ref().map_or(false, |p| p.is_moved(name))
        }
    }

    /// Mark a variable as moved
    pub fn mark_moved(&mut self, name: &str) -> LppoResult<()> {
        if let Some((kind, state)) = self.ownership.get_mut(name) {
            if matches!(state, OwnershipState::Moved | OwnershipState::Dropped) {
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

    /// Mark a variable as dropped (freed at end of scope)
    pub fn mark_dropped(&mut self, name: &str) -> LppoResult<()> {
        if let Some((_, state)) = self.ownership.get_mut(name) {
            *state = OwnershipState::Dropped;
            Ok(())
        } else {
            Err(LppoError::UnknownVariable {
                name: name.to_string(),
                location: None,
                suggestion: None,
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

    /// Check if a variable can be read (not mutably borrowed, not moved)
    pub fn can_read(&self, name: &str) -> LppoResult<()> {
        if self.in_unsafe {
            return Ok(());
        }
        self.check_variable_access(name)?;
        if let Some(borrows) = self.borrows.get(name) {
            if borrows.iter().any(|b| matches!(b.kind, BorrowKind::Mut)) {
                return Err(LppoError::BorrowError {
                    message: format!(
                        "cannot read '{}' while it is mutably borrowed",
                        name
                    ),
                    location: None,
                });
            }
        }
        Ok(())
    }

    /// Check if a variable can be written (not borrowed at all, not moved)
    pub fn can_write(&self, name: &str) -> LppoResult<()> {
        if self.in_unsafe {
            return Ok(());
        }
        self.check_variable_access(name)?;
        if let Some(borrows) = self.borrows.get(name) {
            if !borrows.is_empty() {
                return Err(LppoError::BorrowError {
                    message: format!(
                        "cannot assign to '{}' while it is borrowed",
                        name
                    ),
                    location: None,
                });
            }
        }
        Ok(())
    }

    /// Start a borrow of a variable. Enforces Rust's aliasing rules:
    /// - At any time, either ONE mutable borrow OR any number of immutable borrows, not both.
    pub fn borrow_variable(&mut self, name: &str, kind: BorrowKind) -> LppoResult<()> {
        if self.in_unsafe {
            return Ok(());
        }
        self.check_variable_access(name)?;

        let borrows = self.borrows.entry(name.to_string()).or_default();

        match kind {
            BorrowKind::Mut => {
                // Can't mutably borrow if any borrows exist (imm or mut)
                if !borrows.is_empty() {
                    return Err(LppoError::BorrowError {
                        message: format!(
                            "cannot borrow '{}' mutably as it is already borrowed",
                            name
                        ),
                        location: None,
                    });
                }
                borrows.push(BorrowInfo {
                    kind: BorrowKind::Mut,
                    scope_level: self.scope_level,
                });
            }
            BorrowKind::Imm => {
                // Can't immutably borrow if a mutable borrow exists
                if borrows.iter().any(|b| matches!(b.kind, BorrowKind::Mut)) {
                    return Err(LppoError::BorrowError {
                        message: format!(
                            "cannot borrow '{}' immutably as it is mutably borrowed",
                            name
                        ),
                        location: None,
                    });
                }
                borrows.push(BorrowInfo {
                    kind: BorrowKind::Imm,
                    scope_level: self.scope_level,
                });
            }
        }
        Ok(())
    }

    /// End a borrow of a variable (when a reference goes out of scope)
    pub fn end_borrow(&mut self, name: &str) -> LppoResult<()> {
        if self.in_unsafe {
            return Ok(());
        }
        if let Some(borrows) = self.borrows.get_mut(name) {
            borrows.pop();
            if borrows.is_empty() {
                self.borrows.remove(name);
            }
        }
        Ok(())
    }

    /// Set unsafe mode on or off
    pub fn set_unsafe(&mut self, unsafe_mode: bool) {
        self.in_unsafe = unsafe_mode;
    }

    /// Add a procedure signature
    pub fn add_procedure(
        &mut self,
        name: String,
        params: Vec<TypeAnnotation>,
        return_type: TypeAnnotation,
    ) {
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
/// Type-check a program against the global package context.
pub fn typecheck_program(program: &Program) -> LppoResult<()> {
    typecheck_program_with_context(program, &PackageContext::Global)
}

/// Type-check a program, resolving installed Rust package functions against
/// the given package context (global `~/.LPPO/` or project-local
/// `<project>/.lppo/`).
pub fn typecheck_program_with_context(program: &Program, context: &PackageContext) -> LppoResult<()> {
    let mut ctx = TypeContext::new();

    // Register built-in functions
    // I/O functions
    ctx.add_procedure(
        "println".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "print".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "eprintln".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );

    // File I/O functions
    ctx.add_procedure(
        "read_file".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "write_file".to_string(),
        vec![TypeAnnotation::Str, TypeAnnotation::Str],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "append_file".to_string(),
        vec![TypeAnnotation::Str, TypeAnnotation::Str],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "file_exists".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "delete_file".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Bool,
    );

    // String utility functions
    ctx.add_procedure(
        "str_concat".to_string(),
        vec![TypeAnnotation::Str, TypeAnnotation::Str],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "int_to_str".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "str_to_int".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "str_len".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "str_upper".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "str_lower".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "str_trim".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "str_split".to_string(),
        vec![TypeAnnotation::Str, TypeAnnotation::Str],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Str)),
    );

    // Math utility functions
    ctx.add_procedure(
        "abs".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "min".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "max".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "pow".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );

    // Buffer functions
    ctx.add_procedure(
        "buf_new".to_string(),
        vec![],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "buf_set".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Str],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "buf_get".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "buf_append".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Str],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "buf_len".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "buf_clear".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "buf_byte".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );

    // Random numbers
    ctx.add_procedure(
        "random".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );

    // Math evaluation / helpers
    ctx.add_procedure(
        "eval".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "sqrt".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "either".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );

    // System commands
    ctx.add_procedure(
        "system".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );

    // JSON support
    ctx.add_procedure(
        "json_parse".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "json_stringify".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Str,
    );

    // Collection (Vec / HashMap) built-ins. Vec is backed by Arr<T>; Map<K,V> by Object.
    // Element types are shown as Int64/Str for type-checking; the runtime is dynamic.
    ctx.add_procedure(
        "vec_new".to_string(),
        vec![],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );
    ctx.add_procedure(
        "vec_push".to_string(),
        vec![
            TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );
    ctx.add_procedure(
        "vec_pop".to_string(),
        vec![TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64))],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "vec_get".to_string(),
        vec![
            TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "vec_set".to_string(),
        vec![
            TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );
    ctx.add_procedure(
        "vec_len".to_string(),
        vec![TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64))],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "map_new".to_string(),
        vec![],
        TypeAnnotation::Map(
            Box::new(TypeAnnotation::Str),
            Box::new(TypeAnnotation::Int64),
        ),
    );
    ctx.add_procedure(
        "map_insert".to_string(),
        vec![
            TypeAnnotation::Map(
                Box::new(TypeAnnotation::Str),
                Box::new(TypeAnnotation::Int64),
            ),
            TypeAnnotation::Str,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Map(
            Box::new(TypeAnnotation::Str),
            Box::new(TypeAnnotation::Int64),
        ),
    );
    ctx.add_procedure(
        "map_get".to_string(),
        vec![
            TypeAnnotation::Map(
                Box::new(TypeAnnotation::Str),
                Box::new(TypeAnnotation::Int64),
            ),
            TypeAnnotation::Str,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "map_remove".to_string(),
        vec![
            TypeAnnotation::Map(
                Box::new(TypeAnnotation::Str),
                Box::new(TypeAnnotation::Int64),
            ),
            TypeAnnotation::Str,
        ],
        TypeAnnotation::Map(
            Box::new(TypeAnnotation::Str),
            Box::new(TypeAnnotation::Int64),
        ),
    );
    ctx.add_procedure(
        "map_len".to_string(),
        vec![TypeAnnotation::Map(
            Box::new(TypeAnnotation::Str),
            Box::new(TypeAnnotation::Int64),
        )],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "map_keys".to_string(),
        vec![TypeAnnotation::Map(
            Box::new(TypeAnnotation::Str),
            Box::new(TypeAnnotation::Int64),
        )],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Str)),
    );

    ctx.add_procedure(
        "map_values".to_string(),
        vec![TypeAnnotation::Map(
            Box::new(TypeAnnotation::Str),
            Box::new(TypeAnnotation::Int64),
        )],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );
    // Sets (insertion-ordered, scalar elements).
    ctx.add_procedure(
        "set_new".to_string(),
        vec![],
        TypeAnnotation::Set(Box::new(TypeAnnotation::Int64)),
    );
    ctx.add_procedure(
        "set_insert".to_string(),
        vec![
            TypeAnnotation::Set(Box::new(TypeAnnotation::Int64)),
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Set(Box::new(TypeAnnotation::Int64)),
    );
    ctx.add_procedure(
        "set_contains".to_string(),
        vec![
            TypeAnnotation::Set(Box::new(TypeAnnotation::Int64)),
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Bool,
    );
    ctx.add_procedure(
        "set_remove".to_string(),
        vec![
            TypeAnnotation::Set(Box::new(TypeAnnotation::Int64)),
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Set(Box::new(TypeAnnotation::Int64)),
    );
    ctx.add_procedure(
        "set_len".to_string(),
        vec![TypeAnnotation::Set(Box::new(TypeAnnotation::Int64))],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "set_to_array".to_string(),
        vec![TypeAnnotation::Set(Box::new(TypeAnnotation::Int64))],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );
    // Tuple / enum / struct accessors.
    ctx.add_procedure("tuple_new".to_string(), vec![], TypeAnnotation::Tuple(vec![]));
    ctx.add_procedure(
        "tuple_get".to_string(),
        vec![TypeAnnotation::Tuple(vec![]), TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "enum_tag".to_string(),
        vec![TypeAnnotation::Custom("Enum".to_string())],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "enum_variant".to_string(),
        vec![TypeAnnotation::Custom("Enum".to_string())],
        TypeAnnotation::Str,
    );
    ctx.add_procedure(
        "enum_get".to_string(),
        vec![TypeAnnotation::Custom("Enum".to_string()), TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "struct_field".to_string(),
        vec![
            TypeAnnotation::Custom("Struct".to_string()),
            TypeAnnotation::Str,
        ],
        TypeAnnotation::Int64,
    );

    // Memory pool built-ins.
    ctx.add_procedure(
        "pool_alloc".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "pool_free".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Void,
    );
    ctx.add_procedure("pool_used".to_string(), vec![], TypeAnnotation::Int64);

    // SIMD built-ins (operate on arrays of integers).
    let int_arr = TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64));
    ctx.add_procedure(
        "simd_add".to_string(),
        vec![int_arr.clone(), int_arr.clone()],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_sub".to_string(),
        vec![int_arr.clone(), int_arr.clone()],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_mul".to_string(),
        vec![int_arr.clone(), int_arr.clone()],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_div".to_string(),
        vec![int_arr.clone(), int_arr.clone()],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_splat".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_extract".to_string(),
        vec![int_arr.clone(), TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "simd_insert".to_string(),
        vec![int_arr.clone(), TypeAnnotation::Int64, TypeAnnotation::Int64],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_shuffle".to_string(),
        vec![int_arr.clone(), int_arr.clone()],
        int_arr.clone(),
    );
    ctx.add_procedure(
        "simd_reduce".to_string(),
        vec![int_arr.clone(), TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );

    // Direct memory addressing and pointers.
    ctx.add_procedure("addr_of".to_string(), vec![TypeAnnotation::Str], TypeAnnotation::Int64);
    ctx.add_procedure(
        "ptr_read".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "ptr_write".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Void,
    );
    ctx.add_procedure(
        "ptr_offset".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );

    // Volatile memory operations.
    ctx.add_procedure(
        "volatile_read".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "volatile_write".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Void,
    );

    // Inline assembly.
    ctx.add_procedure(
        "inline_asm".to_string(),
        vec![TypeAnnotation::Str, int_arr.clone()],
        TypeAnnotation::Int64,
    );

    // ANSI colors
    ctx.add_procedure(
        "color".to_string(),
        vec![TypeAnnotation::Str, TypeAnnotation::Str],
        TypeAnnotation::Str,
    );

    // Terminal user interface (TUI) built-ins.
    ctx.add_procedure("tui_init".to_string(), vec![], TypeAnnotation::Int64);
    ctx.add_procedure("tui_end".to_string(), vec![], TypeAnnotation::Int64);
    ctx.add_procedure("tui_clear".to_string(), vec![], TypeAnnotation::Int64);
    ctx.add_procedure(
        "tui_move".to_string(),
        vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_put".to_string(),
        vec![
            TypeAnnotation::Str,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_puts".to_string(),
        vec![
            TypeAnnotation::Str,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Str,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_cls".to_string(),
        vec![
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_box".to_string(),
        vec![
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Str,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_label".to_string(),
        vec![
            TypeAnnotation::Str,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_title".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_fg".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_bg".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_bold".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_underline".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_reverse".to_string(),
        vec![TypeAnnotation::Int64],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_style".to_string(),
        vec![TypeAnnotation::Str],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure("tui_reset".to_string(), vec![], TypeAnnotation::Int64);
    ctx.add_procedure("tui_hide_cursor".to_string(), vec![], TypeAnnotation::Int64);
    ctx.add_procedure("tui_show_cursor".to_string(), vec![], TypeAnnotation::Int64);
    ctx.add_procedure("tui_refresh".to_string(), vec![], TypeAnnotation::Int64);
    // Shading language: run a bundled `.x2d` shader through the CPU backend.
    ctx.add_procedure("x2d_run".to_string(), vec![TypeAnnotation::Str], TypeAnnotation::Void);
    ctx.add_procedure(
        "tui_input".to_string(),
        vec![
            TypeAnnotation::Str,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Str,
    );
    ctx.add_procedure("tui_getch".to_string(), vec![], TypeAnnotation::Str);
    ctx.add_procedure("tui_poll".to_string(), vec![], TypeAnnotation::Str);
    ctx.add_procedure(
        "tui_progress".to_string(),
        vec![
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
            TypeAnnotation::Int64,
        ],
        TypeAnnotation::Int64,
    );
    ctx.add_procedure(
        "tui_size".to_string(),
        vec![],
        TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
    );

    // Ring-0 (kernel privilege) built-ins. Only available when the source file
    // declares `ring 0;` as its first directive. These interact with ring-0
    // resources (port I/O, MSRs, CPUID, CLI/STI, HLT) and are rejected by the
    // type checker unless `ring 0` is declared.
    if program.ring0 {
        // Port I/O
        ctx.add_procedure(
            "outb".to_string(),
            vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
            TypeAnnotation::Void,
        );
        ctx.add_procedure(
            "outw".to_string(),
            vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
            TypeAnnotation::Void,
        );
        ctx.add_procedure(
            "outl".to_string(),
            vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
            TypeAnnotation::Void,
        );
        ctx.add_procedure(
            "inb".to_string(),
            vec![TypeAnnotation::Int64],
            TypeAnnotation::Int64,
        );
        ctx.add_procedure(
            "inw".to_string(),
            vec![TypeAnnotation::Int64],
            TypeAnnotation::Int64,
        );
        ctx.add_procedure(
            "inl".to_string(),
            vec![TypeAnnotation::Int64],
            TypeAnnotation::Int64,
        );
        // Model-specific registers
        ctx.add_procedure(
            "rdmsr".to_string(),
            vec![TypeAnnotation::Int64],
            TypeAnnotation::Int64,
        );
        ctx.add_procedure(
            "wrmsr".to_string(),
            vec![TypeAnnotation::Int64, TypeAnnotation::Int64],
            TypeAnnotation::Void,
        );
        // CPUID
        ctx.add_procedure(
            "cpuid".to_string(),
            vec![TypeAnnotation::Int64],
            TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64)),
        );
        // Interrupt control
        ctx.add_procedure("cli".to_string(), vec![], TypeAnnotation::Void);
        ctx.add_procedure("sti".to_string(), vec![], TypeAnnotation::Void);
        // Halt
        ctx.add_procedure("hlt".to_string(), vec![], TypeAnnotation::Void);
    }

    // Register functions exposed by installed Rust packages (e.g. LPPX2D,
    // LPPX3D) so `use <Package>;` followed by calls to those functions
    // type-checks against their declared signatures.
    NativeRegistry::register_in_type_context_with(&mut ctx, context)?;

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
                    suggestion: Some(
                        "ensure all return statements match the declared return type".to_string(),
                    ),
                });
            }
        }
    }

    Ok(())
}

/// Type check a statement, returning the type if it's a return statement
fn typecheck_statement(
    stmt: &Statement,
    ctx: &mut TypeContext,
) -> LppoResult<Option<TypeAnnotation>> {
    match stmt {
        Statement::Let {
            name,
            typ,
            expr,
            ownership,
            ..
        } => {
            let expr_type = typecheck_expr(expr, ctx)?;
            let float_literal_narrowing = matches!(expr, Expr::Float(_))
                && matches!(typ.normalized(), TypeAnnotation::Float32);
            if !is_polymorphic_collection_call(expr)
                && !float_literal_narrowing
                && !types_compatible(typ, &expr_type)
            {
                return Err(LppoError::TypeError {
                    message: format!(
                        "variable '{}' declared as {:?} but assigned {:?}",
                        name, typ, expr_type
                    ),
                    expected: Some(format!("{:?}", typ)),
                    found: Some(format!("{:?}", expr_type)),
                    location: None,
                    suggestion: Some(
                        "ensure the expression type matches the variable type".to_string(),
                    ),
                });
            }
            ctx.add_variable_with_ownership(name.clone(), typ.clone(), ownership.clone());
            Ok(None)
        }

        Statement::Assignment { target, value } => {
            ctx.can_write(target)?;
            let var_type = ctx
                .get_variable(target)
                .cloned()
                .ok_or_else(|| LppoError::UnknownVariable {
                    name: target.clone(),
                    location: None,
                    suggestion: Some("declare the variable before assigning to it".to_string()),
                })?;
            let value_type = typecheck_expr(value, ctx)?;
            if !types_compatible(&var_type, &value_type) {
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

        Statement::If {
            cond,
            then_branch,
            elifs,
            else_branch,
        } => {
            let cond_type = typecheck_expr(cond, ctx)?;
            if !types_compatible(&TypeAnnotation::Bool, &cond_type) {
                return Err(LppoError::TypeError {
                    message: "if condition must be a boolean".to_string(),
                    expected: Some("Bool".to_string()),
                    found: Some(format!("{:?}", cond_type)),
                    location: None,
                    suggestion: Some(
                        "use a comparison operator like ==, !=, <, >, etc.".to_string(),
                    ),
                });
            }

            // Type check branches in child scopes with proper borrow tracking
            let mut branch_ctx = ctx.child();
            branch_ctx.enter_scope();
            for stmt in then_branch {
                typecheck_statement(stmt, &mut branch_ctx)?;
            }
            branch_ctx.exit_scope();

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
                elif_ctx.enter_scope();
                for stmt in elif_stmts {
                    typecheck_statement(stmt, &mut elif_ctx)?;
                }
                elif_ctx.exit_scope();
            }

            if let Some(else_stmts) = else_branch {
                let mut else_ctx = ctx.child();
                else_ctx.enter_scope();
                for stmt in else_stmts {
                    typecheck_statement(stmt, &mut else_ctx)?;
                }
                else_ctx.exit_scope();
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
            loop_ctx.enter_scope();
            for stmt in body {
                typecheck_statement(stmt, &mut loop_ctx)?;
            }
            loop_ctx.exit_scope();
            Ok(None)
        }

        Statement::For {
            var,
            typ,
            iterable,
            body,
        } => {
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
                            suggestion: Some(
                                "ensure loop variable type matches array element type".to_string(),
                            ),
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
            loop_ctx.enter_scope();
            loop_ctx.add_variable_with_ownership(var.clone(), typ.clone(), OwnershipKind::Own);
            for stmt in body {
                typecheck_statement(stmt, &mut loop_ctx)?;
            }
            loop_ctx.exit_scope();
            Ok(None)
        }

        Statement::Call(qualified_name, args) => {
            let proc_name = qualified_name_to_string(qualified_name);
            let (param_types, _) = ctx
                .get_procedure(&proc_name)
                .ok_or_else(|| LppoError::UnknownProcedure {
                    name: proc_name.clone(),
                    location: None,
                    suggestion: Some(
                        "check that the procedure is defined and spelled correctly".to_string(),
                    ),
                })?
                .clone();

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
            // Enter a new scope for the loop body
            ctx.enter_scope();
            for stmt in body {
                typecheck_statement(stmt, &mut *ctx)?;
            }
            ctx.exit_scope();
            Ok(None)
        }

        Statement::Scope { body, .. } => {
            // Enter a new scope for the block body
            ctx.enter_scope();
            for stmt in body {
                typecheck_statement(stmt, &mut *ctx)?;
            }
            ctx.exit_scope();
            Ok(None)
        }

        Statement::Unsafe { body } => {
            // Inside unsafe blocks, borrow checker is bypassed
            ctx.set_unsafe(true);
            for stmt in body {
                typecheck_statement(stmt, &mut *ctx)?;
            }
            ctx.set_unsafe(false);
            Ok(None)
        }

        Statement::Break | Statement::Continue => Ok(None),

        Statement::ExprStatement(expr) => {
            typecheck_expr(expr, ctx)?;
            Ok(None)
        }

        Statement::IndexedAssignment {
            target,
            index,
            value,
        } => {
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
            // Can't move a borrowed variable
            ctx.can_read(source)?;
            ctx.mark_moved(source)?;
            let src_type = ctx
                .get_variable(source)
                .ok_or_else(|| LppoError::UnknownVariable {
                    name: source.clone(),
                    location: None,
                    suggestion: None,
                })?
                .clone();
            ctx.add_variable_with_ownership(target.clone(), src_type, OwnershipKind::Own);
            Ok(None)
        }

        Statement::Copy { source, target } => {
            ctx.can_read(source)?;
            let src_type = ctx
                .get_variable(source)
                .ok_or_else(|| LppoError::UnknownVariable {
                    name: source.clone(),
                    location: None,
                    suggestion: None,
                })?
                .clone();
            ctx.add_variable_with_ownership(target.clone(), src_type, OwnershipKind::Own);
            Ok(None)
        }

        Statement::Alloc {
            var,
            typ,
            location,
            size,
        } => {
            typecheck_expr(size, ctx)?;
            // Heap-allocated variables need explicit deallocation
            if matches!(location, crate::ast::MemoryLocation::Heap) {
                ctx.mark_heap_allocated(var.clone());
            }
            ctx.add_variable_with_ownership(var.clone(), typ.clone(), OwnershipKind::Own);
            Ok(None)
        }

        Statement::Dealloc(var) => {
            ctx.can_write(var)?;
            ctx.mark_dropped(var)?;
            Ok(None)
        }

        // For now, treat these as no-ops in type checking
        Statement::Match { .. }
        | Statement::StructInit { .. }
        | Statement::Pause { .. }
        | Statement::Spawn { .. } => Ok(None),
    }
}

/// Type check an expression and return its type
fn typecheck_expr(expr: &Expr, ctx: &mut TypeContext) -> LppoResult<TypeAnnotation> {
    match expr {
        Expr::Int(_) => Ok(TypeAnnotation::Int64),
        Expr::Bool(_) => Ok(TypeAnnotation::Bool),
        Expr::Str(_) => Ok(TypeAnnotation::Str),
        Expr::Float(_) => Ok(TypeAnnotation::Float64),

        Expr::Var(name) => {
            // Check ownership state before access (read access = can_read)
            ctx.can_read(name)?;
            ctx.get_variable(name)
                .cloned()
                .ok_or_else(|| LppoError::UnknownVariable {
                    name: name.clone(),
                    location: None,
                    suggestion: Some("declare the variable before using it".to_string()),
                })
        }

        Expr::Ref {
            target,
            is_mut,
            lifetime,
        } => {
            // Create a borrow of the target variable
            match target.as_ref() {
                Expr::Var(name) => {
                    let kind = if *is_mut {
                        BorrowKind::Mut
                    } else {
                        BorrowKind::Imm
                    };
                    ctx.borrow_variable(name, kind)?;
                    let target_type = ctx
                        .get_variable(name)
                        .cloned()
                        .ok_or_else(|| LppoError::UnknownVariable {
                            name: name.clone(),
                            location: None,
                            suggestion: None,
                        })?;
                    Ok(TypeAnnotation::Ref(Box::new(target_type), lifetime.clone()))
                }
                _ => Err(LppoError::type_error(
                    "can only take a reference to a variable".to_string(),
                )),
            }
        }

        Expr::Deref(operand) => {
            // Dereference: the operand must evaluate to a Ref type
            let operand_type = typecheck_expr(operand, ctx)?;
            match operand_type {
                TypeAnnotation::Ref(inner, _) => Ok(*inner),
                _ => Err(LppoError::type_error(format!(
                    "cannot dereference type {:?}, expected a reference",
                    operand_type
                ))),
            }
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
                        suggestion: Some(
                            "ensure all array elements have the same type".to_string(),
                        ),
                    });
                }
            }
            Ok(TypeAnnotation::Arr(Box::new(first_type)))
        }

        Expr::Index { target, index } => {
            if let TypeAnnotation::Tuple(items) = typecheck_expr(target, ctx)?.normalized() {
                if let Expr::Int(i) = index.as_ref() {
                    let idx = if *i < 0 { items.len() as i64 + *i } else { *i };
                    if idx >= 0 && (idx as usize) < items.len() {
                        return Ok(items[idx as usize].clone());
                    }
                }
                return Ok(items.first().cloned().unwrap_or(TypeAnnotation::Void));
            }
            if let TypeAnnotation::Str = typecheck_expr(target, ctx)?.normalized() {
                return Ok(TypeAnnotation::Char);
            }
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
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Pow => {
                    // Support integer or float arithmetic. Both operands must have the same numeric type.
                    let left_is_int = types_compatible(&TypeAnnotation::Int64, &left_type);
                    let right_is_int = types_compatible(&TypeAnnotation::Int64, &right_type);
                    let left_is_f64 =
                        matches!(left_type, TypeAnnotation::Custom(ref n) if n == "F64");
                    let right_is_f64 =
                        matches!(right_type, TypeAnnotation::Custom(ref n) if n == "F64");

                    if left_is_int && right_is_int {
                        return Ok(TypeAnnotation::Int64);
                    }
                    if left_is_f64 && right_is_f64 {
                        return Ok(TypeAnnotation::Custom("F64".to_string()));
                    }

                    return Err(LppoError::TypeError {
                        message: format!(
                            "arithmetic operator {:?} requires numeric operands of the same type",
                            op
                        ),
                        expected: None,
                        found: Some(format!("{:?} and {:?}", left_type, right_type)),
                        location: None,
                        suggestion: Some("use matching numeric types (Int64 or F64)".to_string()),
                    });
                }

                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge => {
                    if !types_compatible(&left_type, &right_type) {
                        return Err(LppoError::TypeError {
                            message: format!(
                                "comparison requires same types, got {:?} and {:?}",
                                left_type, right_type
                            ),
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
                    if types_compatible(&TypeAnnotation::Int64, &operand_type) {
                        Ok(TypeAnnotation::Int64)
                    } else if matches!(operand_type, TypeAnnotation::Custom(ref n) if n == "F64") {
                        Ok(TypeAnnotation::Custom("F64".to_string()))
                    } else {
                        return Err(LppoError::TypeError {
                            message: "negation requires numeric operand (Int64 or F64)".to_string(),
                            expected: Some("Int64 or F64".to_string()),
                            found: Some(format!("{:?}", operand_type)),
                            location: None,
                            suggestion: None,
                        });
                    }
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

        Expr::StructInit { name, fields } => {
            let struct_def = ctx
                .structs
                .get(name)
                .ok_or_else(|| LppoError::type_error(format!("unknown struct '{}'", name)))?;
            let field_types: std::collections::HashMap<String, TypeAnnotation> = struct_def
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.typ.clone()))
                .collect();
            for (field_name, expr) in fields.iter() {
                let field_def = field_types.get(field_name).ok_or_else(|| {
                    LppoError::type_error(format!(
                        "struct '{}' has no field '{}'",
                        name, field_name
                    ))
                })?;
                let expr_type = typecheck_expr(expr, ctx)?;
                if !types_compatible(field_def, &expr_type) {
                    return Err(LppoError::TypeError {
                        message: format!(
                            "field '{}' of struct '{}' expects {:?} but got {:?}",
                            field_name, name, field_def, expr_type
                        ),
                        expected: Some(format!("{:?}", field_def)),
                        found: Some(format!("{:?}", expr_type)),
                        location: None,
                        suggestion: None,
                    });
                }
            }
            Ok(TypeAnnotation::Custom(name.clone()))
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
            let (param_types, return_type) = ctx
                .get_procedure(&proc_name)
                .ok_or_else(|| LppoError::UnknownProcedure {
                    name: proc_name.clone(),
                    location: None,
                    suggestion: Some(
                        "check that the procedure is defined and spelled correctly".to_string(),
                    ),
                })?
                .clone();

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

        Expr::FieldAccess { target, field } => {
            let target_type = typecheck_expr(target, ctx)?;
            if let TypeAnnotation::Custom(struct_name) = &target_type {
                if let Some(struct_def) = ctx.structs.get(struct_name) {
                    if let Some(field_def) = struct_def.fields.iter().find(|f| f.name == *field) {
                        return Ok(field_def.typ.clone());
                    }
                }
            }
            Ok(TypeAnnotation::Void)
        }

        Expr::Closure {
            params,
            return_type,
            ..
        } => Ok(TypeAnnotation::Fn {
            params: params.iter().map(|p| p.typ.clone()).collect(),
            ret: Box::new(return_type.clone().unwrap_or(TypeAnnotation::Void)),
        }),

        Expr::Char(_) => Ok(TypeAnnotation::Char),
        Expr::Byte(_) => Ok(TypeAnnotation::Byte),

        Expr::Tuple(items) => {
            let mut types = Vec::with_capacity(items.len());
            for item in items {
                types.push(typecheck_expr(item, ctx)?);
            }
            Ok(TypeAnnotation::Tuple(types))
        }

        Expr::EnumVariant {
            enum_name,
            variant,
            args,
        } => {
            let mut arg_types = Vec::with_capacity(args.len());
            for arg in args {
                arg_types.push(typecheck_expr(arg, ctx)?);
            }
            // Option / Result are built in and always available.
            match enum_name.as_str() {
                "Option" => {
                    if variant == "Some" {
                        let inner = arg_types
                            .first()
                            .cloned()
                            .unwrap_or(TypeAnnotation::Void);
                        Ok(TypeAnnotation::Option(Box::new(inner)))
                    } else {
                        Ok(TypeAnnotation::Option(Box::new(TypeAnnotation::Void)))
                    }
                }
                "Result" => {
                    // `Ok(v)` carries no error type and `Err(e)` no success type;
                    // they are compatible with any Result<T, E>.
                    let ok = arg_types.first().cloned().unwrap_or(TypeAnnotation::Void);
                    let err = arg_types.get(1).cloned().unwrap_or(TypeAnnotation::Void);
                    Ok(TypeAnnotation::Result(Box::new(ok), Box::new(err)))
                }
                _ => {
                    if let Some(def) = ctx.enums.get(enum_name) {
                        for (i, field_type) in def
                            .variants
                            .iter()
                            .find(|v| v.name == *variant)
                            .map(|v| v.fields.clone())
                            .unwrap_or_default()
                            .iter()
                            .enumerate()
                        {
                            if let Some(found) = arg_types.get(i) {
                                if !types_compatible(field_type, found) {
                                    return Err(LppoError::type_error(format!(
                                        "variant '{}::{}' field {} expects {} but got {}",
                                        enum_name,
                                        variant,
                                        i,
                                        field_type.display_name(),
                                        found.display_name()
                                    )));
                                }
                            }
                        }
                        return Ok(TypeAnnotation::Custom(enum_name.clone()));
                    }
                    Ok(TypeAnnotation::Custom(format!("{}::{}", enum_name, variant)))
                }
            }
        }

        Expr::Async { .. } => Ok(TypeAnnotation::Custom("Future".to_string())),

        Expr::Spawn { .. } => Ok(TypeAnnotation::Custom("JoinHandle".to_string())),

        Expr::Join(_) => Ok(TypeAnnotation::Void),

        Expr::Await(_) => Ok(TypeAnnotation::Int64),

        // For unimplemented expression types, return a placeholder
        _ => Ok(TypeAnnotation::Void),
    }
}

/// Calls whose declared return type cannot express their element type
/// (the runtime is dynamically typed), so `Let` skips the compatibility check.
fn is_polymorphic_collection_call(expr: &Expr) -> bool {
    let name = match expr {
        Expr::Call { func, .. } => match func.as_ref() {
            Expr::Var(n) => n.as_str(),
            _ => return false,
        },
        _ => return false,
    };
    matches!(
        name,
        "vec_new"
            | "vec_push"
            | "vec_set"
            | "map_new"
            | "map_insert"
            | "map_remove"
            | "map_keys"
            | "map_values"
            | "set_new"
            | "set_insert"
            | "set_remove"
            | "set_to_array"
            | "set_items"
            | "tuple_new"
            | "json_parse"
            | "either"
    )
}

/// Check if two types are compatible
fn types_compatible(expected: &TypeAnnotation, actual: &TypeAnnotation) -> bool {
    use TypeAnnotation::*;
    let expected = expected.normalized();
    let actual = actual.normalized();

    let numeric_rank = |t: &TypeAnnotation| -> std::option::Option<u8> {
        match t {
            Int64 => Some(0),
            Byte => Some(1),
            Float32 => Some(2),
            Float64 | Float => Some(3),
            _ => None,
        }
    };

    match (&expected, &actual) {
        (Int64, Int64) => true,
        (Bool, Bool) => true,
        (Char, Char) => true,
        (Str, Str) => true,
        (Void, Void) => true,
        (Arr(e), Arr(a)) => types_compatible(e, a),
        // The runtime is dynamically typed, so element types of built-in
        // collections are advisory only.
        (Set(_), Set(_)) => true,
        (Custom(e), Custom(a)) => e == a,
        (Generic(e_name, e_params), Generic(a_name, a_params)) => {
            e_name == a_name
                && e_params.len() == a_params.len()
                && e_params
                    .iter()
                    .zip(a_params.iter())
                    .all(|(e, a)| types_compatible(e, a))
        }
        // `None` has no payload, so it fits any Option<T>.
        (Option(_), Option(a)) if matches!(**a, Void) => true,
        (Option(e), Option(a)) => types_compatible(e, a),
        (Result(_, _), Result(a_ok, a_err))
            if matches!(**a_ok, Void) || matches!(**a_err, Void) =>
        {
            true
        }
        (Result(e_ok, e_err), Result(a_ok, a_err)) => {
            types_compatible(e_ok, a_ok) && types_compatible(e_err, a_err)
        }
        (Tuple(e_types), Tuple(a_types)) => {
            e_types.len() == a_types.len()
                && e_types
                    .iter()
                    .zip(a_types.iter())
                    .all(|(e, a)| types_compatible(e, a))
        }
        (Map(..), Map(..)) => true,
        (Fn { params: ep, ret: er }, Fn { params: ap, ret: ar }) => {
            ep.len() == ap.len()
                && ep
                    .iter()
                    .zip(ap.iter())
                    .all(|(e, a)| types_compatible(e, a))
                && types_compatible(er, ar)
        }
        (Ref(e_inner, _), Ref(a_inner, _)) => types_compatible(e_inner, a_inner),
        (Own(e_inner), a) | (a, Own(e_inner)) => types_compatible(e_inner, a),
        (Mut(e_inner), a) | (a, Mut(e_inner)) => types_compatible(e_inner, a),
        // Numeric widening: expected wider than actual, or same width.
        _ => match (numeric_rank(&expected), numeric_rank(&actual)) {
            (Some(e), Some(a)) => e >= a,
            _ => false,
        },
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
        assert!(types_compatible(
            &TypeAnnotation::Int64,
            &TypeAnnotation::Int64
        ));
        assert!(types_compatible(
            &TypeAnnotation::Bool,
            &TypeAnnotation::Bool
        ));
        assert!(!types_compatible(
            &TypeAnnotation::Int64,
            &TypeAnnotation::Bool
        ));

        let arr_int = TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64));
        let arr_int2 = TypeAnnotation::Arr(Box::new(TypeAnnotation::Int64));
        assert!(types_compatible(&arr_int, &arr_int2));
    }

    #[test]
    fn test_expr_type_checking() {
        let mut ctx = TypeContext::new();
        ctx.add_variable("x".to_string(), TypeAnnotation::Int64);

        // Test integer literal
        assert_eq!(
            typecheck_expr(&Expr::Int(42), &mut ctx).unwrap(),
            TypeAnnotation::Int64
        );

        // Test boolean literal
        assert_eq!(
            typecheck_expr(&Expr::Bool(true), &mut ctx).unwrap(),
            TypeAnnotation::Bool
        );

        // Test variable lookup
        assert_eq!(
            typecheck_expr(&Expr::Var("x".to_string()), &mut ctx).unwrap(),
            TypeAnnotation::Int64
        );

        // Test unknown variable
        assert!(typecheck_expr(&Expr::Var("y".to_string()), &mut ctx).is_err());
    }

    #[test]
    fn test_binary_op_type_checking() {
        let mut ctx = TypeContext::new();

        // Test arithmetic: 1 + 2
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Int(2)),
        };
        assert_eq!(typecheck_expr(&expr, &mut ctx).unwrap(), TypeAnnotation::Int64);

        // Test comparison: 1 < 2
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Int(1)),
            op: BinaryOp::Lt,
            right: Box::new(Expr::Int(2)),
        };
        assert_eq!(typecheck_expr(&expr, &mut ctx).unwrap(), TypeAnnotation::Bool);

        // Test logical: true and false
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Bool(true)),
            op: BinaryOp::And,
            right: Box::new(Expr::Bool(false)),
        };
        assert_eq!(typecheck_expr(&expr, &mut ctx).unwrap(), TypeAnnotation::Bool);
    }

    #[test]
    fn test_array_type_checking() {
        let mut ctx = TypeContext::new();

        // Test homogeneous array
        let expr = Expr::Array(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)]);
        let result = typecheck_expr(&expr, &mut ctx).unwrap();
        assert!(matches!(result, TypeAnnotation::Arr(_)));

        // Test heterogeneous array (should fail)
        let expr = Expr::Array(vec![Expr::Int(1), Expr::Bool(true)]);
        assert!(typecheck_expr(&expr, &mut ctx).is_err());
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
