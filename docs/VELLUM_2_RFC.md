# Vellum 2.0 — Agent Implementation RFC and Roadmap

Status: **Proposal / research-first** (2026-11-05). This is a brief for coding agents, not an approved implementation architecture.

## Mission

Evolve [Vellum](https://github.com/usoy410/vellum) from a WebKit-only HTML wallpaper runner into a **low-overhead, interactive, wallpaper-first Linux engine**, while **retaining optional full HTML/CSS/JavaScript customization**.

User experience: one command/hotkey toggles wallpaper on/off; wallpaper fits the monitor without browser UI, scrollbars, accidental scrolling, or window decorations; character sprites can blink, breathe, track cursor and react to head/body/legs/throne hit regions, without disrupting normal window interaction. Example benchmark content: an Esdeath-on-ice-throne scene with static background and separately animatable sprites.

**Agents: this document is intentionally open to better designs.** You are encouraged to question the proposed native+WebKit hybrid. If a simpler or faster solution performs better and preserves requirements, present evidence, tradeoffs, proof-of-concept, and migration plan. Do not blindly implement this design.

## Current repository findings (verify against latest master)

- Rust 2021, GTK3, WebKit2GTK, gtk-layer-shell, GDK/GLib; entry point: `src/main.rs`.
- Wayland sets GTK Layer Shell `Background`, anchors to all edges; X11 uses Desktop window hint.
- HTML/URL loading and a Unix-domain socket command `vellum set <path-or-url>`.
- `kill_competitors()` runs pkill against several other wallpaper processes on startup, potentially destructive to user session. Replace this behavior with explicit opt-in conflict handling.
- The Makefile's `user-install` expects `assets/vellum.desktop`, which was missing during installation. Audit the tracked asset and prevent future install failure. Review duplicate autostart paths (systemd enable AND desktop autostart).
- `glib::MainContext::channel` deprecated; do not perform unrelated refactoring without tests.
- Current documentation describes the background as non-interactive, but on user's working Wayland environment HTML hover **and click** were tested successfully on exposed desktop background. Preserve this behavior; do not assume global input capture is possible.
- User-observed Vellum service cgroup: about 217 MB memory shortly after starting; about 297 MB peak over 13m29s; 3.628 seconds aggregate CPU time (approximately 0.45% of *one* core averaged). These are non-controlled observations, NOT authoritative benchmarks, not exclusively the main process, and do not establish ongoing steady-state usage.

## Non-negotiable functional requirements

1. **Native 2D wallpaper mode** that does not initialize WebKit or load a web engine at all. Support static textures, transforms, text, sprite atlases, alpha, bounded hitboxes, idle/hover/click animations and state-machine transitions. Prefer declarative JSON/TOML scenes; choose a versioned schema and document it.
2. **Legacy HTML mode** retains local HTML/CSS/JS and URL support, including WebKit hover/click behavior on exposed desktop, and the existing `vellum set` contract.
3. **Wallpaper defaults**: monitor-sized fixed viewport, content-fit/cover options, no scrolling/scrollbars/overscroll or decorations, no taskbar entry. In HTML mode, apply a default CSS/viewport policy with an explicit opt-out when content needs it.
4. **Input correctness**: native per-region hit testing; pointer events only when allowed/delivered by compositor. Never promise click-through to other windows or read their pointer input; never install global interception or bypass Wayland protections. Preserve focus/workspace and application input. Optional interactive mode only if needed.
5. **Efficient rendering**: no frame production for truly unchanged scenes; cap active animation frame rate; animated textures should be preloaded (atlas preferable), no per-frame asset decoding, incremental GPU texture updates where suitable. Provide Eco/Balanced/Quality/static profiles and optional reduced-motion behavior.
6. **Reliability**: graceful fallback to existing WebKit backend; no deleting users' chosen wallpaper setup, no unconditional `pkill` of competing wallpaper programs; predictable start/stop/toggle; clean IPC and shutdown; handle multi-output, hotplug and DPI scaling.
7. **Desktop coverage**: test wlroots-compatible Wayland sessions (Niri, Hyprland, Sway); test KDE Plasma Wayland separately (Layer Shell may work, but interactions/desktop layering must be verified). GNOME Wayland likely requires a GNOME Shell extension or compositor integration and must be a separate research spike. X11 requires an audited desktop-window backend. No false claims of support prior to validation.
8. **Keep compatibility**: do not break existing CLI, service users, or HTML wallpapers. Use feature flags/modules and migration guidance. Avoid heavy dependencies without benchmark justification.

## Architecture candidates to evaluate (do not lock in prematurely)

**A. Optimize existing GTK3/WebKit**: low implementation cost, maximum web compatibility; lower engineering risk but browser baseline remains.

**B. Native GTK4/GSK 2D + layer-shell adapter**: potentially simpler integration; confirm that chosen GTK4 APIs, rendering invalidation, texture caching, and compatible layer-shell library work on target desktop; benchmark actual cost.

**C. Native wgpu (or alternative small GPU 2D renderer) + Wayland layer-shell**: tighter render loop and frame control; more compositor/platform engineering and driver behavior to maintain.

**D. Other compelling proposal**: agents may recommend embedded lightweight WebView, alternative scene graph, declarative animation engine, Lua/Rhai scripting, or plugin architecture, *only after* quantifying memory/performance and maintenance tradeoffs.

Preferred direction pending proof: shared CLI/IPC/config and separate backends: `native` as lightweight default for native scene files and `web` for HTML. A backend should be lazily initialized; choosing native must not link/start browser subprocesses at runtime if avoidable. Use independent packages/features/binaries if linkage prevents this.

### Module boundaries (illustrative, not prescriptive)

- `core`: config and schema versions, animation state machine/timeline, hit regions, asset/cache policy, profiles.
- `desktop`: layer-shell, Plasma validation, X11, future GNOME extension IPC; display discovery, input routing, scaling.
- `renderer-native`: 2D textures, sprite atlas, text, transforms, particles.
- `renderer-web`: existing WebKit behavior plus wallpaper-first defaults.
- `cli`: `set`, `start`, `stop`, `toggle`, `status`, profile selection, diagnostics.
- `tests/bench`: deterministic sample scene, scripts, measurements.

## Initial scene specification

A versioned file such as `wallpaper.json` or `wallpaper.toml` referencing local assets and containing:
- Canvas sizing strategy (contain/cover/fit, focal point, safe-area), monitor and HiDPI behavior.
- Sorted layers: static image, textured sprite, text, optional particle emitter.
- Packed animation clips / frame timing, easing, pivot points, optional loop and return-to-idle.
- Named hit regions with per-region hover/click events and action/state transition.
- Idle animation and automatic restore; cooldown / random expression variants; deterministic repeatability for tests.
- Performance profile choices with correct idle semantics (truly static scene should be event-driven, not a 10 FPS busy loop).
- File path sandboxing and safe defaults; never evaluate arbitrary JS in native scene schema by default.

Example interaction acceptance: hovering head tilts/looks; clicking head smirks; clicking upper body adjusts posture; clicking legs adjusts pose; hovering throne produces a subtle ice glow. These are design targets, not existing art assets. A flattened image alone cannot provide realistic independent body motion.

## Milestones and deliverables

### M0 — Baseline, audit, acceptance tests
- [ ] Reproduce build/install on target system; fix missing desktop asset and duplicate startup mechanisms.
- [ ] Add an HTML input/hover/click smoke test and a static native reference scene specification.
- [ ] Record distro/kernel/compositor, resolution, refresh rate, scaling, GPU driver and power mode.
- [ ] Capture parent + WebKit subprocess RSS/PSS and service cgroup memory, CPU%, GPU activity where available, frame latency, idle/battery draw where possible, startup and shutdown times.
- [ ] Remove unconditional competitor killing in favor of explicit documented conflict handling, with regression tests.
- [ ] Publish results and a comparison table before renderer choice.

### M1 — Architecture decision record (ADR)
- [ ] Small POC for at least two plausible approaches: optimized WebKit and native GTK4/GSK or equivalent; add wgpu POC if warranted.
- [ ] Compare idle static, 30 FPS sprite animation, click/hover interactions and multiple monitor conditions with identical content/asset resolution.
- [ ] Select architecture based on measurements, code complexity, maintenance burden, GNOME/KDE compatibility, and reproducible tests.
- [ ] **Stop and propose alternatives** if a different approach materially beats the suggested hybrid.

### M2 — Native renderable scene prototype
- [ ] Versioned schema parser; fullscreen fixed viewport; static background, alpha, sprite atlas, text.
- [ ] Reactive animation + event-driven static mode; optional 24/30/60 FPS caps with accurate wall-clock timing.
- [ ] Native per-region hover/click states with reliable reset and bounded update work.
- [ ] Scene resizing and correct output scale; diagnostics for invalid files.

### M3 — CLI, IPC, safety and performance controls
- [ ] Preserve `vellum set <path-or-url>`, auto-select backend by extension or explicit `--engine`.
- [ ] `vellum toggle`, `status`, profile `eco|balanced|quality|static`, configuration schema; restart-safe wallpaper selection.
- [ ] Service start/stop is safe/idempotent; do not auto-disable another wallpaper without user consent.
- [ ] Monitor CPU, memory, frame pacing and leaks through 30-minute idle and interaction benchmarks.

### M4 — Desktop compatibility
- [ ] Niri, Hyprland and Sway: background stacking, exposed-desktop input, multi-monitor, workspace behavior.
- [ ] KDE Plasma Wayland: reproduce Layer Shell rendering/input and test desktop containment interference.
- [ ] GNOME Wayland research spike: prototype supported Shell-extension integration or document blocker; do not claim GNOME support until tested.
- [ ] X11 EWMH Desktop window: verify positioning, clicks, focus, multi-display.
- [ ] Compatibility matrix in README showing tested distro, compositor, backend, and known limitations.

### M5 — Real wallpaper and release
- [ ] Example interactive throne wallpaper using separately authored animated sprite layers (licenses/source and redistribution rights verified).
- [ ] Documentation for HTML mode and native scenes; minimal example; upgrade guide.
- [ ] CI build, formatting/lints, tests, release packaging for targeted distros.
- [ ] Before/after benchmark report against current Vellum, plus regressions and rollback instructions.

## Provisional performance budgets (measure first; adjust via ADR)

- Native static wallpaper: event-driven / zero periodic animation frames.
- Balanced active sprite animation: <=30 FPS; Eco <=15 FPS or event-driven where possible, Quality up to 60.
- Aim for <=100 MB steady-state native renderer/service memory on benchmark machine and <=150 MB peak under sample scene. Include relevant subprocesses and disclose GPU texture memory separately. These are goals, not guaranteed outcomes.
- Aim for CPU no worse than baseline on comparable workload. Do not optimize a synthetic percentage at the expense of battery, UI responsiveness or missed frames.
- No sustained memory increase over a 30-minute repeat interaction/idle test.
- Avoid continuous JS loops, unnecessary translucent full-screen redraws, large video/GIF decoding, per-frame allocations, and high-frequency global polling.
- Report results, methodology and environment; disqualify unmeasured optimization claims.

## Agent collaboration / decision rules

1. Begin with an **architecture review comment or ADR** listing at least 2 viable choices and what would make you change your mind. **You are explicitly invited to suggest a better idea** than the hybrid architecture.
2. Implement milestone-sized PRs, each with a test plan, measurements, compatibility impact and rollback path. Do not ship a single massive rewrite.
3. Prefer simple, maintainable Rust and small dependencies; justify any renderer/runtime addition.
4. Preserve the working HTML wallpaper experience. Do not change default behavior without migration notes.
5. Check Wayland security/input restrictions, compositor protocols and GNOME Shell extension APIs against current official docs; do not rely on assumptions.
6. Keep resource benchmarks honest (service cgroup vs process vs PSS, warmup, power mode, resolution, repeat runs).
7. Flag unknowns and blockers clearly; propose experiments rather than guessing.
8. Submit alternative technical designs as ADRs with evidence; owner may approve any design that better satisfies UX, cost, portability, security, and performance.

## Definition of done

A user can toggle Vellum quickly, choose HTML or native wallpaper, interact with an Esdeath-style sprite scene in supported compositor sessions, never see scrollbars or a resizable webpage by default, and measure demonstrably lower resource usage for native mode under comparable workloads. Existing HTML wallpapers continue to work, unsupported desktops are honestly marked, and the benchmark + compatibility matrix is reproducible.
