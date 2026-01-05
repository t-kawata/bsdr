.PHONY: run check build build-linux-amd64 setup-linux-amd64 up-mysql down-mysql conn-mysql

run:
	cargo run -- ${ARGS}

check:
	cargo check

build:
	cargo build --release

setup-linux-amd64:
	rustup target add x86_64-unknown-linux-gnu
	cargo install cargo-zigbuild

build-linux-amd64:
	cargo zigbuild --release --target x86_64-unknown-linux-gnu

up-mysql:
	cd ./docker && docker compose up -d

down-mysql:
	cd ./docker && docker compose down
	
conn-mysql:
	mysql -h 127.0.0.1 -D bsdr -u asterisk -p

gen-migration:
	@if [ -z "${NAME}" ]; then echo "\033[1;31mError: NAME is empty.\033[0m"; exit 1; fi
	mkdir -p ./src/migration && touch ./src/migration/mod.rs
	sea-orm-cli migrate generate ${NAME} -d ./src/migration

gen-entities:
	@if [ -z "${HOST}" ]; then echo "\033[1;31mError: HOST is empty.\033[0m"; exit 1; fi
	mkdir -p ./src/entities && touch ./src/entities/mod.rs && touch ./src/entities/prelude.rs
	sea-orm-cli generate entity \
		--with-serde both \
		-u mysql://asterisk:yu51043chie3@${HOST}:3306/bsdr \
		-o ./src/entities
	@echo "Patching entity files for JST timestamp behavior..."
	@for file in ./src/entities/*.rs; do \
		basename=$$(basename "$$file"); \
		if [ "$$basename" != "mod.rs" ] && [ "$$basename" != "prelude.rs" ]; then \
			if ! grep -q "impl_jst_timestamp_behavior" "$$file"; then \
				sed -i '' 's/impl ActiveModelBehavior for ActiveModel {}/\/\/ impl ActiveModelBehavior for ActiveModel {}\ncrate::impl_jst_timestamp_behavior!(ActiveModel);/' "$$file"; \
				echo "  Patched: $$basename"; \
			else \
				echo "  Skipped (already patched): $$basename"; \
			fi \
		fi \
	done
	@echo "\033[1;32mEntity generation complete.\033[0m"