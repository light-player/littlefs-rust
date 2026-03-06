# Build C reproducer for resize_too_big. Uses littlefs from reference.
# Run from this dir: make

LFS_SRC ?= ../../../reference
CC ?= cc
CFLAGS := -I$(LFS_SRC) -Wall -Wextra -O2

.PHONY: all clean run

all: resize_too_big

resize_too_big: resize_too_big.c
	$(CC) $(CFLAGS) -o $@ $< \
		$(LFS_SRC)/lfs.c \
		$(LFS_SRC)/lfs_util.c

clean:
	rm -f resize_too_big

run: resize_too_big
	./resize_too_big 2>&1 | tee c-trace.log
