//! Live agent connections: the GUI side of `brickpress mcp --live`.
//!
//! Off by default. When enabled, a per-user local socket accepts requests; each
//! is handed to the UI thread, applied through the same editing core as the
//! mouse (one undo step per request), and answered.

use crate::app::BrickpressApp;
use brickpress_core::ops::{self, Op, OpResult};
use brickpress_mcp::live::{socket_name, Request, Response};
use interprocess::local_socket::{prelude::*, ListenerOptions, Stream};
use std::io::{BufRead, BufReader, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct AgentRequest {
    pub op: Op,
    pub reply: Sender<OpResult>,
}

#[derive(Default)]
pub struct AgentServer {
    enabled: bool,
    running: Option<Arc<AtomicBool>>,
    inbox: Option<Receiver<AgentRequest>>,
    connections: Arc<AtomicUsize>,
    pub last_activity: Option<Instant>,
    pub error: Option<String>,
}

impl AgentServer {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::Relaxed)
    }

    /// Recently changed by an agent (for the status indicator).
    pub fn active(&self) -> bool {
        self.last_activity.is_some_and(|t| t.elapsed() < Duration::from_secs(2))
    }

    pub fn start(&mut self, ctx: &egui::Context) {
        if self.running.is_some() {
            return;
        }
        let name = match socket_name() {
            Ok(n) => n,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        let listener = match ListenerOptions::new().name(name).create_sync() {
            Ok(l) => l,
            Err(e) => {
                self.error =
                    Some(format!("Agent connections could not start: {e}. Is another Brickpress window listening?"));
                self.enabled = false;
                return;
            }
        };
        self.error = None;
        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = channel::<AgentRequest>();
        self.inbox = Some(rx);
        self.running = Some(running.clone());
        let connections = self.connections.clone();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("brickpress-agents".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let tx = tx.clone();
                    let ctx = ctx.clone();
                    let running = running.clone();
                    let connections = connections.clone();
                    std::thread::spawn(move || {
                        connections.fetch_add(1, Ordering::Relaxed);
                        serve(stream, tx, &ctx, &running);
                        connections.fetch_sub(1, Ordering::Relaxed);
                        ctx.request_repaint();
                    });
                }
            })
            .expect("spawn agent listener");
    }

    pub fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            running.store(false, Ordering::Relaxed);
            // Wake the accept loop so it notices.
            if let Ok(name) = socket_name() {
                let _ = Stream::connect(name);
            }
        }
        self.inbox = None;
    }

    pub fn drain(&mut self) -> Vec<AgentRequest> {
        let Some(rx) = &self.inbox else { return vec![] };
        let requests: Vec<AgentRequest> = rx.try_iter().collect();
        if !requests.is_empty() {
            self.last_activity = Some(Instant::now());
        }
        requests
    }
}

fn serve(stream: Stream, tx: Sender<AgentRequest>, ctx: &egui::Context, running: &AtomicBool) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if !running.load(Ordering::Relaxed) {
            return;
        }
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => {
                let (reply_tx, reply_rx) = channel();
                if tx.send(AgentRequest { op: request.op, reply: reply_tx }).is_err() {
                    return;
                }
                ctx.request_repaint();
                let result = reply_rx
                    .recv_timeout(Duration::from_secs(30))
                    .unwrap_or_else(|_| Err("Brickpress did not answer in time.".into()));
                Response::from_result(request.id, result)
            }
            Err(e) => Response::from_result(0, Err(format!("Malformed request: {e}"))),
        };
        let mut text = serde_json::to_string(&response).unwrap_or_default();
        text.push('\n');
        if reader.get_mut().write_all(text.as_bytes()).is_err() {
            return;
        }
    }
}

/// Applies an agent operation on the UI thread.
pub fn handle(app: &mut BrickpressApp, op: Op) -> OpResult {
    if !op.is_read_only() && app.modal_open() {
        return Err("The person has a dialog open in Brickpress. Try again in a moment.".into());
    }
    // Agents edit the document in the active tab; each request is one undo step there.
    let Some(tab) = app.active_tab_mut() else {
        return Err("No project is open in Brickpress. Ask the person to create or open one.".into());
    };
    let result = ops::apply(&mut tab.core, op);
    // Messages meant for a person (toasts) are not shown for agent edits.
    tab.core.take_notice();
    result
}
