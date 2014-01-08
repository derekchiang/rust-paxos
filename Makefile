RUSTC ?= rustc
RUSTFLAGS ?= -O -Z debug-info
BUILDDIR ?= build
MAKE_BUILDDIR = mkdir -p $(BUILDDIR)
RUNNER = $(BUILDDIR)/runner
RUST_LOG = runner,paxos

all: paxos runner

deps:
	# In the future, Rust source code can directly depend on remote repositories,
	# thus freeing us from managing dependencies manually.
	rm -rf deps
	mkdir deps
	git clone git@github.com:derekchiang/rust-object-stream.git deps/rust-object-stream
	rustc deps/rust-object-stream/src/object_stream/lib.rs --out-dir=$(BUILDDIR)

paxos: src/paxos/lib.rs
	$(MAKE_BUILDDIR)
	RUST_LOG=rustc=1 $(RUSTC) $(RUSTFLAGS) src/paxos/lib.rs -L $(BUILDDIR) --out-dir=$(BUILDDIR)

runner: src/runner/main.rs
	$(MAKE_BUILDDIR)
	$(RUSTC) $(RUSTFLAGS) src/runner/main.rs -L $(BUILDDIR) -o $(RUNNER)

clean:
	rm -rf build/
	rm -rf bin/
	rm -rf lib/

run:
	@RUST_LOG=$(RUST_LOG) $(RUNNER)

.PHONY: deps paxos runner clean run
