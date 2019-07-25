.PHONY: build install

build:
	cargo build --release

install: build
	install -m 755 -D target/release/cachereg $(DESTDIR)/usr/sbin/cachereg
	install -m 644 -D etc/cachereg.service $(DESTDIR)/lib/systemd/system/cachereg.service
