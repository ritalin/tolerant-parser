use std::path::PathBuf;
// use tolerant_parser_sdk::wasi::parser_wasi::bindings::parser_world::exports::ritalin::parser;
use wasmtime::{component::{Component, Linker}, Store};
use  crate::{config::CaptureSetting, ritalin::event_capture::{self}};
use crate::{config, runtime};

pub fn run_inspect(runtime: runtime::Runtime, config: config::ProviderConfig, setting: config::CaptureSetting) -> Result<(), anyhow::Error> {
    let provider = config.resolve(setting.engine.as_ref(), setting.input.as_path())?;
    let (bindings, store) = runtime.instanciate::<crate::AppWorld>(provider.path.as_path())?;
    
    let source = std::fs::read_to_string(PathBuf::from(setting.input.as_path()))?;
    let capture_config = event_capture::types::CaptureConfig{
        no_scan: setting.no_scan,
        no_parse: setting.no_parse,
        ignore_case: setting.ignore_case,
    };
    let mut capture = EventCapture::create(bindings, store, &source, capture_config)?;

    print_event_all(&mut capture, &setting, &source)
}

impl runtime::InstanciateWorld for crate::AppWorld {
    type World = crate::AppWorld;
    type State = runtime::RuntimeState;

    fn instantiate_component(store: impl wasmtime::AsContextMut<Data = Self::State>, component: &Component, linker: &Linker<Self::State>) -> Result<Self::World, anyhow::Error> {
        crate::AppWorld::instantiate(store, component, linker)
    }
}

struct EventCapture {
    bindings: crate::AppWorld,
    instance: wasmtime::component::ResourceAny,
    store: Store<runtime::RuntimeState>,
}

impl EventCapture {
    pub fn create(bindings: crate::AppWorld, mut store: Store<runtime::RuntimeState>, source: &str, config: event_capture::types::CaptureConfig) -> Result<Self, anyhow::Error> {
        let instance = bindings.ritalin_event_capture_captures().call_create(&mut store, source, config)?;
        
        Ok(Self { bindings, instance, store })
    }

    pub fn next(&mut self) -> Result<Option<event_capture::types::CaptureEvent>, anyhow::Error> {
        self.bindings.ritalin_event_capture_captures().event_capture().call_next(&mut self.store, self.instance)
    }

    pub fn state_histories(&mut self) -> Result<Vec<u64>, anyhow::Error> {
        self.bindings.ritalin_event_capture_captures().event_capture().call_state_histories(&mut self.store, self.instance)
    }
}

fn print_event_all(capture: &mut EventCapture, setting: &CaptureSetting, source: &str) -> Result<(), anyhow::Error> {
    if ! setting.quiet {
        println!("`{}`", source);
        println!("--------------------------------------------------------------------------------");
    }

    let mut id_gen = IdGenerator::new(1);

    loop {
        let before_states = if setting.show_state { capture.state_histories()? } else { vec![] };
        let Some(event) = capture.next()? else { break };

        if !setting.quiet {
            match event {
                event_capture::types::CaptureEvent::Scan(event) => print_scan_event(&event, &setting, &mut id_gen),
                event_capture::types::CaptureEvent::Parse(event) => {
                let after_states = if setting.show_state { capture.state_histories()? } else { vec![] };
                    print_parse_event(&event, &before_states, &after_states, &setting, &mut id_gen)
                }
            }
        }
    }

    Ok(())
}

struct IdGenerator {
    iter: std::iter::Successors<usize, fn(&usize) -> Option<usize>>,
}

impl IdGenerator {
    pub fn new(init: usize) -> Self {
        Self {
            iter: std::iter::successors(Some(init), |prev| Some(prev + 1)),
        }
    }

    pub fn next(&mut self) -> usize {
        self.iter.next().unwrap()
    }
}

const MAX_LABEL_LEN: usize = 10;

fn print_scan_event(token: &event_capture::types::Token, setting: &CaptureSetting, id_gen: &mut IdGenerator) {
    for (i, event) in token.leading_trivia.iter().enumerate() { 
        println!("[{}] #{}", apply_label_color(&format!("Scan/Leading#{}", i+1), setting, ansi_term::Color::RGB(128, 128, 128)), id_gen.next());
        println!("{:>width$} {}, (offset) {}, (len) {}", "(kind)", event.kind.name, event.offset, event.len, width = MAX_LABEL_LEN);
        println!("{:>width$} `{:?}`", "(value)", event.value, width = MAX_LABEL_LEN);
    }

    {
        let event = &token.main_token;
        println!("[{}] #{}", apply_label_color("Scan/Main", setting, ansi_term::Color::Yellow), id_gen.next());
        println!("{:>width$} {}, (offset) {}, (len) {}", "(kind)", event.kind.name, event.offset, event.len, width = MAX_LABEL_LEN);
        println!("{:>width$} {:?}", "(value)", event.value, width = MAX_LABEL_LEN);
    }

    for (i, event) in token.trailing_trivia.iter().enumerate() {
        println!("[{}] #{}", apply_label_color(&format!("Scan/Trailing#{}", i+1), setting, ansi_term::Color::RGB(128, 128, 128)), id_gen.next());
        println!("{:>width$} {}, (offset) {}, (len) {}", "(kind)", event.kind.name, event.offset, event.len, width = MAX_LABEL_LEN);
        println!("{:>width$} {:?}", "(value)", event.value, width = MAX_LABEL_LEN);
    }
}

