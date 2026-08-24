SHELL := /bin/sh
OUT ?= dist

.PHONY: require-target preflight prepare build stage verify

require-target:
	@test '$(origin TARGET)' = 'command line' && test -n '$(TARGET)' || { echo 'TARGET must be an explicit Make command-line variable' >&2; exit 2; }

preflight: require-target
	@scripts/check-build-environment.sh '$(TARGET)'

prepare: preflight
	@cargo fetch --locked --target '$(TARGET)'

build: prepare
	@cargo build --locked --release --target '$(TARGET)' --bin soksak-sidecar-terminal-wezterm

stage: build
	@scripts/stage-built.sh '$(OUT)' '$(TARGET)'

verify: build
	@node scripts/check-release-workflow.mjs
	@scripts/gate.sh '$(TARGET)'
