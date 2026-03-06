LFS_SRC ?= ../../../reference
CC ?= cc
CFLAGS := -I$(LFS_SRC) -Wall -Wextra -O2

.PHONY: all clean run run-wear

all: repro_exhaustion repro_wear_leveling

repro_exhaustion: repro_exhaustion.c
	$(CC) $(CFLAGS) -o $@ $< \
		$(LFS_SRC)/lfs.c \
		$(LFS_SRC)/lfs_util.c

repro_wear_leveling: repro_wear_leveling.c
	$(CC) $(CFLAGS) -o $@ $< \
		$(LFS_SRC)/lfs.c \
		$(LFS_SRC)/lfs_util.c

clean:
	rm -f repro_exhaustion repro_wear_leveling

run: repro_exhaustion
	./repro_exhaustion 2>&1 | tee c-trace.log

run-wear: repro_wear_leveling
	./repro_wear_leveling 2>&1 | tee c-wear-trace.log
