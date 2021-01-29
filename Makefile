all: web rust

rust: target

target:
	cargo build

WEB_FILES = $(wildcard rplanner-web/src/*)
web: $(WEB_FILES)
	rm -rf web/ && cd rplanner-web && yarn build && cp -r build/ ../web/

clean:
	rm -rf web/

.PHONY: clean target
