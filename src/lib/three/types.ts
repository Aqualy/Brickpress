import type { PlacedPiece } from '../types/document';

export type Point = { x: number; y: number };
export type CameraProjection = 'perspective' | 'orthographic';
export type CameraState = {
  azimuth: number;
  elevation: number;
  target: [number, number, number];
  projection?: CameraProjection;
};
export type CameraAction =
  | 'top'
  | 'isometric'
  | 'reset'
  | 'left'
  | 'right'
  | 'up'
  | 'down'
  | 'pan-left'
  | 'pan-right'
  | 'pan-up'
  | 'pan-down';
export type PreviewRow = { piece: PlacedPiece; color: string };
export type ScreenRect = Point & { width: number; height: number };
