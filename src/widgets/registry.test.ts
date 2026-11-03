// The registry globs src/widgets/*/index.ts for a default export, so a widget whose manifest
// is wrong doesn't fail the build -- it quietly never appears in the gallery. These checks
// notice that from the files alone.
//
// Deliberately not importing registry.ts: that pulls in every widget's Svelte components and
// the theme module, which needs a browser. It also took over a minute. Reading the manifests
// off disk takes milliseconds and catches the same mistakes.
import { describe, expect, it } from 'vitest';
import { CATEGORIES } from './categories';

const fsMod = 'node:fs';
const read = async () => {
  const fs = await import(/* @vite-ignore */ fsMod);
  const dirs: string[] = fs.readdirSync('src/widgets', { withFileTypes: true })
    .filter((d: { isDirectory(): boolean; name: string }) => d.isDirectory())
    .map((d: { name: string }) => d.name);
  return dirs
    .filter((d) => fs.existsSync(`src/widgets/${d}/index.ts`))
    .map((d) => ({ dir: d, src: fs.readFileSync(`src/widgets/${d}/index.ts`, 'utf8') as string }))
    // A folder whose index has no default export is part of another widget, not one itself.
    .filter((w) => w.src.includes('export default'));
};

const field = (src: string, key: string) => new RegExp(`\\b${key}:\\s*'([^']+)'`).exec(src)?.[1];

describe('every widget folder registers', () => {
  it('declares a manifest with an id', async () => {
    const found = await read();
    // A floor, not a count: this only has to notice that discovery works at all.
    expect(found.length).toBeGreaterThan(15);
    for (const w of found) {
      expect(w.src, w.dir).toContain('defineWidget');
      expect(field(w.src, 'id'), w.dir).toBeTruthy();
    }
  });

  it('includes the widgets that should be there', async () => {
    const ids = (await read()).map((w) => field(w.src, 'id'));
    for (const id of ['clock', 'music', 'equalizer', 'shelf', 'water']) {
      expect(ids, `${id} is missing from the gallery`).toContain(id);
    }
  });

  it('never gives two widgets the same id', async () => {
    // Ids are the key the registry builds its map with, and they are stored in config: a
    // duplicate silently replaces the other widget everywhere.
    const ids = (await read()).map((w) => field(w.src, 'id'));
    expect(new Set(ids).size, ids.join(', ')).toBe(ids.length);
  });

  it('lands in a real category rather than silently in Other', async () => {
    const known: string[] = CATEGORIES.map((c) => c.id);
    for (const w of await read()) {
      const cat = field(w.src, 'category');
      // Only _template may leave it out; everything else should pick its shelf.
      if (w.dir.startsWith('_')) continue;
      expect(cat, `${w.dir} has no category`).toBeTruthy();
      expect(known, w.dir).toContain(cat);
    }
  });
});
