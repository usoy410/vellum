PREFIX ?= /usr
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
SYSTEMDUSERUNITDIR ?= $(PREFIX)/lib/systemd/user

all: build

build:
	cargo build --release

install:
	install -Dm755 target/release/vellum $(DESTDIR)$(BINDIR)/vellum
	install -Dm644 assets/vellum.service $(DESTDIR)$(SYSTEMDUSERUNITDIR)/vellum.service

user-install: build
	install -Dm755 target/release/vellum $(HOME)/.local/bin/vellum
	install -Dm644 assets/vellum.service $(HOME)/.config/systemd/user/vellum.service
	sed -i 's|/usr/bin/vellum|$(HOME)/.local/bin/vellum|g' $(HOME)/.config/systemd/user/vellum.service
	mkdir -p $(HOME)/.config/autostart
	install -Dm644 assets/vellum.desktop $(HOME)/.config/autostart/vellum.desktop
	sed -i 's|/usr/bin/vellum|$(HOME)/.local/bin/vellum|g' $(HOME)/.config/autostart/vellum.desktop
	systemctl --user daemon-reload
	systemctl --user enable vellum.service
	systemctl --user restart vellum.service
	@echo "✅ Vellum installed and started for user $(shell whoami)"

user-uninstall:
	systemctl --user stop vellum.service || true
	systemctl --user disable vellum.service || true
	rm -f $(HOME)/.local/bin/vellum
	rm -f $(HOME)/.config/systemd/user/vellum.service
	rm -f $(HOME)/.config/autostart/vellum.desktop
	systemctl --user daemon-reload
	@echo "✅ Vellum uninstalled for user $(shell whoami)"

clean:
	cargo clean

.PHONY: all build install user-install user-uninstall clean
