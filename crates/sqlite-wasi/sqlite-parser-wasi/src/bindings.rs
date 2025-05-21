use parser_wasi::bindings::parsers::ParserImpl;

pub struct ParserComponent;

impl parser_wasi::bindings::parsers::Guest for ParserComponent {
    type Parser = ParserImpl;
    
    fn create() -> parser_wasi::bindings::parsers::Parser {
        let engine = sqlite_engine::create().expect("Failed to nstanciate parser engine");
        parser_wasi::bindings::parsers::Parser::new(ParserImpl::new(engine))
    }
}

parser_wasi::export!(ParserComponent);