fn print_parse_event(event: &event_capture::types::ParseEvent, state_histories_before: &[u64], state_histories_after: &[u64], setting: &CaptureSetting, id_gen: &mut IdGenerator) {
    use event_capture::types::{ParseEvent, TransitionState, ReduceTransitionState};

    match event {
        ParseEvent::Shift(TransitionState{ kind, current, next, edit }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Parse/Shift", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} current: {}, next: {}, edit: {}", "(state)", current.unwrap(), next.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::Reduce(ReduceTransitionState{ kind, current, next, edit, pop_count }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Parse/Reduce", setting), id_gen.next());
            println!("{:>width$} {}, (pop_count) {}", "(kind)", kind.name, pop_count, width = MAX_LABEL_LEN);
            println!("{:>width$} current: {}, next: {}, edit: {}", "(state)", current.unwrap(), next.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::Emit(TransitionState{ kind, edit, .. }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Parse/Emit", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} edit: {}", "(state)", edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::Accept(TransitionState{ kind, current, edit, .. }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Parse/Accept", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} last: {}, edit: {}", "(state)", current.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::PatchDrop(TransitionState{ kind, current, next, edit }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Recover/Drop", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} current: {}, next: {}, edit: {}", "(state)", current.unwrap(), next.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::PatchShift(TransitionState{ kind, current, next, edit }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Recover/Shift", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} current: {}, next: {}, edit: {}", "(state)", current.unwrap(), next.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::PatchReduce(ReduceTransitionState{ kind, current, next, edit, pop_count }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Recover/Reduce", setting), id_gen.next());
            println!("{:>width$} {}, (pop_count) {}", "(kind)", kind.name, pop_count, width = MAX_LABEL_LEN);
            println!("{:>width$} current: {}, next: {}, edit: {}", "(state)", current.unwrap(), next.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::PatchEmit(TransitionState{ kind, edit, .. }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Recover/Emit", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} edit: {}", "(state)", edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::Invalid(TransitionState{ kind, current, edit, .. }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Recover/Invalid", setting), id_gen.next());
            println!("{:>width$} {}", "(kind)", kind.name, width = MAX_LABEL_LEN);
            println!("{:>width$} current: {}, edit: {}", "(state)", current.unwrap(), edit, width = MAX_LABEL_LEN);
        }
        ParseEvent::InvalidEmit(ReduceTransitionState{ kind, pop_count, edit, .. }) => {
            println!("[{}] #{}", apply_parse_event_color(event, "Parse/InvalidEmit", setting), id_gen.next());
            println!("{:>width$}{}, (pop_count) {}", "(kind)", kind.name, pop_count, width = MAX_LABEL_LEN);
            println!("{:>width$} edit: {}", "(state)", edit, width = MAX_LABEL_LEN);
        }
    }

    if setting.show_state {
            let before_states = state_histories_before.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
            let after_state = state_histories_after.iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
            println!("{:>width$} before: [{}]", "(history)", before_states, width = MAX_LABEL_LEN);
            println!("{:>width$} after:  [{}]", "", after_state, width = MAX_LABEL_LEN);
    }
}

fn apply_parse_event_color(event: &event_capture::types::ParseEvent, label: &str, config: &CaptureSetting) -> String {
    use event_capture::types::{ParseEvent, ReduceTransitionState};

    let color = match event {
        ParseEvent::PatchDrop(_) |
        ParseEvent::PatchShift(_) | 
        ParseEvent::PatchReduce(_) |
        ParseEvent::PatchEmit(_) => {
            ansi_term::Color::Red
        }
        ParseEvent::Invalid(_) => {
            ansi_term::Color::RGB(128, 128, 128)
        }
        ParseEvent::Reduce(ReduceTransitionState{ pop_count, .. }) if *pop_count == 0 => {
            ansi_term::Color::RGB(128, 128, 128)
        }
        ParseEvent::Emit(_) | ParseEvent::InvalidEmit(_) => {
            ansi_term::Color::Purple
        }
        _ => ansi_term::Color::Cyan,
    };

    apply_label_color(label, config, color)
}

fn apply_label_color(label: &str, setting: &CaptureSetting, color: ansi_term::Color) -> String {
    if setting.no_color {
        return label.into();
    }

    color.paint(label).to_string()
}

pub fn run_attach_provider(runtime: runtime::Runtime, mut config: config::ProviderConfig, engine: &str, extension: &str, wasm_path: &std::path::Path) -> Result<(), anyhow::Error> {
    let binary = runtime.prebuild(wasm_path)?;
    let provider = config.put(engine, extension)?;

    if let Some(dir_path) = provider.path.parent() {
        std::fs::create_dir_all(dir_path)?;
    }
    std::fs::write(provider.path.as_path(), &binary)?;
    println!("engine updated.");
    
    config.save()?;
    Ok(())
}

pub fn run_detach_provider(_runtime: runtime::Runtime, mut config: config::ProviderConfig, engine: &str) -> Result<(), anyhow::Error> {
    let Some(provider) = config.remove_by_engine(engine) else {
        anyhow::bail!("Warn: Specified engine does not set.");
    };

    std::fs::remove_file(provider.path)?;
    config.save()?;

    Ok(())
}

pub fn run_list(_runtime: runtime::Runtime, config: config::ProviderConfig) -> Result<(), anyhow::Error> {
    for (engine, item) in config.providers() {
        println!("{} ({})", engine, item.extension);
    }

    Ok(())
}