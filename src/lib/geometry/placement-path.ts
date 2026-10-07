export type GridPoint = { x: number; y: number };

// Follow the cells crossed by the actual pointer segment, including sparse
// pointer events. Clip first so movement outside the paper stays inexpensive.
export function placementPath(
  from: GridPoint,
  to: GridPoint,
  board: { width: number; height: number }
): GridPoint[] {
  if (![from.x, from.y, to.x, to.y].every(Number.isFinite)) return [];
  const dx = to.x - from.x,
    dy = to.y - from.y;
  let entry = 0,
    exit = 1;
  for (const [start, delta, limit] of [
    [from.x, dx, board.width],
    [from.y, dy, board.height]
  ]) {
    if (!delta) {
      if (start < 0 || start >= limit) return [];
      continue;
    }
    const a = -start / delta,
      b = (limit - start) / delta;
    entry = Math.max(entry, Math.min(a, b));
    exit = Math.min(exit, Math.max(a, b));
    if (entry > exit) return [];
  }
  const epsilon = 1e-9;
  if (entry === exit) {
    const x = from.x + entry * dx,
      y = from.y + entry * dy;
    if (x < 0 || y < 0 || x >= board.width || y >= board.height) return [];
  }
  const clamp = (value: number, limit: number) => Math.max(0, Math.min(limit - epsilon, value));
  const start = {
    x: clamp(from.x + entry * dx, board.width),
    y: clamp(from.y + entry * dy, board.height)
  };
  const end = {
    x: clamp(from.x + exit * dx, board.width),
    y: clamp(from.y + exit * dy, board.height)
  };
  let x = Math.floor(start.x),
    y = Math.floor(start.y);
  const endX = Math.floor(end.x),
    endY = Math.floor(end.y);
  const stepX = Math.sign(end.x - start.x),
    stepY = Math.sign(end.y - start.y);
  const incrementX = stepX ? 1 / Math.abs(end.x - start.x) : Infinity;
  const incrementY = stepY ? 1 / Math.abs(end.y - start.y) : Infinity;
  let nextX = stepX ? (stepX > 0 ? x + 1 - start.x : start.x - x) * incrementX : Infinity;
  let nextY = stepY ? (stepY > 0 ? y + 1 - start.y : start.y - y) * incrementY : Infinity;
  const cells: GridPoint[] = [{ x, y }];
  while (x !== endX || y !== endY) {
    if (x === endX) {
      y += stepY;
      nextY += incrementY;
    } else if (y === endY) {
      x += stepX;
      nextX += incrementX;
    } else if (Math.abs(nextX - nextY) <= epsilon) {
      x += stepX;
      y += stepY;
      nextX += incrementX;
      nextY += incrementY;
    } else if (nextX < nextY) {
      x += stepX;
      nextX += incrementX;
    } else {
      y += stepY;
      nextY += incrementY;
    }
    cells.push({ x, y });
  }
  return cells;
}
