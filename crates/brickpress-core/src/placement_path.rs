//! Grid cells crossed by a pointer segment, so fast strokes leave no gaps.
//! The segment is clipped to the board first, so movement far outside stays cheap.

use crate::document::Board;

pub fn placement_path(from: (f64, f64), to: (f64, f64), board: Board) -> Vec<(i32, i32)> {
    if ![from.0, from.1, to.0, to.1].iter().all(|v| v.is_finite()) {
        return vec![];
    }
    let (w, h) = (board.width as f64, board.height as f64);
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let (mut entry, mut exit) = (0.0f64, 1.0f64);
    for (start, delta, limit) in [(from.0, dx, w), (from.1, dy, h)] {
        if delta == 0.0 {
            if start < 0.0 || start >= limit {
                return vec![];
            }
            continue;
        }
        let a = -start / delta;
        let b = (limit - start) / delta;
        entry = entry.max(a.min(b));
        exit = exit.min(a.max(b));
        if entry > exit {
            return vec![];
        }
    }
    const EPS: f64 = 1e-9;
    if entry == exit {
        let (x, y) = (from.0 + entry * dx, from.1 + entry * dy);
        if x < 0.0 || y < 0.0 || x >= w || y >= h {
            return vec![];
        }
    }
    let clamp = |v: f64, limit: f64| v.min(limit - EPS).max(0.0);
    let start = (clamp(from.0 + entry * dx, w), clamp(from.1 + entry * dy, h));
    let end = (clamp(from.0 + exit * dx, w), clamp(from.1 + exit * dy, h));
    let (mut x, mut y) = (start.0.floor() as i32, start.1.floor() as i32);
    let (end_x, end_y) = (end.0.floor() as i32, end.1.floor() as i32);
    let sign = |v: f64| {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let (step_x, step_y) = (sign(end.0 - start.0), sign(end.1 - start.1));
    let inc_x = if step_x != 0 { 1.0 / (end.0 - start.0).abs() } else { f64::INFINITY };
    let inc_y = if step_y != 0 { 1.0 / (end.1 - start.1).abs() } else { f64::INFINITY };
    let mut next_x = match step_x {
        0 => f64::INFINITY,
        1 => (x as f64 + 1.0 - start.0) * inc_x,
        _ => (start.0 - x as f64) * inc_x,
    };
    let mut next_y = match step_y {
        0 => f64::INFINITY,
        1 => (y as f64 + 1.0 - start.1) * inc_y,
        _ => (start.1 - y as f64) * inc_y,
    };
    let mut cells = vec![(x, y)];
    while x != end_x || y != end_y {
        if x == end_x {
            y += step_y;
            next_y += inc_y;
        } else if y == end_y {
            x += step_x;
            next_x += inc_x;
        } else if (next_x - next_y).abs() <= EPS {
            x += step_x;
            y += step_y;
            next_x += inc_x;
            next_y += inc_y;
        } else if next_x < next_y {
            x += step_x;
            next_x += inc_x;
        } else {
            y += step_y;
            next_y += inc_y;
        }
        cells.push((x, y));
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;
    const B: Board = Board { width: 8, height: 8 };

    #[test]
    fn fills_skipped_horizontal_cells_in_both_directions() {
        let path: Vec<_> = (1..=6).map(|x| (x, 3)).collect();
        assert_eq!(placement_path((1.2, 3.5), (6.8, 3.5), B), path);
        let mut rev = path.clone();
        rev.reverse();
        assert_eq!(placement_path((6.8, 3.5), (1.2, 3.5), B), rev);
    }

    #[test]
    fn follows_diagonals_and_shallow_lines() {
        assert_eq!(placement_path((0.5, 0.5), (4.5, 4.5), B), (0..5).map(|x| (x, x)).collect::<Vec<_>>());
        assert_eq!(placement_path((0.5, 0.5), (4.5, 1.5), B), vec![(0, 0), (1, 0), (2, 0), (2, 1), (3, 1), (4, 1)]);
    }

    #[test]
    fn clips_outside_movement() {
        assert_eq!(placement_path((-1e6, 2.5), (1e6, 2.5), B), (0..8).map(|x| (x, 2)).collect::<Vec<_>>());
        assert!(placement_path((8.0, 0.0), (8.0, 7.0), B).is_empty());
        assert!(placement_path((9.0, 7.0), (7.0, 9.0), B).is_empty());
        assert!(placement_path((-2.0, -2.0), (-1.0, -1.0), B).is_empty());
    }

    #[test]
    fn stationary_pointer_and_invalid_input() {
        assert_eq!(placement_path((2.5, 3.5), (2.5, 3.5), B), vec![(2, 3)]);
        assert!(placement_path((f64::NAN, 0.0), (0.0, 0.0), B).is_empty());
    }
}
