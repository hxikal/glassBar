// The only file that talks to Rust. Types mirror src-tauri structs (camelCase).
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

// Events Rust sends to one dock window only (sounds, alerts, auto-hide...). A plain listen()
// also hears events aimed at other windows, so with a dock on every monitor each sound
// would play once per dock.
const own = <T>(name: string, cb: (p: T) => void) => listen<T>(name, (e) => cb(e.payload), { target: getCurrentWindow().label });
import type { Edge } from './geometry';

export type WidgetSize = 'small' | 'medium' | 'large';
export interface WidgetInst { id: string; instance: string; size: WidgetSize; settings: any; /** false = off: kept with its settings, not on the dock */ enabled?: boolean }
export interface BarCfg {
  edge: Edge; monitor: number | null; length: number; size: 'compact' | 'normal' | 'large';
  floating: { x: number; y: number } | null; reserveSpace: boolean;
  nativeTaskbar: 'show' | 'autohide' | 'hidden'; autoHide: boolean;
  /** Width of a left/right dock. */
  sideWidth: SideWidth;
  /** Space between the dock and its screen edge (px). */
  gap: number;
  /** Space on the other side, between the dock and the apps' windows (px); unset = `gap`. */
  gapInner?: number | null;
  /** A copy of the dock on every other monitor too. */
  allDisplays?: boolean;
}
export type SideWidth = 'slim' | 'regular' | 'wide';
export const SIDE_PX: Record<SideWidth, number> = { slim: 56, regular: 72, wide: 96 };
export interface Profile { bar: BarCfg; widgets: WidgetInst[] }
/** Appearance (see lib/appearance.ts). Opacities 0..1; shadow/border/iconScale/spacing are multipliers. */
export interface Theme {
  mode: 'auto' | 'light' | 'dark'; preset: 'light' | 'dark' | 'glass' | 'liquid' | 'system' | 'custom';
  style: 'glassbar' | 'mac' | 'windows' | 'gnome' | 'kde' | 'cinnamon'; accent: string;
  blur: number; opacity: number; panelOpacity: number; cardOpacity: number;
  nativeBlur: boolean; material: 'acrylic' | 'mica' | 'blur'; bg: string | null; contrast: 'normal' | 'high';
  radius: number; shadow: number; border: number; iconScale: number; spacing: number; motion: 'full' | 'reduced' | 'off';
  saved: Record<string, Record<string, unknown>>;
}
/** What this Windows can draw; `applied` = the material really behind the dock now. */
export interface AppearanceSupport { build: number; acrylic: boolean; mica: boolean; blur: boolean; transparency: boolean; applied: 'acrylic' | 'mica' | 'blur' | 'none' }
export type LocationMode = 'auto' | 'windows' | 'ip' | 'manual';
export interface LocationCfg { mode: LocationMode; lat: number | null; lon: number | null; name: string; tz: string | null }
export interface Config {
  version: number; activeProfile: string; profiles: Record<string, Profile>;
  theme: Theme; startup: { launchAtLogin: boolean };
  dnd: boolean; location: LocationCfg; deviceSounds: { connect: boolean; disconnect: boolean; charger: boolean; volume: number };
  /** Show the charge level across the whole dock when the charger is plugged in or unplugged. */
  chargerFlash: boolean;
  /** Settings › Dock alerts (see app/dockAlert.ts). */
  dockAlerts: import('../app/dockAlert').DockAlertsCfg;
  banners: Banners;
  /** Settings › Performance: the chosen mode; with `auto` GlassBar picks Normal or Low itself. */
  performance: { mode: PerfMode; auto: boolean };
}
export type PerfMode = 'low' | 'normal' | 'high';
/** One of GlassBar's processes, named the way Task Manager names it. */
export interface ProcUsage { pid: number; name: string; ramMb: number; cpu: number; gpu: number | null }
/** One reading of GlassBar and its WebView2 helpers: private working set (MB, Task Manager's
 *  Memory), % of the whole CPU, 3D GPU % (null without GPU counters), bytes GlassBar downloaded
 *  since start, and the same per process (only on the newest sample; they add up to the totals). */
export interface PerfSample { at: number; ramMb: number; cpu: number; gpu: number | null; netBytes: number; procs: ProcUsage[] }
/** The mode in force; `reason` is Auto's why, only while Auto is on. */
export interface PerfState { mode: PerfMode; auto: boolean; reason: string | null }
/** A real charger plug/unplug. `percent` is null on a PC with no battery. */
export interface ChargerFlip { plugged: boolean; percent: number | null; secsLeft: number | null }
/** One timed lyric line. `text` is empty for a gap between verses. */
export interface LyricLine { atMs: number; text: string }
/** Synced lyrics for a track. `lines` empty = this track has none. */
/** source 'offline': LRCLIB couldn't be reached (asked again shortly), not 'this song has none'. */
export interface Lyrics { lines: LyricLine[]; source: 'local' | 'lrclib' | 'none' | 'offline'; fetchedAt: number }

