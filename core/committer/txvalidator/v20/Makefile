ROOT_DIR := $(dir $(realpath $(lastword $(MAKEFILE_LIST))))

build:
	cd lib/verification_tool && cargo build --release
	cp lib/verification_tool/target/release/libverify.so lib/
	go build -ldflags="-r $(ROOT_DIR)lib" correctness_tool.go

run: build
	./correctness_tool
