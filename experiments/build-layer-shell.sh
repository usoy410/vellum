#!/bin/sh
set -eu
# Run from repository root; all generated code/binaries stay outside the checkout.
output=${1:?usage: sh experiments/build-layer-shell.sh /path/to/output}
mkdir -p "$output"
cc -O2 -Wall -Wextra -Werror experiments/renderer-layer-shell.c -o "$output/layer" $(pkg-config --cflags --libs gtk+-3.0 gtk-layer-shell-0) -lm
cc -O2 -Wall -Wextra -Werror experiments/renderer.c -o "$output/reference" $(pkg-config --cflags --libs gtk+-3.0) -lm
wayland-scanner client-header tests/wayland/wlr-virtual-pointer-unstable-v1.xml "$output/virtual-pointer.h"
wayland-scanner private-code tests/wayland/wlr-virtual-pointer-unstable-v1.xml "$output/virtual-pointer.c"
cc -Wall -Wextra -Werror -I"$output" tests/wayland/pointer-seat.c "$output/virtual-pointer.c" -o "$output/pointer-seat" $(pkg-config --cflags --libs wayland-client)
