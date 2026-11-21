//! Timing on the largest documents (10,000 pieces). Run in release:
//! `cargo test --release -p brickpress-render --test perf -- --nocapture`

use brickpress_core::persistence::document::create_document;
use brickpress_core::{Board, EditorCore, PlacedPiece, Tool};
use brickpress_render::atlas::ShapeAtlas;
use brickpress_render::scene::{build_pieces, hit_test, Overlay, RenderMode};
use std::sync::Arc;
use std::time::Instant;

fn full_board() -> EditorCore {
    let mut doc = create_document(false);
    doc.board = Board { width: 128, height: 128 };
    let pieces: Vec<PlacedPiece> = (0..100)
        .flat_map(|y| {
            (0..100).map(move |x| {
                PlacedPiece::new(
                    format!("p{x}-{y}"),
                    if (x + y) % 3 == 0 { "25269" } else { "3070" },
                    x,
                    y,
                    ((x * 7 + y) % 4) * 90,
                    (x * 131 + y) as i64,
                )
            })
        })
        .collect();
    doc.passes[0].pieces = Arc::new(pieces);
    EditorCore::new(doc)
}

fn time<R>(label: &str, runs: u32, mut f: impl FnMut() -> R) -> R {
    let mut out = f();
    let start = Instant::now();
    for _ in 0..runs {
        out = f();
    }
    let per = start.elapsed() / runs;
    println!("{label:<46} {per:>10.2?}");
    out
}

#[test]
fn large_documents_stay_interactive() {
    let atlas = ShapeAtlas::build();
    let mut core = full_board();
    assert_eq!(core.doc().piece_count(), 10_000);
    core.choose_piece("3070");
    time("hover validity (cached index)", 200, || core.placement_problem(110, 110));
    core.set_tool(Tool::Select);
    let instances = time("instances, design (10k pieces)", 20, || {
        build_pieces(&atlas, core.doc(), RenderMode::Design, &Overlay::default())
    });
    assert_eq!(instances.len(), 10_000);
    time("instances, print (10k pieces)", 20, || {
        build_pieces(&atlas, core.doc(), RenderMode::Print, &Overlay::default())
    });
    time("hit test (exact surface)", 200, || hit_test(&atlas, core.doc(), 50.5, 50.5).map(|(_, p)| p.uid.clone()));
    let ids: Vec<_> = core.doc().passes[0].pieces.iter().take(1000).map(|p| p.uid.clone()).collect();
    core.select_many(ids, false);
    time("drag validity, 1,000 selected", 20, || core.can_move_selected(0, 0));
    let start = Instant::now();
    assert!(core.move_selected(0, 100) || core.move_selected(20, 0));
    println!("{:<46} {:>10.2?}", "move 1,000 pieces (one commit)", start.elapsed());
    let start = Instant::now();
    core.undo();
    println!("{:<46} {:>10.2?}", "undo", start.elapsed());
}
