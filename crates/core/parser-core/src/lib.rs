
pub mod event_dispatcher;
pub mod node_handler;
pub mod error_recovery;
pub mod syntax_tree;
pub mod capture;
mod state_stack;

mod metadata;
pub use metadata::{NodeMetadataKey, NodeMetadata, NodeType, PatchAction};

mod parser;
pub use parser::DefaultPasrser as Parser;
pub use parser::{ParseMode, ParserConfig};
pub use error_recovery::RecoveryPenalty;

pub mod incremental;

pub type NodeId = (std::time::Instant, u64); 
