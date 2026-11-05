# M1 exploratory programs

Build `renderer.c` with GTK3 pkg-config flags and `-lm` for native; define WEB
and use webkit2gtk-4.1 flags for web. Outputs belong outside the checkout.
Native accepts `--animate`; web accepts an absolute URI to scene.html, with
`#animate` for animation. These are ordinary windows, not desktop integrations.
The native texture is generated once; static mode installs no animation timer.
Neither spike is a full scene parser or atlas engine.

cloud-smoke.py records three short parent-only runs on isolated 1920x1080 Xvfb
using the workspace toolchain paths. Warmup 1 second, sample 3 seconds. It is
specific to the onboarding environment and is not the ADR benchmark harness.
It does not measure child processes, cgroups, GPU, presentation latency or power.
Do not use its numbers to rank production backends. Native/web interaction
visuals differ; normalize those before interaction benchmarks. Full comparative
runs, actual delivered input, multi-monitor, GTK4 and Wayland remain pending.

Manual HTML acceptance: load scene.html through Vellum, hover and click the
square and verify counters; switch away and back using `vellum set`; verify
other windows retain input/focus. Repeat on each target compositor. Not yet run.
Reference static scene: version 1, 1920x1080 canvas, contain scaling, background
#142438, one preloaded 128x128 alpha texture at (256,256), no periodic frames,
bounded square hit region; active variant moves x by 128*sin(elapsed_seconds)
at at most 30 Hz. This is a specification, not an accepted production schema.

For real desktop acceptance and process-tree sampling, use
[the HTML baseline procedure](../docs/HTML_BASELINE.md) and
`tools/desktop_baseline.py`. The older cloud-smoke.py remains a historical,
parent-only feasibility probe; do not use it for service-level comparisons.

## GTK4/GSK comparison

`renderer-gtk4.c` is a GtkWidget snapshot implementation: a background color node,
a cached GdkMemoryTexture, and bounded pointer/click state. It schedules no timer
in static mode. Both native spikes now use a 34 ms animation interval (at most
29.42 timer callbacks/second, not a claim about presentation rate). The former
33 ms interval could exceed the 30 FPS cap. These remain ordinary windows.
GTK4 is an optional experiment dependency, not a production dependency.

Build from the repo root with GTK3 and GTK4 development packages installed:

```sh
sh experiments/build-native.sh /tmp/vellum-native-bin
```

Run on a dedicated test display; keep the pointer outside the sprite during
resource measurements. For a software-renderer control under Xvfb, explicitly
set `GDK_BACKEND=x11 GSK_RENDERER=cairo GTK_A11Y=none`. Do not reuse those settings
as GPU/Wayland benchmark settings. Use a metadata file as described in
[HTML baseline](../docs/HTML_BASELINE.md):

```sh
python3 experiments/compare_native.py \
  --gtk3 /tmp/vellum-native-bin/gtk3 --gtk4 /tmp/vellum-native-bin/gtk4 \
  --metadata /tmp/vellum-host.json --output /tmp/vellum-native-runs
```

The runner executes five runs per backend/workload in seeded shuffled order;
10 s warmup, 60 s measurement by default. It records binary hashes, order,
renderer settings, per-process samples and logs. Existing output directories
are rejected; failed launches/runs remain incomplete. It stops only the child
it launched. Run `python3 -m unittest discover -s tests -p test_native_comparison.py`
for orchestration tests (dummy processes; not rendering tests).

Validate the rendered image and click feedback separately before benchmarking.
On an isolated Xvfb server, `import -window root screenshot.png` captures the
image and `xdotool mousemove 300 300 click 1` tests the sprite; inspect the gold
outline and `click delivered` log. Never run synthetic input against the user's
normal desktop. Wayland acceptance must use compositor-delivered real input.
GTK4 prints the selected renderer type, so a Cairo run cannot be mistaken for
GPU-accelerated GSK. No browser code is included in either native spike.
