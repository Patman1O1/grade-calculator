# Builds the `grade` command-line tool and its tests.
#
#   make            build build/grade
#   make test       run the C tests (PDF figures + engine rules)
#   make difftest   compare against the TypeScript engine on random courses (needs Node 22.18+)
#   make sanitize   run the tests and difftest under AddressSanitizer + UBSan
#   make examples   regenerate examples/*.json from src/grade/courses.ts (needs Node)
#   make install    install to $(PREFIX)/bin (default /usr/local)

CC      ?= cc
# -ffp-contract=off keeps the compiler from fusing a*b+c into one FMA
# instruction, which would change results compared with JavaScript.
CFLAGS  ?= -O2 -g
# Required flags live apart from CFLAGS so `make CFLAGS=...` cannot drop them.
REQUIRED_CFLAGS := -std=c11 -ffp-contract=off -Iinclude \
           -Wall -Wextra -Wpedantic -Wshadow -Wconversion -Wno-sign-conversion
LDLIBS  += -lm
PREFIX  ?= /usr/local
BUILD   ?= build

ENGINE  := src/grade.c src/json.c src/course_json.c
HEADERS := include/grade.h src/json.h src/course_json.h

.PHONY: all test difftest sanitize examples install clean

all: $(BUILD)/grade

$(BUILD):
	mkdir -p $@

$(BUILD)/grade: src/main.c $(ENGINE) $(HEADERS) | $(BUILD)
	$(CC) $(REQUIRED_CFLAGS) $(CFLAGS) -o $@ src/main.c $(ENGINE) $(LDLIBS)

$(BUILD)/test_grade: tests/test_grade.c $(ENGINE) $(HEADERS) | $(BUILD)
	$(CC) $(REQUIRED_CFLAGS) $(CFLAGS) -Isrc -o $@ tests/test_grade.c $(ENGINE) $(LDLIBS)

test: $(BUILD)/test_grade $(BUILD)/grade
	$(BUILD)/test_grade examples

difftest: $(BUILD)/grade
	node tools/difftest.mjs $(BUILD)/grade

sanitize:
	$(MAKE) BUILD=build/sanitize CFLAGS="-O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer -fno-sanitize-recover=all" test difftest

examples:
	node tools/export-examples.mjs

install: $(BUILD)/grade
	install -d $(DESTDIR)$(PREFIX)/bin
	install -m 755 $(BUILD)/grade $(DESTDIR)$(PREFIX)/bin/grade

clean:
	rm -rf build
