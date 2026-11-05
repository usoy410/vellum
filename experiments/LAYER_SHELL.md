# Native layer-shell integration spike

This is an opt-in M1 experiment for the Niri/Hyprland-first direction. It is not
wired into `vellum set`, the service, or the production HTML binary. It does not
parse native scene files yet and does not establish Niri/Hyprland compatibility.

## Behavior

- Native GTK3/Cairo, no WebKit initialization or linkage.
- One background layer surface per GdkMonitor, all edges anchored, exclusive
  zone -1 (do not reserve application work area), keyboard mode NONE.
- Independent preloaded texture, hover/click state and optional animation per
  output. A 1920x1080 reference canvas is uniformly contained/centered in logical
  output coordinates. GTK handles buffer scale; hit testing inverts the same
  transform. Real fractional scaling still needs compositor-specific tests.
- Static scenes have no animation timer; pointer motion redraws only on a state
  change. Animated scenes use a 34 ms timer and elapsed wall-clock motion.
- Output removal destroys its surface and timer; output addition creates a new
  scene. SIGTERM/SIGINT remove all surfaces/timers and return success.
- Unsupported/no Wayland display fails explicitly instead of opening an ordinary
  fallback window. No competitor termination, keyboard grabbing or global input
  interception. The renderer only handles pointer events the compositor delivers.

## Build and try

GTK3, gtk-layer-shell and Wayland development packages, C compiler, pkg-config
and wayland-scanner are needed for the complete test build. Run from repo root:

```sh
sh experiments/build-layer-shell.sh /tmp/vellum-layer-build
/tmp/vellum-layer-build/layer
# Optional animation:
/tmp/vellum-layer-build/layer --animate
```

The experiment uses the current Wayland display and draws on all its outputs;
try it in a dedicated test session first. It does not stop an existing wallpaper
application. If testing in your desktop, manage overlapping wallpapers explicitly
and stop this prototype with SIGTERM or Ctrl-C. No installation is required.

## Reproducible headless tests

Additional test dependencies: Sway (with headless wlroots/pixman), swaymsg, grim,
Python 3 and Pillow. The tests create their own 0700 runtime directory, compositor,
Wayland socket and private IPC connection; inherited display/socket selectors
are removed. The virtual pointer helper is **test-only** and is never used by
wallpaper runtime. No synthetic input is sent to the user's desktop.

```sh
VELLUM_LAYER_BINARY=/tmp/vellum-layer-build/layer \
VELLUM_REFERENCE_BINARY=/tmp/vellum-layer-build/reference \
VELLUM_POINTER_SEAT=/tmp/vellum-layer-build/pointer-seat \
VELLUM_TEST_ARTIFACTS=/tmp/vellum-wayland-run-01 \
python3 -m unittest discover -s tests -p test_layer_shell.py -v
```

Use a fresh artifact directory. Without the three binary variables, integration
tests are skipped; a skipped run does not validate anything. The test suite
stops only its own processes. Logs and screenshots are retained when requested.
It waits for screenshot pixels to verify presented content: configure and click
callbacks can arrive before the matching frame, so callbacks alone are not used
as visual acceptance.

## Results on 2026-11-05

Debian 13 x86_64; Linux 6.18.57; Sway 1.10.1 / wlroots 0.18.2, headless/pixman;
GTK3 3.24.49 and gtk-layer-shell 0.9.0. Two virtual outputs: 1920x1080 and 1280x720.
The final three-test suite passed three consecutive runs with the compiled
binaries (none skipped), after fixing test timing around first presentation and
pointer handoff to covering applications:

1. Static protocol/rendering/input/output/focus scenario: background layer=0,
   anchor=15, keyboard interactivity=0 and exclusive zone=-1 recorded in client
   protocol logs. Screenshots verify background/texture and gold click feedback.
   A covering application receives its own click without changing wallpaper
   state. Clicking exposed wallpaper preserves the application's seat focus.
   Scale-2 configuration and hit testing pass; disabling and re-enabling the
   second output removes/recreates its scene and renders correctly. Static idle
   produces no additional damage in the short observation. Process mappings
   contain no WebKit; SIGTERM returns zero and removes all scenes.
2. Animation continues on the remaining output after removal of the other output;
   graceful shutdown returns zero without GLib critical messages.
3. Missing Wayland display fails with exit 2 and no fallback window.

[Machine-readable result summary](results/layer-shell-headless-sway-2026-11-05.json).
This is functional integration evidence, not a GPU/power benchmark or a claim
about physical monitor hotplug. No hardware GPU, Niri, Hyprland, Plasma or GNOME
session was tested. Long-duration leakage, multiple renderer instances, real
fractional scaling, suspend/resume and compositor restart still need validation.

## Next gate and rollback

Run these same interaction/output/focus scenarios on Niri and Hyprland. Keep the
existing HTML path operational while a separate Rust native worker and versioned
scene parser are proposed in small PRs. KDE gets a later compatibility pass;
GNOME remains separate integration research. This experiment does not choose a
permanent native renderer or create a public scene format.

Rollback: revert this experiment and its tests. Production code, CLI, service,
HTML/CSS/JS behavior and dependencies are unchanged.
