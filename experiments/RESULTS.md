# Cloud feasibility results — 2026-11-05

Debian 13 x86_64, Linux 6.18.57; GTK3 3.24.49, WebKitGTK 2.54.0;
isolated Xvfb 1920x1080x24. No actual desktop compositor, GPU/power profile,
output refresh or scale validation. Raw runs: cloud-smoke-results.json.

| Candidate | Outcome | Parent PSS | Parent one-core CPU |
|---|---|---|---|
| GTK3/Cairo static | Process remained alive for all 3 samples | 9.5–29.1 MiB | 0 ticks observed |
| GTK3/Cairo 33 ms timer | Process remained alive for all 3 samples | 9.56 MiB | 1.33–10.99% |
| WebKit static | All 3 starts failed: missing system WebKitNetworkProcess | unavailable | unavailable |

These are feasibility observations, not acceptance benchmarks. No screenshot or
input verification was performed. Samples are only 3 seconds after 1 second
warmup, with parent-only accounting and substantial run variance. A zero tick
delta is not proof of zero energy. Timer frequency is not presentation rate.
The comparison cannot establish a speed/memory winner; native full acceptance,
WebKit baseline, GTK4, multiple outputs and actual compositor measurements remain
outstanding. Do not compare these numbers to the user's service-cgroup anecdote.

Both C spikes compile with -O2 -Wall -Wextra -Werror using pkg-config. The native
source contains no WebKit dependency and no timer in static mode. This supports
continuing the low-complexity native-worker experiment, not selecting Cairo for
production. Keep HTML as the production path until matching workload, input and
host benchmark gates in ADR 0001 pass.

Suggested PR description: M0/M1 research scaffolding and provisional ADR; related
to #1, does not close either milestone. Validation: both spikes compile, six
native lifecycle samples complete, three WebKit starts fail as recorded.
Compatibility: no production renderer changes. Rollback: revert research commits.
