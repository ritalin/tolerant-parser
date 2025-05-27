use std::path::PathBuf;
mod config;

use ansi_term::Color;
use config::CmdConfig;
use parser_core::{capture::{CaptureEvent, ParseEventCapture}, event_dispatcher::ParseEvent};

fn main() -> Result<(), anyhow::Error> {
    let cmd_config = {
        use clap::Parser;
        config::CmdConfig::parse()
    };

    let source = std::fs::read_to_string(PathBuf::from(cmd_config.input.clone()))?;
    let engine = sqlite_engine::create()?;
    let mut capture = ParseEventCapture::create(&source, cmd_config.to_capture_config(), engine)?;
    
    println!("`{}`", source);
    println!("--------------------------------------------------------------------------------");

    while let Some(event) = capture.next()? {
        match event {
            CaptureEvent::Scan(event) => print_scan_event(&event, &cmd_config),
            CaptureEvent::Parse(event) => print_parse_event(&event, &cmd_config),
        }
    }

    Ok(())
}

fn print_scan_event(token: &scanner_core::Token, config: &CmdConfig) {
    if let Some(trivias) = token.leading_trivia.as_ref() {
        for (i, event) in trivias.iter().enumerate() { 
            println!("[{}]", apply_label_color(&format!("Scan/Leading#{}", i+1), config, Color::RGB(128, 128, 128)));
            println!("     (kind) name: {}, id: {}, (offset) {}, (len) {}", event.kind.text, event.kind.id, event.offset, event.len);
            println!("    (value) `{:?}`", event.value);
        }
    }

    {
            let event = &token.main;
            println!("[{}]", apply_label_color("Scan/Main", config, Color::Yellow));
            println!("     (kind) name: {}, id: {}, (offset) {}, (len) {}", event.kind.text, event.kind.id, event.offset, event.len);
            println!("    (value) {:?}", event.value);
    }

    if let Some(trivias) = token.trailing_trivia.as_ref() {
        for (i, event) in trivias.iter().enumerate() {
            println!("[{}]", apply_label_color(&format!("Scan/Trailing#{}", i+1), config, Color::RGB(128, 128, 128)));
            println!("     (kind) name: {}, id: {}, (offset) {}, (len) {}", event.kind.text, event.kind.id, event.offset, event.len);
            println!("    (value) {:?}", event.value);
        }
    }
}

fn print_parse_event(event: &ParseEvent, config: &CmdConfig) {
    match event {
        ParseEvent::Shift { kind, current_state, next_state, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Parse/Shift", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::Reduce { kind, current_state, next_state, pop_count, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Parse/Reduce", config));
            println!("     (kind) name: {}, id: {}, (pop_count) {}", kind.text, kind.id, pop_count);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::Emit { kind, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Parse/Emit", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) edit: {}", edit_state);
        }
        ParseEvent::Accept { kind, last_state, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Parse/Accept", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) last: {}, edit: {}", last_state, edit_state);
        }
        ParseEvent::RecoverDrop { kind, current_state, next_state, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Drop", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::RecoverShift { kind, current_state, next_state, edit_state, .. } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Shift", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::RecoverReduce { kind, current_state, next_state, pop_count, edit_state, .. } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Reduce", config));
            println!("     (kind) name: {}, id: {}, (pop_count) {}", kind.text, kind.id, pop_count);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
    }
}

fn apply_label_color(label: &str, config: &CmdConfig, color: ansi_term::Color) -> String {
    if config.no_color {
        return label.into();
    }

    color.paint(label).to_string()
}

fn apply_parse_event_color(event: &ParseEvent, label: &str, config: &CmdConfig) -> String {
    let color = match event {
        ParseEvent::RecoverDrop { .. } |
        ParseEvent::RecoverShift { .. } | 
        ParseEvent::RecoverReduce { .. } => {
            ansi_term::Color::Red
        }
        ParseEvent::Reduce { pop_count, .. } if *pop_count == 0 => {
            ansi_term::Color::RGB(128, 128, 128)
        }
        ParseEvent::Emit { .. } => {
            ansi_term::Color::Purple
        }
        _ => ansi_term::Color::Cyan,
    };

    apply_label_color(label, config, color)
}