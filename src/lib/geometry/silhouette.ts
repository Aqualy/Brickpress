import { getPiece, surfaceScale } from '../catalog/catalog';
import type { PlacedPiece } from '../types/document';

type Point = { x: number; y: number };
type Surface = { contours: Point[][]; fillRule: 'nonzero' | 'evenodd' };
const epsilon = 1e-9;
// Maximum arc chord error in studs: 0.0008 mm at the nominal stud pitch.
const arcTolerance = 1e-4;
const pathCache = new Map<string, Point[][] | null>();
const surfaceCache = new Map<string, Surface | null>();
const collisionCache = new Map<string, boolean>();
const cross = (a: Point, b: Point, c: Point) =>
  (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);

function arcPoints(start: Point, end: Point, args: number[]): Point[] {
  let [rx, ry, degrees, largeArc, sweep] = args;
  rx = Math.abs(rx);
  ry = Math.abs(ry);
  if (!rx || !ry) return [end];
  if (start.x === end.x && start.y === end.y) return [];
  // SVG endpoint-to-center conversion, including out-of-range radius correction.
  // https://www.w3.org/TR/SVG/implnote.html#ArcConversionEndpointToCenter
  const phi = (degrees * Math.PI) / 180;
  const cos = Math.cos(phi),
    sin = Math.sin(phi);
  const dx = (start.x - end.x) / 2,
    dy = (start.y - end.y) / 2;
  const x = cos * dx + sin * dy,
    y = -sin * dx + cos * dy;
  const correction = (x * x) / (rx * rx) + (y * y) / (ry * ry);
  if (correction > 1) {
    rx *= Math.sqrt(correction);
    ry *= Math.sqrt(correction);
  }
  const rx2 = rx * rx,
    ry2 = ry * ry;
  const factor =
    (largeArc === sweep ? -1 : 1) *
    Math.sqrt(Math.max(0, (rx2 * ry2 - rx2 * y * y - ry2 * x * x) / (rx2 * y * y + ry2 * x * x)));
  const cx = (factor * rx * y) / ry,
    cy = (-factor * ry * x) / rx;
  const center = {
    x: cos * cx - sin * cy + (start.x + end.x) / 2,
    y: sin * cx + cos * cy + (start.y + end.y) / 2
  };
  const u = { x: (x - cx) / rx, y: (y - cy) / ry };
  const v = { x: (-x - cx) / rx, y: (-y - cy) / ry };
  const theta = Math.atan2(u.y, u.x);
  let delta = Math.atan2(u.x * v.y - u.y * v.x, u.x * v.x + u.y * v.y);
  if (!sweep && delta > 0) delta -= 2 * Math.PI;
  if (sweep && delta < 0) delta += 2 * Math.PI;
  const step = 2 * Math.acos(Math.max(-1, 1 - arcTolerance / Math.max(rx, ry)));
  const count = Math.max(1, Math.ceil(Math.abs(delta) / step));
  return Array.from({ length: count }, (_, i) => {
    if (i === count - 1) return end;
    const angle = theta + (delta * (i + 1)) / count;
    const x = rx * Math.cos(angle),
      y = ry * Math.sin(angle);
    return { x: center.x + cos * x - sin * y, y: center.y + sin * x + cos * y };
  });
}

function parseContours(path: string): Point[][] {
  const tokens = path.match(/[a-zA-Z]|[-+]?(?:\d*\.?\d+)(?:[eE][-+]?\d+)?/g) ?? [];
  const contours: Point[][] = [];
  let contour: Point[] = [],
    current = { x: 0, y: 0 },
    start = current;
  let command = '',
    index = 0;
  const append = (point: Point) => {
    const last = contour.at(-1);
    if (!last || last.x !== point.x || last.y !== point.y) contour.push(point);
    current = point;
  };
  while (index < tokens.length) {
    if (/^[a-z]$/i.test(tokens[index])) command = tokens[index++];
    const op = command.toUpperCase(),
      relative = command !== op;
    if (op === 'Z') {
      current = start;
      command = '';
      continue;
    }
    const count = ({ M: 2, L: 2, H: 1, V: 1, A: 7 } as Record<string, number>)[op];
    if (!count) throw new Error('Unsupported catalog path command.');
    const args = tokens.slice(index, index + count).map(Number);
    if (args.length !== count || args.some((n) => !Number.isFinite(n)))
      throw new Error('Invalid catalog path.');
    index += count;
    const point = (x: number, y: number) => ({
      x: x + (relative ? current.x : 0),
      y: y + (relative ? current.y : 0)
    });
    if (op === 'M') {
      start = point(args[0], args[1]);
      contour = [];
      contours.push(contour);
      append(start);
      command = relative ? 'l' : 'L';
    } else if (op === 'L') append(point(args[0], args[1]));
    else if (op === 'H') append({ x: args[0] + (relative ? current.x : 0), y: current.y });
    else if (op === 'V') append({ x: current.x, y: args[0] + (relative ? current.y : 0) });
    else {
      const end = point(args[5], args[6]);
      for (const p of arcPoints(current, end, args)) append(p);
    }
  }
  for (const contour of contours) {
    const first = contour[0],
      last = contour.at(-1)!;
    if (first.x === last.x && first.y === last.y) contour.pop();
  }
  if (!contours.length || contours.some((c) => c.length < 3))
    throw new Error('Empty catalog path.');
  return contours;
}

function poseKey(p: PlacedPiece) {
  return `${p.pieceId}:${p.rotation}:${!!p.mirrorX}:${!!p.mirrorY}`;
}

