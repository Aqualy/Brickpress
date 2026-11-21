//! Renders a few reference-like scenes in print mode, for tuning the print
//! engine by eye: `cargo run --release -p brickpress-render --example print_lab -- OUT_DIR [px_per_stud]`.

use brickpress_core::persistence::document::create_document;
use brickpress_core::persistence::preferences::GridAppearance;
use brickpress_core::{Board, PlacedPiece, PressDocument};
use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::board2d::Renderer2D;
use brickpress_render::export::{encode_png, render_rgba, ExportMode, ExportOptions, EXPORT_FORMAT};
use brickpress_render::gpu::Gpu;
use std::sync::Arc;

fn scene(name: &str, w: i32, h: i32, paper: &str, passes: &[(&str, &[&str])]) -> PressDocument {
    let mut doc = create_document(false);
    doc.name = name.into();
    doc.board = Board { width: w, height: h };
    doc.paper.color = paper.into();
    doc.passes.truncate(1);
    let template = doc.passes[0].clone();
    doc.passes.clear();
    let mut seed = 1000i64;
    for (pi, (color, rows)) in passes.iter().enumerate() {
        let mut pass = template.clone();
        pass.id = format!("p{pi}").into();
        pass.color = (*color).into();
        pass.registration_seed = 77 + pi as i64 * 991;
        let mut pieces = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let (id, rot) = match c {
                    '#' => ("3070", 0),
                    'o' => ("98138", 0),
                    '0'..='3' => ("25269", (c as i32 - '0' as i32) * 90),
                    'Q' => ("3068", 0),
                    'R' => ("14769", 0),
                    'a'..='d' => ("27925", (c as i32 - 'a' as i32) * 90),
                    'L' => ("87079", 0),
                    _ => continue,
                };
                seed = (seed * 48271) % 2_147_483_647;
                pieces.push(PlacedPiece::new(format!("{pi}-{x}-{y}"), id, x as i32, y as i32, rot, seed));
            }
        }
        pass.pieces = Arc::new(pieces);
        doc.passes.push(pass);
    }
    doc
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().expect("output directory"));
    let px: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(48);
    let only: Option<String> = args.next();
    std::fs::create_dir_all(&out).unwrap();
    let gpu = Gpu::headless().expect("gpu");
    let renderer = Renderer2D::new(&gpu.device, &gpu.queue, EXPORT_FORMAT, Arc::new(ShapeAtlas::build()));

    let peace = scene(
        "peace",
        16,
        11,
        "#efe1c6",
        &[(
            "#151515",
            &[
                "################",
                "#oooo###o#2##0#1",
                "####oo####o#0o##",
                "##1#o##ooo#o##o#",
                "#oo#o###2#0#oo##",
                "################",
                "#oooo#o#oo#2#0##",
                "##3##o##o###o##o",
                "#o##oo#o##o#oo##",
                "########o##1##0#",
                "################",
            ],
        )],
    );
    let loud = scene(
        "loud",
        14,
        8,
        "#f3dc2a",
        &[(
            "#c41f28",
            &[
                "#...###.#..#.#",
                "#...#.#.#..#.#",
                "#...#.#.#..#.#",
                "###.###.####.#",
                "..............",
                "1###0..####...",
                "#...#..#...#..",
                "2###3..####...",
            ],
        )],
    );
    let onward = scene(
        "onward",
        12,
        8,
        "#f7f6f0",
        &[
            (
                "#a6d8d0",
                &[
                    "a-b-a-b-a-b-",
                    "------------",
                    "d-c-d-c-d-c-",
                    "------------",
                    "a-b-a-b-a-b-",
                    "------------",
                    "d-c-d-c-d-c-",
                ],
            ),
            (
                "#d8282a",
                &["..#..#..####", "..#..#..#...", "..####..####", "..#..#..#...", "..#..#..####", "............"],
            ),
        ],
    );
    let bug = scene(
        "bug",
        10,
        10,
        "#fbfbf8",
        &[(
            "#121212",
            &[
                "a-##b-....",
                "--##--....",
                "o.Q-o.....",
                "..--......",
                "L---L---..",
                "--------..",
                "R-#R-#R-..",
                "--#--#--..",
                "1##0.1##0.",
                "#..#.#..#.",
            ],
        )],
    );
    let scenes = [("peace", peace), ("loud", loud), ("onward", onward), ("bug", bug)];
    for (name, settings) in brickpress_core::settings::print_presets() {
        let slug = name.to_lowercase().replace(' ', "-");
        if let Some(o) = &only {
            if !o.split(',').any(|o| o == slug) {
                continue;
            }
        }
        for (scene_name, doc) in &scenes {
            let mut doc = doc.clone();
            doc.print_settings = settings;
            let options =
                ExportOptions { mode: ExportMode::Print, paper: true, grid: false, width: px * doc.board.width as u32 };
            let image = render_rgba(&gpu.device, &gpu.queue, &renderer, &doc, GridAppearance::Flat, &options).unwrap();
            std::fs::write(out.join(format!("{scene_name}-{slug}.png")), encode_png(&image).unwrap()).unwrap();
        }
    }
}
