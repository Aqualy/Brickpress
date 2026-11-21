//! Print and paper presets and the print controls' ranges.

use crate::document::{PaperSettings, PrintKey, PrintSettings};

pub const PRINT_DEFAULTS: PrintSettings = PrintSettings {
    ink_amount: 0.82,
    pressure: 0.6,
    ink_coverage: 0.93,
    roller_consistency: 0.72,
    piece_wear: 0.28,
    edge_wear: 0.24,
    paper_tooth: 0.35,
    texture_scale: 1.0,
    misalignment: 0.22,
    registration_error: 0.22,
    print_roughness: 0.32,
};

/// Built-in print presets, in menu order.
pub fn print_presets() -> Vec<(&'static str, PrintSettings)> {
    let d = PRINT_DEFAULTS;
    vec![
        (
            "Clean",
            PrintSettings {
                ink_amount: 1.0,
                pressure: 0.8,
                ink_coverage: 1.0,
                roller_consistency: 1.0,
                piece_wear: 0.0,
                edge_wear: 0.0,
                paper_tooth: 0.0,
                misalignment: 0.0,
                registration_error: 0.0,
                print_roughness: 0.0,
                ..d
            },
        ),
        (
            "Fresh Press",
            PrintSettings { ink_coverage: 0.97, piece_wear: 0.1, edge_wear: 0.08, print_roughness: 0.15, ..d },
        ),
        ("Normal", d),
        (
            "Worn",
            PrintSettings { piece_wear: 0.7, edge_wear: 0.65, roller_consistency: 0.5, print_roughness: 0.6, ..d },
        ),
        (
            "Dry Ink",
            PrintSettings {
                ink_amount: 0.46,
                ink_coverage: 0.84,
                roller_consistency: 0.38,
                paper_tooth: 0.65,
                print_roughness: 0.68,
                ..d
            },
        ),
        (
            "Heavy Pressure",
            PrintSettings {
                pressure: 0.95,
                ink_amount: 0.95,
                ink_coverage: 0.98,
                edge_wear: 0.12,
                print_roughness: 0.2,
                ..d
            },
        ),
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct PrintControl {
    pub key: PrintKey,
    pub id: &'static str,
    pub label: &'static str,
    pub min: f64,
    pub max: f64,
    pub help: &'static str,
}

const fn control(
    key: PrintKey,
    id: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    help: &'static str,
) -> PrintControl {
    PrintControl { key, id, label, min, max, help }
}

pub const PRINT_CONTROLS: [PrintControl; 11] = [
    control(
        PrintKey::InkAmount,
        "inkAmount",
        "Ink amount",
        0.0,
        1.0,
        "How much ink the roller lays down. Low values print pale and patchy.",
    ),
    control(
        PrintKey::Pressure,
        "pressure",
        "Pressure",
        0.0,
        1.0,
        "Press pressure. More pressure fills edges and deepens colour.",
    ),
    control(
        PrintKey::InkCoverage,
        "inkCoverage",
        "Ink coverage",
        0.7,
        1.0,
        "Share of each surface that takes ink. Lower values add dropout.",
    ),
    control(
        PrintKey::RollerConsistency,
        "rollerConsistency",
        "Roller consistency",
        0.0,
        1.0,
        "Evenness of the roller. Low values add broad light and dark patches.",
    ),
    control(PrintKey::PieceWear, "pieceWear", "Piece wear", 0.0, 1.0, "Scuffs and pits on the printing surfaces."),
    control(PrintKey::EdgeWear, "edgeWear", "Edge wear", 0.0, 1.0, "Rounds and erodes the edges of every piece."),
    control(PrintKey::PaperTooth, "paperTooth", "Paper tooth", 0.0, 1.0, "Paper texture showing through the ink."),
    control(
        PrintKey::TextureScale,
        "textureScale",
        "Texture scale",
        0.4,
        2.5,
        "Size of the grain and dropout texture.",
    ),
    control(
        PrintKey::Misalignment,
        "misalignment",
        "Misalignment",
        0.0,
        1.0,
        "Small shifts and turns of individual pieces.",
    ),
    control(
        PrintKey::RegistrationError,
        "registrationError",
        "Registration error",
        0.0,
        1.0,
        "Offset between ink passes, as on a multi-colour press run.",
    ),
    control(
        PrintKey::PrintRoughness,
        "printRoughness",
        "Print roughness",
        0.0,
        1.0,
        "Ragged, uneven edges where ink meets paper.",
    ),
];

pub fn print_control(key: PrintKey) -> &'static PrintControl {
    PRINT_CONTROLS.iter().find(|c| c.key == key).expect("every key has a control")
}

pub fn print_control_by_id(id: &str) -> Option<&'static PrintControl> {
    PRINT_CONTROLS.iter().find(|c| c.id == id)
}

/// Built-in paper stocks, in menu order. The seed is left to the caller.
pub fn paper_presets() -> Vec<PaperSettings> {
    let p = |preset: &str, color: &str, grain, tooth, fibers| PaperSettings {
        preset: preset.into(),
        color: color.into(),
        grain,
        tooth,
        fibers,
        brightness: 1.0,
        seed: 0,
    };
    vec![
        p("Bright White", "#ffffff", 0.1, 0.15, 0.08),
        p("Natural White", "#f6f4ec", 0.22, 0.3, 0.16),
        p("Warm Ivory", "#f1e8d4", 0.25, 0.35, 0.2),
        p("Recycled", "#d9d2bf", 0.5, 0.55, 0.7),
        p("Newsprint", "#e4dec9", 0.35, 0.3, 0.45),
        p("Heavy Cotton", "#f6f1e4", 0.4, 0.7, 0.3),
    ]
}

pub fn paper_preset(name: &str, seed: i64) -> Option<PaperSettings> {
    paper_presets().into_iter().find(|p| p.preset == name).map(|p| PaperSettings { seed, ..p })
}

/// Default ink swatches.
pub const STARTER_SWATCHES: [&str; 7] = ["#252a26", "#ce4936", "#315eae", "#3d634b", "#e3ae36", "#795186", "#e17b39"];
