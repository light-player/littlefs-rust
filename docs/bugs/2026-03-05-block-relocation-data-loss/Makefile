LFS_SRC ?= ../../../reference
CC ?= cc
CFLAGS := -I$(LFS_SRC) -Wall -Wextra -O2

.PHONY: all clean run

all: repro_exhaustion

repro_exhaustion: repro_exhaustion.c
	$(CC) $(CFLAGS) -o $@ $< \
		$(LFS_SRC)/lfs.c \
		$(LFS_SRC)/lfs_util.c

clean:
	rm -f repro_exhaustion

run: repro_exhaustion
	./repro_exhaustion 2>&1 | tee c-trace.log
