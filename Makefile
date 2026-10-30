.PHONY: all kernel programs image run clean fmt lint

all: image

kernel:
	cargo build --release -p rustdos-kernel

programs:
	mkdir -p build
	nasm -f bin programs/hello.asm -o build/HELLO.COM

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
