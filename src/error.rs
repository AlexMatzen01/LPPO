use std::fmt::{self, Display, Formatter};

/// Convenient result alias for LPPO operations.
pub type LppoResult<T> = Result<T, LppoError>;

/// Represents any error that can occur while processing an LPPO program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LppoError {
    /// Raised by the lexer when an unexpected character is encountered.
    UnexpectedCharacter { ch: char, position: usize },
    /// Raised by the lexer when a string literal is not properly terminated.
    UnterminatedString { position: usize },
    /// Raised by the parser when a token was not expected at the given location.
    UnexpectedToken { expected: &'static str, position: usize },
    /// Raised by the parser when the input ends unexpectedly.
    UnexpectedEndOfInput,
    /// Raised by the compiler when a procedure is declared more than once.
    DuplicateProcedure(String),
    /// Raised by the compiler/runtime when the entry procedure is missing.
    UnknownProcedure(String),
    /// Raised when a variable is referenced but not defined in the current scope.
    UnknownVariable(String),
    /// Raised when an operation receives operands of incorrect types.
    TypeError(String),
}

impl Display for LppoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LppoError::UnexpectedCharacter { ch, position } => {
                write!(f, "unexpected character '{ch}' at position {position}")
            }
            LppoError::UnterminatedString { position } => {
                write!(f, "unterminated string literal at position {position}")
            }
            LppoError::UnexpectedToken { expected, position } => {
                write!(f, "expected {expected} at position {position}")
            }
            LppoError::UnexpectedEndOfInput => write!(f, "unexpected end of input"),
            LppoError::DuplicateProcedure(name) => {
                write!(f, "duplicate procedure '{name}'")
            }
            LppoError::UnknownProcedure(name) => {
                write!(f, "unknown procedure '{name}'")
            }
            LppoError::UnknownVariable(name) => {
                write!(f, "unknown variable '{name}'")
            }
            LppoError::TypeError(msg) => {
                write!(f, "type error: {msg}")
            }
        }
    }
}

impl std::error::Error for LppoError {}
