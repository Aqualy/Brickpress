export function seededRandom(seed: number) {
  let value = seed >>> 0;
  return () => {
    value += 0x6d2b79f5;
    let t = value;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
export const newSeed = () => Math.floor(Math.random() * 2147483647);
export const newId = () =>
  globalThis.crypto?.randomUUID?.() ??
  `p-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
