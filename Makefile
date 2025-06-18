
build:
	cargo build --release

build-libretro:
	# example: make build-libretro target=android
	@TARGET=""
	@if [ "$(target)" = "android" ]; then \
		TARGET="--target aarch64-linux-android"; \
	fi; \
	cargo build -p libretro_core --lib --release $$TARGET

