// Writes the client's app icons (src-tauri/icons). Run with `bun scripts/icons.ts`.
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (bytes: Uint8Array) => {
  let c = 0xffffffff;
  for (const b of bytes) c = crcTable[(c ^ b) & 0xff]! ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type: string, data: Uint8Array) => {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  out.set(new TextEncoder().encode(type), 4);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc(out.subarray(4, 8 + data.length)));
  return out;
};

/** A gold disc with a dark ring and a white controller-style plus: the app mark. */
export function iconPng(size: number): Uint8Array {
  const raw = new Uint8Array(size * (size * 4 + 1));
  const c = (size - 1) / 2;
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    for (let x = 0; x < size; x++) {
      const d = Math.hypot(x - c, y - c) / (size / 2);
      const i = y * (size * 4 + 1) + 1 + x * 4;
      const arm = size * 0.09;
      const len = size * 0.27;
      const plus = (Math.abs(x - c) < arm && Math.abs(y - c) < len) || (Math.abs(y - c) < arm && Math.abs(x - c) < len);
      let rgba: [number, number, number, number] = [0, 0, 0, 0];
      if (d <= 0.96) rgba = d > 0.84 ? [40, 28, 18, 255] : plus ? [255, 250, 240, 255] : [230, 160, 40, 255];
      raw.set(rgba, i);
    }
  }
  const header = new Uint8Array(13);
  const h = new DataView(header.buffer);
  h.setUint32(0, size);
  h.setUint32(4, size);
  header.set([8, 6, 0, 0, 0], 8);
  const parts = [
    new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", new Uint8Array()),
  ];
  return Uint8Array.from(parts.flatMap((p) => [...p]));
}

function ico(png: Uint8Array, size: number): Uint8Array {
  const out = new Uint8Array(22 + png.length);
  const v = new DataView(out.buffer);
  v.setUint16(2, 1, true);
  v.setUint16(4, 1, true);
  out[6] = size >= 256 ? 0 : size;
  out[7] = size >= 256 ? 0 : size;
  v.setUint16(10, 1, true);
  v.setUint16(12, 32, true);
  v.setUint32(14, png.length, true);
  v.setUint32(18, 22, true);
  out.set(png, 22);
  return out;
}

if (import.meta.main) {
  const dir = import.meta.dir + "/../src-tauri/icons/";
  writeFileSync(dir + "icon.png", iconPng(256));
  writeFileSync(dir + "32x32.png", iconPng(32));
  writeFileSync(dir + "128x128.png", iconPng(128));
  writeFileSync(dir + "icon.ico", ico(iconPng(256), 256));
}
