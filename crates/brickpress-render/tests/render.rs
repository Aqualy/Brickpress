//! Renders real documents on a headless adapter. Set BRICKPRESS_ARTIFACTS to
//! a directory to keep the images for visual review.

use brickpress_core::persistence::document::create_document;
use brickpress_core::persistence::preferences::GridAppearance;
use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::board2d::Renderer2D;
use brickpress_render::export::{encode_png, render_rgba, ExportMode, ExportOptions, EXPORT_FORMAT};
use brickpress_render::gpu::Gpu;
use brickpress_render::three::camera::OrbitCamera;
use brickpress_render::three::{Renderer3D, Scene3D};
use std::collections::HashSet;
use std::sync::Arc;

fn save(name: &str, png: &[u8]) {
    if let Some(dir) = std::env::var_os("BRICKPRESS_ARTIFACTS") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(name), png).unwrap();
    }
}

fn gpu() -> Option<Gpu> {
    match Gpu::headless() {
        Ok(gpu) => Some(gpu),
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            None
        }
    }
}

#[test]
fn renders_design_and_print_exports() {
    let Some(gpu) = gpu() else { return };
    let renderer = Renderer2D::new(&gpu.device, &gpu.queue, EXPORT_FORMAT, Arc::new(ShapeAtlas::build()));
    let doc = create_document(true);
    for (mode, paper, grid) in [
        (ExportMode::Design, true, false),
        (ExportMode::Print, true, false),
        (ExportMode::Print, false, false),
        (ExportMode::Design, true, true),
    ] {
        let options = ExportOptions { mode, paper, grid, width: 1024 };
        let image = render_rgba(&gpu.device, &gpu.queue, &renderer, &doc, GridAppearance::Flat, &options).unwrap();
        assert_eq!((image.width, image.height), (1024, 1024));
        let distinct: HashSet<&[u8]> = image.pixels.chunks(4).collect();
        assert!(distinct.len() > 50, "{mode:?}: image looks blank ({} colours)", distinct.len());
        if !paper {
            assert_eq!(image.pixels[3], 0, "corner is transparent without paper");
        } else {
            assert_eq!(image.pixels[3], 255);
        }
        save(&format!("{}-paper{paper}-grid{grid}.png", mode.id()), &encode_png(&image).unwrap());
    }
}

#[test]
fn print_exports_are_deterministic() {
    let Some(gpu) = gpu() else { return };
    let renderer = Renderer2D::new(&gpu.device, &gpu.queue, EXPORT_FORMAT, Arc::new(ShapeAtlas::build()));
    let doc = create_document(true);
    let options = ExportOptions { mode: ExportMode::Print, paper: true, grid: false, width: 512 };
    let a = render_rgba(&gpu.device, &gpu.queue, &renderer, &doc, GridAppearance::Flat, &options).unwrap();
    let b = render_rgba(&gpu.device, &gpu.queue, &renderer, &doc, GridAppearance::Flat, &options).unwrap();
    assert!(a.pixels == b.pixels);
}

#[test]
fn large_exports_are_tiled_seamlessly() {
    let Some(gpu) = gpu() else { return };
    let renderer = Renderer2D::new(&gpu.device, &gpu.queue, EXPORT_FORMAT, Arc::new(ShapeAtlas::build()));
    let doc = create_document(true);
    let options = ExportOptions { mode: ExportMode::Design, paper: true, grid: false, width: 4100 };
    let image = render_rgba(&gpu.device, &gpu.queue, &renderer, &doc, GridAppearance::Embossed, &options).unwrap();
    assert_eq!(image.width, 4100);
    save("design-tiled.png", &encode_png(&image).unwrap());
}

#[test]
fn renders_the_3d_view() {
    let Some(gpu) = gpu() else { return };
    let mut renderer = Renderer3D::new(&gpu.device, EXPORT_FORMAT);
    let doc = create_document(true);
    let scene = Scene3D::build(&doc, &HashSet::new(), (0, 0), &[], true);
    renderer.upload(&gpu.device, &gpu.queue, &scene);
    let mut camera = OrbitCamera::default();
    let frame = camera.frame(scene.board, 960.0, 720.0, (64.0, 64.0));
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.render(&gpu.device, &gpu.queue, &mut encoder, [960, 720], &frame, scene.board, [0.0, 0.34, 0.85], true);
    let texture = renderer.resolved_texture().unwrap();
    let pixels = brickpress_render::export::read_texture(&gpu.device, &gpu.queue, encoder, texture, 960, 720).unwrap();
    let opaque = pixels.chunks(4).filter(|p| p[3] == 255).count();
    assert!(opaque > 960 * 720 / 5, "the board fills a good part of the view ({opaque})");
    let image = brickpress_render::export::RgbaImage { width: 960, height: 720, pixels };
    save("three.png", &encode_png(&image).unwrap());
}

#[test]
fn print_presets_gallery() {
    let Some(gpu) = gpu() else { return };
    if std::env::var_os("BRICKPRESS_ARTIFACTS").is_none() {
        return;
    }
    let renderer = Renderer2D::new(&gpu.device, &gpu.queue, EXPORT_FORMAT, Arc::new(ShapeAtlas::build()));
    for (name, settings) in brickpress_core::settings::print_presets() {
        let mut doc = create_document(true);
        doc.print_settings = settings;
        let options = ExportOptions { mode: ExportMode::Print, paper: true, grid: false, width: 700 };
        let image = render_rgba(&gpu.device, &gpu.queue, &renderer, &doc, GridAppearance::Flat, &options).unwrap();
        save(&format!("preset-{}.png", name.to_lowercase().replace(' ', "-")), &encode_png(&image).unwrap());
    }
}
