/*
 * C reproducer for test_entries_resize_too_big.
 * Same sequence as Rust test: 200-char path, create 40B, read, truncate+write 400B, read.
 * Config: 2048 blocks, 512 cache (matches upstream).
 *
 * Build: make -f Makefile (from this directory)
 * Run: ./resize_too_big 2>&1 | tee c-trace.log
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "lfs.h"

#define LFS_READ_SIZE  16
#define LFS_PROG_SIZE  16
#define LFS_BLOCK_SIZE 512
#define LFS_BLOCK_COUNT 2048
#define LFS_CACHE_SIZE 512

static uint8_t *ram_buffer;
static lfs_t lfs;
static struct lfs_config cfg;
static uint8_t read_buf[LFS_CACHE_SIZE];
static uint8_t prog_buf[LFS_CACHE_SIZE];
static uint8_t lookahead_buf[LFS_BLOCK_SIZE];

static int ram_read(const struct lfs_config *c, lfs_block_t block,
        lfs_off_t off, void *buffer, lfs_size_t size) {
    (void)c;
    memcpy(buffer, ram_buffer + block * LFS_BLOCK_SIZE + off, size);
    return 0;
}

static int ram_prog(const struct lfs_config *c, lfs_block_t block,
        lfs_off_t off, const void *buffer, lfs_size_t size) {
    (void)c;
    memcpy(ram_buffer + block * LFS_BLOCK_SIZE + off, buffer, size);
    return 0;
}

static int ram_erase(const struct lfs_config *c, lfs_block_t block) {
    (void)c;
    memset(ram_buffer + block * LFS_BLOCK_SIZE, 0xff, LFS_BLOCK_SIZE);
    return 0;
}

static int ram_sync(const struct lfs_config *c) {
    (void)c;
    return 0;
}

int main(void) {
    ram_buffer = calloc(LFS_BLOCK_COUNT, LFS_BLOCK_SIZE);
    if (!ram_buffer) {
        fprintf(stderr, "calloc failed\n");
        return 1;
    }

    cfg.context = NULL;
    cfg.read = ram_read;
    cfg.prog = ram_prog;
    cfg.erase = ram_erase;
    cfg.sync = ram_sync;
    cfg.read_size = LFS_READ_SIZE;
    cfg.prog_size = LFS_PROG_SIZE;
    cfg.block_size = LFS_BLOCK_SIZE;
    cfg.block_count = LFS_BLOCK_COUNT;
    cfg.cache_size = LFS_CACHE_SIZE;
    cfg.lookahead_size = LFS_BLOCK_SIZE;
    cfg.read_buffer = read_buf;
    cfg.prog_buffer = prog_buf;
    cfg.lookahead_buffer = lookahead_buf;
    cfg.block_cycles = -1;
    cfg.compact_thresh = (lfs_size_t)-1;
    cfg.name_max = 255;
    cfg.file_max = 2147483647;
    cfg.attr_max = 1022;

    int err = lfs_format(&lfs, &cfg);
    if (err) {
        fprintf(stderr, "lfs_format failed: %d\n", err);
        free(ram_buffer);
        return 1;
    }
    fprintf(stderr, "format OK\n");

    err = lfs_mount(&lfs, &cfg);
    if (err) {
        fprintf(stderr, "lfs_mount failed: %d\n", err);
        free(ram_buffer);
        return 1;
    }
    fprintf(stderr, "mount OK\n");

    char path[1024];
    memset(path, 'm', 200);
    path[200] = '\0';

    uint8_t wbuffer[1024];
    uint8_t rbuffer[1024];
    memset(wbuffer, 'c', sizeof(wbuffer));

    lfs_file_t file;

    /* Create with 40 bytes */
    err = lfs_file_open(&lfs, &file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC);
    if (err) {
        fprintf(stderr, "file_open(create 40) failed: %d\n", err);
        goto fail;
    }
    lfs_ssize_t n = lfs_file_write(&lfs, &file, wbuffer, 40);
    if (n != 40) {
        fprintf(stderr, "file_write(40) returned %ld\n", (long)n);
        lfs_file_close(&lfs, &file);
        goto fail;
    }
    err = lfs_file_close(&lfs, &file);
    if (err) {
        fprintf(stderr, "file_close failed: %d\n", err);
        goto fail;
    }
    fprintf(stderr, "create 40 OK\n");

    /* Read 40 bytes */
    err = lfs_file_open(&lfs, &file, path, LFS_O_RDONLY);
    if (err) {
        fprintf(stderr, "file_open(read 40) failed: %d\n", err);
        goto fail;
    }
    n = lfs_file_read(&lfs, &file, rbuffer, 40);
    if (n != 40) {
        fprintf(stderr, "file_read(40) returned %ld\n", (long)n);
        lfs_file_close(&lfs, &file);
        goto fail;
    }
    if (memcmp(rbuffer, wbuffer, 40) != 0) {
        fprintf(stderr, "read 40 data mismatch\n");
        lfs_file_close(&lfs, &file);
        goto fail;
    }
    err = lfs_file_close(&lfs, &file);
    if (err) {
        fprintf(stderr, "file_close failed: %d\n", err);
        goto fail;
    }
    fprintf(stderr, "read 40 OK\n");

    /* Truncate and write 400 bytes */
    err = lfs_file_open(&lfs, &file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC);
    if (err) {
        fprintf(stderr, "file_open(trunc 400) failed: %d\n", err);
        goto fail;
    }
    n = lfs_file_write(&lfs, &file, wbuffer, 400);
    if (n != 400) {
        fprintf(stderr, "file_write(400) returned %ld\n", (long)n);
        lfs_file_close(&lfs, &file);
        goto fail;
    }
    err = lfs_file_close(&lfs, &file);
    if (err) {
        fprintf(stderr, "file_close failed: %d\n", err);
        goto fail;
    }
    fprintf(stderr, "trunc+write 400 OK\n");

    /* Read 400 bytes */
    err = lfs_file_open(&lfs, &file, path, LFS_O_RDONLY);
    if (err) {
        fprintf(stderr, "file_open(read 400) failed: %d\n", err);
        goto fail;
    }
    n = lfs_file_read(&lfs, &file, rbuffer, 400);
    if (n != 400) {
        fprintf(stderr, "file_read(400) returned %ld\n", (long)n);
        lfs_file_close(&lfs, &file);
        goto fail;
    }
    if (memcmp(rbuffer, wbuffer, 400) != 0) {
        fprintf(stderr, "read 400 data mismatch\n");
        lfs_file_close(&lfs, &file);
        goto fail;
    }
    err = lfs_file_close(&lfs, &file);
    if (err) {
        fprintf(stderr, "file_close failed: %d\n", err);
        goto fail;
    }
    fprintf(stderr, "read 400 OK\n");

    err = lfs_unmount(&lfs);
    if (err) {
        fprintf(stderr, "lfs_unmount failed: %d\n", err);
        free(ram_buffer);
        return 1;
    }
    free(ram_buffer);
    fprintf(stderr, "PASS\n");
    return 0;

fail:
    lfs_unmount(&lfs);
    free(ram_buffer);
    return 1;
}
