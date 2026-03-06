/*
 * C reproducer with tracing. Dumps metadata pair chain state after each remove.
 * Uses lfs_dir_open to get initial mdir, then walks internal pair/tail/count.
 *
 * Build: make repro_remove26_trace
 * Run:   ./repro_remove26_trace 2>&1 | tee c-trace-detail.log
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "lfs.h"

#define LFS_READ_SIZE   16
#define LFS_PROG_SIZE   16
#define LFS_BLOCK_SIZE  512
#define LFS_BLOCK_COUNT 128
#define LFS_CACHE_SIZE  512

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

#define CHECK(expr) do { \
    int _err = (expr); \
    if (_err) { \
        fprintf(stderr, "FAIL: %s returned %d at line %d\n", #expr, _err, __LINE__); \
        exit(1); \
    } \
} while(0)

/* Dump root dir chain using dir_open + internal walk via dir_read.
 * We open "/", read the mdir state, then iterate reading entries
 * and tracking when the mdir changes (split follow). */
static void dump_dir_state(lfs_t *fs, const char *label) {
    lfs_dir_t dir;
    CHECK(lfs_dir_open(fs, &dir, "/"));

    fprintf(stderr, "  %s: head=[%u,%u] m.pair=[%u,%u] m.count=%u m.split=%d",
        label, dir.head[0], dir.head[1],
        dir.m.pair[0], dir.m.pair[1], dir.m.count, dir.m.split);
    if (dir.m.split) {
        fprintf(stderr, " m.tail=[%u,%u]", dir.m.tail[0], dir.m.tail[1]);
    }
    fprintf(stderr, "\n");

    /* Read all entries, tracking pair changes */
    struct lfs_info info;
    uint32_t prev_pair0 = dir.m.pair[0];
    while (1) {
        int rc = lfs_dir_read(fs, &dir, &info);
        if (rc == 0) break;
        if (rc < 0) { fprintf(stderr, "    dir_read err: %d\n", rc); break; }

        if (dir.m.pair[0] != prev_pair0) {
            fprintf(stderr, "    → followed tail to pair=[%u,%u] count=%u split=%d",
                dir.m.pair[0], dir.m.pair[1], dir.m.count, dir.m.split);
            if (dir.m.split) {
                fprintf(stderr, " tail=[%u,%u]", dir.m.tail[0], dir.m.tail[1]);
            }
            fprintf(stderr, "\n");
            prev_pair0 = dir.m.pair[0];
        }
    }

    CHECK(lfs_dir_close(fs, &dir));
}

int main(void) {
    ram_buffer = calloc(LFS_BLOCK_COUNT, LFS_BLOCK_SIZE);
    if (!ram_buffer) { fprintf(stderr, "calloc failed\n"); return 1; }

    memset(&cfg, 0, sizeof(cfg));
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

    const char alphas[] = "abcdefghijklmnopqrstuvwxyz";
    int SIZE = 10;
    int FILES = 26;

    CHECK(lfs_format(&lfs, &cfg));
    CHECK(lfs_mount(&lfs, &cfg));

    for (int j = 0; j < FILES; j++) {
        char path[2] = { alphas[j], '\0' };
        lfs_file_t file;
        CHECK(lfs_file_open(&lfs, &file, path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL));
        for (int i = 0; i < SIZE; i++) {
            assert(lfs_file_write(&lfs, &file, &alphas[j], 1) == 1);
        }
        CHECK(lfs_file_close(&lfs, &file));
    }
    CHECK(lfs_unmount(&lfs));
    fprintf(stderr, "Created %d files\n\n", FILES);

    CHECK(lfs_mount(&lfs, &cfg));
    dump_dir_state(&lfs, "BEFORE removes");

    lfs_file_t file;
    CHECK(lfs_file_open(&lfs, &file, "zzz", LFS_O_WRONLY | LFS_O_CREAT));

    for (int j = 0; j < FILES; j++) {
        assert(lfs_file_write(&lfs, &file, "~", 1) == 1);
        CHECK(lfs_file_sync(&lfs, &file));

        char path[2] = { alphas[j], '\0' };
        CHECK(lfs_remove(&lfs, path));

        char label[64];
        snprintf(label, sizeof(label), "after remove '%c' (#%d)", alphas[j], j+1);
        dump_dir_state(&lfs, label);
    }
    CHECK(lfs_file_close(&lfs, &file));

    fprintf(stderr, "\nFINAL after file_close:\n");
    dump_dir_state(&lfs, "FINAL");

    CHECK(lfs_unmount(&lfs));
    free(ram_buffer);
    fprintf(stderr, "\nPASS\n");
    return 0;
}