function surface(p: PlacedPiece): Surface | null {
  const key = poseKey(p);
  if (surfaceCache.has(key)) return surfaceCache.get(key) ?? null;
  const piece = getPiece(p.pieceId);
  if (!piece) return null;
  if (!pathCache.has(piece.id)) {
    try {
      pathCache.set(piece.id, parseContours(piece.geometry.path));
    } catch {
      pathCache.set(piece.id, null);
    }
  }
  const contours = pathCache.get(piece.id);
  if (!contours) {
    surfaceCache.set(key, null);
    return null;
  }
  const width = piece.footprint.widthStuds,
    height = piece.footprint.heightStuds;
  const rotation = ((p.rotation % 360) + 360) % 360;
  const swapped = rotation % 180 !== 0;
  const prepared: Surface = {
    fillRule: piece.geometry.fillRule,
    contours: contours.map((c) =>
      c.map((point) => {
        let x = (point.x - width / 2) * surfaceScale;
        let y = (point.y - height / 2) * surfaceScale;
        // Match the renderer: surface scale, rotation, then world-axis reflections.
        if (rotation === 90) [x, y] = [-y, x];
        else if (rotation === 180) [x, y] = [-x, -y];
        else if (rotation === 270) [x, y] = [y, -x];
        return {
          x: (p.mirrorX ? -x : x) + (swapped ? height : width) / 2,
          y: (p.mirrorY ? -y : y) + (swapped ? width : height) / 2
        };
      })
    )
  };
  surfaceCache.set(key, prepared);
  return prepared;
}

// Returns boundary separately; touching edges do not occupy shared surface area.
function location(point: Point, shape: Surface): -1 | 0 | 1 {
  let winding = 0;
  for (const contour of shape.contours) {
    for (let i = 0; i < contour.length; i++) {
      const a = contour[i],
        b = contour[(i + 1) % contour.length];
      const side = cross(a, b, point);
      if (
        Math.abs(side) <= epsilon &&
        point.x >= Math.min(a.x, b.x) - epsilon &&
        point.x <= Math.max(a.x, b.x) + epsilon &&
        point.y >= Math.min(a.y, b.y) - epsilon &&
        point.y <= Math.max(a.y, b.y) + epsilon
      )
        return 0;
      if (a.y <= point.y && b.y > point.y && side > 0) winding++;
      else if (a.y > point.y && b.y <= point.y && side < 0) winding--;
    }
  }
  return (shape.fillRule === 'evenodd' ? Math.abs(winding) % 2 : winding !== 0) ? 1 : -1;
}

function crosses(a: Point, b: Point, c: Point, d: Point) {
  if (
    Math.max(a.x, b.x) < Math.min(c.x, d.x) ||
    Math.max(c.x, d.x) < Math.min(a.x, b.x) ||
    Math.max(a.y, b.y) < Math.min(c.y, d.y) ||
    Math.max(c.y, d.y) < Math.min(a.y, b.y)
  )
    return false;
  const abC = cross(a, b, c),
    abD = cross(a, b, d);
  const cdA = cross(c, d, a),
    cdB = cross(c, d, b);
  return (
    ((abC > epsilon && abD < -epsilon) || (abC < -epsilon && abD > epsilon)) &&
    ((cdA > epsilon && cdB < -epsilon) || (cdA < -epsilon && cdB > epsilon))
  );
}

function containsSurface(a: Surface, b: Surface) {
  for (const contour of a.contours) {
    for (let i = 0; i < contour.length; i++) {
      const start = contour[i],
        end = contour[(i + 1) % contour.length];
      if (location(start, b) === 1) return true;
      // Interior probes also catch identical or collinear contained outlines,
      // whose vertices can all lie on the other shape's boundary.
      const dx = end.x - start.x,
        dy = end.y - start.y,
        length = Math.hypot(dx, dy);
      if (!length) continue;
      for (const sign of [-1, 1]) {
        const point = {
          x: (start.x + end.x) / 2 - ((sign * dy) / length) * 1e-7,
          y: (start.y + end.y) / 2 + ((sign * dx) / length) * 1e-7
        };
        if (location(point, a) === 1 && location(point, b) === 1) return true;
      }
    }
  }
  return false;
}

/** Narrow phase for pieces whose coarse stud masks share a cell. */
export function surfacesOverlap(a: PlacedPiece, b: PlacedPiece): boolean {
  const key = `${poseKey(a)}|${poseKey(b)}|${b.x - a.x},${b.y - a.y}`;
  const cached = collisionCache.get(key);
  if (cached !== undefined) return cached;
  const first = surface(a),
    localSecond = surface(b);
  // Unknown future path commands retain conservative mask collision behavior.
  if (!first || !localSecond) return true;
  const second: Surface = {
    fillRule: localSecond.fillRule,
    contours: localSecond.contours.map((c) =>
      c.map((p) => ({ x: p.x + b.x - a.x, y: p.y + b.y - a.y }))
    )
  };
  let overlaps = false;
  crossing: for (const ca of first.contours)
    for (const cb of second.contours) {
      for (let i = 0; i < ca.length; i++)
        for (let j = 0; j < cb.length; j++) {
          if (crosses(ca[i], ca[(i + 1) % ca.length], cb[j], cb[(j + 1) % cb.length])) {
            overlaps = true;
            break crossing;
          }
        }
    }
  overlaps ||= containsSurface(first, second) || containsSurface(second, first);
  if (collisionCache.size >= 4096) collisionCache.clear();
  collisionCache.set(key, overlaps);
  return overlaps;
}
