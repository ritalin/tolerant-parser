pub mod core {
    pub mod engine_core;
    pub mod scanner_core;
    pub mod parser_core;
}

pub use core::engine_core::SyntaxKind;

#[cfg(feature = "test_support")]
pub mod support {
    pub mod test_support;
}