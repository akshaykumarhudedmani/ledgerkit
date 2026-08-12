# CI

.PHONY: test fmt clippy build demo verify-demo clean

build:
	cargo build --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

demo: build
	cargo run -p ledgerkit-cli -- init --dir .demo
	cargo run -p ledgerkit-cli -- import fixtures/csv/generic/sample.csv \
		--account assets:bank:checking --adapter generic_csv --commodity USD --dir .demo
	cargo run -p ledgerkit-cli -- dedupe --dir .demo
	cargo run -p ledgerkit-cli -- rules apply --file fixtures/rules/default.yaml --dir .demo
	cargo run -p ledgerkit-cli -- reconcile --account assets:bank:checking --balance 2409.20 --as-of 2026-01-07 --commodity USD --dir .demo
	cargo run -p ledgerkit-cli -- verify --dir .demo
	cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo

verify-demo: demo

clean:
	cargo clean
	rm -rf .demo .ledgerkit
