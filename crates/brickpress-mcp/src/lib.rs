//! Brickpress for agents: a Model Context Protocol server.
//!
//! `brickpress mcp` serves over stdio. By default it edits projects headlessly
//! (files under `--root`); with `--live` it drives the running Brickpress
//! window through [`live`], so the person sees every change and can undo it.

pub mod live;
pub mod server;

use rmcp::ServiceExt;
use std::path::PathBuf;

pub struct Options {
    pub live: bool,
    pub root: PathBuf,
}

/// Runs the server until the client disconnects.
pub fn run_stdio(options: Options) -> anyhow::Result<()> {
    let backend = if options.live {
        server::Backend::Live(live::LiveClient::connect().map_err(anyhow::Error::msg)?)
    } else {
        let mut core = brickpress_core::EditorCore::default();
        core.new_document();
        server::Backend::Headless { core: Box::new(core), file: None }
    };
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(async move {
        let service = server::BrickpressMcp::new(backend, options.root).serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    })
}
