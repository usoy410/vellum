# Isolated compositor test helper

`pointer-seat.c` only creates a virtual pointer device so the private headless
Sway seat advertises pointer capability. Test motion/buttons are then sent via
that private Sway's IPC. This is not part of either renderer or production Vellum;
no global input interception or virtual input dependency is added to wallpaper
runtime. Do not run this helper against your normal desktop.

Protocol fixture: upstream wlr-protocols
https://github.com/swaywm/wlr-protocols/blob/master/unstable/wlr-virtual-pointer-unstable-v1.xml
retrieved 2026-11-05. Its copyright and MIT-style license are retained in the XML.
The fixture is committed so test builds need no downloads or moving upstream
branch. wayland-scanner generates headers/code into the requested build directory.
