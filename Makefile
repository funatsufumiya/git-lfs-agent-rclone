PREFIX?=/usr/local

# CFLAGS=$(shell pkg-config --cflags libbsd-overlay)
# LDFLAGS=$(shell pkg-config --libs libbsd-overlay)

CC=zig cc
# TARGET=x86_64-linux-musl

# if target specified, set target
ifneq ($(TARGET),)
TARGET_FLAGS=-target $(TARGET)
else
TARGET_FLAGS=
endif

# if target specified, warn it
ifneq ($(TARGET),)
$(warning [WARN] Cross compiling target is set to '$(TARGET)')
endif

# if darwin and target is not set
ifeq  ($(shell uname) $(TARGET), Darwin )
STD=-std=c99
MARCH=
else
STD=
MARCH=-march=native
endif


all: git-lfs-agent-scp

git-lfs-agent-scp: main.c
	$(CC) $(CFLAGS) $(STD) $(TARGET_FLAGS) -pedantic-errors -Wall -Wextra -Wpedantic $(MARCH) -Os $< -o $@ $(LDFLAGS)

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
