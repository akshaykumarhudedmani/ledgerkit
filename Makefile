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
	cargo run -p ledgerkit-cli -- account add --id assets:cash --type asset --commodity INR --dir .demo
	cargo run -p ledgerkit-cli -- account add --id expenses:food --type expense --commodity INR --dir .demo
	cargo run -p ledgerkit-cli -- tx add --date 2026-03-01 --payee Cafe \
		--posting assets:cash=-250.00:INR --posting expenses:food=250.00:INR --dir .demo
	cargo run -p ledgerkit-cli -- balance --account assets:cash --dir .demo
	cargo run -p ledgerkit-cli -- verify --dir .demo
	cargo run -p ledgerkit-cli -- replay --dir .demo
	cargo run -p ledgerkit-cli -- export --format beancount --out .demo/ledger.bean --dir .demo

verify-demo: demo

clean:
	cargo clean
	rm -rf .demo .ledgerkit
