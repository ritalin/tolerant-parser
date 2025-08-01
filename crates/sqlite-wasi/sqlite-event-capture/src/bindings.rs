use tolerant_parser_sdk::wasi::event_capture_wasi::event_captures;

struct CaptureComponent;

impl event_captures::Guest for CaptureComponent {
    type EventCapture = event_captures::EventCaptureImpl;
    
    fn create(source: String,config: event_captures::CaptureConfig,) -> event_captures::EventCapture {
        let engine = sqlite_engine::create().expect("can not initialize");
        event_captures::EventCapture::new(event_captures::EventCaptureImpl::new(source, config, engine))
    }
}

tolerant_parser_sdk::export_capture!(CaptureComponent);