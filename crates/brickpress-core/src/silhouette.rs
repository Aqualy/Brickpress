//! Exact surface geometry: SVG path flattening and the narrow-phase overlap test.
//!
//! Touching edges do not count as overlap, so fitted curved pieces (a quarter
//! round inside a quarter-ring cutout) can sit together with overlap disabled.

use crate::catalog::FillRule;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

const fn pt(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[derive(Clone, Debug)]
pub struct Surface {
    pub contours: Vec<Vec<Point>>,
    pub fill_rule: FillRule,
}

const EPSILON: f64 = 1e-9;
/// Maximum arc chord error in studs: 0.0008 mm at the nominal stud pitch.
pub const ARC_TOLERANCE: f64 = 1e-4;

fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// SVG endpoint-to-centre arc conversion, including out-of-range radius correction.
/// <https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter>
pub fn arc_points(start: Point, end: Point, args: &[f64], tolerance: f64) -> Vec<Point> {
    let (mut rx, mut ry, degrees, large_arc, sweep) =
        (args[0].abs(), args[1].abs(), args[2], args[3] != 0.0, args[4] != 0.0);
    if rx == 0.0 || ry == 0.0 {
        return vec![end];
    }
    if start == end {
        return vec![];
    }
    let phi = degrees.to_radians();
    let (sin, cos) = phi.sin_cos();
    let dx = (start.x - end.x) / 2.0;
    let dy = (start.y - end.y) / 2.0;
    let x = cos * dx + sin * dy;
    let y = -sin * dx + cos * dy;
    let correction = (x * x) / (rx * rx) + (y * y) / (ry * ry);
    if correction > 1.0 {
        rx *= correction.sqrt();
        ry *= correction.sqrt();
    }
    let (rx2, ry2) = (rx * rx, ry * ry);
    let factor = if large_arc == sweep { -1.0 } else { 1.0 }
        * ((rx2 * ry2 - rx2 * y * y - ry2 * x * x) / (rx2 * y * y + ry2 * x * x)).max(0.0).sqrt();
    let cx = factor * rx * y / ry;
    let cy = -factor * ry * x / rx;
    let center = pt(cos * cx - sin * cy + (start.x + end.x) / 2.0, sin * cx + cos * cy + (start.y + end.y) / 2.0);
    let u = pt((x - cx) / rx, (y - cy) / ry);
    let v = pt((-x - cx) / rx, (-y - cy) / ry);
    let theta = u.y.atan2(u.x);
    let mut delta = (u.x * v.y - u.y * v.x).atan2(u.x * v.x + u.y * v.y);
    if !sweep && delta > 0.0 {
        delta -= std::f64::consts::TAU;
    }
    if sweep && delta < 0.0 {
        delta += std::f64::consts::TAU;
    }
    let step = 2.0 * (1.0 - tolerance / rx.max(ry)).max(-1.0).acos();
    let count = ((delta.abs() / step).ceil() as usize).max(1);
    (0..count)
        .map(|i| {
            if i == count - 1 {
                return end;
            }
            let angle = theta + delta * (i + 1) as f64 / count as f64;
            let (px, py) = (rx * angle.cos(), ry * angle.sin());
            pt(center.x + cos * px - sin * py, center.y + sin * px + cos * py)
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum PathError {
    Unsupported,
    Invalid,
    Empty,
}

fn tokenize(path: &str) -> Vec<&str> {
    // Letters, or numbers of the form [-+]?(\d*\.?\d+)([eE][-+]?\d+)?
    let bytes = path.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() {
            tokens.push(&path[i..i + 1]);
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i;
        if j < bytes.len() && (bytes[j] == b'-' || bytes[j] == b'+') {
            j += 1;
        }
        let digits_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == b'.' {
            let k = j + 1;
            let mut m = k;
            while m < bytes.len() && bytes[m].is_ascii_digit() {
                m += 1;
            }
            if m > k {
                j = m;
            }
        }
        if j == digits_start {
            i = start + 1;
            continue;
        }
        if j < bytes.len() && (bytes[j] == b'e' || bytes[j] == b'E') {
            let mut k = j + 1;
            if k < bytes.len() && (bytes[k] == b'-' || bytes[k] == b'+') {
                k += 1;
            }
            let exp_start = k;
            while k < bytes.len() && bytes[k].is_ascii_digit() {
                k += 1;
            }
            if k > exp_start {
                j = k;
            }
        }
        tokens.push(&path[start..j]);
        i = j;
    }
    tokens
}

/// Flattens a catalog path (M/L/H/V/A/Z, absolute or relative) into closed contours.
pub fn parse_contours(path: &str) -> Result<Vec<Vec<Point>>, PathError> {
    parse_contours_with(path, ARC_TOLERANCE)
}

pub fn parse_contours_with(path: &str, tolerance: f64) -> Result<Vec<Vec<Point>>, PathError> {
    let tokens = tokenize(path);
    let mut contours: Vec<Vec<Point>> = Vec::new();
    let mut current = pt(0.0, 0.0);
    let mut start = current;
    let mut command = String::new();
    let mut index = 0;
    fn append(contours: &mut [Vec<Point>], current: &mut Point, point: Point) {
        if let Some(contour) = contours.last_mut() {
            if contour.last() != Some(&point) {
                contour.push(point);
            }
        }
        *current = point;
    }
    while index < tokens.len() {
        if tokens[index].as_bytes()[0].is_ascii_alphabetic() {
            command = tokens[index].to_string();
            index += 1;
        }
        let op = command.to_ascii_uppercase();
        let relative = command != op;
        if op == "Z" {
            current = start;
            command.clear();
            continue;
        }
        let count = match op.as_str() {
            "M" | "L" => 2,
            "H" | "V" => 1,
            "A" => 7,
            _ => return Err(PathError::Unsupported),
        };
        let args: Vec<f64> = tokens[index..(index + count).min(tokens.len())]
            .iter()
            .map(|t| t.parse::<f64>().unwrap_or(f64::NAN))
            .collect();
        if args.len() != count || args.iter().any(|n| !n.is_finite()) {
            return Err(PathError::Invalid);
        }
        index += count;
        let off = if relative { current } else { pt(0.0, 0.0) };
        match op.as_str() {
            "M" => {
                start = pt(args[0] + off.x, args[1] + off.y);
                contours.push(Vec::new());
                append(&mut contours, &mut current, start);
                command = if relative { "l".into() } else { "L".into() };
            }
            "L" => append(&mut contours, &mut current, pt(args[0] + off.x, args[1] + off.y)),
            "H" => {
                let p = pt(args[0] + off.x, current.y);
                append(&mut contours, &mut current, p)
            }
            "V" => {
                let p = pt(current.x, args[0] + off.y);
                append(&mut contours, &mut current, p)
            }
            _ => {
                let end = pt(args[5] + off.x, args[6] + off.y);
                for p in arc_points(current, end, &args, tolerance) {
                    append(&mut contours, &mut current, p);
                }
            }
        }
    }
    for contour in &mut contours {
        if contour.len() > 1 && contour.first() == contour.last() {
            contour.pop();
        }
    }
    if contours.is_empty() || contours.iter().any(|c| c.len() < 3) {
        return Err(PathError::Empty);
    }
    Ok(contours)
}

/// Places canonical contours in pose-local coordinates, matching the renderer:
/// surface scale about the centre, rotation, then world-axis reflections, with
/// the origin at the top-left of the rotated footprint.
#[allow(clippy::too_many_arguments)]
pub fn pose_surface(
    contours: &[Vec<Point>],
    fill_rule: FillRule,
    width: f64,
    height: f64,
    scale: f64,
    rotation: i32,
    mirror_x: bool,
    mirror_y: bool,
) -> Surface {
    let rotation = rotation.rem_euclid(360);
    let swapped = rotation % 180 != 0;
    let (w, h) = if swapped { (height, width) } else { (width, height) };
    Surface {
        fill_rule,
        contours: contours
            .iter()
            .map(|c| {
                c.iter()
                    .map(|p| {
                        let (mut x, mut y) = ((p.x - width / 2.0) * scale, (p.y - height / 2.0) * scale);
                        (x, y) = match rotation {
                            90 => (-y, x),
                            180 => (-x, -y),
                            270 => (y, -x),
                            _ => (x, y),
                        };
                        pt(if mirror_x { -x } else { x } + w / 2.0, if mirror_y { -y } else { y } + h / 2.0)
                    })
                    .collect()
            })
            .collect(),
    }
}

/// -1 outside, 0 on the boundary, 1 inside.
fn location(point: Point, shape: &Surface, dx: f64, dy: f64) -> i8 {
    let mut winding = 0i32;
    let p = pt(point.x - dx, point.y - dy);
    for contour in &shape.contours {
        for i in 0..contour.len() {
            let a = contour[i];
            let b = contour[(i + 1) % contour.len()];
            let side = cross(a, b, p);
            if side.abs() <= EPSILON
                && p.x >= a.x.min(b.x) - EPSILON
                && p.x <= a.x.max(b.x) + EPSILON
                && p.y >= a.y.min(b.y) - EPSILON
                && p.y <= a.y.max(b.y) + EPSILON
            {
                return 0;
            }
            if a.y <= p.y && b.y > p.y && side > 0.0 {
                winding += 1;
            } else if a.y > p.y && b.y <= p.y && side < 0.0 {
                winding -= 1;
            }
        }
    }
    let inside = match shape.fill_rule {
        FillRule::Evenodd => winding.abs() % 2 == 1,
        FillRule::Nonzero => winding != 0,
    };
    if inside {
        1
    } else {
        -1
    }
}

fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    if a.x.max(b.x) < c.x.min(d.x)
        || c.x.max(d.x) < a.x.min(b.x)
        || a.y.max(b.y) < c.y.min(d.y)
        || c.y.max(d.y) < a.y.min(b.y)
    {
        return false;
    }
    let ab_c = cross(a, b, c);
    let ab_d = cross(a, b, d);
    let cd_a = cross(c, d, a);
    let cd_b = cross(c, d, b);
    ((ab_c > EPSILON && ab_d < -EPSILON) || (ab_c < -EPSILON && ab_d > EPSILON))
        && ((cd_a > EPSILON && cd_b < -EPSILON) || (cd_a < -EPSILON && cd_b > EPSILON))
}

/// Does `a` (offset by `a_off`) have interior area inside `b` (offset by `b_off`)?
fn contains_surface(a: &Surface, a_off: Point, b: &Surface, b_off: Point) -> bool {
    for contour in &a.contours {
        for i in 0..contour.len() {
            let start = pt(contour[i].x + a_off.x, contour[i].y + a_off.y);
            let e = contour[(i + 1) % contour.len()];
            let end = pt(e.x + a_off.x, e.y + a_off.y);
            if location(start, b, b_off.x, b_off.y) == 1 {
                return true;
            }
            // Interior probes also catch identical or collinear contained outlines,
            // whose vertices can all lie on the other shape's boundary.
            let (dx, dy) = (end.x - start.x, end.y - start.y);
            let length = dx.hypot(dy);
            if length == 0.0 {
                continue;
            }
            for sign in [-1.0, 1.0] {
                let probe = pt(
                    (start.x + end.x) / 2.0 - sign * dy / length * 1e-7,
                    (start.y + end.y) / 2.0 + sign * dx / length * 1e-7,
                );
                if location(probe, a, a_off.x, a_off.y) == 1 && location(probe, b, b_off.x, b_off.y) == 1 {
                    return true;
                }
            }
        }
    }
    false
}

/// Narrow-phase test for two surfaces whose coarse masks share a cell.
/// `offset` is the position of `b` relative to `a`, in studs.
pub fn surfaces_overlap(a: &Surface, b: &Surface, offset: (i32, i32)) -> bool {
    let zero = pt(0.0, 0.0);
    let off = pt(offset.0 as f64, offset.1 as f64);
    for ca in &a.contours {
        for cb in &b.contours {
            for i in 0..ca.len() {
                let (p1, p2) = (ca[i], ca[(i + 1) % ca.len()]);
                for j in 0..cb.len() {
                    let q1 = cb[j];
                    let q2 = cb[(j + 1) % cb.len()];
                    if crosses(p1, p2, pt(q1.x + off.x, q1.y + off.y), pt(q2.x + off.x, q2.y + off.y)) {
                        return true;
                    }
                }
            }
        }
    }
    contains_surface(a, zero, b, off) || contains_surface(b, off, a, zero)
}

type PairKey = (u16, u16, i32, i32);
static PAIR_CACHE: Mutex<Option<HashMap<PairKey, bool>>> = Mutex::new(None);

/// Cached narrow phase between two catalog poses. Unknown geometry stays
/// conservative: a pose without a surface always collides on shared cells.
pub fn poses_overlap(a: u16, b: u16, offset: (i32, i32)) -> bool {
    let key = (a, b, offset.0, offset.1);
    {
        let guard = PAIR_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(hit) = guard.as_ref().and_then(|m| m.get(&key)) {
            return *hit;
        }
    }
    let catalog = crate::catalog::catalog();
    let result = match (&catalog.pose(a).surface, &catalog.pose(b).surface) {
        (Some(sa), Some(sb)) => surfaces_overlap(sa, sb, offset),
        _ => true,
    };
    let mut guard = PAIR_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    if map.len() >= 16_384 {
        map.clear();
    }
    map.insert(key, result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_relative_commands_and_arcs() {
        let c = parse_contours("M0 0 h1 v1 H0 Z").unwrap();
        assert_eq!(c[0], vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0), pt(0.0, 1.0)]);
        let arc = parse_contours("M0 0 H1 A1 1 0 0 1 0 1 Z").unwrap();
        assert!(arc[0].len() > 10);
        assert_eq!(parse_contours("M0 0 C1 1 2 2 3 3"), Err(PathError::Unsupported));
        assert_eq!(parse_contours("M0 0 H1"), Err(PathError::Empty));
        assert_eq!(tokenize("M-1.5e2.5,3"), vec!["M", "-1.5e2", ".5", "3"]);
    }
}
