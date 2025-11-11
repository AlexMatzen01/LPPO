pub mod ast;
pub mod compiler;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;

pub use compiler::compile_program;
pub use error::{LppoError, LppoResult};
pub use parser::parse_program;
pub use runtime::Executor;

/// Parse and compile LPPO source into an executable program.
pub fn build_executor(source: &str) -> LppoResult<Executor> {
	let ast = parse_program(source)?;
	let procedures = compile_program(ast)?;
	Ok(Executor::new(procedures))
}

/// Compile source and return serialized bytes (JSON) suitable for embedding.
pub fn compile_to_bytes(source: &str) -> LppoResult<Vec<u8>> {
	let ast = parse_program(source)?;
	let procedures = compile_program(ast)?;
	runtime::serialize_procedures(&procedures)
}

/// Load an executor from serialized bytes.
pub fn load_executor_from_bytes(bytes: &[u8]) -> LppoResult<Executor> {
	runtime::executor_from_bytes(bytes)
}