export interface Alert { id: number; source: string; title: string; body: string; at: number; missed: boolean; read: boolean; data: unknown }
/** Pop-up banners: on/off, seconds shown, next to the dock or in the corner, what a click
 *  does, sources with no banner, and the alert sound. */
export interface Banners { show: boolean; seconds: 3 | 5 | 8 | 10; position: 'auto' | 'corner'; onClick: 'highlight' | 'open'; muted: string[]; sound: boolean }
export interface Todo { id: number; text: string; done: boolean; dueAt: number | null; createdAt: number }
export interface Note { id: number; text: string; color: string; pinned: boolean; images: string[]; updatedAt: number; deletedAt: number | null }
export type Phase = 'work' | 'short' | 'long';
/** volume 0–100; goal = focus sessions a day; dimOthers dims the other dock widgets while focusing. */
export interface PomoCfg { workMin: number; shortMin: number; longMin: number; every: number; autoStart: boolean; sound: boolean; volume: number; goal: number; dimOthers: boolean;
  /** Show the focus session across the whole dock (hover the dock for the widgets). Default on. */ focusDock?: boolean }
export interface FocusDay { day: string; sessions: number; minutes: number; longest: number }
/** week = the last 7 days, oldest first (today last); streak in days; longest session in minutes. */
export interface PomoState { pomo: { phase: Phase; done: number; endsAt: number | null; pausedMs: number | null; completed: Phase | null }; cfg: PomoCfg; label: string; today: FocusDay; week: FocusDay[]; streak: number; longest: number }
export interface Timer { id: number; label: string; durationMs: number; endsAt: number | null; remainingMs: number | null }
/** Downloads widget: finished files, newest first; total size of the folder. */
export interface DlItem { path: string; name: string; size: number; at: number }
export interface DlState { folder: string; items: DlItem[]; totalSize: number; error: string | null }
/** Recycle Bin: total and per fixed drive (bytes, items, free bytes on that drive). */
export interface RecycleDrive { root: string; size: number; items: number; free: number }
export interface RecycleInfo { size: number; items: number; drives: RecycleDrive[] }
/** An item in the Recycle Bin: `id` its place in the bin, `path` where it came from. */
export interface BinItem { id: string; name: string; path: string; from: string; deletedAt: number; size: number; folder: boolean }
/** Files dropped on the bin: how many moved, and [path, reason] for each that didn't. */
export interface TrashResult { moved: number; failed: [string, string][] }
export interface Stopwatch { startedAt: number | null; accMs: number; laps: number[] }
export interface Place { lat: number; lon: number; name: string; source: 'windows' | 'ip' | 'manual'; tz?: string; accuracy?: number; via?: string; note?: string }
/** The shared location: `place` null = Unavailable (`reason` says why). */
export interface LocationStatus { mode: LocationMode; place: Place | null; reason: string | null; lastKnown: Place | null; at: number }
export interface Weather {
  fetchedAt: number; unit: 'c' | 'f'; place: Place | null; offline: boolean;
  current: { temp: number; code: number; isDay: boolean; feelsLike?: number | null; humidity?: number | null; windSpeed?: number | null; windDir?: number | null };
  hourly: { time: number; temp: number; code: number; pop?: number | null }[];
  daily: { date: number; min: number; max: number; code: number; sunrise?: number | null; sunset?: number | null; pop?: number | null }[];
}
/** Tasbih: count in the current step, which phrase step, full rounds, taps today. */
export interface TasbihState { count: number; round: number; step: number; today: number; day: string }
export interface PrayerDay { date: string; zone: string; source: 'jakim' | 'aladhan'; hijri: string | null; times: [string, number][] }
export interface SolatInfo { days: PrayerDay[]; zoneName: string | null; place: Place | null }
export type Band = 'good' | 'moderate' | 'unhealthy' | 'very_unhealthy' | 'hazardous';
export interface Station { id: string; name: string; state: string; lat: number; lon: number; api: number | null; class: string; pollutant: string; at: number }
export interface IpuReading {
  kind: 'ipu' | 'aqi_us'; value: number; band: Band; station: Station | null; distanceKm: number | null;
  nearby: [Station, number][]; place: Place | null; fetchedAt: number; offline: boolean; stations: Station[];
}
export interface Battery { percent: number; charging: boolean; onAc: boolean; secsLeft: number | null }
export type PowerMode = 'efficiency' | 'balanced' | 'performance';
/** otherPlan: set when the PC isn't on the Balanced plan (power modes then do nothing). */
export interface PowerInfo { mode: PowerMode | null; saverOn: boolean | null; otherPlan: string | null }
export interface Sample { cpu: number; mem: number; gpu: number | null; rxBps: number; txBps: number; at: number; cpuTemp: number | null }
export interface Rates { base: string; rates: Record<string, number>; rateTime: number; fetchedAt: number; source: string; cached: boolean }
export interface WinRect { id: number; title: string; x: number; y: number; w: number; h: number }
export interface SnapView { img: string; purpose: 'ocr' | 'region' | 'window' | 'scroll' | 'color' | 'display'; windows: WinRect[]; /** pointer in screenshot px */ cursor: [number, number] | null;
  /** each display in the shot (screenshot px): the overlay spans them all */ displays: [number, number, number, number][];
  /** "Display 2 · 1920×1080 · Main", one per display */ displayNames: string[] }
