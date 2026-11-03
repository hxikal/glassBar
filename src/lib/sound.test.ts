import { describe, expect, it, vi } from 'vitest';
import { ADHANS, adhanOf, playAdzan, stopSound } from './sound';

// Web Audio, faked: the adzan is decoded and played through an AudioBufferSourceNode.
class FakeSource { buffer: unknown = null; onended: (() => void) | null = null; connect = () => fakeGain; start = vi.fn(); stop = () => this.onended?.(); }
const fakeGain = { gain: { setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn() }, connect: () => ({}) };
class FakeContext {
  static last: FakeSource | null = null;
  currentTime = 0; destination = {};
  resume = async () => {}; suspend = async () => {};
  decodeAudioData = async () => ({ duration: 180 });
  createGain = () => fakeGain;
  createBufferSource = () => (FakeContext.last = new FakeSource());
}
vi.stubGlobal('AudioContext', FakeContext);
vi.stubGlobal('fetch', async () => ({ arrayBuffer: async () => new ArrayBuffer(0) }));
const flush = () => new Promise((r) => setTimeout(r));
const ended = () => FakeContext.last?.onended?.();

describe('playAdzan', () => {
  it('pauses playing media, then resumes those apps when the adzan ends', async () => {
    const media = { pause: vi.fn(async () => ['Spotify.exe']), resume: vi.fn(async () => {}) };
    playAdzan('a.mp3', false, true, media);
    expect(media.pause).toHaveBeenCalledOnce();
    await flush();
    expect(media.resume).not.toHaveBeenCalled();
    ended();
    await flush();
    expect(media.resume).toHaveBeenCalledWith(['Spotify.exe']);
  });

  it('resumes when the adzan is stopped, and only once', async () => {
    const media = { pause: vi.fn(async () => ['x']), resume: vi.fn(async () => {}) };
    playAdzan('a.mp3', true, true, media);
    await flush();
    stopSound();
    stopSound();
    ended();
    await flush();
    expect(media.resume).toHaveBeenCalledOnce();
  });

  it('starts the adzan only after the pause is done', async () => {
    let done!: (ids: string[]) => void;
    const media = { pause: vi.fn(() => new Promise<string[]>((r) => (done = r))), resume: vi.fn(async () => {}) };
    const before = FakeContext.last;
    playAdzan('a.mp3', false, true, media);
    await flush();
    expect(FakeContext.last).toBe(before);
    done([]);
    await flush();
    expect(FakeContext.last).not.toBe(before);
    stopSound();
  });

  it('leaves media alone when off, or when nothing was playing', async () => {
    const off = { pause: vi.fn(async () => ['x']), resume: vi.fn(async () => {}) };
    playAdzan('a.mp3', false, false, off);
    stopSound();
    const idle = { pause: vi.fn(async () => []), resume: vi.fn(async () => {}) };
    playAdzan('a.mp3', false, true, idle);
    await flush();
    stopSound();
    await flush();
    expect(off.pause).not.toHaveBeenCalled();
    expect(off.resume).not.toHaveBeenCalled();
    expect(idle.resume).not.toHaveBeenCalled();
  });

  it('a short adzan stops at the end of the opening takbir', async () => {
    playAdzan('a.mp3', true, false, { pause: async () => [], resume: async () => {} });
    await flush();
    expect(FakeContext.last?.start).toHaveBeenCalledWith(0, 0, 43.3);
    stopSound();
  });

  it('a replaced adzan neither ends nor resumes the music: the new one does, once', async () => {
    const media = { pause: vi.fn(async () => ['Spotify.exe']), resume: vi.fn(async () => {}) };
    const firstEnd = vi.fn(), secondEnd = vi.fn();
    playAdzan('a.mp3', false, true, media, 1, firstEnd);
    await flush();
    media.pause.mockResolvedValueOnce([]); // the music is already paused by the first
    playAdzan('b.mp3', false, true, media, 1, secondEnd);
    await flush();
    expect(firstEnd).not.toHaveBeenCalled();
    expect(media.resume).not.toHaveBeenCalled();
    ended();
    await flush();
    expect(secondEnd).toHaveBeenCalledOnce();
    expect(media.resume).toHaveBeenCalledWith(['Spotify.exe']);
  });

  it('a bundled recording stops at its own takbir pause', async () => {
    playAdzan('a.mp3', ADHANS.madina.short, false, { pause: async () => [], resume: async () => {} });
    await flush();
    expect(FakeContext.last?.start).toHaveBeenCalledWith(0, 0, 45);
    stopSound();
  });
});

describe('adhanOf', () => {
  it('defaults to the Madina recording when unset or unknown', () => {
    expect(adhanOf()).toBe(ADHANS.madina);
    expect(adhanOf('toString')).toBe(ADHANS.madina);
    expect(adhanOf('classic')).toBe(ADHANS.classic);
  });
});
