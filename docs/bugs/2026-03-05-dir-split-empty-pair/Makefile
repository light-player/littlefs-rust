# Build C reproducer. Uses littlefs from reference.
# Run from this dir: make run

LFS_SRC ?= ../../../reference
CC ?= cc
CFLAGS := -I$(LFS_SRC) -Wall -Wextra -O2

.PHONY: all clean run

all: repro_remove26 repro_remove26_trace

repro_remove26: repro_remove26.c
	$(CC) $(CFLAGS) -o $@ $< \
		$(LFS_SRC)/lfs.c \
		$(LFS_SRC)/lfs_util.c

repro_remove26_trace: repro_remove26_trace.c
	$(CC) $(CFLAGS) -o $@ $< \
		$(LFS_SRC)/lfs.c \
		$(LFS_SRC)/lfs_util.c

clean:
	rm -f repro_remove26 repro_remove26_trace

run: repro_remove26
	./repro_remove26 2>&1 | tee c-trace.log

run-trace: repro_remove26_trace
	./repro_remove26_trace 2>&1 | tee c-trace-detail.log
