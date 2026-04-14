PREFIX ?= /usr
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
SYSTEMDUSERUNITDIR ?= $(PREFIX)/lib/systemd/user

all: build

build:
	cargo build --release

install:
	install -Dm755 target/release/vellum $(DESTDIR)$(BINDIR)/vellum
	install -Dm644 vellum.service $(DESTDIR)$(SYSTEMDUSERUNITDIR)/vellum.service
	mkdir -p $(DESTDIR)/etc/xdg/autostart
	install -Dm644 vellum.desktop $(DESTDIR)/etc/xdg/autostart/vellum.desktop

clean:
	cargo clean

.PHONY: all build install clean
