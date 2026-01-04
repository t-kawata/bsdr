.PHONY: run build build-linux-amd64 setup-linux-amd64 up-mysql down-mysql conn-mysql

run:
	cargo run -- ${ARGS}

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