PREFIX?=/usr/local
# CFLAGS=$(shell pkg-config --cflags libbsd-overlay)
# LDFLAGS=$(shell pkg-config --libs libbsd-overlay)
CC=zig cc
ifeq  ($(shell uname),Darwin)
STD=-std=c99
MARCH=
else
STD=
MARCH=-march=native
endif

all: git-lfs-agent-scp

git-lfs-agent-scp: main.c
	$(CC) $(CFLAGS) $(STD) -pedantic-errors -Wall -Wextra -Wpedantic $(MARCH) -Os $< -o $@ $(LDFLAGS)

install: git-lfs-agent-scp
	install -m 755 git-lfs-agent-scp $(PREFIX)/bin

uninstall:
	rm -f $(PREFIX)/bin/git-lfs-agent-scp

test: git-lfs-agent-scp
	./test.sh

clangformat:
	clang-format -style=file -i main.c

clean:
	rm -f git-lfs-agent-scp

.PHONY: all install uninstall test clangformat clean
