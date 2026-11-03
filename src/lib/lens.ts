// ---- The lens ----
// A displacement map for a rounded rectangle: inside a band along the edge each pixel
// samples from further in, more so nearer the rim, so the picture bends there like the edge
// of a thick lens. R and G hold the x and y shift (128 = none).
export function lensMap(w: number, h: number, r: number, band: number): { data: Uint8ClampedArray<ArrayBuffer> } {
  const d = { data: new Uint8ClampedArray(w * h * 4) };
  const cx = w / 2, cy = h / 2;
  r = Math.min(r, cx, cy);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = (y * w + x) * 4;
    d.data[i + 2] = 128; d.data[i + 3] = 255;
    const sx = Math.sign(x + 0.5 - cx), sy = Math.sign(y + 0.5 - cy);
    const qx = Math.abs(x + 0.5 - cx) - (cx - r), qy = Math.abs(y + 0.5 - cy) - (cy - r);
    const depth = r - Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) - Math.min(Math.max(qx, qy), 0);
    const t = Math.max(0, 1 - depth / band);
    if (t === 0) { d.data[i] = 128; d.data[i + 1] = 128; continue; }
    let nx = 0, ny = 0;
    if (qx > 0 && qy > 0) { const n = Math.hypot(qx, qy); nx = (sx * qx) / n; ny = (sy * qy) / n; }
    else if (qx > qy) nx = sx; else ny = sy;
    const m = t * t;
    d.data[i] = Math.round(128 - 127 * nx * m);
    d.data[i + 1] = Math.round(128 - 127 * ny * m);
  }
  return d;
}
