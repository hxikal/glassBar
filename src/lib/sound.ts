import { emit } from '@tauri-apps/api/event';
import classicUrl from '../assets/adhan.ogg?url';
import madinaUrl from '../assets/adhan-madina.ogg?url';
import nabawiUrl from '../assets/adhan-nabawi.ogg?url';
import zailiUrl from '../assets/adhan-zaili.ogg?url';
// Alert sounds. The chime is synthesized, so no audio file ships with the app.
let ctx: AudioContext | null = null;
let idle: ReturnType<typeof setTimeout>;

export const chime = (volume = 1) => tones([[880, 0], [1318.5, 0.18]], 0.25 * Math.max(0, Math.min(1, volume)));

/** Device plugged in: soft rising pair. Removed: falling pair. */
export const deviceTone = (connected: boolean, volume = 0.4) =>
  tones(connected ? [[660, 0], [990, 0.09]] : [[990, 0], [660, 0.09]], 0.2 * Math.max(0, Math.min(1, volume)), 0.22);

/** Camera shutter for screenshots: a short filtered noise burst. Quiet. */
export async function shutter() {
  ctx ??= new AudioContext();
  clearTimeout(idle);
  await ctx.resume();
  const t = ctx.currentTime, len = 0.09;
  const buf = ctx.createBuffer(1, Math.floor(ctx.sampleRate * len), ctx.sampleRate);
  const d = buf.getChannelData(0);
  for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / d.length, 3);
  const src = ctx.createBufferSource(), f = ctx.createBiquadFilter(), g = ctx.createGain();
  src.buffer = buf; f.type = 'bandpass'; f.frequency.value = 2600; g.gain.value = 0.35;
  src.connect(f).connect(g).connect(ctx.destination);
  src.start(t);
  idleLater();
}

/** Volume feedback: a short soft pop, like Windows' volume ding. It plays through the
 *  speakers at the volume just set, so it sounds exactly as loud as that level. Throttled
 *  while dragging; `final` (slider released) always plays. */
let lastBlip = 0;
export function volumeBlip(final = false) {
  const now = performance.now();
  if (!final && now - lastBlip < 120) return;
  lastBlip = now;
  emit('sound:blip').catch(() => {}); // the Equalizer skips it (it may be in another window)
  tones([[880, 0]], 0.28, 0.14);
}

/** OCR done: a soft high tick. */
export const tick = () => tones([[1568, 0]], 0.12, 0.12);

/** Charger plugged in: bright rising C–E–G. Unplugged: soft falling E–A, lower and rounder. */
export const chargerTone = (plugged: boolean, volume = 0.4) => {
  const v = 0.22 * Math.max(0, Math.min(1, volume));
  return plugged
    ? tones([[1046.5, 0], [1318.5, 0.08], [1568, 0.16]], v, 0.3, 'triangle')
    : tones([[659.3, 0], [440, 0.13]], v, 0.38, 'sine');
};

async function tones(notes: readonly (readonly [number, number])[], peak: number, len = 0.45, wave: OscillatorType = 'sine') {
  ctx ??= new AudioContext();
  clearTimeout(idle);
  await ctx.resume();
  const t = ctx.currentTime;
  for (const [freq, at] of notes) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = wave;
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(0, t + at);
    gain.gain.linearRampToValueAtTime(Math.max(peak, 0.0011), t + at + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.001, t + at + len);
    osc.connect(gain).connect(ctx.destination);
    osc.start(t + at);
    osc.stop(t + at + len + 0.05);
  }
  // A running AudioContext keeps the audio device (and a render thread) busy.
  idleLater();
}

/** A running AudioContext keeps the audio device (and a render thread) busy: let it sleep
 *  a moment after the last sound, but never while the adzan plays. */
function idleLater() {
  clearTimeout(idle);
  idle = setTimeout(() => { if (!current) ctx?.suspend(); }, 1500);
}

/** The adzan now playing (or loading). */
let current: { stop: () => void } | null = null;
let onDone: ((replaced: boolean) => void) | undefined;

/** The current file ended, was stopped or `replaced` by another: run its `done` once. */
function finish(p: typeof current, replaced = false) {
  if (!p || current !== p) return;
  current = null;
  const f = onDone;
  onDone = undefined;
  f?.(replaced);
  idleLater();
}

/** The bundled adzan recordings, the default first. `short`: where the opening takbir ends, in
 *  seconds (the pause after it, measured with ffmpeg astats / silencedetect). */
