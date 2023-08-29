PREFIX?=/usr/local
# CFLAGS=$(shell pkg-config --cflags libbsd-overlay)
# LDFLAGS=$(shell pkg-config --libs libbsd-overlay)
CC=gcc

all: git-lfs-agent-scp

git-lfs-agent-scp: main.c
	$(CC) $(CFLAGS) -pedantic-errors -Wall -Wextra -Wpedantic -march=native -Os $< -o $@ $(LDFLAGS)

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
