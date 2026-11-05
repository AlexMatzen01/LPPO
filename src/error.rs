use std::fmt::{self, Display, Formatter};

/// Convenient result alias for LPPO operations.
/// Note: LppoError has large variants; boxing them would require extensive refactoring.
/// For now, we accept the performance trade-off for better error messages.
#[allow(clippy::result_large_err)]
pub type LppoResult<T> = Result<T, LppoError>;

/// Source location information for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub position: usize,
    pub length: usize,
}

impl SourceLocation {
    pub fn new(position: usize) -> Self {
        Self {
            line: 0,
            column: 0,
            position,
            length: 1,
        }
    }

    pub fn with_line_col(line: usize, column: usize, position: usize) -> Self {
        Self {
            line,
            column,
            position,
            length: 1,
        }
    }

    pub fn with_length(mut self, length: usize) -> Self {
        self.length = length;
        self
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

/// Represents any error that can occur while processing an LPPO program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LppoError {
    /// Raised by the lexer when an unexpected character is encountered.
    UnexpectedCharacter {
        ch: char,
        location: SourceLocation,
        suggestion: Option<String>,
    },
    /// Raised by the lexer when a string literal is not properly terminated.
    UnterminatedString {
        location: SourceLocation,
        suggestion: Option<String>,
    },
    /// Raised by the parser when a token was not expected at the given location.
    UnexpectedToken {
        expected: String,
        found: String,
        location: SourceLocation,
        suggestion: Option<String>,
    },
    /// Raised by the parser when the input ends unexpectedly.
    UnexpectedEndOfInput {
        expected: String,
        suggestion: Option<String>,
    },
    /// Raised by the compiler when a procedure is declared more than once.
    DuplicateProcedure {
        name: String,
        first_location: Option<SourceLocation>,
        second_location: Option<SourceLocation>,
        suggestion: Option<String>,
    },
    /// Raised by the compiler/runtime when the entry procedure is missing.
    UnknownProcedure {
        name: String,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },
    /// Raised when a variable is referenced but not defined in the current scope.
    UnknownVariable {
        name: String,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },
    /// Raised when an operation receives operands of incorrect types.
    TypeError {
        message: String,
        expected: Option<String>,
        found: Option<String>,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },
    /// Division by zero error
    DivisionByZero { location: Option<SourceLocation> },
    /// Index out of bounds
    IndexOutOfBounds {
        index: i64,
        length: i64,
        location: Option<SourceLocation>,
    },
    /// Invalid array access
    InvalidArrayAccess {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Stack overflow in recursion
    StackOverflow { procedure: String, depth: usize },
    /// Memory allocation failure
    AllocationError {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Lifetime violation
    LifetimeError {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Ownership violation
    OwnershipError {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Move after move error
    MoveAfterMove {
        variable: String,
        location: Option<SourceLocation>,
    },
    /// Borrow checker error
    BorrowError {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Pattern matching error
    PatternMatchError {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Invalid cast
    InvalidCast {
        from: String,
        to: String,
        location: Option<SourceLocation>,
    },
    /// Module not found
    ModuleNotFound {
        name: String,
        location: Option<SourceLocation>,
    },
    /// Invalid syntax
    SyntaxError {
        message: String,
        location: Option<SourceLocation>,
        suggestion: Option<String>,
    },
    /// Generic runtime error
    RuntimeError {
        message: String,
        location: Option<SourceLocation>,
        stack_trace: Vec<String>,
    },
    /// I/O operation error
    IoError {
        message: String,
        location: Option<SourceLocation>,
    },
}

impl LppoError {
    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            LppoError::UnexpectedCharacter { .. }
            | LppoError::UnterminatedString { .. }
            | LppoError::UnexpectedToken { .. }
            | LppoError::UnexpectedEndOfInput { .. }
            | LppoError::DuplicateProcedure { .. }
            | LppoError::SyntaxError { .. } => ErrorSeverity::Error,

            LppoError::UnknownProcedure { .. }
            | LppoError::UnknownVariable { .. }
            | LppoError::TypeError { .. }
            | LppoError::DivisionByZero { .. }
            | LppoError::IndexOutOfBounds { .. }
            | LppoError::InvalidArrayAccess { .. }
            | LppoError::AllocationError { .. }
            | LppoError::LifetimeError { .. }
            | LppoError::OwnershipError { .. }
            | LppoError::MoveAfterMove { .. }
            | LppoError::BorrowError { .. }
            | LppoError::PatternMatchError { .. }
            | LppoError::InvalidCast { .. }
            | LppoError::ModuleNotFound { .. }
            | LppoError::RuntimeError { .. }
            | LppoError::IoError { .. } => ErrorSeverity::Error,

            LppoError::StackOverflow { .. } => ErrorSeverity::Fatal,
        }
    }

    /// Get a helpful suggestion for fixing this error, if available
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            LppoError::UnexpectedCharacter { suggestion, .. }
            | LppoError::UnterminatedString { suggestion, .. }
            | LppoError::UnexpectedToken { suggestion, .. }
            | LppoError::UnexpectedEndOfInput { suggestion, .. }
            | LppoError::DuplicateProcedure { suggestion, .. }
            | LppoError::UnknownProcedure { suggestion, .. }
            | LppoError::UnknownVariable { suggestion, .. }
            | LppoError::TypeError { suggestion, .. }
            | LppoError::SyntaxError { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        }
    }

    /// Get the source location associated with this error, if available
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            LppoError::UnexpectedCharacter { location, .. } => Some(location),
            LppoError::UnterminatedString { location, .. } => Some(location),
            LppoError::UnexpectedToken { location, .. } => Some(location),
            LppoError::DuplicateProcedure {
                second_location, ..
            } => second_location.as_ref(),
            LppoError::UnknownProcedure { location, .. }
            | LppoError::UnknownVariable { location, .. }
            | LppoError::TypeError { location, .. }
            | LppoError::DivisionByZero { location }
            | LppoError::IndexOutOfBounds { location, .. }
            | LppoError::InvalidArrayAccess { location, .. }
            | LppoError::AllocationError { location, .. }
            | LppoError::LifetimeError { location, .. }
            | LppoError::OwnershipError { location, .. }
            | LppoError::MoveAfterMove { location, .. }
            | LppoError::BorrowError { location, .. }
            | LppoError::PatternMatchError { location, .. }
            | LppoError::InvalidCast { location, .. }
            | LppoError::ModuleNotFound { location, .. }
            | LppoError::SyntaxError { location, .. }
            | LppoError::RuntimeError { location, .. } => location.as_ref(),
            _ => None,
        }
    }

    /// Format error with full details including location and suggestion
    pub fn format_detailed(&self, source: Option<&str>) -> String {
        let mut output = String::new();

        // Error severity prefix
        let severity_prefix = match self.severity() {
            ErrorSeverity::Info => "info",
            ErrorSeverity::Warning => "warning",
            ErrorSeverity::Error => "error",
            ErrorSeverity::Fatal => "fatal error",
        };

        output.push_str(&format!("{}: {}\n", severity_prefix, self));

        // Add location context if available
        if let Some(location) = self.location() {
            output.push_str(&format!(
                "  --> line {}, column {}\n",
                location.line + 1,
                location.column + 1
            ));

            // Show source code context if available
            if let Some(src) = source {
                if let Some(line_text) = src.lines().nth(location.line) {
                    output.push_str("   |\n");
                    output.push_str(&format!("{:3} | {}\n", location.line + 1, line_text));
                    output.push_str(&format!(
                        "   | {}{}\n",
                        " ".repeat(location.column),
                        "^".repeat(location.length.max(1))
                    ));
                }
            }
        }

        // Add suggestion if available
        if let Some(suggestion) = self.suggestion() {
            output.push_str(&format!("  = help: {}\n", suggestion));
        }

        // Add stack trace for runtime errors
        if let LppoError::RuntimeError { stack_trace, .. } = self {
            if !stack_trace.is_empty() {
                output.push_str("\nStack trace:\n");
                for (i, frame) in stack_trace.iter().enumerate() {
                    output.push_str(&format!("  {} at {}\n", i, frame));
                }
            }
        }

        output
    }
}

impl Display for LppoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LppoError::UnexpectedCharacter { ch, location, .. } => {
                write!(
                    f,
                    "unexpected character '{}' at line {}, column {}",
                    ch,
                    location.line + 1,
                    location.column + 1
                )
            }
            LppoError::UnterminatedString { location, .. } => {
                write!(
                    f,
                    "unterminated string literal at line {}, column {}",
                    location.line + 1,
                    location.column + 1
                )
            }
            LppoError::UnexpectedToken {
                expected,
                found,
                location,
                ..
            } => {
                write!(
                    f,
                    "expected {} but found '{}' at line {}, column {}",
                    expected,
                    found,
                    location.line + 1,
                    location.column + 1
                )
            }
            LppoError::UnexpectedEndOfInput { expected, .. } => {
                write!(f, "unexpected end of input, expected {}", expected)
            }
            LppoError::DuplicateProcedure { name, .. } => {
                write!(f, "duplicate procedure '{}'", name)
            }
            LppoError::UnknownProcedure { name, .. } => {
                write!(f, "unknown procedure '{}'", name)
            }
            LppoError::UnknownVariable { name, .. } => {
                write!(f, "unknown variable '{}'", name)
            }
            LppoError::TypeError {
                message,
                expected,
                found,
                ..
            } => {
                if let (Some(exp), Some(fnd)) = (expected, found) {
                    write!(
                        f,
                        "type error: expected {}, found {}: {}",
                        exp, fnd, message
                    )
                } else {
                    write!(f, "type error: {}", message)
                }
            }
            LppoError::DivisionByZero { .. } => {
                write!(f, "division by zero")
            }
            LppoError::IndexOutOfBounds { index, length, .. } => {
                write!(
                    f,
                    "index {} out of bounds for array of length {}",
                    index, length
                )
            }
            LppoError::InvalidArrayAccess { message, .. } => {
                write!(f, "invalid array access: {}", message)
            }
            LppoError::StackOverflow { procedure, depth } => {
                write!(
                    f,
                    "stack overflow in procedure '{}' at depth {}",
                    procedure, depth
                )
            }
            LppoError::AllocationError { message, .. } => {
                write!(f, "memory allocation error: {}", message)
            }
            LppoError::LifetimeError { message, .. } => {
                write!(f, "lifetime error: {}", message)
            }
            LppoError::OwnershipError { message, .. } => {
                write!(f, "ownership error: {}", message)
            }
            LppoError::MoveAfterMove { variable, .. } => {
                write!(f, "use of moved value '{}'", variable)
            }
            LppoError::BorrowError { message, .. } => {
                write!(f, "borrow checker error: {}", message)
            }
            LppoError::PatternMatchError { message, .. } => {
                write!(f, "pattern matching error: {}", message)
            }
            LppoError::InvalidCast { from, to, .. } => {
                write!(f, "invalid cast from {} to {}", from, to)
            }
            LppoError::ModuleNotFound { name, .. } => {
                write!(f, "module '{}' not found", name)
            }
            LppoError::SyntaxError { message, .. } => {
                write!(f, "syntax error: {}", message)
            }
            LppoError::RuntimeError { message, .. } => {
                write!(f, "runtime error: {}", message)
            }
            LppoError::IoError { message, .. } => {
                write!(f, "I/O error: {}", message)
            }
        }
    }
}

