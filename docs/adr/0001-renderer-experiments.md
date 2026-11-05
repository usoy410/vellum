# ADR 0001: measure before selecting a native renderer

Status: proposed; M0/M1 incomplete. Related: https://github.com/usoy410/vellum/issues/1.

Preserve HTML/URL loading, `vellum set`, and compositor-delivered HTML input.
No renderer replacement is authorized by this ADR. Keep experimental programs
outside the production binary. Native must run without WebKit loaded.

## Candidates and decision gates

| Candidate | Benefit to test | Cost / reason to reject |
|---|---|---|
| Existing WebKit with fixed viewport and event-driven static content | Full HTML compatibility; smallest migration | Browser baseline; cannot alone meet native requirement |
| GTK3/Cairo native worker, reuse current layer-shell adapter | Existing desktop dependencies; no new GPU abstraction | CPU rasterization may lose badly at 4K, alpha and animation |
| GTK4/GSK native worker | Retained GPU scene graph, texture caching | Separate GTK4 layer-shell integration; driver behavior |
| Direct Wayland + wgpu | Explicit frame scheduling and texture ownership | Text, outputs, input, X11 and GPU lifecycle maintenance |

Provisional recommendation: independently launched web/native workers with a
small shared CLI/control protocol, not both toolkits in one process. Existing
ELF dependency inspection shows the current binary dynamically depends on
WebKit even for `set`; merely branching before WebView construction does not
isolate native memory/deployment dependencies. Separate binaries offer a
verifiable dependency boundary. This is structural evidence, not a measured
memory win. Start by testing GTK3/Cairo as the low-complexity alternative; select
GSK if its measured animation/4K cost justifies the migration. Consider wgpu only
if toolkit frame pacing or power remains unacceptable. No plugin ABI or scripting
runtime until a demonstrated content requirement needs it.

## Experiment contract

Use identical 1920x1080 background and 128x128 alpha sprite; test static,
30 Hz wall-clock animation, delivered hover/click, then two outputs and 2x scale.
Warm for 10 s, sample for 60 s, repeat 5 times with randomized candidate order.
Repeat at 4K and on battery. Record distro/kernel, compositor/version, GPU/driver,
power profile, output refresh/scale, toolkit versions and commit. Verify visible
content and input before recording a successful renderer result.

Measure process-tree RSS (shared pages double counted), summed PSS, dedicated
service cgroup memory.current/peak, CPU seconds / wall seconds (one-core basis),
startup-to-first-frame and shutdown time. Record frame intervals separately from
presentation latency; timers are not presented frames. GPU/power require host
counters; unavailable is not zero. Do a 30-minute leak run only after the short
comparison passes. A shared session cgroup is not attributable to Vellum.

Static must schedule no periodic redraws. Preload assets and avoid per-frame
file decoding. Report library mappings and child processes to prove native
isolation. Host compositor tests remain necessary; Xvfb is only a smoke test.

## Audit at 58a19e1

- Unconditional competitor pkill runs before GTK initialization, even if startup fails.
- Missing desktop asset and two startup mechanisms in Makefile; retain systemd
  as the single supported installer path, with explicit manual migration guidance.
- DEB dependency names WebKit 4.0, but current crate/binary uses 4.1.
- IPC unlinks an existing socket without testing daemon ownership; second startup
  can strand the first daemon. Unbounded read_to_string can block all later clients.
  Address in a separate IPC PR with adversarial/restart tests.
- `/run/user/<uid>` ignores XDG_RUNTIME_DIR; no graceful socket cleanup.
- One window, no explicit output selection/hotplug; multi-output unsupported until tested.
- No fixed HTML viewport policy; introducing one requires opt-out and HTML tests.
- No existing tests; tracked target artifacts make ordinary builds dirty. Set
  CARGO_TARGET_DIR outside the checkout during development.
- README's blanket Wayland/X11 coverage and noninteractive wording exceed evidence.

## Compatibility and rollback

No new production renderer or CSS policy in this research change. Experimental
windows exercise only delivered pointer events, never global input. Exposed
background behavior, focus/workspace and layer-shell require Niri/Hyprland/Sway
and separate Plasma tests. GNOME integration remains a separate research task.
Revert this research commit to remove experiments without affecting HTML support.
