# Native renderer feasibility comparison — 2026-11-05

Scope: M1 experiment, not renderer selection or desktop compatibility certification.
The owner prioritizes Niri/Hyprland/similar Wayland compositors, with Plasma later
and GNOME as a future integration investigation.

## What ran

Debian 13 x86_64, Linux 6.18.57; GTK3 3.24.49, GTK4 4.18.6; `cc -O2`.
Isolated Xvfb at 1920x1080, scale 1, no window manager or `/dev/dri`. GTK3 uses
Cairo; GTK4 explicitly uses `GskCairoRenderer`. No hardware GPU, real refresh,
presentation latency, battery draw or compositor behavior was measured.

Both prototypes display a solid background and a preloaded 128x128 alpha texture
at (256,256); animated mode moves it sinusoidally with wall-clock time. Static
mode has no animation timer. Both animation timers use 34 ms, below 30 updates/s.
A timer callback is not proof of a presented frame.

Before measurement, separate Xvfb smoke runs verified 1920x1080 screenshots,
visible sprites and gold click outlines plus `click delivered` logs. Background
pixels matched (20,36,56); click outlines matched (255,179,51); the sprite differed
by one red-channel unit due to alpha rounding (GTK3 44 vs GTK4 45). These are
functionally comparable procedural scenes, not pixel-identical art assets.
X11 synthetic clicks were sent only to the isolated server, never the user's
session. Hover/focus and exposed-desktop behavior on Wayland remain untested.

## Exploratory measurements

Three runs per candidate/workload in seeded shuffled order; two-second warmup,
five-second sampling, one-second interval. All 12 runs completed. These shorter
parameters test feasibility; the runner defaults to five runs with 10/60 seconds
for subsequent comparisons. CPU percentages below are means within each run,
expressed as percentage of one core. Memory is median process-tree PSS per run.

| Candidate | Workload | CPU means across runs | PSS medians across runs (MiB) |
|---|---|---|---|
| GTK3/Cairo | static | 0, 0, 0 | 29.108, 9.514, 9.514 |
| GTK3/Cairo | animated | 0.999, 1.198, 1.000 | 9.540, 9.544, 9.544 |
| GTK4/GSK Cairo | static | 0, 0, 0 | 46.282, 46.421, 46.462 |
| GTK4/GSK Cairo | animated | 1.198, 1.997, 1.800 | 47.997, 46.689, 46.350 |

[Raw samples, metadata, binary hashes, order and logs](results/native-xvfb-2026-11-05.json)
are retained. Measurements exclude Xvfb and desktop services. RSS and PSS have
different meanings; do not compare this PSS table to the user's service-cgroup
memory observation. Zero CPU means no tick delta observed, not zero power.
The first GTK3 memory sample differs substantially from later runs; its cause
was not isolated. Preserve it, and repeat longer runs before drawing conclusions.
Sampling may miss transient/reparented descendants; see the baseline procedure.

## Recommendation and what would change it

Keep GTK3/Cairo as the low-complexity native candidate for the first layer-shell
prototype. This simple software-rendering workload does not show a reason to
force a GTK4 migration: GTK3 remains viable and shares the existing desktop
integration stack. This is **not** evidence that GTK3 beats GPU-backed GSK on
real hardware. GTK4 has been shown to be implementable, not disqualified.

Continue with separate native/web worker binaries so native operation can avoid
browser linkage. Neither native experiment links WebKit. Preserve the production
HTML worker; no production renderer or dependencies change in this PR.

Next evidence gate: compare these native candidates with hardware-backed GSK on
Niri and Hyprland, using real output scales, 1080p/4K and matching layered assets.
Test layer-shell background placement, no keyboard-focus acquisition, delivered
pointer regions, workspace transitions and output hotplug. Select GSK if its
frame pacing/power/scaling improves enough to justify the extra toolkit/adapter;
consider wgpu only if toolkit limits persist. Do not build a second desktop
integration stack merely on a presumed GPU speedup.

Plasma follows its own desktop containment/input tests. GNOME needs a separate
supported integration path; the layer-shell library documents no GNOME Wayland
support. No compositor support is claimed from these Xvfb results.

Validation: both native spikes and the existing WebKit spike compile with
`-Wall -Wextra -Werror`; 8 collector tests and 2 comparison orchestration tests
pass; native screenshots/click logs inspected. Production HTML rendering remains
unverified in this cloud host due to WebKit system helper paths.
Rollback: revert this experiment PR; the production binary is untouched.
