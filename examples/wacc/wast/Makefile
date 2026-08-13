wasm = $(subst wast,wasm,$(wildcard *.wast))

all: $(wasm)

%.wasm: %.wast
	wat2wasm --debug-parser $< -o $@

clean:
	for f in $(wasm); do \
		rm -f $${f}; \
	done

.PHONY: all clean
