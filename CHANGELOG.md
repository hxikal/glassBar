# Changelog

## 0.6.11 — 2026-11-02

### New
- **Settings › Storage** shows what GlassBar keeps on your PC, widget by widget, with a total. Each row offers only the cleanups that fit it, says what it removes and how much, and asks first. The database is backed up before any cleanup, and the newest backup is always kept.
- **Boost** (Settings › Performance and the System Monitor panel) frees memory held by idle background apps and can lower background apps that hog the CPU. It never closes or kills anything on its own; "Close apps I'm not using" is a separate list you tick.
- **Choose the azan voice** in Waktu Solat: Madina with rain (the new default), Madina (Nabawi), Jeddah (Al-Zaili) or Classic. Short adzan stops at each voice's own takbir pause. Your own recording still replaces the voice.
- **Water** shows the time to the next reminder, and the panel says when reminders are off, with a Turn on button.
- **App Folders** show 2–4 folders on the dock (3 by default) with a "+n" chip that opens All folders. Right-click a folder there to open or delete it.
- **Waktu Solat:** Tasbih pin and hide buttons in its panel, and switches in the panel options.

### Fixed
- **Volume follows the mouse wheel smoothly** on the sound and music pills and panels; it no longer jumps back mid-scroll.
- **An adzan replaced by another** (Test pressed twice) no longer ends early: Stop stays, and paused music resumes only at the real end.

### Removed
- **The Companion widget.** It is dropped from saved profiles.

## 0.6.10 — 2026-11-02

### Fixed
- **Store apps show in search and when adding apps.** Spotify, WhatsApp, Calculator, Notepad, Settings, Photos and other Microsoft Store apps now appear in GlassBar search, in Folders › Add app and in Apps › Pin from installed apps. They open the way the Start menu opens them, with their own icons.
- **Settings › Performance shows GlassBar's real memory use.** It counted apps you opened from the dock as GlassBar's (it once showed 4 GB). It now counts GlassBar and its WebView2 helpers only, the way Task Manager does.
- **"The file was damaged" no longer repeats on every start.** A small fault in the screen-time index is repaired once and stays repaired. A second GlassBar launch no longer touches the database.

### New
- **Settings › Performance lists each process** (GlassBar, WebView2 Manager, GPU Process, Renderers…) with its own memory, CPU and GPU, named as in Task Manager.

## 0.6.9 — 2026-11-02

### New
- **A clearer Settings window.** The menu has an icon for every page and is sorted into four groups (Dock, Alerts, System, GlassBar), on its own colour so it stands apart from the page. Up and Down arrows move through it. Each page opens with a short line on what it controls, and long pages are split into titled cards (the Bar page: Placement, Size, Spacing, Behaviour, Windows taskbar).
- **Settings follows your theme.** The menu, page and cards take their colours from Appearance: light or dark, your custom background colour, your accent and your design style's corners. Glass and Liquid Glass give the menu a soft wash of the accent.
- **Updates moved into About.** About now shows the version, a Check now button for updates and What's new together.
- **Easier to read:** on/off settings are switches, slider values sit in a small badge next to their name, and keyboard focus is clearly outlined.

### Fixed
- **Widgets › Search** no longer draws a box inside its box.
- **Profiles:** the widget count lines up with the profile name.

## 0.6.8 — 2026-10-30

### New
- **Focus across the dock:** while a Pomodoro focus session runs, the whole dock shows the time left, a timeline from start to end, your session dots, and Pause, Skip and Stop. Point at the dock and your widgets come back so you can use them as usual; move away and the focus view returns. On by default; switch it off in the Pomodoro settings ("Show focus across the dock").
- **What's new after an update:** the first time a new version starts, a card by the dock shows what changed. Close it, or open all changes in Settings › About.
- **Mirror › Square:** a square shape between Circle and Rectangle; photos save square too.
- **Capture on every display:** with more than one screen, area and window screenshots, scrolling screenshots, OCR Snap and the colour picker cover all of them, so you can select on any screen. The hint appears on the screen with your pointer.
- **Full-screen screenshot: choose the display.** With more than one screen, Full screen freezes them all and lets you pick: point at a display and click (or press its number), or choose All displays (A).
- **Sliders follow the mouse wheel** in Settings and panels (rest the pointer on one first, so scrolling a panel still scrolls it).

