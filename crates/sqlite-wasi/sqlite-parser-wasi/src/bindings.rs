use tolerant_parser_sdk::core::engine_core::scanner_engine::CaseSensitivity;
use tolerant_parser_sdk::core::parser_core::{self, ParseMode, RecoveryPenalty};
use tolerant_parser_sdk::wasi::parser_wasi::{self, bindings::parsers::{ParserImpl, IncrementalParserImpl}};

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

tolerant_parser_sdk::export_parser!(ParserComponent);