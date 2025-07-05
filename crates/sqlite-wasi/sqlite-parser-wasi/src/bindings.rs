use engine_core::scanner_engine::CaseSensitivity;
use parser_core::{ParseMode, RecoveryPenalty};
use parser_wasi::bindings::parsers::{ParserImpl, IncrementalParserImpl};

pub struct ParserComponent;

impl parser_wasi::bindings::parsers::Guest for ParserComponent {
    type Parser = ParserImpl;
    type IncrementalParser = IncrementalParserImpl;
    
    fn create() -> parser_wasi::bindings::parsers::Parser {
        let engine = sqlite_engine::create().expect("Failed to nstanciate parser engine");
        let config = parser_core::ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
            case_sensitive: CaseSensitivity::Insensitive,
        };

        parser_wasi::bindings::parsers::Parser::new(ParserImpl::new(engine, config))
    }
}

parser_wasi::export!(ParserComponent);