export interface ShotItem { id: number; at: number; w: number; h: number; kind: 'full' | 'region' | 'window' | 'scroll'; thumb: string }
export interface ClipItem { id: number; at: number; kind: 'text' | 'image'; text: string | null; thumb: string | null; w: number | null; h: number | null; pinned: boolean }
export type CaptureDone =
  | { kind: 'shot'; id: number; thumb: string; w: number; h: number; shotKind: string }
  | { kind: 'ocr'; text: string; copied: boolean; lines: number }
  | { kind: 'color'; hex: string; text: string }
  | { kind: 'error'; text: string };
export interface OcrItem { id: number; at: number; text: string; mode: 'text' | 'code' | 'terminal'; engine: string }
export interface PinTarget { path: string; args: string }
export interface InstalledApp { name: string; path: string; args: string }
export interface WifiNetwork { ssid: string; bars: number; secured: boolean; security: string; connected: boolean; saved: boolean }
export interface BtDevice { id: string; name: string; kind: string; paired: boolean; connected: boolean; /** % when the connected device reports it */ battery: number | null }
/** Keep awake: until = Unix ms, null = until turned off; screen = the screen stays on too. */
export interface AwakeState { on: boolean; until: number | null; screen: boolean }
/** The default recording device: mute, level 0..1, the recording devices. */
export interface MicState { muted: boolean; volume: number; devices: AudioDevice[] }
/** An app using the microphone or camera now (exe empty for Store apps). */
export interface UsingApp { name: string; exe: string; since: number }
export interface InUse { mic: UsingApp[]; cam: UsingApp[] }
export interface TrayIcon { id: string; tip: string; icon: string | null; exe: string; promoted: boolean; rev: number }
export interface KeyboardLayout { id: number; code: string; name: string; locale: string; active: boolean }
export interface DiskInfo { mount: string; label: string; total: number; available: number; removable: boolean; fs: string }
export interface ScreenToday { total: number; apps: { exe: string; name: string; seconds: number }[] }
export type Sound = { kind: 'chime'; volume?: number } | { kind: 'azan'; src: string; short?: boolean; pauseMusic?: boolean; volume?: number } | { kind: 'adhan'; short: boolean; pauseMusic?: boolean; volume?: number } | { kind: 'connect' | 'disconnect' | 'plug' | 'unplug'; volume: number };

export interface AppWindow { id: number; title: string; exe: string; active: boolean; minimized: boolean; aumid?: string | null }
export interface AudioDevice { id: string; name: string; isDefault: boolean }
export interface AudioSession { pid: number; name: string; exe: string; volume: number; muted: boolean }
export interface AudioState { volume: number; muted: boolean; devices: AudioDevice[] }
export interface MediaSource { name: string; id: string; exe: string | null; window: number | null; aumid: string | null }
export interface MediaInfo {
  app: string; title: string; artist: string; album: string; art: string | null; playing: boolean;
  positionMs: number; durationMs: number; updatedAtMs: number;
  canNext: boolean; canPrev: boolean;
}
export interface RadioInfo { kind: 'wifi' | 'bluetooth' | 'other'; name: string; on: boolean }
export interface Hotspot { on: boolean; ssid: string; clients: number; maxClients: number }
/** Wireless displays nearby (Miracast) and whether this PC is casting to one now. */
/** Today's water: total and goal in ml, each drink, the last 7 days (oldest first). */
export interface WaterToday { total: number; goal: number; cup: number; entries: { at: number; ml: number }[]; week: { day0: number; ml: number }[]; hot: boolean }
export interface CastState { casting: boolean; displays: { id: string; name: string }[] }
export type DisplayMode = 'internal' | 'clone' | 'extend' | 'external';

