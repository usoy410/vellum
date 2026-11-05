#!/bin/sh
set -eu
# Run from the repository root. Output directory must be supplied explicitly.
output=${1:?usage: sh experiments/build-native.sh /path/to/output}
mkdir -p "$output"
cc -O2 -Wall -Wextra -Werror experiments/renderer.c -o "$output/gtk3" $(pkg-config --cflags --libs gtk+-3.0) -lm
cc -O2 -Wall -Wextra -Werror experiments/renderer-gtk4.c -o "$output/gtk4" $(pkg-config --cflags --libs gtk4) -lm