impl std::error::Error for LppoError {}

// Helper functions for creating errors with backward compatibility
impl LppoError {
    /// Create an UnexpectedCharacter error at a known `SourceLocation`.
    pub fn unexpected_char_at(ch: char, location: SourceLocation) -> Self {
        LppoError::UnexpectedCharacter {
            ch,
            location,
            suggestion: None,
        }
    }

    /// Backwards-compatible helper which builds a minimal `SourceLocation` from a raw position.
    pub fn unexpected_char(ch: char, position: usize) -> Self {
        Self::unexpected_char_at(ch, SourceLocation::new(position))
    }

    /// Create an UnterminatedString error at a known `SourceLocation`.
    pub fn unterminated_string_at(location: SourceLocation) -> Self {
        LppoError::UnterminatedString {
            location,
            suggestion: Some("add a closing quote '\"' to terminate the string".to_string()),
        }
    }

    /// Backwards-compatible wrapper that uses a raw position.
    pub fn unterminated_string(position: usize) -> Self {
        Self::unterminated_string_at(SourceLocation::new(position))
    }

    /// Create an UnexpectedToken error at a known `SourceLocation`.
    pub fn unexpected_token_at(expected: &str, found: &str, location: SourceLocation) -> Self {
        LppoError::UnexpectedToken {
            expected: expected.to_string(),
            found: found.to_string(),
            location,
            suggestion: None,
        }
    }