export const ipc = {
  // config
  getConfig: () => invoke<Config>('get_config'),
  saveConfig: (config: Config) => invoke<void>('save_config', { config }),
  exportConfig: () => invoke<string>('export_config'),
  importConfig: (text: string) => invoke<void>('import_config', { text }),
  // bar + windows
  applyBar: () => invoke<void>('apply_bar'),
  /** Move the dock to this length (and spacing) without saving: a slider or handle being dragged. */
  barPreview: (length: number, gap: number | null = null, gapInner: number | null = null) => invoke<void>('bar_preview', { length, gap, gapInner }),
  barDrag: (on: boolean) => invoke<void>('bar_drag', { on }),
  /** Magnification: lengthen this dock by `along` CSS px (0 = normal). */
  barGrow: (along: number) => invoke<void>('bar_grow', { along }),
  magnifyPrepare: () => invoke<void>('magnify_prepare'),
  /** Show the magnifier over `r` (this dock's CSS px). */
  magnifyShow: (r: { x: number; y: number; width: number; height: number }) => invoke<void>('magnify_show', { x: r.x, y: r.y, w: r.width, h: r.height }),
  magnifyFrame: (icons: MagIcon[]) => invoke<void>('magnify_frame', { icons }),
  magnifyHide: () => invoke<void>('magnify_hide'),
  /** Minimize effects: the effect to play and where each window goes (this dock's CSS px). */
  effectsTargets: (t: EffectTargets) => invoke<void>('effects_targets', { t }),
  /** Restore a minimized window with the effect played in reverse, out of the dock. */
  appRestoreAnimated: (id: number) => invoke<void>('app_restore_animated', { id }),
  mouseDown: () => invoke<boolean>('mouse_down'),
  restoreTaskbar: () => invoke<void>('restore_taskbar'),
  setEditMode: (on: boolean) => invoke<void>('set_edit_mode', { on }),
  openSettings: (section?: string) => invoke<void>('open_settings_cmd', { section: section ?? null }),
  popupOpen: (instance: string, anchor: DOMRect, width: number, height: number, focus = true) =>
    invoke<void>('popup_open', { instance, x: anchor.x, y: anchor.y, w: anchor.width, h: anchor.height, width, height, focus }),
  popupLeave: () => invoke<void>('popup_leave'),
  thumbsShow: (items: { id: number; x: number; y: number; w: number; h: number }[]) => invoke<number[]>('thumbs_show', { items }),
  thumbsClear: () => invoke<void>('thumbs_clear'),
  /** Live window pictures drawn into this dock window (minimized-window spots); [] clears them. */
  barThumbs: (items: { id: number; x: number; y: number; w: number; h: number }[]) => invoke<number[]>('bar_thumbs', { items }),
  barReveal: (show: boolean) => invoke<void>('bar_reveal', { show }),
  popupHide: () => invoke<void>('popup_hide'),
  popupReveal: () => invoke<void>('popup_reveal'),
  healthPong: (seq: number) => invoke<void>('health_pong', { seq }),
  /** The version to show "What's new" for, if GlassBar was just updated. */
  whatsNewPending: () => invoke<string | null>('whats_new_pending'),
  whatsNewSeen: () => invoke<void>('whats_new_seen'),
  popupLeaveClose: () => invoke<void>('popup_leave_close'),
  /** Close a hover-opened panel now (a right-click on the dock); click-opened ones stay. */
  popupHoverHide: () => invoke<void>('popup_hover_hide'),
  /** A popup menu (ours or another app's, e.g. a tray icon's) is on screen. */
  menuOpen: () => invoke<boolean>('menu_open'),
  /** Close GlassBar (gives the screen space and the taskbar back). */
  appQuit: () => invoke<void>('app_quit'),
  /** A dock alert is about to show for `ms`: false over a full-screen app; slides an
   *  auto-hidden dock out for it when that setting is on. */
  dockAlertBegin: (kind: string, ms: number) => invoke<boolean>('dock_alert_begin', { kind, ms }),
  dockAlertEnd: () => invoke<void>('dock_alert_end'),
  /** Settings › Dock alerts › Test: show a sample alert of this kind on the dock. */
  dockAlertTest: (kind: string) => invoke<void>('dock_alert_test', { kind }),
  bannerFit: (width: number, height: number) => invoke<void>('banner_fit', { width, height }),
  bannerHide: () => invoke<void>('banner_hide'),
  jumpTo: (source: string, data: unknown) => invoke<void>('jump_to', { source, data }),
  /** Make the open popup exactly this tall (CSS px): panels size themselves to their content. */
  popupFit: (height: number) => invoke<void>('popup_fit', { height }),
  /** Make the open popup this wide (CSS px), keeping its place: e.g. room for lyrics. */
  popupWidth: (width: number) => invoke<void>('popup_width', { width }),
  // alerts
  alertsList: () => invoke<Alert[]>('alerts_list'),
  alertsMarkRead: () => invoke<void>('alerts_mark_read'),
  alertsDismiss: (id: number) => invoke<void>('alerts_dismiss', { id }),
  alertsClear: () => invoke<void>('alerts_clear'),
  // to-do
  todoList: () => invoke<Todo[]>('todo_list'),
  todoAdd: (text: string, dueAt: number | null) => invoke<void>('todo_add', { text, dueAt }),
  todoUpdate: (t: Todo) => invoke<void>('todo_update', { id: t.id, text: t.text, done: t.done, dueAt: t.dueAt }),
  todoDelete: (id: number) => invoke<void>('todo_delete', { id }),
  todoClearDone: () => invoke<void>('todo_clear_done'),
  // notes
  notesList: (trash = false) => invoke<Note[]>('notes_list', { trash }),
  noteCreate: () => invoke<Note>('note_create'),
  noteUpdate: (n: Note) => invoke<void>('note_update', { id: n.id, text: n.text, color: n.color, pinned: n.pinned }),
  noteDelete: (id: number) => invoke<void>('note_delete', { id }),
  noteRestore: (id: number) => invoke<void>('note_restore', { id }),
  noteAddImage: (id: number, data: string) => invoke<string>('note_add_image', { id, data }),
  noteRemoveImage: (id: number, name: string) => invoke<void>('note_remove_image', { id, name }),
  noteImage: (name: string) => invoke<string>('note_image', { name }),
  noteDragImage: (name: string) => invoke<void>('note_drag_image', { name }),
  // focus
  pomoGet: () => invoke<PomoState>('pomo_get'),
  pomoStart: () => invoke<PomoState>('pomo_start'),
  pomoPause: () => invoke<PomoState>('pomo_pause'),
  pomoSkip: () => invoke<PomoState>('pomo_skip'),
  pomoReset: () => invoke<PomoState>('pomo_reset'),
  pomoSetPhase: (phase: Phase) => invoke<PomoState>('pomo_set_phase', { phase }),
  timersList: () => invoke<Timer[]>('timers_list'),
  timerAdd: (label: string, durationMs: number) => invoke<void>('timer_add', { label, durationMs }),
  timerPause: (id: number) => invoke<void>('timer_pause', { id }),
  timerResume: (id: number) => invoke<void>('timer_resume', { id }),
  timerDelete: (id: number) => invoke<void>('timer_delete', { id }),
  timerSnooze: (id: number, ms: number) => invoke<void>('timer_snooze', { id, ms }),
  downloadsState: () => invoke<DlState>('downloads_state'),
  downloadsOpen: (path: string) => invoke<void>('downloads_open', { path }),
  downloadsReveal: (path: string) => invoke<void>('downloads_reveal', { path }),
  downloadsOpenFolder: () => invoke<void>('downloads_open_folder'),
  downloadsOpenWith: (path: string) => invoke<void>('downloads_open_with', { path }),
  downloadsRename: (path: string, name: string) => invoke<string>('downloads_rename', { path, name }),
  downloadsRecycle: (path: string) => invoke<void>('downloads_recycle', { path }),
  downloadsDrag: (path: string) => invoke<void>('downloads_drag', { path }),
  fileIcon: (path: string) => invoke<string | null>('file_icon', { path }),
  /** Hand a drag to Windows so shelved files can land in another app. `shelf:dropped` comes
   *  back with the paths once a drop really happens; a cancelled drag reports nothing. */
  shelfDrag: (instance: string, paths: string[]) => invoke<void>('shelf_drag', { instance, paths }),
  /** Add files with a picker instead of a drag. Empty = cancelled. */
  shelfPick: () => invoke<string[]>('shelf_pick'),
  recycleInfo: () => invoke<RecycleInfo>('recycle_info'),
  recycleEmpty: (root: string | null) => invoke<void>('recycle_empty', { root }),
  /** The newest items in the bin (a big bin takes a moment). */
  trashItems: (limit: number) => invoke<BinItem[]>('trash_items', { limit }),
  /** Restore by item id, or by original path (Undo after a drop). Returns how many came back. */
  trashRestore: (keys: string[]) => invoke<number>('trash_restore', { keys }),
  /** Move files to the Recycle Bin (never a permanent delete). */
  trashRecycle: (paths: string[]) => invoke<TrashResult>('trash_recycle', { paths }),
  /** Empty with Windows' own "are you sure" dialog. */
  trashEmptyConfirm: () => invoke<void>('trash_empty_confirm'),
  swGet: () => invoke<Stopwatch>('sw_get'),
  swToggle: () => invoke<Stopwatch>('sw_toggle'),
  swLap: () => invoke<Stopwatch>('sw_lap'),
  swReset: () => invoke<Stopwatch>('sw_reset'),
  // location + weather
  locationStatus: (force = false) => invoke<LocationStatus>('location_status', { force }),
  locationClear: () => invoke<LocationStatus>('location_clear'),
  locationOpenMap: (lat: number, lon: number) => invoke<void>('location_open_map', { lat, lon }),
  locationSearch: (query: string) => invoke<Place[]>('location_search', { query }),
  weatherGet: (force = false) => invoke<Weather | null>('weather_get', { force }),
  // waktu solat
  solatDays: (force = false) => invoke<SolatInfo>('solat_days', { force }),
  solatZones: () => invoke<[string, string, string][]>('solat_zones'),
  solatTestSound: (settings: object) => invoke<void>('solat_test_sound', { settings }),
  tasbihGet: () => invoke<TasbihState>('tasbih_get'),
  /** +1 (a tap) or -1 (undo); `steps` = the targets of the current dhikr, e.g. [33, 33, 34]. */
  tasbihAdd: (delta: 1 | -1, steps: number[]) => invoke<TasbihState>('tasbih_add', { delta, steps }),
  tasbihReset: () => invoke<TasbihState>('tasbih_reset'),
  // IPU
  ipuGet: (force = false) => invoke<IpuReading | null>('ipu_get', { force }),
  // battery + system monitor
  batteryGet: () => invoke<Battery | null>('battery_get'),
  sysmonHistory: () => invoke<Sample[]>('sysmon_history'),
  perfHistory: () => invoke<PerfSample[]>('perf_history'),
  perfState: () => invoke<PerfState>('perf_state'),
  /** The Performance page is open: sample every second (a lease; call every 2 s). */
  perfWatch: () => invoke<void>('perf_watch'),
  /** Is the auto-hidden bar slid away right now? */
  barAway: () => invoke<boolean>('bar_away'),
  disksList: () => invoke<DiskInfo[]>('disks_list'),
  powerGet: () => invoke<PowerInfo>('power_get'),
  powerSetMode: (mode: PowerMode) => invoke<void>('power_set_mode', { mode }),
  powerUseBalanced: () => invoke<void>('power_use_balanced'),
  // screen time
  screentimeToday: () => invoke<ScreenToday>('screentime_today'),
  screentimeWeek: () => invoke<[string, number][]>('screentime_week'),
  // start + search
  startMenu: () => invoke<void>('start_menu'),
  windowsSearch: () => invoke<void>('windows_search'),
  /** Open Windows Search with `text` typed in (the search panel's fallback). */
  windowsSearchText: (text: string) => invoke<void>('windows_search_text', { text }),
  // apps
  appsList: () => invoke<AppWindow[]>('apps_list'),
  appIcon: (exe: string, window: number | null = null, aumid: string | null = null) => invoke<string | null>('app_icon', { exe, window, aumid }),
  appActivate: (id: number) => invoke<void>('app_activate', { id }),
  appClose: (id: number) => invoke<void>('app_close', { id }),
  appLaunch: (path: string, args = '', aumid: string | null = null) => invoke<void>('app_launch', { path, args, aumid }),
  jumplist: (exe: string) => invoke<{ name: string; path: string }[]>('jumplist', { exe }),
  // audio
  audioState: () => invoke<AudioState>('audio_state'),
  audioSetVolume: (volume: number) => invoke<void>('audio_set_volume', { volume }),
  audioSetMute: (muted: boolean) => invoke<void>('audio_set_mute', { muted }),
  audioSetDefault: (id: string) => invoke<void>('audio_set_default', { id }),
  audioSessions: () => invoke<AudioSession[]>('audio_sessions'),
  audioSetSessionVolume: (pid: number, volume: number) => invoke<void>('audio_set_session_volume', { pid, volume }),
  audioSetSessionMute: (pid: number, muted: boolean) => invoke<void>('audio_set_session_mute', { pid, muted }),
  // media
  mediaInfo: () => invoke<MediaInfo | null>('media_info'),
  mediaControl: (action: 'toggle' | 'next' | 'prev') => invoke<void>('media_control', { action }),
  mediaSeek: (positionMs: number) => invoke<void>('media_seek', { positionMs }),
  /** The app that is playing (name, its window and program). */
  mediaSource: () => invoke<MediaSource | null>('media_source'),
  /** Bring the playing app to the front, or start it. */
  mediaSourceOpen: () => invoke<void>('media_source_open'),
  /** `lookup` false never touches the network: only a local .lrc or the cache is used. */
  lyricsGet: (title: string, artist: string, album: string, durationMs: number, lookup: boolean) =>
    invoke<Lyrics>('lyrics_get', { title, artist, album, durationMs, lookup }),
  lyricsFolder: () => invoke<string>('lyrics_folder'),
  /** Pause all playing media; returns the paused apps' ids for `mediaResume`. */
  mediaPausePlaying: () => invoke<string[]>('media_pause_playing'),
  mediaResume: (ids: string[]) => invoke<void>('media_resume', { ids }),
  // quick settings
  radios: () => invoke<RadioInfo[]>('radios_list'),
  setRadio: (kind: RadioInfo['kind'], on: boolean) => invoke<void>('radio_set', { kind, on }),
  setAirplane: (on: boolean) => invoke<void>('airplane_set', { on }),
  hotspotGet: () => invoke<Hotspot>('hotspot_get'),
  nightLightGet: () => invoke<boolean>('night_light_get'),
  nightLightSet: (on: boolean) => invoke<void>('night_light_set', { on }),
  /** Nearby sharing: 0 off, 1 my devices only, 2 everyone nearby. */
  nearbyGet: () => invoke<number>('nearby_get'),
  nearbySet: (mode: number) => invoke<void>('nearby_set', { mode }),
  castGet: () => invoke<CastState>('cast_get'),
  waterToday: () => invoke<WaterToday>('water_today'),
  waterAdd: (ml: number) => invoke<WaterToday>('water_add', { ml }),
  waterRemove: (at: number) => invoke<WaterToday>('water_remove', { at }),
  /** 'Later': no water reminder for this many minutes. */
  waterSnooze: (minutes: number) => invoke<void>('water_snooze', { minutes }),
  hotspotSet: (on: boolean) => invoke<void>('hotspot_set', { on }),
  awakeGet: () => invoke<AwakeState>('awake_get'),
  /** minutes null or 0 = until turned off. */
  awakeSet: (minutes: number | null, screen: boolean) => invoke<AwakeState>('awake_set', { minutes, screen }),
  awakeOff: () => invoke<AwakeState>('awake_off'),
  micState: () => invoke<MicState>('mic_state'),
  micSetMute: (muted: boolean) => invoke<void>('mic_set_mute', { muted }),
  micSetVolume: (volume: number) => invoke<void>('mic_set_volume', { volume }),
  micSetDefault: (id: string) => invoke<void>('mic_set_default', { id }),
  privacyInUse: () => invoke<InUse>('privacy_in_use'),
  fxRates: (base: string, force = false) => invoke<Rates>('fx_rates', { base, force }),
  ocrSnapStart: (purpose: SnapView['purpose'] = 'ocr') => invoke<void>('ocr_snap_start', { purpose }),
  ocrShot: () => invoke<SnapView | null>('ocr_shot'),
  ocrSnapFinish: (x: number, y: number, w: number, h: number, window: number | null = null) => invoke<string>('ocr_snap_finish', { x, y, w, h, window }),
  /** One display: saved at once. Several: the overlay opens to pick one (or all). */
  shotFullscreen: () => invoke<number>('shot_fullscreen'),
  shotsList: () => invoke<ShotItem[]>('shots_list'),
  shotsCopy: (id: number) => invoke<void>('shots_copy', { id }),
  shotsDelete: (id: number | null) => invoke<void>('shots_delete', { id }),
  shotsOpen: (id: number | null) => invoke<void>('shots_open', { id }),
  /** Stop a running scrolling screenshot; what was captured so far is saved. */
  shotScrollStop: () => invoke<void>('shot_scroll_stop'),
  recordScreen: (kind: 'area' | 'app') => invoke<void>('record_screen', { kind }),
  /** Save a Mirror photo (PNG) or video (WebM); the bytes go raw, not as JSON. Returns the path. */
  mirrorSave: (kind: 'photo' | 'video', bytes: Uint8Array) => invoke<string>('mirror_save', bytes, { headers: { kind } }),
  clipList: (query: string | null = null) => invoke<ClipItem[]>('clip_list', { query }),
  clipCopy: (id: number) => invoke<void>('clip_copy', { id }),
  clipPin: (id: number, on: boolean) => invoke<void>('clip_pin', { id, on }),
  clipDelete: (id: number | null) => invoke<void>('clip_delete', { id }),
  ocrSnapCancel: () => invoke<void>('ocr_snap_cancel'),
  ocrSnapShow: () => invoke<void>('ocr_snap_show'),
  ocrHistory: () => invoke<OcrItem[]>('ocr_history'),
  ocrHistoryDelete: (id: number | null) => invoke<void>('ocr_history_delete', { id }),
  ocrCopy: (text: string) => invoke<void>('ocr_copy', { text }),
  ocrHotkeyStatus: () => invoke<{ hotkey: string; error: string | null }>('ocr_hotkey_status'),
  colorHotkeyStatus: () => invoke<{ hotkey: string; error: string | null }>('color_hotkey_status'),
  /** A colour was picked in the overlay: `text` goes to the clipboard, the card shows `hex`. */
  colorPickFinish: (hex: string, text: string) => invoke<void>('color_pick_finish', { hex, text }),
  appPinTarget: (path: string) => invoke<PinTarget>('app_pin_target', { path }),
  appBrowse: () => invoke<PinTarget | null>('app_browse'),
  appsInstalled: () => invoke<InstalledApp[]>('apps_installed'),
  pathsExist: (paths: string[]) => invoke<boolean[]>('paths_exist', { paths }),
  wifiList: (scan: boolean) => invoke<{ current: string | null; networks: WifiNetwork[] }>('wifi_list', { scan }),
  wifiConnect: (ssid: string, password: string | null) => invoke<void>('wifi_connect', { ssid, password }),
  wifiDisconnect: () => invoke<void>('wifi_disconnect'),
  btList: (discover: boolean) => invoke<BtDevice[]>('bt_list', { discover }),
  btPair: (id: string) => invoke<void>('bt_pair', { id }),
  btUnpair: (id: string) => invoke<void>('bt_unpair', { id }),
  openSettingsUri: (uri: string) => invoke<void>('open_settings_uri', { uri }),
  /** Keep the live output-level meter running ~3 s more (the music visualizer calls this every 2 s). */
  audioMeter: () => invoke<void>('audio_meter'),
  audioSpectrum: () => invoke<void>('audio_spectrum'),
  audioSpectrumStream: () => invoke<string | null>('audio_spectrum_stream'),
  trayRefresh: () => invoke<void>('tray_refresh'),
  trayIcons: () => invoke<TrayIcon[]>('tray_icons'),
  trayClick: (id: string, button: 'left' | 'right' | 'double') => invoke<boolean>('tray_click', { id, button }),
  clockSystem12h: () => invoke<boolean | null>('clock_system_12h'),
  transparencyEffects: () => invoke<boolean>('transparency_effects'),
  appearanceSupport: () => invoke<AppearanceSupport>('appearance_support'),
  wallpaperBehind: () => invoke<{ min: number; max: number } | null>('wallpaper_behind'),
  langList: () => invoke<KeyboardLayout[]>('lang_list'),
  langSet: (id: number) => invoke<void>('lang_set', { id }),
  windowsNotifications: () => invoke<void>('windows_notifications'),
  showDesktop: () => invoke<void>('show_desktop'),
  displayMode: () => invoke<DisplayMode | null>('display_mode'),
  displayCount: () => invoke<number>('display_count'),
  setDisplayMode: (mode: DisplayMode) => invoke<void>('display_set_mode', { mode }),
  brightness: () => invoke<number | null>('brightness_get'),
  setBrightness: (value: number) => invoke<void>('brightness_set', { value }),
};

