# 🎞️ Vellum

Vellum is a lightweight, high-performance HTML wallpaper runner for Linux. It allows you to use any web content (HTML, CSS, JS, or remote URLs) as a persistent, borderless, and non-interactive desktop background.

Vellum is built with Rust and WebKit2GTK, supporting both **Wayland** (via `gtk-layer-shell`) and **X11** (via EWMH hints).

![Vellum Demo](https://via.placeholder.com/800x450?text=Vellum+Wallpaper+Runner)

## ✨ Features

- **🌐 Web-Based**: Render anything from simple HTML files to complex web applications.
- **🚀 Dynamic Control**: Change your wallpaper on the fly without restarting the daemon thanks to our built-in IPC.
- **🖼️ Cross-Platform**: Native support for Wayland (Hyprland, Sway) and X11 (KDE, GNOME, etc.).
- **👻 Transparent & Interactive-Free**: Clean, non-interactive background that stays below your windows.
- **🤖 Automated Releases**: Ready-to-go packages for different distributions.

## 📥 Installation

You can download pre-built packages for your distribution from our [Releases Page](https://github.com/usoy/vellum/releases).

### Debian/Ubuntu (DEB)
```bash
sudo apt install ./vellum-*.deb
```

### Fedora/RHEL (RPM)
```bash
sudo dnf install ./vellum-*.rpm
```

### AppImage
```bash
chmod +x vellum-*.AppImage
./vellum-*.AppImage
```

### tar.gz (Universal)
```bash
tar -xzvf vellum-*.tar.gz
cd vellum-*
make user-install
```

## 📦 Distribution

Vellum is designed to be easily distributed. We provide:
- **Makefile** for standard system-wide installations.
- **PKGBUILD** for Arch Linux (AUR).
- **GitHub Actions** for automated building of DEB, RPM, AppImage, and tar.gz formats.

## 🎮 Usage

### Manage the Daemon
Vellum runs as a background service. Manage it with `systemctl` or directly via `vellum`:
```bash
systemctl --user start vellum.service    # Start
systemctl --user stop vellum.service     # Stop
systemctl --user status vellum.service   # Check status
vellum restart                           # Convenient daemon restart
```

### Change Wallpaper
Update your wallpaper instantly using either direct path syntax or the `set` command:
```bash
# Direct syntax (auto-sends to running daemon, or starts daemon if not running)
vellum ~/htmlwall/index.html

# Local file via set
vellum set ~/wallpapers/beach.html

# Remote URL
vellum set https://google.com
```

### Refresh & Clear Cache
When editing HTML, CSS, or JavaScript for your wallpaper, refresh immediately without restarting or retyping long paths:
```bash
# Reload current wallpaper and bypass WebKit cache
vellum refresh
vellum -r

# Explicitly purge cache and reload
vellum refresh --cache
vellum refresh -c

# Update wallpaper path and refresh in one command
vellum refresh ~/htmlwall/index.html
```

### Autostart on Reboot / Relogin & Persistence
Vellum automatically remembers your last set wallpaper across reboots and relogins. When the background service starts up, it restores your active wallpaper automatically so you never have to re-apply it manually.

You can configure automatic startup using either `systemctl` or the built-in `autostart` command:
```bash
# Enable automatic startup on reboot/relogin
vellum autostart enable

# Check current status and active wallpaper
vellum status
vellum autostart status

# Disable automatic startup
vellum autostart disable
```

### Help & Options
```bash
vellum --help     # Display CLI help anytime (even if another instance is running)
vellum --version  # Display version information
vellum status     # Display daemon status, autostart, and active wallpaper
```

Vellum is released under the [MIT License](LICENSE).

---
Created with ❤️ by **Arjay Alforte**

### Startup and coexistence

Vellum leaves other wallpaper applications running. If backgrounds conflict,
stop the other application explicitly using its own controls before starting
Vellum. HTML/URL loading and `vellum set` are unchanged.

The Makefile uses only the systemd user service for automatic startup. It no
longer installs an XDG autostart entry. If an earlier installation created
`~/.config/autostart/vellum.desktop` or `/etc/xdg/autostart/vellum.desktop`, inspect
and remove that Vellum entry yourself to avoid duplicate startup. Installation
and uninstallation do not delete existing desktop entries.

### IPC lifecycle

The legacy `vellum set <path-or-url>` protocol remains one UTF-8 URL per Unix
connection, completed by EOF. Requests are limited to 64 KiB and must finish
within two seconds; empty, invalid UTF-8 and NUL-containing requests are ignored.
The client has a two-second write timeout. As before, a successful client write
is not an acknowledgement that WebKit loaded the page.

The endpoint remains `/run/user/<uid>/vellum.sock`. A persistent adjacent
`vellum.sock.lock` serializes daemon startup; do not delete that lock file while
Vellum is running. An active socket (including an older daemon's socket) is
preserved. Only a refused, owned Unix socket is recovered as stale. Files and
symlinks at the endpoint are never replaced. Normal exit, SIGINT and SIGTERM
remove the socket only if its inode still matches this daemon's socket. SIGKILL
cannot run cleanup; the next startup recovers the stale socket.

Requests are processed serially, so one stalled client can delay later clients
by up to two seconds. This is local reliability protection, not a guarantee
against a malicious same-user flood. Runtime directory migration, acknowledged
commands and queue/backpressure changes are separate follow-up work.
