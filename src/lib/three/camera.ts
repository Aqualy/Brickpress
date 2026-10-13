import * as THREE from 'three';
import type { CameraState } from './types';

export type BoardCamera = THREE.OrthographicCamera | THREE.PerspectiveCamera;

export function frameBoardCamera(
  camera: BoardCamera,
  state: CameraState,
  board: { width: number; height: number },
  width: number,
  height: number,
  zoom: number | 'fit'
) {
  const { azimuth, elevation, target } = state;
  // Distance depends on board dimensions, never its angle. Zoom changes the
  // projection, keeping orbit and pan independent of automatic framing.
  const distance = Math.max(20, Math.hypot(board.width, board.height)) * 1.5;
  camera.position.set(
    target[0] + Math.sin(azimuth) * Math.cos(elevation) * distance,
    target[1] + Math.sin(elevation) * distance,
    target[2] + Math.cos(azimuth) * Math.cos(elevation) * distance
  );
  camera.lookAt(...target);
  camera.updateMatrixWorld();
  let scale = zoom === 'fit' ? 1 : zoom / 100;
  if (zoom === 'fit' && board.width && board.height) {
    const corners = [-1, 1].flatMap((x) =>
      [-1, 1].flatMap((z) =>
        [0, 1].map((y) =>
          new THREE.Vector3((x * board.width) / 2, y, (z * board.height) / 2).applyMatrix4(
            camera.matrixWorldInverse
          )
        )
      )
    );
    // Account for depth when fitting: nearer corners project larger. Use
    // symmetric bounds because the orbit target stays at the screen center.
    const depthScale = (p: THREE.Vector3) =>
      camera instanceof THREE.PerspectiveCamera ? distance / -p.z : 1;
    const halfWidth = Math.max(...corners.map((p) => Math.abs(p.x * depthScale(p))));
    const halfHeight = Math.max(...corners.map((p) => Math.abs(p.y * depthScale(p))));
    scale = Math.min((width - 64) / (halfWidth * 64), (height - 170) / (halfHeight * 64));
  }
  scale = Math.max(0.03, Math.min(6, scale));
  if (camera instanceof THREE.PerspectiveCamera) {
    camera.aspect = width / height;
    // Both modes use 32 px/stud at 100% on the orbit target's depth plane.
    camera.zoom =
      (64 * scale * distance * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2))) / height;
  } else {
    camera.left = -width / (64 * scale);
    camera.right = -camera.left;
    camera.top = height / (64 * scale);
    camera.bottom = -camera.top;
  }
  camera.updateProjectionMatrix();
  return scale;
}
