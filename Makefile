SHELL := /bin/sh
OUT ?= dist

.PHONY: require-target preflight lock prepare build stage verify benchmark

require-target:
	@test '$(origin TARGET)' = 'command line' && test -n '$(TARGET)' || { echo 'TARGET must be an explicit Make command-line variable' >&2; exit 2; }

preflight: require-target
	@scripts/check-build-environment.sh '$(TARGET)'

lock: preflight
	@cargo metadata --format-version 1 > /dev/null

prepare: preflight
	@cargo fetch --locked --target '$(TARGET)'

build: prepare
	@node scripts/check-cursor-contract.mjs
	@cargo build --locked --release --target '$(TARGET)' --bin soksak-sidecar-terminal-wezterm

stage: build
	@scripts/stage-built.sh '$(OUT)' '$(TARGET)'

verify: build
	@node scripts/check-release-workflow.mjs
	@scripts/gate.sh '$(TARGET)'

benchmark: verify
	@case '$(BENCH_OUT)' in /*) ;; *) echo 'BENCH_OUT must be an explicit absolute output directory' >&2; exit 2 ;; esac
	@test -x "$$SOKSAK_PTYD_BIN" || { echo 'SOKSAK_PTYD_BIN must name the product-owned PTY executable' >&2; exit 2; }
	@SOKSAK_BENCH_OUT='$(BENCH_OUT)' cargo test --locked --release --target '$(TARGET)' --test bench -- --ignored --nocapture
