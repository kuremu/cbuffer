ifeq ($(PREFIX),)
	PREFIX := /usr/local
endif

CARGO = cargo

all:
	$(MAKE) build
	$(MAKE) doc

build:
	$(CARGO) build --release

clean:
	$(CARGO) clean

check:
	$(MAKE) build
	$(MAKE) test

test:
	$(CARGO) test

bench:
	$(CARGO) bench

doc:
	$(CARGO) doc

target/release/cbuffer:
	$(MAKE) build

install: target/release/cbuffer
	install -d $(DESTDIR)$(PREFIX)/bin/
	install -m 755 target/release/cbuffer $(DESTDIR)$(PREFIX)/bin/
	install -d $(DESTDIR)$(PREFIX)/bin/
	install -m 755 crecord $(DESTDIR)$(PREFIX)/bin/

uninstall:
	rm -fv ${DESTDIR}${PREFIX}/bin/cbuffer
	rm -fv ${DESTDIR}${PREFIX}/bin/crecord

.PHONY: all build clean check test bench doc
