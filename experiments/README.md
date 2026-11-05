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
