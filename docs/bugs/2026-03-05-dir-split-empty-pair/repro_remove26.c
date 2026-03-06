/*
 * C reproducer for test_interspersed_remove_files with FILES=26.
 * Same sequence as Rust test: create 26 files (a-z), unmount, remount,
 * open "zzz", write+sync+remove interleaved, then verify dir listing.
 *
 * Build: make -f Makefile
 * Run:   ./repro_remove26 2>&1 | tee c-trace.log
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

int main(void) {
    ram_buffer = calloc(LFS_BLOCK_COUNT, LFS_BLOCK_SIZE);
    if (!ram_buffer) {
        fprintf(stderr, "calloc failed\n");
        return 1;
    }

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

    /* Create FILES files with SIZE bytes each */
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
    fprintf(stderr, "Created %d files OK\n", FILES);

    /* Remount, open "zzz", interleave writes+syncs with removes */
    CHECK(lfs_mount(&lfs, &cfg));

    lfs_file_t file;
    CHECK(lfs_file_open(&lfs, &file, "zzz", LFS_O_WRONLY | LFS_O_CREAT));

    for (int j = 0; j < FILES; j++) {
        assert(lfs_file_write(&lfs, &file, "~", 1) == 1);
        CHECK(lfs_file_sync(&lfs, &file));

        char path[2] = { alphas[j], '\0' };
        CHECK(lfs_remove(&lfs, path));
        fprintf(stderr, "  removed '%c'\n", alphas[j]);
    }
    CHECK(lfs_file_close(&lfs, &file));

    /* Verify directory listing */
    lfs_dir_t dir;
    CHECK(lfs_dir_open(&lfs, &dir, "/"));

    struct lfs_info info;
    fprintf(stderr, "\n=== Directory listing ===\n");
    int entry_idx = 0;
    while (true) {
        int rc = lfs_dir_read(&lfs, &dir, &info);
        if (rc == 0) break;
        if (rc < 0) {
            fprintf(stderr, "  dir_read error: %d\n", rc);
            break;
        }
        fprintf(stderr, "  [%d] type=%d size=%u name=\"%s\"\n",
                entry_idx, info.type, (unsigned)info.size, info.name);
        entry_idx++;
    }
    fprintf(stderr, "=== Total: %d entries ===\n", entry_idx);

    CHECK(lfs_dir_close(&lfs, &dir));

    /* Verify expected: ".", "..", "zzz" only */
    CHECK(lfs_dir_open(&lfs, &dir, "/"));

    assert(lfs_dir_read(&lfs, &dir, &info) == 1);
    assert(strcmp(info.name, ".") == 0);
    assert(info.type == LFS_TYPE_DIR);

    assert(lfs_dir_read(&lfs, &dir, &info) == 1);
    assert(strcmp(info.name, "..") == 0);
    assert(info.type == LFS_TYPE_DIR);

    assert(lfs_dir_read(&lfs, &dir, &info) == 1);
    assert(strcmp(info.name, "zzz") == 0);
    assert(info.type == LFS_TYPE_REG);
    assert(info.size == (lfs_size_t)FILES);

    assert(lfs_dir_read(&lfs, &dir, &info) == 0);

    CHECK(lfs_dir_close(&lfs, &dir));

    /* Verify "zzz" content */
    CHECK(lfs_file_open(&lfs, &file, "zzz", LFS_O_RDONLY));
    for (int i = 0; i < FILES; i++) {
        uint8_t buf;
        assert(lfs_file_read(&lfs, &file, &buf, 1) == 1);
        assert(buf == '~');
    }
    CHECK(lfs_file_close(&lfs, &file));

    CHECK(lfs_unmount(&lfs));
    free(ram_buffer);

    fprintf(stderr, "\nPASS\n");
    return 0;
}
