import { describe, expect, it } from 'vitest';
import { OrthographicCamera, PerspectiveCamera, Raycaster, Vector2, Vector3, Plane } from 'three';
import { frameBoardCamera, type BoardCamera } from './camera';
import type { CameraState } from './types';

const board = { width: 16, height: 24 };
const width = 800,
  height = 700;
function span(camera: BoardCamera, a: Vector3, b: Vector3) {
  const p = a.clone().project(camera),
    q = b.clone().project(camera);
  return Math.hypot(((q.x - p.x) * width) / 2, ((q.y - p.y) * height) / 2);
}
const factories = {
  perspective: () => new PerspectiveCamera(40, 1, 0.1, 2000),
  orthographic: () => new OrthographicCamera(-1, 1, 1, -1, 0.1, 2000)
};
describe.each(Object.entries(factories))('%s projection', (_, makeCamera) => {
  it('fits the entire board without clipping and holds target scale through orbit and pan', () => {
    const camera = makeCamera();
    const state: CameraState = { azimuth: Math.PI / 4, elevation: Math.PI / 3, target: [0, 0, 0] };
    const zoom = frameBoardCamera(camera, state, board, width, height, 'fit');
    for (const x of [-8, 8])
      for (const z of [-12, 12])
        for (const y of [0, 1]) {
          const p = new Vector3(x, y, z).project(camera);
          expect(Math.abs(p.x)).toBeLessThanOrEqual((width - 64) / width + 1e-10);
          expect(Math.abs(p.y)).toBeLessThanOrEqual((height - 170) / height + 1e-10);
          expect(p.z).toBeGreaterThan(-1);
          expect(p.z).toBeLessThan(1);
        }
    const distance = camera.position.length();
    for (const [azimuth, elevation] of [
      [0, 0.17],
      [2, 0.4],
      [-4, 1.3],
      [7, Math.PI / 2 - 0.0001]
    ]) {
      Object.assign(state, { azimuth, elevation, target: [3, 2, -4] });
      expect(frameBoardCamera(camera, state, board, width, height, zoom * 100)).toBeCloseTo(
        zoom,
        12
      );
      const target = new Vector3(...state.target);
      expect(camera.position.distanceTo(target)).toBeCloseTo(distance, 10);
      const right = new Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
      expect(span(camera, target, target.clone().add(right))).toBeCloseTo(32 * zoom, 8);
    }
  });
  it('round-trips piece surfaces between screen coordinates and placement rays', () => {
    const camera = makeCamera();
    frameBoardCamera(
      camera,
      { azimuth: 1.2, elevation: 0.6, target: [2, 0, -1] },
      board,
      width,
      height,
      150
    );
    const plane = new Plane(new Vector3(0, 1, 0), -0.4);
    const ray = new Raycaster();
    for (const x of [-5, 0, 6])
      for (const z of [-8, 0, 9]) {
        const world = new Vector3(x, 0.4, z),
          ndc = world.clone().project(camera);
        ray.setFromCamera(new Vector2(ndc.x, ndc.y), camera);
        const hit = ray.ray.intersectPlane(plane, new Vector3())!;
        expect(hit.distanceTo(world)).toBeLessThan(1e-8);
      }
  });
});
it('linear perspective scales with depth while orthographic keeps parallel sizes', () => {
  for (const [projection, factory] of Object.entries(factories)) {
    const camera = factory();
    frameBoardCamera(
      camera,
      { azimuth: 0, elevation: 0.8, target: [0, 0, 0] },
      board,
      width,
      height,
      100
    );
    const distance = camera.position.length();
    const forward = camera.getWorldDirection(new Vector3());
    const right = new Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
    const near = forward.clone().multiplyScalar(-3),
      far = forward.clone().multiplyScalar(3);
    const nearSpan = span(camera, near, near.clone().add(right));
    const farSpan = span(camera, far, far.clone().add(right));
    expect(nearSpan / farSpan).toBeCloseTo(
      projection === 'perspective' ? (distance + 3) / (distance - 3) : 1,
      10
    );
  }
});
