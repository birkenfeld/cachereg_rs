.PHONY: build install release-patch release-minor release

build:
	# workaround for buster
	CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release

install: build
	install -m 755 -D target/release/cachereg $(DESTDIR)/usr/sbin/cachereg
	install -m 644 -D etc/cachereg.service $(DESTDIR)/lib/systemd/system/cachereg.service

release-patch:
	MODE="patch" $(MAKE) release

release-minor:
	MODE="minor" $(MAKE) release

release:
	ssh jenkins.admin.frm2 -p 29417 build -v -s -p GERRIT_PROJECT=$(shell git config --get remote.origin.url | rev | cut -d '/' -f -3 | rev) -p ARCH=any -p MODE=$(MODE) ReleasePipeline
