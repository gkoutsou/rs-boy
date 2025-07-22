
build:
	cargo build --release

build-libretro:
	# example: make build-libretro target=android
	@TARGET=""
	@if [ "$(target)" = "android" ]; then \
		TARGET="--target aarch64-linux-android"; \
	elif [ "$(target)" = "ios" ]; then\
		TARGET="--target aarch64-apple-ios"; \
	fi; \
	cargo build -p libretro_core --lib --release $$TARGET

test:
	cargo test --no-fail-fast