/*
 * C reproducer for test_exhaustion_wear_leveling.
 * ERASE_CYCLES=20, BLOCK_CYCLES=10, 256 blocks, half start at max wear.
 * C should handle CORRUPT internally and only return NOSPC.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include "lfs.h"

#define BLOCK_SIZE  512
#define BLOCK_COUNT 256
#define CACHE_SIZE  512
#define ERASE_CYCLES 20
#define BLOCK_CYCLES 10
#define FILES 10

static uint8_t *ram_buffer;
static uint32_t wear[BLOCK_COUNT];

static uint32_t test_prng(uint32_t *state) {
    uint32_t x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    return x;
}

static int ram_read(const struct lfs_config *c, lfs_block_t block,
        lfs_off_t off, void *buffer, lfs_size_t size) {
    (void)c;
    if (wear[block] >= ERASE_CYCLES) {
        return LFS_ERR_CORRUPT;
    }
    memcpy(buffer, ram_buffer + block * BLOCK_SIZE + off, size);
    return 0;
}

static int ram_prog(const struct lfs_config *c, lfs_block_t block,
        lfs_off_t off, const void *buffer, lfs_size_t size) {
    (void)c;
    if (wear[block] >= ERASE_CYCLES) {
        return LFS_ERR_CORRUPT;
    }
    memcpy(ram_buffer + block * BLOCK_SIZE + off, buffer, size);
    return 0;
}

static int ram_erase(const struct lfs_config *c, lfs_block_t block) {
    (void)c;
    if (wear[block] >= ERASE_CYCLES) {
        return LFS_ERR_CORRUPT;
    }
    wear[block]++;
    memset(ram_buffer + block * BLOCK_SIZE, 0xff, BLOCK_SIZE);
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
    ram_buffer = calloc(BLOCK_COUNT, BLOCK_SIZE);
    assert(ram_buffer);

    /* Run 0: half blocks usable */
    uint32_t run_block_count[2] = {BLOCK_COUNT/2, BLOCK_COUNT};
    uint32_t run_cycles[2] = {0, 0};

    for (int run = 0; run < 2; run++) {
        memset(wear, 0, sizeof(wear));
        for (uint32_t b = 0; b < BLOCK_COUNT; b++) {
            wear[b] = (b < run_block_count[run]) ? 0 : ERASE_CYCLES;
        }
        memset(ram_buffer, 0, BLOCK_COUNT * BLOCK_SIZE);

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

        uint32_t cycle = 0;
        int done = 0;
        while (!done) {
            CHECK(lfs_mount(&lfs, &cfg));
            for (uint32_t i = 0; i < FILES && !done; i++) {
                char path[64];
                sprintf(path, "roadrunner/test%u", i);
                uint32_t prng = cycle * i;
                lfs_size_t size = 1 << ((test_prng(&prng) % 10) + 2);

                lfs_file_t file;
                CHECK(lfs_file_open(&lfs, &file, path,
                        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_TRUNC));
                for (lfs_size_t j = 0; j < size; j++) {
                    char c = 'a' + (test_prng(&prng) % 26);
                    int res = lfs_file_write(&lfs, &file, &c, 1);
                    if (res == LFS_ERR_NOSPC) {
                        lfs_file_close(&lfs, &file);
                        lfs_unmount(&lfs);
                        done = 1;
                        break;
                    }
                    if (res != 1) {
                        fprintf(stderr, "UNEXPECTED: write returned %d at cycle=%u file=%u byte=%u\n",
                                res, cycle, i, j);
                        exit(1);
                    }
                }
                if (done) break;
                int err = lfs_file_close(&lfs, &file);
                if (err == LFS_ERR_NOSPC) {
                    lfs_unmount(&lfs);
                    done = 1;
                    break;
                }
                CHECK(err);
            }
            if (!done) {
                /* verify */
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
                            fprintf(stderr, "MISMATCH cycle=%u file=%u byte=%u expected='%c' got='%c'\n",
                                    cycle, i, j, c, r);
                            exit(1);
                        }
                    }
                    CHECK(lfs_file_close(&lfs, &file));
                }
                CHECK(lfs_unmount(&lfs));
                cycle++;
            }
        }
        run_cycles[run] = cycle;
        fprintf(stderr, "run %d (%u blocks usable): %u cycles\n", run, run_block_count[run], cycle);
    }

    assert(run_cycles[1] * 110 / 100 > 2 * run_cycles[0]);
    fprintf(stderr, "\nPASS: %u vs %u cycles (ratio %.2f)\n",
            run_cycles[0], run_cycles[1],
            (double)run_cycles[1] / (double)run_cycles[0]);
    free(ram_buffer);
    return 0;
}
