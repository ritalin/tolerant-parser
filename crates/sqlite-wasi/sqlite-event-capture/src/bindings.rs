use std::cell::RefCell;
use crate::bindings::event_capture_world::exports::ritalin::event_capture;
use tolerant_parser_sdk::core::{engine_core::{self, scanner_engine::CaseSensitivity}, parser_core, scanner_core};

pub mod event_capture_world;

pub struct EventCaptureImpl {
    inner: RefCell<parser_core::capture::ParseEventCapture>,
}

impl EventCaptureImpl {
    fn new(source: String,config: event_capture::captures::CaptureConfig,) -> Self {
        let engine = sqlite_engine::create().expect("Failed to instanciate parser engine");
        let inner = parser_core::capture::ParseEventCapture::create(&source, config.into(), engine).expect("can not init a event capture handler.");
        
        Self { inner: RefCell::new(inner) }
    }
}

impl event_capture::captures::GuestEventCapture for EventCaptureImpl {
    fn next(&self,) -> Option<event_capture::captures::CaptureEvent> {
        self.inner.borrow_mut().next().expect("Can not parse source here.").map(Into::into)
    }
}

impl From<parser_core::capture::CaptureEvent> for event_capture::captures::CaptureEvent {
    fn from(value: parser_core::capture::CaptureEvent) -> Self {
        match value {
            parser_core::capture::CaptureEvent::Scan(token) => {
                event_capture::captures::CaptureEvent::Scan(token.into())
            }
            parser_core::capture::CaptureEvent::Parse(parse_event) => {
                event_capture::captures::CaptureEvent::Parse(parse_event.into())
            }
        }
    }
}

impl From<scanner_core::Token> for event_capture::types::Token {
    fn from(value: scanner_core::Token) -> Self {
        let leading_trivia: Vec<event_capture::types::ScanEvent> = value.leading_trivia.map(|trivia| trivia.into_iter().map(|x| x.into()).collect()).unwrap_or_default();
        let trailing_trivia: Vec<event_capture::types::ScanEvent> = value.trailing_trivia.map(|trivia| trivia.into_iter().map(|x| x.into()).collect()).unwrap_or_default();

        Self {
            leading_trivia,
            main_token: value.main.into(),
            trailing_trivia,
        }
    }
}

impl From<engine_core::scanner_engine::ScanEvent> for event_capture::types::ScanEvent {
    fn from(value: engine_core::scanner_engine::ScanEvent) -> Self {
        Self {
            kind: value.kind.into(),
            offset: value.offset as u64,
            len: value.len as u64,
            value: value.value,
        }
    }
}

impl From<parser_core::event_dispatcher::ParseEvent> for event_capture::types::ParseEvent {
    fn from(value: parser_core::event_dispatcher::ParseEvent) -> Self {
        use event_capture::types::{TransitionState, ReduceTransitionState};

        match value {
            parser_core::event_dispatcher::ParseEvent::Shift { kind, current_state, next_state, edit_state } => {
                event_capture::types::ParseEvent::Shift(TransitionState{ 
                    kind: kind.into(), current: Some(current_state as u64), next: Some(next_state as u64), edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::Reduce { kind, current_state, next_state, pop_count, edit_state } => {
                event_capture::types::ParseEvent::Reduce(ReduceTransitionState{
                    kind: kind.into(), current: Some(current_state as u64), next: Some(next_state as u64), edit: edit_state as u64,
                    pop_count: pop_count as u64,              
                })
            }
            parser_core::event_dispatcher::ParseEvent::Emit { kind, edit_state } => {
                event_capture::types::ParseEvent::Emit(TransitionState{
                    kind: kind.into(), current: None, next: None, edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::Accept { kind, last_state, edit_state } => {
                event_capture::types::ParseEvent::Accept(TransitionState{
                    kind: kind.into(), current: Some(last_state as u64), next: None, edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::PatchDrop { kind, current_state, next_state, edit_state } => {
                event_capture::types::ParseEvent::PatchDrop(TransitionState{
                    kind: kind.into(), current: Some(current_state as u64), next: Some(next_state as u64), edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::PatchShift { kind, current_state, next_state, edit_state } => {
                event_capture::types::ParseEvent::PatchShift(TransitionState{
                    kind: kind.into(), current: Some(current_state as u64), next: Some(next_state as u64), edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::PatchReduce { kind, current_state, next_state, pop_count, edit_state } => {
                event_capture::types::ParseEvent::PatchReduce(ReduceTransitionState{
                    kind: kind.into(), current: Some(current_state as u64), next: Some(next_state as u64), edit: edit_state as u64,
                    pop_count: pop_count as u64,              
                })
            }
            parser_core::event_dispatcher::ParseEvent::PatchEmit { kind, edit_state } => {
                event_capture::types::ParseEvent::PatchEmit(TransitionState{
                    kind: kind.into(), current: None, next: None, edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::Invalid { kind, current_state, edit_state } => {
                event_capture::types::ParseEvent::Invalid(TransitionState{
                    kind: kind.into(), current: Some(current_state as u64), next: None, edit: edit_state as u64
                })
            }
            parser_core::event_dispatcher::ParseEvent::InvalidEmit { kind, edit_state, pop_count } => {
                event_capture::types::ParseEvent::InvalidEmit(ReduceTransitionState{
                    kind: kind.into(), current: None, next: None, edit: edit_state as u64,
                    pop_count: pop_count as u64
                })
            }
        }
    }
}

impl From<event_capture::captures::CaptureConfig> for parser_core::capture::EventCaptureConfig {
    fn from(value: event_capture::captures::CaptureConfig) -> Self {
        Self {
            mode: parser_core::ParseMode::ByStatement,
            no_scan: value.no_scan,
            no_parse: value.no_parse,
            case_sensitive: if value.ignore_case { CaseSensitivity::Insensitive } else { CaseSensitivity::Sensitive },
        }
    }
}

struct CaptureComponent;

impl event_capture::captures::Guest for CaptureComponent {
    type EventCapture = EventCaptureImpl;
    
    fn create(source: String,config: event_capture::captures::CaptureConfig,) -> event_capture::captures::EventCapture {
        event_capture::captures::EventCapture::new(EventCaptureImpl::new(source, config))
    }
}

event_capture_world::export_capture!(CaptureComponent);