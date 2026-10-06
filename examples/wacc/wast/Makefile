wasm = $(subst wast,wasm,$(wildcard *.wast))

# The wacc crate directory; this Makefile lives at examples/wacc/wast.
ROOT := $(abspath ../../..)
GUESTS := unlock log signature_lock signature_first abi
COMPONENTS_DIR := $(ROOT)/target/components

all: $(wasm)

%.wasm: %.wast
	wat2wasm --debug-parser $< -o $@

guests:
	@mkdir -p $(COMPONENTS_DIR)
	@for g in $(GUESTS); do \
		echo "building guest $$g"; \
		cargo build --target wasm32-wasip2 --release --manifest-path $(ROOT)/examples/wacc/$$g/Cargo.toml || exit 1; \
		cp $(ROOT)/examples/wacc/$$g/target/wasm32-wasip2/release/$$g.wasm $(COMPONENTS_DIR)/$$g.wasm || exit 1; \
	done

fixtures: all guests

clean:
	for f in $(wasm); do \
		rm -f $${f}; \
	done
	rm -rf $(COMPONENTS_DIR)

.PHONY: all clean guests fixtures