use std::path::PathBuf;
mod config;

use ansi_term::Color;
use config::CmdConfig;
use tolerant_parser_sdk::core::{parser_core::{capture::{CaptureEvent, ParseEventCapture}, event_dispatcher::ParseEvent}};
use tolerant_parser_sdk::core::scanner_core;

fn main() -> Result<(), anyhow::Error> {
    let cmd_config = {
        use clap::Parser;
        config::CmdConfig::parse()
    };

    let source = std::fs::read_to_string(PathBuf::from(cmd_config.input.clone()))?;
    let engine = sqlite_engine::create()?;
    let mut capture = ParseEventCapture::create(&source, cmd_config.to_capture_config(), engine)?;
    
    if ! cmd_config.quiet {
        println!("`{}`", source);
        println!("--------------------------------------------------------------------------------");
    }

    while let Some(event) = capture.next()? {
        if !cmd_config.quiet {
            match event {
                CaptureEvent::Scan(event) => print_scan_event(&event, &cmd_config),
                CaptureEvent::Parse(event) => print_parse_event(&event, &cmd_config),
            }
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
        ParseEvent::PatchDrop { kind, current_state, next_state, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Drop", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::PatchShift { kind, current_state, next_state, edit_state, .. } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Shift", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::PatchReduce { kind, current_state, next_state, pop_count, edit_state, .. } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Reduce", config));
            println!("     (kind) name: {}, id: {}, (pop_count) {}", kind.text, kind.id, pop_count);
            println!("    (state) current: {}, next: {}, edit: {}", current_state, next_state, edit_state);
        }
        ParseEvent::PatchEmit { kind, edit_state, .. } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Emit", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) edit: {}", edit_state);
        }
        ParseEvent::Invalid { kind, current_state, edit_state } => {
            println!("[{}]", apply_parse_event_color(event, "Recover/Invalid", config));
            println!("     (kind) name: {}, id: {}", kind.text, kind.id);
            println!("    (state) current: {}, edit: {}", current_state, edit_state);
        }
        ParseEvent::InvalidEmit { kind, edit_state, pop_count } => {
            println!("[{}]", apply_parse_event_color(event, "Parse/InvalidEmit", config));
            println!("     (kind) name: {}, id: {}, (pop_count) {}", kind.text, kind.id, pop_count);
            println!("    (state) edit: {}", edit_state);
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
        ParseEvent::PatchDrop { .. } |
        ParseEvent::PatchShift { .. } | 
        ParseEvent::PatchReduce { .. } |
        ParseEvent::PatchEmit { .. } => {
            ansi_term::Color::Red
        }
        ParseEvent::Invalid { .. } => {
            ansi_term::Color::RGB(128, 128, 128)
        }
        ParseEvent::Reduce { pop_count, .. } if *pop_count == 0 => {
            ansi_term::Color::RGB(128, 128, 128)
        }
        ParseEvent::Emit { .. } | ParseEvent::InvalidEmit { .. } => {
            ansi_term::Color::Purple
        }
        _ => ansi_term::Color::Cyan,
    };

    apply_label_color(label, config, color)
}