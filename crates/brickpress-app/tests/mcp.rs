//! Drives `brickpress mcp` over stdio exactly as an MCP client would.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Client {
    fn start(root: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_brickpress"))
            .args(["mcp", "--root"])
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start brickpress mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self { child, stdin, stdout, next: 1 };
        let init = client.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "brickpress-test", "version": "1" }
            }),
        );
        assert_eq!(init["serverInfo"]["name"], "brickpress");
        client.notify("notifications/initialized", json!({}));
        client
    }

    fn send(&mut self, message: Value) {
        let mut line = message.to_string();
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(self.stdout.read_line(&mut line).unwrap() > 0, "server closed stdout");
            let message: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("bad json {e}: {line}"));
            if message["id"] == json!(id) {
                if let Some(error) = message.get("error") {
                    panic!("{method} failed: {error}");
                }
                return message["result"].clone();
            }
        }
    }

    fn tool(&mut self, name: &str, arguments: Value) -> Value {
        let result = self.request("tools/call", json!({ "name": name, "arguments": arguments }));
        assert_ne!(result["isError"], json!(true), "{name}: {result}");
        result
    }

    fn tool_text(&mut self, name: &str, arguments: Value) -> String {
        self.tool(name, arguments)["content"][0]["text"].as_str().unwrap_or_default().to_string()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn an_agent_can_compose_preview_save_and_export() {
    let root = tempfile::tempdir().unwrap();
    let mut c = Client::start(root.path());

    let tools = c.request("tools/list", json!({}));
    let names: Vec<&str> = tools["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for expected in
        ["catalog_list", "pieces_place", "pieces_place_grid", "render_preview", "export_image", "document_save", "undo"]
    {
        assert!(names.contains(&expected), "missing tool {expected}: {names:?}");
    }

    let catalog: Value = serde_json::from_str(&c.tool_text("catalog_list", json!({ "category": "curve" }))).unwrap();
    assert!(catalog["count"].as_u64().unwrap() >= 5);

    c.tool("document_new", json!({ "name": "Agent letter", "width": 12, "height": 9 }));
    let placed: Value = serde_json::from_str(&c.tool_text(
        "pieces_place_grid",
        json!({ "x": 1, "y": 1, "rows": ["1###0", "#...#", "#####", "#...#", "#...#"] }),
    ))
    .unwrap();
    assert_eq!(placed["placed"], 16);
    let blocked: Value =
        serde_json::from_str(&c.tool_text("pieces_place", json!({ "pieces": [{ "piece": "3070", "x": 1, "y": 1 }] })))
            .unwrap();
    assert_eq!(blocked["placed"], 0);
    assert!(blocked["results"][0]["reason"].as_str().unwrap().contains("overlaps"));

    let ascii = c.tool_text("board_ascii", json!({}));
    assert!(ascii.lines().nth(1).unwrap().starts_with(".1AAA0"), "{ascii}");

    let preview = c.tool("render_preview", json!({ "mode": "print", "width": 320 }));
    assert_eq!(preview["content"][0]["type"], "image");
    assert_eq!(preview["content"][0]["mimeType"], "image/png");
    assert!(preview["content"][0]["data"].as_str().unwrap().len() > 1000);

    c.tool("document_save", json!({ "path": "letter.brickpress.json" }));
    let saved = std::fs::read_to_string(root.path().join("letter.brickpress.json")).unwrap();
    assert!(saved.contains("\"name\": \"Agent letter\""));
    c.tool("export_image", json!({ "path": "letter.png", "width": 600 }));
    assert!(std::fs::metadata(root.path().join("letter.png")).unwrap().len() > 1000);
    c.tool("export_image", json!({ "path": "letter.svg", "mode": "design", "width": 600 }));
    assert!(std::fs::read_to_string(root.path().join("letter.svg")).unwrap().starts_with("<svg"));

    // Paths outside the workspace are refused.
    let escape =
        c.request("tools/call", json!({ "name": "document_save", "arguments": { "path": "../outside.json" } }));
    assert_eq!(escape["isError"], json!(true));

    // Each tool call is one undo step.
    c.tool("undo", json!({}));
    let info: Value = serde_json::from_str(&c.tool_text("document_info", json!({}))).unwrap();
    assert_eq!(info["pieces"], 0, "undo removed the whole grid placement");
}
