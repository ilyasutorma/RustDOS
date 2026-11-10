.PHONY: all kernel programs image run clean fmt lint

all: image

kernel:
	cargo build --release -p rustdos-kernel

programs:
	mkdir -p build
	for f in programs/*.asm; do \
		n=$$(basename $$f .asm | tr a-z A-Z); \
		nasm -f bin $$f -o build/$$n.COM || exit 1; \
	done

image: kernel programs
	./scripts/mkimage.sh

run: image
	./scripts/run-qemu.sh

fmt:
	cargo fmt --all

lint:
	cargo clippy --release -p rustdos-kernel

clean:
	cargo clean
	rm -rf build
