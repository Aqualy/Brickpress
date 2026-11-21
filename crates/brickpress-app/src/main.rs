//! Brickpress: a studio for brick compositions and simulated letterpress prints.
//!
//! `brickpress [project]` opens the editor. `brickpress mcp [--live] [--root DIR]`
//! runs the Model Context Protocol server for agents over stdio.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use brickpress_app::{app, storage};
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "Brickpress — brick compositions and simulated letterpress prints

Usage:
  brickpress [PROJECT]              Open the editor (optionally a .brickpress.json file)
  brickpress mcp [--live] [--root DIR]
                                    Serve the Model Context Protocol over stdio.
                                    Headless by default: projects and exports are
                                    read and written under DIR (default: the current
                                    directory). With --live, drive the open
                                    Brickpress window instead (enable Help ▸ Allow
                                    Agent Connections there first).
  brickpress --version
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--help" | "-h") => {
            print!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Some("--version" | "-V") => {
            println!("brickpress {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("mcp") => return run_mcp(&args[1..]),
        _ => {}
    }
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,wgpu_hal=error,wgpu_core=error,brickpress=info"),
    )
    .init();
    let open = args.first().map(PathBuf::from).filter(|p| p.is_file());
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Brickpress")
        .with_app_id(storage::APP_ID)
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([960.0, 600.0]);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        persist_window: true,
        centered: true,
        ..Default::default()
    };
    match eframe::run_native("Brickpress", options, Box::new(move |cc| Ok(Box::new(app::BrickpressApp::new(cc, open)))))
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Brickpress could not start: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_mcp(args: &[String]) -> ExitCode {
    let mut live = false;
    let mut root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--live" => live = true,
            "--root" => {
                i += 1;
                match args.get(i) {
                    Some(dir) => root = PathBuf::from(dir),
                    None => {
                        eprintln!("--root needs a directory");
                        return ExitCode::FAILURE;
                    }
                }
            }
            other => {
                eprintln!("Unknown option {other}\n\n{HELP}");
                return ExitCode::FAILURE;
            }
        }
        i += 1;
    }
    // stdout carries the protocol; logs go to stderr only.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,wgpu_hal=error,wgpu_core=error"))
        .target(env_logger::Target::Stderr)
        .init();
    match brickpress_mcp::run_stdio(brickpress_mcp::Options { live, root }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("brickpress mcp: {e}");
            ExitCode::FAILURE
        }
    }
}
