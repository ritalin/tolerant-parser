pub mod parser_world;
mod syntax_tree_world;
mod types_world;
mod resource;

pub mod parsers {
    pub use super::parser_world::exports::ritalin::parser::parsers::{Guest, Parser};
    pub use super::resource::ParserImpl;
}

mod parser_types {
    pub use super::types_world::exports::ritalin::parser::types::*;
}
mod syntax {
    pub use super::syntax_tree_world::exports::ritalin::parser::syntaxes::*;
}
use resource::SyntaxTreeComponent;
syntax_tree_world::export!(SyntaxTreeComponent);

types_world::export!(ParserTypesComponent);