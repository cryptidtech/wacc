wasm = $(subst wast,wasm,$(wildcard *.wast))

all: $(wasm)

%.wasm: %.wast
	wat2wasm $< -o ../../target/$@

clean:
	for f in $(wasm); do \
		rm -f ../../target/$${f}; \
	done

.PHONY: all clean
