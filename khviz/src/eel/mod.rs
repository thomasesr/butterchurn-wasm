pub mod eval;
pub mod lexer;
pub mod parser;

pub use eval::EelEnv;
pub use parser::{parse, Ast};
