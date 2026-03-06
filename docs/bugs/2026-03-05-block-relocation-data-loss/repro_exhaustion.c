/*
 * C reproducer for block_cycles>0 data corruption.
 * Matches Rust test: format, mkdir "roadrunner", write+verify 10 files per cycle.
 * Rust fails at cycle=4, file=5. Does C pass?
 *
 * Build: make
 * Run:   ./repro_exhaustion
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "lfs.h"

#define BLOCK_SIZE  512
#define BLOCK_COUNT 256
#define CACHE_SIZE  512
#define BLOCK_CYCLES 5
#define FILES 10
#define MAX_CYCLES 10

static uint8_t *ram_buffer;

static int ram_read(const struct lfs_config *c, lfs_block_t block,
        lfs_off_t off, void *buffer, lfs_size_t size) {
    (void)c;
    memcpy(buffer, ram_buffer + block * BLOCK_SIZE + off, size);
    return 0;
}

static int ram_prog(const struct lfs_config *c, lfs_block_t block,
        lfs_off_t off, const void *buffer, lfs_size_t size) {
    (void)c;
    memcpy(ram_buffer + block * BLOCK_SIZE + off, buffer, size);
    return 0;
}

static int ram_erase(const struct lfs_config *c, lfs_block_t block) {
    (void)c;
    memset(ram_buffer + block * BLOCK_SIZE, 0xff, BLOCK_SIZE);
    return 0;
}

static int ram_sync(const struct lfs_config *c) {
    (void)c;
    return 0;
}

/* TEST_PRNG from littlefs test runner — xorshift32 */
static uint32_t test_prng(uint32_t *state) {
    uint32_t x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    return x;
}

#define CHECK(expr) do { \
    int _err = (expr); \
    if (_err) { \
        fprintf(stderr, "FAIL: %s returned %d at line %d\n", #expr, _err, __LINE__); \
        exit(1); \
    } \
} while(0)

int main(void) {
    ram_buffer = calloc(BLOCK_COUNT, BLOCK_SIZE);
    assert(ram_buffer);

    uint8_t read_buf[CACHE_SIZE];
    uint8_t prog_buf[CACHE_SIZE];
    uint8_t lookahead_buf[BLOCK_SIZE];

    struct lfs_config cfg;
    memset(&cfg, 0, sizeof(cfg));
    cfg.read = ram_read;
    cfg.prog = ram_prog;
    cfg.erase = ram_erase;
    cfg.sync = ram_sync;
    cfg.read_size = 16;
    cfg.prog_size = 16;
    cfg.block_size = BLOCK_SIZE;
    cfg.block_count = BLOCK_COUNT;
    cfg.cache_size = CACHE_SIZE;
    cfg.lookahead_size = BLOCK_SIZE;
    cfg.read_buffer = read_buf;
    cfg.prog_buffer = prog_buf;
    cfg.lookahead_buffer = lookahead_buf;
    cfg.block_cycles = BLOCK_CYCLES;
    cfg.compact_thresh = (lfs_size_t)-1;
    cfg.name_max = 255;
    cfg.file_max = 2147483647;
    cfg.attr_max = 1022;

    lfs_t lfs;
    CHECK(lfs_format(&lfs, &cfg));
    CHECK(lfs_mount(&lfs, &cfg));
    CHECK(lfs_mkdir(&lfs, "roadrunner"));
    CHECK(lfs_unmount(&lfs));

    for (uint32_t cycle = 0; cycle < MAX_CYCLES; cycle++) {
        CHECK(lfs_mount(&lfs, &cfg));

        /* Write phase */
        for (uint32_t i = 0; i < FILES; i++) {
            char path[64];
            sprintf(path, "roadrunner/test%u", i);
            uint32_t prng = cycle * i;
            lfs_size_t size = 1 << ((test_prng(&prng) % 10) + 2);

            lfs_file_t file;
            CHECK(lfs_file_open(&lfs, &file, path,
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC));
            for (lfs_size_t j = 0; j < size; j++) {
                char c = 'a' + (test_prng(&prng) % 26);
                assert(lfs_file_write(&lfs, &file, &c, 1) == 1);
            }
            CHECK(lfs_file_close(&lfs, &file));
        }

        /* Read/verify phase */
        for (uint32_t i = 0; i < FILES; i++) {
            char path[64];
            sprintf(path, "roadrunner/test%u", i);
            uint32_t prng = cycle * i;
            lfs_size_t size = 1 << ((test_prng(&prng) % 10) + 2);

            lfs_file_t file;
            CHECK(lfs_file_open(&lfs, &file, path, LFS_O_RDONLY));
            for (lfs_size_t j = 0; j < size; j++) {
                char c = 'a' + (test_prng(&prng) % 26);
                char r;
                assert(lfs_file_read(&lfs, &file, &r, 1) == 1);
                if (r != c) {
                    fprintf(stderr,
                        "MISMATCH cycle=%u file=%u byte=%u/%u: "
                        "expected=%d('%c') got=%d('%c')\n",
                        cycle, i, j, size, (int)c, c, (int)r, r);
                    exit(1);
                }
            }
            CHECK(lfs_file_close(&lfs, &file));
        }

        CHECK(lfs_unmount(&lfs));
        fprintf(stderr, "cycle %u OK\n", cycle);
    }

    fprintf(stderr, "\nPASS: %d cycles\n", MAX_CYCLES);
    free(ram_buffer);
    return 0;
}
