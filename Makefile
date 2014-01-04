RUSTC ?= rustc
RUSTFLAGS ?= -O -Z debug-info
BUILDDIR ?= build
MAKE_BUILDDIR = mkdir -p $(BUILDDIR)
RUNNER = $(BUILDDIR)/runner
RUST_LOG = runner,paxos

all: paxos runner

paxos: src/paxos/lib.rs
	$(MAKE_BUILDDIR)
	$(RUSTC) $(RUSTFLAGS) src/paxos/lib.rs --out-dir=$(BUILDDIR)

runner: src/runner/main.rs
	$(MAKE_BUILDDIR)
	$(RUSTC) $(RUSTFLAGS) src/runner/main.rs -L $(BUILDDIR) -o $(RUNNER)

clean:
	rm -rf build/
	rm -rf bin/
	rm -rf lib/

run:
	@RUST_LOG=$(RUST_LOG) $(RUNNER)

.PHONY: paxos runner clean run
