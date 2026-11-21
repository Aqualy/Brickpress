//! Orbit camera for the 3D view (port of 1.x `three/camera.ts`).
//!
//! The camera distance depends only on the board's size, never its angle, and
//! zoom changes the projection. Orbit and pan therefore never change scale.
//! At 100 % both projections show 32 px per stud on the target's depth plane.

use glam::{Mat4, Vec3, Vec4Swizzles};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Projection {
    Perspective,
    Orthographic,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom {
    Fit,
    /// 1.0 = 100 %.
    Scale(f32),
}

#[derive(Clone, Copy, Debug)]
pub enum CameraAction {
    Top,
    Isometric,
    Reset,
    Left,
    Right,
    Up,
    Down,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
}

pub const FOV_DEGREES: f32 = 40.0;
pub const MIN_ELEVATION: f32 = 0.17;
pub const MAX_ELEVATION: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;
pub const MIN_SCALE: f32 = 0.03;
pub const MAX_SCALE: f32 = 6.0;

#[derive(Clone, Copy, Debug)]
pub struct OrbitCamera {
    pub azimuth: f32,
    pub elevation: f32,
    pub target: Vec3,
    pub projection: Projection,
    pub zoom: Zoom,
    /// Scale actually in use (after fitting and clamping).
    pub scale: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            azimuth: std::f32::consts::FRAC_PI_4,
            elevation: std::f32::consts::FRAC_PI_3,
            target: Vec3::ZERO,
            projection: Projection::Perspective,
            zoom: Zoom::Fit,
            scale: 1.0,
        }
    }
}

/// View and projection for one frame.
#[derive(Clone, Copy, Debug)]
pub struct CameraFrame {
    pub view: Mat4,
    pub projection: Mat4,
    pub eye: Vec3,
    pub scale: f32,
    pub distance: f32,
    /// Perspective: tan(fov / 2) horizontally and vertically. Orthographic:
    /// half extents in world units. Used for exact rays.
    pub half: (f32, f32),
    pub perspective: bool,
}

impl CameraFrame {
    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }
    /// World → pixel coordinates (origin top-left) in a `width`×`height` target.
    pub fn project(&self, p: Vec3, width: f32, height: f32) -> (f32, f32, f32) {
        let clip = self.view_projection() * p.extend(1.0);
        let ndc = clip.xyz() / clip.w;
        ((ndc.x + 1.0) * 0.5 * width, (1.0 - ndc.y) * 0.5 * height, ndc.z)
    }
    /// A world-space ray through a pixel, computed in view space (no matrix
    /// inversion, which loses precision in f32).
    pub fn ray(&self, x: f32, y: f32, width: f32, height: f32) -> (Vec3, Vec3) {
        let ndc_x = x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - y / height * 2.0;
        let world = self.view.inverse();
        if self.perspective {
            let dir = Vec3::new(ndc_x * self.half.0, ndc_y * self.half.1, -1.0);
            (self.eye, world.transform_vector3(dir).normalize())
        } else {
            let origin = world.transform_point3(Vec3::new(ndc_x * self.half.0, ndc_y * self.half.1, 0.0));
            (origin, world.transform_vector3(Vec3::NEG_Z).normalize())
        }
    }
    /// Where a pixel's ray meets the horizontal plane at `height`.
    pub fn intersect_plane(&self, x: f32, y: f32, width: f32, height: f32, plane_y: f32) -> Option<Vec3> {
        let (origin, dir) = self.ray(x, y, width, height);
        if dir.y.abs() < 1e-6 {
            return None;
        }
        let t = (plane_y - origin.y) / dir.y;
        (t > 0.0).then(|| origin + dir * t)
    }
    pub fn right(&self) -> Vec3 {
        self.view.inverse().x_axis.truncate()
    }
    pub fn up(&self) -> Vec3 {
        self.view.inverse().y_axis.truncate()
    }
}

impl OrbitCamera {
    fn distance(board: (f32, f32)) -> f32 {
        board.0.hypot(board.1).max(20.0) * 1.5
    }

