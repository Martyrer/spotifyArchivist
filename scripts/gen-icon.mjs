import { writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";

const W = 512, H = 512;
const raw = Buffer.alloc(H * (1 + W * 4));
let p = 0;
for (let y = 0; y < H; y++) {
  raw[p++] = 0;
  for (let x = 0; x < W; x++) {
    const r = Math.floor(20 + (x / W) * 60);
    const g = Math.floor(180 + (y / H) * 40);
    const b = 80;
    raw[p++] = r; raw[p++] = g; raw[p++] = b; raw[p++] = 255;
  }
}
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const byte of buf) {
    c ^= byte;
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const len = Buffer.alloc(4); len.writeUInt32BE(data.length, 0);
  const tb = Buffer.from(type);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(Buffer.concat([tb, data])), 0);
  return Buffer.concat([len, tb, data, crc]);
};
const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
const idat = deflateSync(raw);
const png = Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
writeFileSync(process.argv[2] || "app-icon.png", png);
console.log("wrote", png.length, "bytes");
