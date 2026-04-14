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
./setup.sh
```

## 📦 Distribution

Vellum is designed to be easily distributed. We provide:
- **Makefile** for standard system-wide installations.
- **PKGBUILD** for Arch Linux (AUR).
- **GitHub Actions** for automated building of DEB, RPM, AppImage, and tar.gz formats.

## 🎮 Usage

### Manage the Daemon
Vellum runs as a background service. Manage it with `systemctl`:
```bash
systemctl --user start vellum.service    # Start
systemctl --user stop vellum.service     # Stop
systemctl --user status vellum.service   # Check status
```

### Change Wallpaper
Update your wallpaper instantly using the `set` command:
```bash
# Local file
vellum set ~/wallpapers/beach.html

# Remote URL
vellum set https://google.com
```

Vellum is released under the [MIT License](LICENSE).

---
Created with ❤️ by **Arjay Alforte**