    fn view(&self, distance: f32) -> (Mat4, Vec3) {
        let eye = self.target
            + Vec3::new(
                self.azimuth.sin() * self.elevation.cos(),
                self.elevation.sin(),
                self.azimuth.cos() * self.elevation.cos(),
            ) * distance;
        (Mat4::look_at_rh(eye, self.target, Vec3::Y), eye)
    }

    /// Frames the board in a `width`×`height` target. `margins` are the pixels
    /// kept clear when fitting (for overlays).
    pub fn frame(&mut self, board: (f32, f32), width: f32, height: f32, margins: (f32, f32)) -> CameraFrame {
        let distance = Self::distance(board);
        let (view, eye) = self.view(distance);
        let mut scale = match self.zoom {
            Zoom::Fit => 1.0,
            Zoom::Scale(s) => s,
        };
        if self.zoom == Zoom::Fit && board.0 > 0.0 && board.1 > 0.0 {
            let (mut half_w, mut half_h) = (0.0f32, 0.0f32);
            for x in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    for y in [0.0, 1.0] {
                        let p = view.transform_point3(Vec3::new(x * board.0 / 2.0, y, z * board.1 / 2.0));
                        let depth = if self.projection == Projection::Perspective { distance / -p.z } else { 1.0 };
                        half_w = half_w.max((p.x * depth).abs());
                        half_h = half_h.max((p.y * depth).abs());
                    }
                }
            }
            scale = ((width - margins.0) / (half_w * 64.0)).min((height - margins.1) / (half_h * 64.0));
        }
        let scale = scale.clamp(MIN_SCALE, MAX_SCALE);
        self.scale = scale;
        let aspect = width / height.max(1.0);
        let (projection, half) = match self.projection {
            Projection::Perspective => {
                let zoom = 64.0 * scale * distance * (FOV_DEGREES.to_radians() / 2.0).tan() / height;
                let tan = (FOV_DEGREES.to_radians() / 2.0).tan() / zoom;
                (Mat4::perspective_rh(2.0 * tan.atan(), aspect, 0.1, 2000.0), (tan * aspect, tan))
            }
            Projection::Orthographic => {
                let (hw, hh) = (width / (64.0 * scale), height / (64.0 * scale));
                (Mat4::orthographic_rh(-hw, hw, -hh, hh, 0.1, 2000.0), (hw, hh))
            }
        };
        let perspective = self.projection == Projection::Perspective;
        CameraFrame { view, projection, eye, scale, distance, half, perspective }
    }

    /// Fixes the fitted scale so orbiting does not refit.
    pub fn freeze_fit(&mut self) {
        if self.zoom == Zoom::Fit {
            self.zoom = Zoom::Scale(self.scale);
        }
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.freeze_fit();
        self.azimuth -= dx * 0.008;
        self.elevation = (self.elevation + dy * 0.008).clamp(MIN_ELEVATION, MAX_ELEVATION);
    }

    pub fn pan(&mut self, frame: &CameraFrame, dx: f32, dy: f32) {
        self.freeze_fit();
        let scale = 1.0 / (32.0 * self.scale);
        self.target += frame.right() * (-dx * scale) + frame.up() * (dy * scale);
    }

    pub fn set_zoom(&mut self, zoom: Zoom) {
        if zoom == Zoom::Fit {
            self.target = Vec3::ZERO;
        }
        self.zoom = zoom;
    }

    pub fn action(&mut self, frame: &CameraFrame, action: CameraAction) {
        match action {
            CameraAction::Top | CameraAction::Isometric | CameraAction::Reset => {
                let top = matches!(action, CameraAction::Top);
                self.azimuth = if top { 0.0 } else { std::f32::consts::FRAC_PI_4 };
                self.elevation = if top { MAX_ELEVATION } else { std::f32::consts::FRAC_PI_3 };
                self.target = Vec3::ZERO;
                self.zoom = Zoom::Fit;
            }
            CameraAction::PanLeft => self.pan(frame, 32.0, 0.0),
            CameraAction::PanRight => self.pan(frame, -32.0, 0.0),
            CameraAction::PanUp => self.pan(frame, 0.0, 32.0),
            CameraAction::PanDown => self.pan(frame, 0.0, -32.0),
            CameraAction::Left => self.orbit(24.0, 0.0),
            CameraAction::Right => self.orbit(-24.0, 0.0),
            CameraAction::Up => self.orbit(0.0, 16.0),
            CameraAction::Down => self.orbit(0.0, -16.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: (f32, f32) = (16.0, 24.0);
    const W: f32 = 800.0;
    const H: f32 = 700.0;
    const MARGINS: (f32, f32) = (64.0, 170.0);

    fn span(f: &CameraFrame, a: Vec3, b: Vec3) -> f32 {
        let (px, py, _) = f.project(a, W, H);
        let (qx, qy, _) = f.project(b, W, H);
        (qx - px).hypot(qy - py)
    }

    #[test]
    fn fits_without_clipping_and_holds_scale_through_orbit_and_pan() {
        for projection in [Projection::Perspective, Projection::Orthographic] {
            let mut cam = OrbitCamera { projection, ..Default::default() };
            let f = cam.frame(BOARD, W, H, MARGINS);
            let zoom = f.scale;
            for x in [-8.0, 8.0] {
                for z in [-12.0, 12.0] {
                    for y in [0.0, 1.0] {
                        let clip = f.view_projection() * Vec3::new(x, y, z).extend(1.0);
                        let ndc = clip.xyz() / clip.w;
                        assert!(ndc.x.abs() <= (W - 64.0) / W + 1e-4, "{projection:?} {ndc}");
                        assert!(ndc.y.abs() <= (H - 170.0) / H + 1e-4);
                        assert!(ndc.z > 0.0 && ndc.z < 1.0);
                    }
                }
            }
            let distance = f.eye.length();
            for (azimuth, elevation) in [(0.0, 0.17), (2.0, 0.4), (-4.0, 1.3), (7.0, MAX_ELEVATION)] {
                cam.azimuth = azimuth;
                cam.elevation = elevation;
                cam.target = Vec3::new(3.0, 2.0, -4.0);
                cam.zoom = Zoom::Scale(zoom);
                let f = cam.frame(BOARD, W, H, MARGINS);
                assert!((f.scale - zoom).abs() < 1e-6);
                assert!(((f.eye - cam.target).length() - distance).abs() < 1e-3);
                let s = span(&f, cam.target, cam.target + f.right());
                assert!((s - 32.0 * zoom).abs() < 1e-2, "{projection:?}: {s} vs {}", 32.0 * zoom);
            }
        }
    }

    #[test]
    fn round_trips_surfaces_between_screen_and_rays() {
        for projection in [Projection::Perspective, Projection::Orthographic] {
            let mut cam = OrbitCamera {
                azimuth: 1.2,
                elevation: 0.6,
                target: Vec3::new(2.0, 0.0, -1.0),
                projection,
                zoom: Zoom::Scale(1.5),
                scale: 1.0,
            };
            let f = cam.frame(BOARD, W, H, MARGINS);
            for x in [-5.0, 0.0, 6.0] {
                for z in [-8.0, 0.0, 9.0] {
                    let world = Vec3::new(x, 0.4, z);
                    let (px, py, _) = f.project(world, W, H);
                    let hit = f.intersect_plane(px, py, W, H, 0.4).unwrap();
                    assert!(hit.distance(world) < 1e-3, "{projection:?} {hit} {world}");
                }
            }
        }
    }

    #[test]
    fn perspective_scales_with_depth_orthographic_does_not() {
        for projection in [Projection::Perspective, Projection::Orthographic] {
            let mut cam =
                OrbitCamera { azimuth: 0.0, elevation: 0.8, projection, zoom: Zoom::Scale(1.0), ..Default::default() };
            let f = cam.frame(BOARD, W, H, MARGINS);
            let forward = (cam.target - f.eye).normalize();
            let (near, far) = (forward * -3.0, forward * 3.0);
            let ratio = span(&f, near, near + f.right()) / span(&f, far, far + f.right());
            let expected =
                if projection == Projection::Perspective { (f.distance + 3.0) / (f.distance - 3.0) } else { 1.0 };
            assert!((ratio - expected).abs() < 1e-3, "{projection:?} {ratio} {expected}");
        }
    }
}
