// Beperkt het aantal gelijktijdige miniatuuraanvragen, zodat een groot raster FFmpeg niet overspoelt.

const MAX = 3;
let running = 0;
const waiting: (() => void)[] = [];

export async function limited<T>(fn: () => Promise<T>): Promise<T> {
  if (running >= MAX) await new Promise<void>((res) => waiting.push(res));
  running++;
  try {
    return await fn();
  } finally {
    running--;
    waiting.shift()?.();
  }
}