### Fixed
- **The dock is centred.** It sat 8 px to the right (and 1 px low): Windows' invisible window border was counted as part of the dock. The gaps at both ends are now equal, at any length. Panels open exactly above their widget for the same reason.
- **Length and spacing sliders are smooth:** the dock follows them while you drag, and the setting is saved once you let go. Other sliders (music and volume lengths, Equalizer, Mirror, water) no longer save on every step, which made them stutter.
- **Moving widgets lands where you point.** Holding a widget and moving it now works on every widget, also when you press on one of its buttons (Downloads' file, Start). Widgets that grow while pointed at (Downloads) no longer throw the target off, the dock scrolls when you drag near either end, and a shelved file or icon image can't hijack the move.

## 0.6.7 — 2026-10-30

### New
- **Mirror widget:** your webcam as a mirror, one click away on the dock. Choose a circle or a rectangle, flip it like a real mirror, and adjust the brightness, zoom and a ring light that brightens your face in a dark room. Take a photo (with an optional 3-second timer) or record a video. They save what you see: photos go to Pictures › GlassBar Mirror, videos to Videos › GlassBar Mirror. More options: contrast, a grid, and which camera to use. The camera is on only while the panel is open.
- **Recycle Bin widget:** a can on the dock that shows at a glance whether the bin is full, and how much is in it. Drop files on it to move them to the Recycle Bin (never deleted outright), with Undo. Its panel lists what you deleted most recently with a Restore button for each, and empties the bin after asking. Right-click: open, empty (Windows asks first), restore the last item. It updates within a couple of seconds when you delete something anywhere.
- **Countdowns widget:** days to Hari Raya, Merdeka, Tahun Baru Cina, a deadline or any date, with a progress ring on the dock. Start from a preset (Hijri and Chinese dates are calculated, so they may differ by a day) or set your own date and time. Repeating ones move to next year by themselves. A notification on the day, and a day before if you like.
- **Quick Settings › Keep awake:** one tile stops the PC sleeping (and the screen dimming) for 30 minutes to 4 hours or until you turn it off. A cup on the dock shows it's on; it always ends when GlassBar closes.
- **Quick Settings › Microphone:** mute every app's microphone with one tile. Its page has the mic level, the input device, and which apps are using the microphone or camera right now. On the dock, a muted mic shows red, and an orange dot (mic) or green dot (camera) shows when an app is using it. Click the mic there to mute or unmute.
- **Quick Settings › Bluetooth battery:** connected headphones, phones and other devices that report their battery show it in the panel and on the Bluetooth page, with one notification when one drops to 15 %.
- **Capture › Colour picker:** a fifth button and a hotkey (Win+Shift+C, or Ctrl+Alt+C when that's taken) freeze the screen and show a magnifier. Click a pixel and its colour is copied as HEX, RGB or HSL; the last 16 colours are kept.
- **Waktu Solat › Hijri date and Tasbih:** the panel shows today's Hijri date in Malay (JAKIM's, or calculated), turning to the next day at Maghrib. A Tasbih counter sits beside the prayer pill (click to count, scroll down to undo, right-click for target and dhikr) and in the panel, with Tasbih Fatimah (33 · 33 · 34) and reset by holding. Both can be switched off.
- **Search that calculates:** the Search button now opens GlassBar's search. Type a sum (`15% of 2400 + 18`), a conversion (`5 km to mi`, `100 f to c`) or money (`100 usd to myr`) and the answer is there as you type; Enter copies it. It also finds installed apps, and passes anything else to Windows Search or the web. Choose Windows Search instead in the App Launcher's Start & Search options.

## 0.6.6 — 2026-10-29

### Changed
- **Scrolling screenshots keep a site's side menu.** Parts of the window that don't scroll at the sides (a site's menu, the browser's tab strip) used to be cut off. They now appear as on the first screen, and further down they continue in their own background colour instead of repeating. Only the scrollbar is left out.

## 0.6.5 — 2026-10-29

### Fixed
- **The adzan no longer shows up on the music player.** It played as a media session, so Windows put it on the dock's player (and its controls). It now plays like GlassBar's other alert sounds, so the player keeps showing your music.
- **The Equalizer ignores the volume level.** It used to listen after Windows' volume, so turning the volume down shrank the bars and turning it up grew them. It now listens to the apps themselves, before the volume.
- **The Equalizer leaves out GlassBar's own sounds:** the adzan, chimes and the volume tick no longer make the bars move. Only your music and other apps do. (On Windows older than 10 version 2004 it works as before.)

## 0.6.4 — 2026-10-29

### New
- **Stop a scrolling screenshot:** while it scrolls, the Screenshot buttons on the dock turn into a red Stop button. Press it to end the capture and keep what's been captured so far.
- **Seek from the dock:** the music widget's progress line is now a seek bar. Hover it to see the time under your pointer, click or drag to jump there, or use the arrow keys (5 s).

### Fixed
- **Scrolling screenshots reach the bottom of web pages.** On pages with photos (galleries, image-heavy sections) the capture stopped halfway, because the browser redraws pictures slightly differently after each scroll and loads them in as they appear. Pages with a fixed background picture (the frame behind the content stays put) stopped after the first screen for the same reason. It now uses how far one wheel step scrolls the page (a browser's usual distance at first, then what it measures) when frames don't line up exactly, and the shorter last step at the bottom, but only when that distance is clearly the best match, so repeating lists aren't misjoined.

## 0.6.3 — 2026-10-29

### Fixed
- **To-do, Notes and Pomodoro save again.** After two copies of GlassBar overlapped (an update or a self-repair restart), GlassBar's link to its database could go bad: adding a to-do or note said "database disk image is malformed", and Pomodoro's Start did nothing. GlassBar now reopens the database by itself when that happens and tries again, so nothing is lost. If the file itself is ever damaged, it is checked at start, a copy is kept, and everything still readable is moved to a fresh one.
- **A daily backup of your data:** once a day at start GlassBar copies its database to `glassbar.db.bak` (keeping the day before as `.bak.old`), so to-dos, notes and history can always be brought back.
- Start-up clean-up no longer deletes screenshot and clipboard image files just because the database doesn't list them (an empty database used to mean every one of them was deleted).
- Notes and Pomodoro now show an error when something can't be saved, instead of failing silently.
- **Self-repair restarts once.** A stuck GlassBar no longer starts a new copy every 5 seconds, and the new copy closes a stuck old one, so two never use the database at once.
- **Weather, IPU and Waktu Solat no longer say "Location unavailable" after sleep or a network blip.** When a location check fails, they keep using the last place that worked (shown as "Last known") and GlassBar keeps trying.

### Changed
- **Fresh data after sleep and when the network changes:** Weather, IPU and Waktu Solat fetch new data right away, instead of waiting up to 30 minutes. They still refresh on start and on their usual timers (Weather 30 min, IPU 15 min, prayer times daily).
- **Refresh buttons** on the IPU and Waktu Solat panels.
- **Quick Settings:** the Wi-Fi and Bluetooth tiles show what they're connected to (network name, device names). If Windows won't say, the tile tells you why instead of "Not connected".

## 0.6.2 — 2026-10-29

### Fixed
- **The dock no longer goes dead** (letters instead of app icons, music and widgets not responding). When Windows Location stopped answering, usually after sleep or while screens and networks changed, every location check by Weather, prayer times and IPU waited forever and used up GlassBar's workers. Windows Location now gets 20 seconds, and other widgets use the last known place while a check runs.
- **Self-repair:** if the dock stops answering because GlassBar's workers are all stuck, it restarts at once instead of only reloading.
- Scrolling the volume on the dock no longer shows the volume dock alert. Some audio drivers echo every change GlassBar makes, and the echoes looked like changes from elsewhere. Volume keys still show it.

## 0.6.1 — 2026-10-28

### Fixed
- **Clicking an app on the dock now switches to that app.** With magnification on, a dock with more widgets than fit slid its icons under the pointer while you hovered, so a click often landed on the app next to the one you meant, and seemed to do nothing.
- **Magnification:** the enlarged icons sit on their own icons again (they were drawn off to one side, leaving a gap). Icons now slide apart around the pointer like on a Mac, and the one under the pointer stays put. The dock no longer gets longer while you hover.
- A window whose app icon is scrolled out of view minimizes into the middle of the dock instead of flying off-screen.

## 0.6.0 — 2026-10-28

### New
- **Magnification (App Launcher):** icons grow under your pointer and their neighbours a little less, popping out above the dock like on a Mac. The dock gets longer while you hover if they need the room. Off by default; turn it on and set the size in the App Launcher's options.
- **Minimize windows using Genie, Scale or Suck:** a minimizing window flies into its app's icon, and comes back out when you restore it from the dock. Genie curves it in; Scale simply shrinks it; Suck pulls it in from a corner. On by default, or Off.
- **Animate opening applications:** an app's icon bounces until its window appears (at most 10 s). On by default.
- **Minimize windows into application icon:** turn it off and each minimized window gets its own spot at the end of the launcher, with a live picture of the window.
- **Settings › Bar › Display:** Main display, **All displays** (a dock on every screen), or one display you choose.

### Changed
- Changing the volume no longer makes the Equalizer bars jump: they skip GlassBar's own volume tick.
- Settings › Performance marks the active mode clearly, and the graph shows the reading under your pointer.

### Notes
- Windows can't bend other apps' windows, so the effects are drawn by GlassBar over the screen using Windows' live window pictures. Genie is made of thin strips, so very close up its curve is slightly stepped.
- While a minimize effect is on, Windows' own minimize, restore and maximize animations are switched off so they don't play under GlassBar's. Set the effect to **Off** to get them back.
- The reverse effect plays when you restore a window from GlassBar; restoring from Alt-Tab or elsewhere doesn't animate.
- Windows of apps running as administrator (when GlassBar isn't) minimize without the effect.
- In Low performance mode the minimize effect is always Scale, icons don't bounce (the dot blinks) and magnification follows without easing.

## 0.5.0 — 2026-10-28

### New
- **Settings › Performance:** see what GlassBar itself uses — RAM, CPU, GPU and data downloaded — counting its WebView2 helpers too, with a graph of the last hour for each.
- **Performance modes: Low, Normal and High.** Each card says what it does, and the page lists what the chosen mode changes in every widget.
  - **Low:** the lightest. Animations stop unless they show something, the Equalizer draws at 15 fps, System monitor checks every 10 s, and a popup or banner hidden for 30 s gives its memory back (it opens a moment slower).
  - **Normal** (the default): smooth where it counts, calm when idle — the Equalizer up to 30 fps, the battery wave only while charging.
  - **High:** everything at full smoothness, including 60 fps and animations while the dock is auto-hidden.
- **Auto:** drops to Low while your PC is short on memory or CPU, or battery saver is on, and goes back to Normal once it's calm. The page says why it chose.

### Changed
- Animations pause while the dock is auto-hidden (except in High).
- GlassBar uses less CPU when idle: the music badge, liquid battery wave, Equalizer (when silent) and the Companion's sway no longer redraw the dock non-stop.
- System monitor reads the temperature only when it's shown.

## 0.4.0 — 2026-10-27

### New
- **Dock alerts:** for a moment the whole dock says one thing, then goes back to your widgets. The charger flash is now one of them, joined by:
  - **Volume and brightness**, when they change from your keys, Windows or another app (not GlassBar's own sliders). Windows' own volume and brightness pop-up is hidden while GlassBar shows its own (switchable).
  - **Prayer time**, with **Stop adhan** while the adzan plays. The adzan now plays whatever your banner settings are.
  - **Pomodoro**: "Focus done" / "Break over", with Start, Pause and Skip.
  - **Timer**: "Time's up", with +1 min and +5 min.
  - **Water reminders**, with "+250 ml" and "Later".
  An alert stays while your mouse is on it, slides an auto-hidden dock out (and back), stays out of full-screen apps (except prayer time), and has its own switch and **Test** button in **Settings › Dock alerts** — where the charger flash switch now lives too.
- **Water widget:** a daily goal (fixed, or from your weight with a little more on hot days), one-tap cups with Undo, today's drinks and the last 7 days, and reminders every so often after your last drink. The widget and its panel fill with water up to today's level.
- **App Folders widget:** phone-style folders of apps on the dock. Add apps from a searchable list, drag icons in from the App Launcher or drop programs from Explorer, rename, reorder, drag out to remove.
- **Lyrics** work on every music design, as a second part of the widget, and the whole song shows beside the player in the music panel — click a line to jump there. A **Lyrics** switch sits at the top of the panel. Songs with no lyrics say so; the line hides while the music is paused.
- **Music:** the playing app (Spotify, Chrome, Edge, Firefox…) shows on the album art and in the panel — click it to bring that app to the front.
- **Quick Settings:** Night light, Nearby sharing and Cast tiles; Hotspot's › opens its settings; **Edit tiles** to choose which show; split tiles readable on any accent colour.
- **Battery › Fill design:** the battery part fills to the charge level, % in the middle, a bolt while plugged in.
- **Equalizer:** shrinks to a small square when silent and grows back with the music; the radial ring sizes itself; **Fill the whole widget** (on by default).
- **File Shelf:** shows the newest file and "+N", opening on hover.
- **Settings › Bar:** set the space towards the screen edge and towards your windows separately.
- **Windows logo colour** for the Start button (App Launcher › Start & Search).
- **Close GlassBar** in the dock's right-click menu.

### Changed
- Prayer time, Pomodoro and Timer no longer make banners — the dock tells you (they're still in the Notification Center), and they're gone from Settings › Notifications.
- The music panel opens on click only (hover is an option). Lyrics sit beside the player however the panel opens.
- A widget that opens up under the pointer scrolls fully into view; dragging a widget or an app icon near either end scrolls the dock.

### Fixed
- Lyrics missing for songs that have them (too strict a length match, a busy lyrics site, wrong "no lyrics" answers remembered for a day).
- Clicking a lyric line jumped back and forth before settling.
- A hover panel could stay open after a right-click on the dock.
- Scrolling the dock changed the volume when passing over a volume control.
- Labels were unreadable on a white, yellow or light accent colour.
- Tray icons didn't fold away when the mouse left them.

## 0.3.0 — 2026-10-27

### New
- **The dock shows your charge level when the charger moves.** Plug the charger in or unplug it and the whole dock becomes a battery display for a couple of seconds — the level as a filled background, the percentage, and how long is left — then it goes back to your widgets. No notification. On by default; turn it off in Settings › Sounds. It only ever reacts to a real plug change, never to the battery level ticking down while it stays plugged in.
- **Music › Custom design:** pick every part yourself. Album art (circle, rounded or square, and its size), title, artist, album, the app name, the progress bar, each of Previous / Play-Pause / Next / volume, elapsed / left / total time in any combination, and either the output level meter or the real frequency spectrum in seven looks. Player, Classic and Mini are unchanged.
- **Lyrics** (Music › Lyrics): the line that's playing, beside the pill, on the left or the right. Choose its length, text size, weight, italics or capitals, and how each line arrives — fading, sliding up, sliding in from the side, typing itself out, karaoke (the words fill in as they're sung), or just appearing. Off by default, because turning it on looks lyrics up online.
- **File Shelf:** park up to 5 files by the dock and pick them up later. Drag them in, click to open, right-click for Show in folder or Open with, and drag one out into Explorer, a chat or an email to take it off the shelf. It remembers only where the files are, so yours are never moved, copied or deleted, and it survives a restart. A file that's been moved or deleted since shows struck through instead of vanishing.
- **Companion:** a small creature on the dock. It blinks on its own, giggles when you wiggle the pointer across it (and gets dizzy if you keep going), winks when clicked, and changes its eyes for what's going on: the charger, a battery under 15 %, music playing (it sings along), a prayer time arriving, a Pomodoro working or on a break, Do Not Disturb, and being left alone. Fifteen faces. Every trigger can be switched off, and its size, background, eye colour, hands and blink rate are yours. It holds still whenever the dock isn't on screen.

### Changed
- The Equalizer and the Music widget's spectrum are now the same piece of code, so they look and cost the same.

## 0.2.7 — 2026-10-27

### New
- **App Launcher:** the icons behind "+N" slide open smoothly when you hover, and fold away just as smoothly when you leave.
- **Quick Settings › Display:** with only one display connected, Duplicate, Extend and Second only are dimmed and say so, instead of failing with an error.
- **Clipboard** keeps your 20 latest copies (pinned ones on top, not counted, never removed).

### Fixed
- The Equalizer keeps moving while a right-click menu is open (it froze until the menu closed).
- A thin bright line on the Equalizer's right edge at some lengths (at 125 % display scaling).
- Pomodoro on the dock: the play/pause button starts and pauses straight away, without opening the panel.
- Leftover clipboard images from older versions are cleaned up.

## 0.2.6 — 2026-10-27

### New
- **Clipboard:** pin copies. Pinned ones stay at the top and are never removed, not even by Clear. The hover menu shows pinned copies first.
- **Screenshots and OCR** keep only your 5 latest (older screenshots are deleted from GlassBar's folder).

### Removed
- The **Ask Claude** and **AI Usage** widgets, and with them OCR's Claude engine (OCR now always uses Windows OCR, offline).

### Fixed
- Panels (the screenshot card, hover menus) could stay a little too short and show a scrollbar, or jiggle at 125 % display scaling. They now always fit their content, also when opened again.

## 0.2.5 — 2026-10-27

### Fixed
- **A stuck dock repairs itself.** After sleep the dock could keep showing but stop reacting to clicks. GlassBar now checks every few seconds that the dock still answers; if not, it reloads it, and restarts itself if that doesn't help. Time asleep never counts.
- The dock is placed again after waking from sleep and shortly after start-up, so it can't stay at the wrong edge.
- The Equalizer settings panel opened empty when its colour was set to Solid.

### New
- A small diagnostics log (`%APPDATA%\GlassBar\glassbar.log`): start-ups, wake-ups, monitor changes, where the dock was placed and any self-repair. No personal data.

## 0.2.4 — 2026-10-26

### New
- **Equalizer widget:** a live spectrum of what's playing, in many looks (Bars, Mirror, Glow spikes, LED, particle wave, line/area, radial), with an optional reflection, presets and colours from the album art. It pauses on battery to save power.
- **Hover panels:** hover the music player to see its panel. Hover a Capture button for a quick menu: Screenshot (Full screen, Area, Window, Scrolling), Recording (area or screen, current app), OCR (Snap text and its mode) or your last 5 copies. A click still opens the full panel.

### Fixed
- No black flash when a screenshot or OCR selection is released, and the overlay opens and closes without fading.
- The "Screenshot copied" card no longer appears as an empty box before its picture.
- Hover panels no longer close by themselves just after opening (a tooltip or another window appearing under the pointer).

## 0.2.3 — 2026-10-25

### New
- **Settings › Bar › Space from the edge:** set the gap between the dock and its screen edge (0–48 px), in any position.
- **App Launcher:** put Start & Search on the left or the right of your apps. Choose how many icons show; the rest collapse into a "+N" chip, and hovering the launcher shows them all. The app you're using always stays visible.
- **Waktu Solat:** adzan volume.
- The arrows that page through more widgets show only while the pointer is on the dock.
- Clock shows the weather by default.
- System Monitor: the Recycle Bin icon sits above its size.

### Fixed
- Screenshot and OCR Snap no longer flash a black screen before the overlay appears.
- After a restart the Windows taskbar stays hidden, and GlassBar starts with Windows from the installed app (it could point at an old copy, making the dock slow to appear).

## 0.2.2 — 2026-10-24

### New
- **AI Usage:** a "Claude plan (claude.ai)" source. Sign in to claude.ai once and see Current session and This week, like claude.ai › Settings › Usage. Works for people who use the Claude app rather than Claude Code.
- **Music (Classic):** Previous and Next buttons at every size, not only Large.

### Fixed
- Scrolling screenshots capture the whole page in browsers: they wait for smooth scrolling to finish, ignore side panels and vertical tabs that don't scroll, cope with moving pictures and videos, leave out the taskbar or a dock covering the window, and no longer show the scrollbar or corner marks at each join.

## 0.2.1 — 2026-10-24

### New
- Right-click the dock › **Show the desktop** (after Task Manager).
- New installs start with Windows, hide the Windows taskbar, and show tray icons on the dock. Change them in Settings › Startup, Settings › Bar and Quick Settings.

### Fixed
- Auto-hide: resting the pointer on the screen edge below the dock no longer makes it hide and reappear over and over.
- The dock stays above other apps: it comes to the front every time it slides in, and clicking the desktop no longer drops it behind windows.

## 0.2.0 — 2026-10-24

### New
- **Settings › Appearance:** themes (Light, Dark, Glass, System, Custom) and design styles inspired by macOS, Windows, GNOME, KDE Plasma and Cinnamon, with a live preview in all four dock positions. Choose the Windows material (Acrylic, Mica or classic blur) or turn blur off; set separate opacity for the dock, panels and widget cards; pick accent and background colours, contrast, corner radius, shadows, borders, icon size, spacing and animations. Save your own presets, revert single settings, or reset. Effects this Windows can't draw say so and fall back.
- **Liquid Glass theme:** Apple-style glass for the dock, flyouts and widgets: a soft sheen that follows the pointer, bright rims, white toggles when on, thin sliders, and a faint colour rim on open panels. The tint rises automatically where your wallpaper would make text hard to read.
- **Settings › Location:** one shared location for Weather, sunrise/sunset, Waktu Solat, IPU and the world clock. Choose Automatic, Windows Location, IP or Manual; see the source, accuracy, time zone and when it was last checked. Widgets show where their location comes from and link here.
- **Settings › Widgets:** a visual widget manager with a live preview of your dock, categories, search, on/off switches that keep a widget's settings, sizes, drag-to-reorder and layout presets.
- **Settings › About:** version and this changelog.
- **Left/right dock:** a side width option (Slim, Regular, Wide) and designs made for a side dock for System Monitor, Clock & Weather, IPU, Pomodoro / To-Do / Notes and Quick Settings (connectivity, volume, battery).
- Notification banners next to the dock that jump to their widget, per-widget banner switches and Do Not Disturb.
- Downloads widget; Recycle Bin in the System Monitor.
- Capture widget: screenshots (full screen, area, window, scrolling page), recording, OCR Snap and clipboard history, with icon styles.
- Pomodoro, To-Do and Notes in one Productivity widget.
- **Pomodoro redesign:** one colour per phase (focus, short break, long break) across the whole card; a ring that drains as time runs out; Focus/Short/Long switch; session dots that say what comes next; keyboard shortcuts (Space, R, S, 1–3); today's focus with a daily goal, a 7-day chart, your streak and longest session; optional auto-start, sound volume and dimming the rest of the dock while you focus. On the dock: a mini ring (S), with the time (M) and the session (L); hover for quick pause/resume.
- Quick Settings gathers volume, battery, keyboard language, Wi-Fi, Bluetooth, mobile hotspot and real tray icons.
- Weather joins the clock: forecast, feels-like, humidity, wind, rain chance and a sunrise/sunset arc.
- Clock formats (12/24 h, date presets, custom pattern, layouts) and world clocks with day/night.
- Waktu Solat with adzan at prayer time (short, full or normal alert) and a prayer pill.
- IPU air quality with a Malaysia state/station browser, other-country AQI and favourites.
- **AI Usage:** how much of your Claude and ChatGPT limits you have used. Connect an Anthropic or OpenAI Admin key (kept in Windows Credential Manager, checked on save), Claude Code (read from its local logs, no sign-in), your Claude plan limits (from Claude Code's login, no sign-in, unofficial) or your ChatGPT plan (sign-in window, unofficial). On the dock: rings (S), the most urgent number (M) or a row per provider with time to reset (L), coloured by how full. The panel shows the limit, session / today / week / month, token split, a 30-day chart, models, projects and a cost estimate in USD and RM. Banners at 80% and 95% and when a limit resets. Only limits a source publishes are shown.
- Currency converter, Ask Claude (quick questions), Time Progress, Screen Time with a mini graph, Timer & Stopwatch.
- System Monitor: CPU, RAM, network, disk and CPU temperature.
- App launcher: pinning, drag-reorder, live window previews, recent-file jump lists, Start & Search buttons.
- Music player with album art and a live level visualizer.
- Left and right dock layouts, auto-hide, reserved screen space, and a right-click menu for position.
- Charger and USB device sounds.
- Automatic updates from GitHub releases.

### Fixed
- The dock, panels and notification banners no longer show a black background behind their glass; Acrylic and Blur keep blurring while the dock isn't the active window.
- The bar keeps its size and position after restarts, DPI changes and auto-hide changes.
- Panels fit their content and follow the light theme when Windows transparency effects are off.
- Store apps launch with their real icons and survive app updates.
- Tray icons appear reliably after start-up.
- Battery reacts instantly to plugging in and unplugging.
- Menus, hover previews and drag-to-reorder work every time.

## 0.1.0 — 2026-10-22

### New
- First version: a glass bar that replaces the Windows taskbar, with placement on any edge, a settings window, a popup panel per widget and five widgets (Start, Apps, Music, Quick Settings, Clock).
