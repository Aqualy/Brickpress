//! The live connection between `brickpress mcp --live` and a running GUI.
//!
//! A per-user local socket (a named pipe on Windows, a Unix socket elsewhere)
//! carries newline-delimited JSON: `{"id":1,"op":{...}}` requests and
//! `{"id":1,"ok":...}` or `{"id":1,"error":"..."}` replies. The GUI only
//! listens while "Allow agent connections" is switched on.

use brickpress_core::ops::{Op, OpResult};
use interprocess::local_socket::{prelude::*, GenericFilePath, GenericNamespaced, Name, Stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};

pub const PROTOCOL: u32 = 1;

/// The per-user socket name.
pub fn socket_name() -> std::io::Result<Name<'static>> {
    let user = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_else(|_| "user".into());
    let user: String = user.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    // Tests (and side-by-side instances) can choose their own socket.
    let base = std::env::var("BRICKPRESS_SOCKET").unwrap_or_else(|_| format!("brickpress-{user}.sock"));
    if GenericNamespaced::is_supported() {
        base.to_ns_name::<GenericNamespaced>().map(|n| n.into_owned())
    } else {
        let dir =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
        format!("{dir}/{base}").to_fs_name::<GenericFilePath>().map(|n| n.into_owned())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    pub op: Op,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn from_result(id: u64, result: OpResult) -> Self {
        match result {
            Ok(value) => Self { id, ok: Some(value), error: None },
            Err(error) => Self { id, ok: None, error: Some(error) },
        }
    }
    pub fn into_result(self) -> OpResult {
        match (self.ok, self.error) {
            (_, Some(e)) => Err(e),
            (Some(v), None) => Ok(v),
            (None, None) => Ok(Value::Null),
        }
    }
}

/// Client side, used by the MCP server in `--live` mode.
pub struct LiveClient {
    reader: BufReader<Stream>,
    next: u64,
}

impl LiveClient {
    pub fn connect() -> Result<Self, String> {
        let name = socket_name().map_err(|e| e.to_string())?;
        let stream = Stream::connect(name).map_err(|_| {
            "Brickpress is not running, or agent connections are off. In Brickpress, choose Help ▸ Allow Agent Connections.".to_string()
        })?;
        Ok(Self { reader: BufReader::new(stream), next: 1 })
    }

    pub fn call(&mut self, op: Op) -> OpResult {
        let id = self.next;
        self.next += 1;
        let mut line = serde_json::to_string(&Request { id, op }).map_err(|e| e.to_string())?;
        line.push('\n');
        self.reader
            .get_mut()
            .write_all(line.as_bytes())
            .map_err(|e| format!("The connection to Brickpress was lost: {e}"))?;
        let mut reply = String::new();
        let n = self.reader.read_line(&mut reply).map_err(|e| format!("The connection to Brickpress was lost: {e}"))?;
        if n == 0 {
            return Err(
                "Brickpress closed the connection (agent connections were switched off or the app quit).".into()
            );
        }
        let response: Response = serde_json::from_str(&reply).map_err(|e| e.to_string())?;
        response.into_result()
    }
}