export const events = {
  configChanged: (cb: (c: Config) => void) => listen<Config>('config:changed', (e) => cb(e.payload)),
  behindChanged: (cb: (b: { min: number; max: number } | null) => void) => listen<{ min: number; max: number } | null>('appearance:behind', (e) => cb(e.payload)),
  materialChanged: (cb: (m: AppearanceSupport['applied']) => void) => listen<AppearanceSupport['applied']>('appearance:material', (e) => cb(e.payload)),
  captureDone: (cb: (d: CaptureDone) => void) => listen<CaptureDone>('capture:done', (e) => cb(e.payload)),
  /** A scrolling screenshot started (true) or ended (false). */
  captureScrolling: (cb: (on: boolean) => void) => listen<boolean>('capture:scrolling', (e) => cb(e.payload)),
  editMode: (cb: (on: boolean) => void) => listen<boolean>('editmode', (e) => cb(e.payload)),
  healthPing: (cb: (seq: number) => void) => own<number>('health:ping', cb),
  popupShow: (cb: (instance: string) => void) => listen<string>('popup:show', (e) => cb(e.payload)),
  popupHide: (cb: () => void) => listen('popup:hide', () => cb()),
  popupLeave: (cb: () => void) => listen('popup:leave', () => cb()),
  appsChanged: (cb: (w: AppWindow[]) => void) => listen<AppWindow[]>('apps:changed', (e) => cb(e.payload)),
  audioChanged: (cb: (s: AudioState) => void) => listen<AudioState>('audio:changed', (e) => cb(e.payload)),
  mediaChanged: (cb: (m: MediaInfo | null) => void) => listen<MediaInfo | null>('media:changed', (e) => cb(e.payload)),
  alertsChanged: (cb: () => void) => listen('alerts:changed', () => cb()),
  downloadsNew: (cb: (i: DlItem) => void) => listen<DlItem>('downloads:new', (e) => cb(e.payload)),
  bannerPush: (cb: (n: import('./notify').Note) => void) => listen<import('./notify').Note>('banner:push', (e) => cb(e.payload)),
  barJump: (cb: (j: { source: string; data: unknown; open: boolean }) => void) => own<{ source: string; data: unknown; open: boolean }>('bar:jump', cb),
  location: (cb: (s: LocationStatus) => void) => listen<LocationStatus>('location:status', (e) => cb(e.payload)),
  settingsSection: (cb: (s: string) => void) => listen<string>('settings:section', (e) => cb(e.payload)),
  sysmonSample: (cb: (s: Sample) => void) => listen<Sample>('sysmon:sample', (e) => cb(e.payload)),
  perfSample: (cb: (s: PerfSample) => void) => listen<PerfSample>('perf:sample', (e) => cb(e.payload)),
  /** The auto-hidden bar slid away (true) or back in (false). */
  magnifyFrame: (cb: (icons: MagIcon[]) => void) => listen<MagIcon[]>('magnify:frame', (e) => cb(e.payload)),
  magnifyReset: (cb: () => void) => own<null>('magnify:reset', () => cb()),
  barAway: (cb: (away: boolean) => void) => own<boolean>('bar:away', cb),
  soundPlay: (cb: (s: Sound) => void) => own<Sound>('sound:play', cb),
  chargerFlip: (cb: (f: ChargerFlip) => void) => own<ChargerFlip>('charger:flip', cb),
  waterChanged: (cb: (t: WaterToday) => void) => listen<WaterToday>('water:changed', (e) => cb(e.payload)),
  dockAlert: (cb: (a: import('../app/dockAlert').DockAlert) => void) => own<import('../app/dockAlert').DockAlert>('dock:alert', cb),
  shelfDropped: (cb: (d: { instance: string; paths: string[] }) => void) => own<{ instance: string; paths: string[] }>('shelf:dropped', cb),
  /** Back from sleep or the network changed (after the location re-check): fetch fresh data. */
  refresh: (cb: () => void) => listen('widgets:refresh', () => cb()),
  /** Any "<name>:changed" event from Rust (todos, notes, timers, ...). */
  changed: (name: string, cb: () => void) => listen(`${name}:changed`, () => cb()),
};
export type { UnlistenFn };
/** One enlarged dock icon, in the magnifier window's CSS px (x, y = top-left). */
type CssRect = { x: number; y: number; w: number; h: number };
export interface EffectTargets { effect: 'genie' | 'scale' | 'suck' | 'none'; toIcon: boolean; perWindow: Record<number, CssRect>; end: CssRect | null; dock: CssRect }
export interface MagIcon { src: string; x: number; y: number; size: number }
