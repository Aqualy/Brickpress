//! Tracing-guide metadata. The image itself lives beside the state records and
//! never enters the project file or exports.

use serde::{Deserialize, Serialize};

pub const MAX_TRACE_BYTES: u64 = 20_000_000;
pub const MAX_PIXELS: u64 = 40_000_000;
pub const TRACE_MIMES: [&str; 6] = ["image/png", "image/jpeg", "image/webp", "image/gif", "image/avif", "image/bmp"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceMeta {
    pub asset_id: String,
    pub mime: String,
    pub name: String,
    pub image_width: f64,
    pub image_height: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    pub visible: bool,
    pub locked: bool,
}

pub const LAYOUT_ERROR: &str = "Use valid tracing coordinates and a positive width up to 1,024 studs.";

impl TraceMeta {
    pub fn validate(&self) -> Result<(), String> {
        let positive = [self.image_width, self.image_height, self.width, self.height];
        let ok = TRACE_MIMES.contains(&self.mime.as_str())
            && self.name.chars().count() <= 160
            && positive.iter().all(|n| n.is_finite() && *n > 0.0)
            && self.image_width * self.image_height <= MAX_PIXELS as f64
            && [self.x, self.y, self.opacity].iter().all(|n| n.is_finite())
            && (0.0..=1.0).contains(&self.opacity);
        if ok {
            Ok(())
        } else {
            Err("The saved tracing guide is invalid. Upload it again.".into())
        }
    }

    /// Fits the image inside the board, centred.
    pub fn fit(&mut self, board_width: f64, board_height: f64) {
        let scale = (board_width / self.image_width).min(board_height / self.image_height);
        self.width = self.image_width * scale;
        self.height = self.image_height * scale;
        self.center(board_width, board_height);
    }

    pub fn center(&mut self, board_width: f64, board_height: f64) {
        self.x = (board_width - self.width) / 2.0;
        self.y = (board_height - self.height) / 2.0;
    }

    /// Applies a layout change, keeping the aspect ratio when the width changes.
    pub fn with_layout(&self, x: f64, y: f64, width: f64, opacity: f64) -> Result<TraceMeta, String> {
        let height = width * self.image_height / self.image_width;
        let finite = [x, y, width, height, opacity].iter().all(|n| n.is_finite());
        if !finite
            || width <= 0.0
            || width > 1024.0
            || height > 1024.0
            || x.abs() > 1024.0
            || y.abs() > 1024.0
            || !(0.0..=1.0).contains(&opacity)
        {
            return Err(LAYOUT_ERROR.into());
        }
        Ok(TraceMeta { x, y, width, height, opacity, ..self.clone() })
    }
}

pub fn mime_for_extension(ext: &str) -> Option<&'static str> {
    Some(match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> TraceMeta {
        TraceMeta {
            asset_id: "a".into(),
            mime: "image/png".into(),
            name: "guide.png".into(),
            image_width: 200.0,
            image_height: 100.0,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            opacity: 0.4,
            visible: true,
            locked: true,
        }
    }

    #[test]
    fn fits_and_keeps_aspect() {
        let mut m = meta();
        m.fit(16.0, 16.0);
        assert_eq!((m.width, m.height, m.x, m.y), (16.0, 8.0, 0.0, 4.0));
        let wider = m.with_layout(1.0, 2.0, 10.0, 0.5).unwrap();
        assert_eq!(wider.height, 5.0);
        assert!(m.with_layout(0.0, 0.0, 2000.0, 0.5).is_err());
        assert!(m.with_layout(0.0, 0.0, 2.0, 1.5).is_err());
        assert!(m.validate().is_ok());
        assert!(TraceMeta { mime: "text/plain".into(), ..m }.validate().is_err());
    }
}
