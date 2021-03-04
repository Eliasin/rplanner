all: web rust

rust: target

target:
	cargo build

WEB_SRC_FILES = $(wildcard rplanner-client/src/**/*.rs)
WEB_CSS_FILES = $(wildcard rplanner-client/css/*.css)
WEB_HTML_FILES = $(wildcard rplanner-client/*.html)
WEB_FILES = $(WEB_SRC_FILES) $(WEB_CSS_FILES) $(WEB_HTML_FILES)
web: $(WEB_FILES)
	rm -rf web/ && cd rplanner-client && trunk build && cp -r dist/ ../web/

clean:
	rm -rf web/

.PHONY: clean target