export const ADHANS = {
  // Muhammad Marwan Qassas (Madinah) over rain; the pause is at 43.3-46.5 s.
  madina: { label: 'Madina, with rain', url: madinaUrl, short: 45 },
  // Muhammad Marwan Qassas, Masjid an-Nabawi; the pause is at 43.5-47.2 s.
  nabawi: { label: 'Madina', url: nabawiUrl, short: 46 },
  // Abdullah Al-Zaili, Masjid Al-Furqan, Jeddah (raised 4 dB to match); the pause is at 36-40 s.
  zaili: { label: 'Jeddah, Abdullah Al-Zaili', url: zailiUrl, short: 39 },
  // "Beautiful adhan" by Adam-synagda, Wikimedia Commons, CC0; the pause is at 43.0-43.7 s.
  classic: { label: 'Classic', url: classicUrl, short: 43.3 },
};
export type AdhanVoice = keyof typeof ADHANS;
/** The chosen recording; unknown or unset = the default. */
export const adhanOf = (voice?: string) => ADHANS[voice && Object.hasOwn(ADHANS, voice) ? voice as AdhanVoice : 'madina'];
/** Short adzan of the user's own file: the classic recording's cut. */
export const SHORT_ADHAN_S = ADHANS.classic.short;
const FADE_S = 1.5;

/** Play a sound file (the adzan): whole, or with `short` only its opening (to SHORT_ADHAN_S, or
 *  to `short` seconds), faded out.
 *  `done` runs once when it ends, is stopped or is replaced (then with `replaced` true).
 *  Web Audio, not an <audio> element: an <audio> element is a media session, so Windows put
 *  the adzan on the dock's music player (and it took over the player's controls). */
export function playFile(src: string, short: boolean | number = false, done?: (replaced: boolean) => void, volume = 1) {
  stopCurrent(true);
  let node: AudioBufferSourceNode | null = null;
  const me = { stop: () => { try { node?.stop(); } catch { /* not started yet */ } } };
  current = me;
  onDone = done;
  const v = Math.max(0, Math.min(1, volume));
  (async () => {
    ctx ??= new AudioContext();
    clearTimeout(idle);
    await ctx.resume();
    const buf = await ctx.decodeAudioData(await (await fetch(src)).arrayBuffer());
    if (current !== me) return; // stopped while it was loading
    const s = ctx.createBufferSource(), g = ctx.createGain();
    s.buffer = buf;
    s.connect(g).connect(ctx.destination);
    const t = ctx.currentTime;
    g.gain.setValueAtTime(v, t);
    s.onended = () => finish(me);
    node = s;
    if (short) {
      const end = Math.min(short === true ? SHORT_ADHAN_S : short, buf.duration);
      g.gain.setValueAtTime(v, t + Math.max(0, end - FADE_S));
      g.gain.linearRampToValueAtTime(0, t + end);
      s.start(t, 0, end);
    } else s.start(t);
  })().catch(() => { if (current === me) { finish(me); chime(); } });
}

export const stopSound = () => stopCurrent(false);
function stopCurrent(replaced: boolean) {
  const c = current;
  if (!c) return;
  finish(c, replaced); // first: stopping may fire its onended at once, which must not count as an end
  c.stop();
}

/** Media paused for the adzan: resumed once, when the last adzan ends. */
let pausedForAdzan: string[] = [];

/** Play the adzan; with `pauseMusic`, pause other playing media first and resume it afterwards.
 *  `onEnd` runs once when it ends or is stopped (stopSound). An adzan replaced by another (Test
 *  pressed twice, or during the real one) neither resumes the music nor ends: the new one does
 *  both when it ends -- else the music came back mid-adzan and the Stop button went away. */
export function playAdzan(src: string, short: boolean | number, pauseMusic: boolean, media: { pause: () => Promise<string[]>; resume: (ids: string[]) => Promise<void> }, volume = 1, onEnd?: () => void) {
  const play = (ids: string[]) => {
    pausedForAdzan = [...new Set([...pausedForAdzan, ...ids])];
    playFile(src, short, (replaced) => {
      if (replaced) return;
      const paused = pausedForAdzan;
      pausedForAdzan = [];
      if (paused.length) media.resume(paused).catch(() => {});
      onEnd?.();
    }, volume);
  };
  if (!pauseMusic) return play([]);
  void media.pause().catch(() => [] as string[]).then(play);
}
