# HTML desktop baseline (M0/M1)

Status: harness validated on Linux; real HTML rendering/input baseline pending.
This change does not select a renderer or alter production HTML behavior.

## Prepare a controlled run

Use a desktop session with Vellum's native dependencies installed. Build the
revision being tested; record its SHA, build profile and toolkit versions. Stop
only your own previous Vellum instance using its normal service controls. Do not
kill other wallpaper programs automatically. Test exposed desktop input before
measuring; a running PID alone does not establish rendering success.

For the existing systemd installation:

```sh
systemctl --user start vellum.service
vellum set "$(pwd)/experiments/scene.html"
systemctl --user show vellum.service -p MainPID -p ControlGroup
```

Run from the repository root. Use the same executable for daemon and client.
If the service restarts, discard the run and get its new PID. The collector never
starts, stops, changes, or sends input to an application.

Record metadata in a file outside the checkout, for example `/tmp/vellum-host.json`:

```json
{
  "compositor": "REPLACE: name and version; Wayland or X11",
  "outputs": "REPLACE: count, resolutions, refresh rates and per-output scales",
  "gpu_driver": "REPLACE: GPU model and driver/version",
  "power_mode": "REPLACE: AC/battery, power profile, brightness",
  "workload": "scene.html static; 1920x1080 asset canvas; note actual viewport",
  "binary_revision": "REPLACE: git SHA, debug/release, GTK and WebKit versions"
}
```

Then substitute the numeric MainPID below:

```sh
python3 tools/desktop_baseline.py --pid 12345 \
  --metadata /tmp/vellum-host.json --output /tmp/vellum-static-01.json
```

Defaults: 10-second warmup, 60-second sample period, 1-second interval. Run five
independent samples, naming each output uniquely; existing output is never
overwritten. Repeat in randomized candidate order when native backends are
ready. Keep resolution, scale, assets, power mode and unrelated workload fixed.
For the animated HTML fixture, make a local copy of scene.html replacing
`if(location.hash==='#animate')` with `if(true)`, and load that copy with `set`.
Record the fixture hash and modification. The existing CLI treats file URIs
as local paths, so appending a URI fragment to `set` is not supported. Do not silently change production
URL parsing in this measurement PR.

## Manual acceptance checklist — record each result separately

Use `experiments/scene.html`, which has fixed viewport CSS, a preloaded square
and no periodic work in static mode. These are fixture policies, not new defaults
in Vellum. For each target compositor record pass/fail/not-run plus observations:

1. Initial load: background and square appear without browser chrome; no scroll
   or viewport movement on wheel/touchpad input. Record screenshot dimensions.
2. Exposed desktop: hover increments the hover counter, outlines the square;
   clicking increments the click counter and changes its color.
3. Cover the square with another app: interacting with that app must not change
   wallpaper counters or steal keyboard focus. No global pointer capture.
4. `vellum set` to a second local HTML file, then back: verify visible content
   actually changes, scripts still run, and the process does not restart.
5. Test an owner-controlled HTTPS wallpaper separately; record loading and input.
   Offline/local success does not validate remote loading or its network policy.
6. SIGTERM/service stop: record time until all Vellum/WebKit processes exit and
   the owned socket disappears. Restart and repeat `set`/input. Do not use SIGKILL
   for normal shutdown measurements.
7. Repeat on two monitors, different scale factors and after hotplug/workspace
   changes. Record per-output behavior; current code does not establish support.

Capture startup-to-visible-content with a consistent external recording method.
CPU timestamps or timer callbacks do not prove first presentation/frame latency.
Record unmeasured latency, GPU activity and power as unavailable, never zero.

Niri/Hyprland/Sway each need results; Plasma needs separate desktop-containment
validation. GNOME Wayland remains a separate integration investigation. Xvfb
smoke results do not establish support for any of these desktops.

## Reading the collector output

- Parent and currently observable descendants are sampled, including ordinary
  WebKit child processes. Each process is identified by PID plus start ticks.
- RSS sums double-count shared pages; summed PSS apportions them. Neither
  includes all GPU allocations. A denied/missing memory reading makes the
  aggregate null. Process scan failures are reported, not silently counted as 0.
- CPU is a percentage of **one core**, from user+system ticks. Intervals with
  membership changes or scan errors have null CPU; PID reuse invalidates deltas.
  Processes born and exited entirely between samples, or reparented away from
  the daemon, may escape observation. Sampling is non-atomic; these measurements
  are estimates, not a complete service accounting ledger.
- `sampling_completed` means collection finished, not that rendering passed.
  Acceptance stays `not assessed` until the separate checklist is completed.
- Root exit or interruption returns failure and retains an incomplete JSON
  report. Do not include incomplete runs in performance medians.

For service memory/CPU, additionally record the unit's dedicated cgroup
`memory.current`, `memory.peak` and `cpu.stat` over the same interval. Verify the
cgroup contains only this service; never attribute a shared session cgroup to
Vellum. Report cgroup and process metrics separately. Peak may include earlier
work since cgroup creation; record that lifetime rather than calling it run peak.
GPU/power/presentation metrics require host tools and are not collected here.

## Current cloud evidence and next gate

The merged IPC code builds and its 8 tests pass. On this Debian 13 cloud host,
WebKitGTK 2.54.0 is extracted under a workspace prefix, but its release helper
path is `/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/WebKitNetworkProcess`. That path
and `/run/user/1000` remain absent; the non-root user cannot populate their
parents. No real-desktop input or WebKit resource comparison is claimed.

The sampler's tests cover process-tree discovery on a live parent/child pair,
PID identity, missing memory, stat parsing and metric arithmetic. A short native
Xvfb run can exercise the collector but does not replace the HTML baseline.

Next gate: complete the checklist and five static/animated runs on a supported
real desktop, then compare GTK3/Cairo and GTK4/GSK under matching conditions.
Rollback: remove this documentation, collector and its tests; production runtime
and HTML protocol are unchanged.
