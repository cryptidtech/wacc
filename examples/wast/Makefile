wasm = $(subst wast,wasm,$(wildcard *.wast))

all: $(wasm)

%.wasm: %.wast
	wat2wasm --debug-parser $< -o ../../target/$@

clean:
	for f in $(wasm); do \
		rm -f ../../target/$${f}; \
	done

.PHONY: all clean
