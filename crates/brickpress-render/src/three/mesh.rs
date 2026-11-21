//! Extruded piece meshes for the 3D view, built from the catalog outlines.

use brickpress_core::catalog::{catalog, FillRule};
use brickpress_core::silhouette::{parse_contours_with, Point};
use bytemuck::{Pod, Zeroable};
use lyon_tessellation::geom::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};

pub const PLATE_HEIGHT: f32 = 0.4;
const BOTTOM: f32 = 0.013;
const TOP_GAP: f32 = 0.012;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Canonical piece x, y and height z.
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    fn push(&mut self, position: [f32; 3], normal: [f32; 3]) -> u32 {
        self.vertices.push(Vertex { position, normal });
        self.vertices.len() as u32 - 1
    }
}

pub fn piece_height(surface_height_plates: f64) -> f32 {
    surface_height_plates as f32 * PLATE_HEIGHT
}

fn inside(p: Point, contours: &[Vec<Point>], rule: FillRule) -> bool {
    crate::atlas::signed_distance(p, contours, rule) < 0.0
}

/// Offsets every contour inward by `delta` (mitered, with a limit).
fn inset(contours: &[Vec<Point>], rule: FillRule, delta: f64) -> Vec<Vec<Point>> {
    contours
        .iter()
        .map(|c| {
            let n = c.len();
            // Inward normal of each edge i (c[i] → c[i+1]).
            let normals: Vec<(f64, f64)> = (0..n)
                .map(|i| {
                    let (a, b) = (c[i], c[(i + 1) % n]);
                    let (dx, dy) = (b.x - a.x, b.y - a.y);
                    let len = dx.hypot(dy).max(1e-12);
                    let normal = (-dy / len, dx / len);
                    let mid = Point { x: (a.x + b.x) / 2.0 + normal.0 * 1e-4, y: (a.y + b.y) / 2.0 + normal.1 * 1e-4 };
                    if inside(mid, contours, rule) {
                        normal
                    } else {
                        (-normal.0, -normal.1)
                    }
                })
                .collect();
            (0..n)
                .map(|i| {
                    let n1 = normals[(i + n - 1) % n];
                    let n2 = normals[i];
                    let (mx, my) = (n1.0 + n2.0, n1.1 + n2.1);
                    let len = mx.hypot(my);
                    if len < 1e-9 {
                        return Point { x: c[i].x + n2.0 * delta, y: c[i].y + n2.1 * delta };
                    }
                    let (mx, my) = (mx / len, my / len);
                    let cos = (mx * n2.0 + my * n2.1).max(0.34);
                    let l = delta / cos;
                    Point { x: c[i].x + mx * l, y: c[i].y + my * l }
                })
                .collect()
        })
        .collect()
}

/// An extruded surface: a tessellated top and smooth or hard side walls.
pub fn extrude(contours: &[Vec<Point>], rule: FillRule, height: f32) -> Mesh {
    let mut mesh = Mesh::default();
    let top = height - TOP_GAP;
    // Top cap.
    let mut builder = Path::builder();
    for c in contours {
        builder.begin(point(c[0].x as f32, c[0].y as f32));
        for p in &c[1..] {
            builder.line_to(point(p.x as f32, p.y as f32));
        }
        builder.end(true);
    }
    let path = builder.build();
    let mut buffers: VertexBuffers<Vertex, u32> = VertexBuffers::new();
    let options = FillOptions::tolerance(1e-4).with_fill_rule(match rule {
        FillRule::Nonzero => lyon_tessellation::FillRule::NonZero,
        FillRule::Evenodd => lyon_tessellation::FillRule::EvenOdd,
    });
    let _ = FillTessellator::new().tessellate_path(
        &path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| Vertex {
            position: [v.position().x, v.position().y, top],
            normal: [0.0, 0.0, 1.0],
        }),
    );
    mesh.vertices.extend(buffers.vertices);
    mesh.indices.extend(buffers.indices);

    // Walls: smooth across shallow angles (curves), hard at corners.
    for c in contours {
        let n = c.len();
        let outward: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let (a, b) = (c[i], c[(i + 1) % n]);
                let (dx, dy) = ((b.x - a.x) as f32, (b.y - a.y) as f32);
                let len = dx.hypot(dy).max(1e-12);
                let normal = (dy / len, -dx / len);
                let mid = Point {
                    x: (a.x + b.x) / 2.0 + normal.0 as f64 * 1e-4,
                    y: (a.y + b.y) / 2.0 + normal.1 as f64 * 1e-4,
                };
                if inside(mid, contours, rule) {
                    (-normal.0, -normal.1)
                } else {
                    normal
                }
            })
            .collect();
        let vertex_normal = |i: usize, edge: usize| {
            let own = outward[edge];
            let other = if edge == i { outward[(i + n - 1) % n] } else { outward[(edge + 1) % n] };
            if own.0 * other.0 + own.1 * other.1 > 0.82 {
                let (x, y) = (own.0 + other.0, own.1 + other.1);
                let l = x.hypot(y);
                [x / l, y / l, 0.0]
            } else {
                [own.0, own.1, 0.0]
            }
        };
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = (c[i], c[j]);
            let na = vertex_normal(i, i);
            let nb = vertex_normal(j, i);
            let v0 = mesh.push([a.x as f32, a.y as f32, BOTTOM], na);
            let v1 = mesh.push([b.x as f32, b.y as f32, BOTTOM], nb);
            let v2 = mesh.push([b.x as f32, b.y as f32, top], nb);
            let v3 = mesh.push([a.x as f32, a.y as f32, top], na);
            mesh.indices.extend([v0, v1, v2, v0, v2, v3]);
        }
    }
    mesh
}