    /// Backwards-compatible wrapper that accepts a raw position.
    pub fn unexpected_token(expected: &str, found: &str, position: usize) -> Self {
        Self::unexpected_token_at(expected, found, SourceLocation::new(position))
    }

    /// Create an UnexpectedEndOfInput error (simplified)
    pub fn unexpected_eof(expected: &str) -> Self {
        LppoError::UnexpectedEndOfInput {
            expected: expected.to_string(),
            suggestion: None,
        }
    }

    /// Create a DuplicateProcedure error (simplified)
    pub fn duplicate_procedure(name: String) -> Self {
        LppoError::DuplicateProcedure {
            name,
            first_location: None,
            second_location: None,
            suggestion: Some("use a different name or remove one of the definitions".to_string()),
        }
    }

    /// Create an UnknownProcedure error (simplified)
    pub fn unknown_procedure(name: String) -> Self {
        LppoError::UnknownProcedure {
            name,
            location: None,
            suggestion: Some(
                "check that the procedure is defined and spelled correctly".to_string(),
            ),
        }
    }

    /// Create an UnknownVariable error (simplified)
    pub fn unknown_variable(name: String) -> Self {
        LppoError::UnknownVariable {
            name,
            location: None,
            suggestion: Some("check that the variable is declared before use".to_string()),
        }
    }

    /// Create a TypeError error (simplified)
    pub fn type_error(message: String) -> Self {
        LppoError::TypeError {
            message,
            expected: None,
            found: None,
            location: None,
            suggestion: None,
        }
    }

    pub fn io_error(message: String) -> Self {
        LppoError::IoError {
            message,
            location: None,
        }
    }
}