/// One mesh per catalog piece, in catalog order.
pub fn piece_meshes() -> Vec<Mesh> {
    let gap = catalog().design_gap_studs() / 2.0;
    catalog()
        .pieces
        .iter()
        .map(|p| match parse_contours_with(&p.geometry.path, 1e-3) {
            Ok(contours) => {
                let inset = inset(&contours, p.geometry.fill_rule, gap);
                extrude(&inset, p.geometry.fill_rule, piece_height(p.surface_height_plates))
            }
            Err(_) => Mesh::default(),
        })
        .collect()
}

/// A stud on the baseplate: a short cylinder centred on the cell.
pub fn stud_mesh() -> Mesh {
    let mut mesh = Mesh::default();
    let (r, h, segments) = (0.3f32, 0.17f32, 28);
    let center = mesh.push([0.5, 0.5, h], [0.0, 0.0, 1.0]);
    let ring: Vec<(f32, f32)> = (0..segments)
        .map(|i| {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            (a.cos(), a.sin())
        })
        .collect();
    let top: Vec<u32> = ring.iter().map(|(c, s)| mesh.push([0.5 + c * r, 0.5 + s * r, h], [0.0, 0.0, 1.0])).collect();
    for i in 0..segments {
        mesh.indices.extend([center, top[i], top[(i + 1) % segments]]);
    }
    for i in 0..segments {
        let j = (i + 1) % segments;
        let (a, b) = (ring[i], ring[j]);
        let v0 = mesh.push([0.5 + a.0 * r, 0.5 + a.1 * r, 0.0], [a.0, a.1, 0.0]);
        let v1 = mesh.push([0.5 + b.0 * r, 0.5 + b.1 * r, 0.0], [b.0, b.1, 0.0]);
        let v2 = mesh.push([0.5 + b.0 * r, 0.5 + b.1 * r, h], [b.0, b.1, 0.0]);
        let v3 = mesh.push([0.5 + a.0 * r, 0.5 + a.1 * r, h], [a.0, a.1, 0.0]);
        mesh.indices.extend([v0, v1, v2, v0, v2, v3]);
    }
    mesh
}

/// The baseplate slab, in board studs (0..w, 0..h), top at z = 0.
pub fn plate_mesh(width: f32, height: f32) -> Mesh {
    let mut mesh = Mesh::default();
    let depth = -0.32;
    let up = [0.0, 0.0, 1.0];
    let v: Vec<u32> = [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]]
        .iter()
        .map(|p| mesh.push([p[0], p[1], 0.0], up))
        .collect();
    mesh.indices.extend([v[0], v[1], v[2], v[0], v[2], v[3]]);
    let corners = [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]];
    let normals = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]];
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let n = normals[i];
        let v0 = mesh.push([a[0], a[1], depth], n);
        let v1 = mesh.push([b[0], b[1], depth], n);
        let v2 = mesh.push([b[0], b[1], 0.0], n);
        let v3 = mesh.push([a[0], a[1], 0.0], n);
        mesh.indices.extend([v0, v1, v2, v0, v2, v3]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_piece_has_a_mesh_inset_inside_its_footprint() {
        let meshes = piece_meshes();
        for (p, m) in catalog().pieces.iter().zip(&meshes) {
            assert!(!m.indices.is_empty(), "{}", p.id);
            for v in &m.vertices {
                assert!(v.position[0] >= 0.0 && v.position[0] <= p.footprint.width_studs as f32, "{}", p.id);
                assert!(v.position[1] >= 0.0 && v.position[1] <= p.footprint.height_studs as f32, "{}", p.id);
            }
        }
    }
}
