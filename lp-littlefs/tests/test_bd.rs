//! Block device tests.
//!
//! Corresponds to upstream test_bd.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_bd.toml
//!
//! These tests validate the block device abstraction, not littlefs itself.

use lp_littlefs::{BlockDevice, RamBlockDevice};

/// Default test geometry. Easy to change for different upstream geometries.
fn default_geometry() -> (u32, u32) {
    let block_size = 512;
    let block_count = 128;
    (block_size, block_count)
}

// --- test_bd_one_block ---
// Upstream: erase block 0, prog in chunks, read back and verify.
// Uses (block_index + offset + j) % 251 to avoid powers-of-two aliasing.
#[test]
fn test_bd_one_block() {
    let (block_size, block_count) = default_geometry();
    let bd = RamBlockDevice::new(block_size, block_count);

    let read_size = 16u32;
    let prog_size = 16u32;
    let mut buffer = vec![0u8; read_size.max(prog_size) as usize];

    // Erase block 0
    bd.erase(0).unwrap();

    // Prog in chunks
    for i in (0..block_size).step_by(prog_size as usize) {
        for j in 0..prog_size {
            buffer[j as usize] = ((i + j) % 251) as u8;
        }
        bd.prog(0, i, &buffer[..prog_size as usize]).unwrap();
    }

    // Read back in chunks
    for i in (0..block_size).step_by(read_size as usize) {
        bd.read(0, i, &mut buffer[..read_size as usize]).unwrap();
        for j in 0..read_size {
            assert_eq!(
                buffer[j as usize],
                ((i + j) % 251) as u8,
                "offset {i}, byte {j}"
            );
        }
    }
}

// --- test_bd_two_block ---
// Upstream: prog block 0, read block 0, prog block 1, read block 1, re-read block 0.
#[test]
fn test_bd_two_block() {
    let (block_size, block_count) = default_geometry();
    let bd = RamBlockDevice::new(block_size, block_count);

    let read_size = 16u32;
    let prog_size = 16u32;
    let buf_len = read_size.max(prog_size) as usize;
    let mut buffer = vec![0u8; buf_len];

    for block in [0u32, 1u32] {
        bd.erase(block).unwrap();
        for i in (0..block_size).step_by(prog_size as usize) {
            for j in 0..prog_size {
                buffer[j as usize] = ((block as u64 + i as u64 + j as u64) % 251) as u8;
            }
            bd.prog(block, i, &buffer[..prog_size as usize]).unwrap();
        }
    }

    for block in [0u32, 1u32] {
        for i in (0..block_size).step_by(read_size as usize) {
            bd.read(block, i, &mut buffer[..read_size as usize])
                .unwrap();
            for j in 0..read_size {
                assert_eq!(
                    buffer[j as usize],
                    ((block as u64 + i as u64 + j as u64) % 251) as u8,
                    "block {block}, offset {i}, byte {j}"
                );
            }
        }
    }

    for i in (0..block_size).step_by(read_size as usize) {
        bd.read(0, i, &mut buffer[..read_size as usize]).unwrap();
        for j in 0..read_size {
            assert_eq!(
                buffer[j as usize],
                ((i + j) % 251) as u8,
                "re-read block 0, offset {i}, byte {j}"
            );
        }
    }
}

// --- test_bd_last_block ---
// Upstream: prog block 0, read block 0, prog block_count-1, read block_count-1.
#[test]
fn test_bd_last_block() {
    let (block_size, block_count) = default_geometry();
    let bd = RamBlockDevice::new(block_size, block_count);

    let read_size = 16u32;
    let prog_size = 16u32;
    let buf_len = read_size.max(prog_size) as usize;
    let mut buffer = vec![0u8; buf_len];
    let last_block = block_count - 1;

    bd.erase(0).unwrap();
    for i in (0..block_size).step_by(prog_size as usize) {
        for j in 0..prog_size {
            buffer[j as usize] = ((i + j) % 251) as u8;
        }
        bd.prog(0, i, &buffer[..prog_size as usize]).unwrap();
    }

    for i in (0..block_size).step_by(read_size as usize) {
        bd.read(0, i, &mut buffer[..read_size as usize]).unwrap();
        for j in 0..read_size {
            assert_eq!(
                buffer[j as usize],
                ((i + j) % 251) as u8,
                "block 0, offset {i}, byte {j}"
            );
        }
    }

    bd.erase(last_block).unwrap();
    for i in (0..block_size).step_by(prog_size as usize) {
        for j in 0..prog_size {
            buffer[j as usize] = ((last_block as u64 + i as u64 + j as u64) % 251) as u8;
        }
        bd.prog(last_block, i, &buffer[..prog_size as usize])
            .unwrap();
    }

    for i in (0..block_size).step_by(read_size as usize) {
        bd.read(last_block, i, &mut buffer[..read_size as usize])
            .unwrap();
        for j in 0..read_size {
            assert_eq!(
                buffer[j as usize],
                ((last_block as u64 + i as u64 + j as u64) % 251) as u8,
                "block {last_block}, offset {i}, byte {j}"
            );
        }
    }
}

// --- test_bd_powers_of_two ---
// Upstream: write/read every power-of-two block index (1, 2, 4, 8, ...)
#[test]
fn test_bd_powers_of_two() {
    let (block_size, block_count) = default_geometry();
    let bd = RamBlockDevice::new(block_size, block_count);

    let read_size = 16u32;
    let prog_size = 16u32;
    let buf_len = read_size.max(prog_size) as usize;
    let mut buffer = vec![0u8; buf_len];

    let mut block = 1u32;
    while block < block_count {
        bd.erase(block).unwrap();
        for i in (0..block_size).step_by(prog_size as usize) {
            for j in 0..prog_size {
                buffer[j as usize] = ((block as u64 + i as u64 + j as u64) % 251) as u8;
            }
            bd.prog(block, i, &buffer[..prog_size as usize]).unwrap();
        }

        for i in (0..block_size).step_by(read_size as usize) {
            bd.read(block, i, &mut buffer[..read_size as usize])
                .unwrap();
            for j in 0..read_size {
                assert_eq!(
                    buffer[j as usize],
                    ((block as u64 + i as u64 + j as u64) % 251) as u8,
                    "block {block}, offset {i}, byte {j}"
                );
            }
        }

        block *= 2;
    }

    block = 1;
    while block < block_count {
        for i in (0..block_size).step_by(read_size as usize) {
            bd.read(block, i, &mut buffer[..read_size as usize])
                .unwrap();
            for j in 0..read_size {
                assert_eq!(
                    buffer[j as usize],
                    ((block as u64 + i as u64 + j as u64) % 251) as u8,
                    "re-read block {block}, offset {i}, byte {j}"
                );
            }
        }
        block *= 2;
    }
}

// --- test_bd_fibonacci ---
// Upstream: write/read every fibonacci block index (1, 1, 2, 3, 5, 8, 13, ...)
#[test]
fn test_bd_fibonacci() {
    let (block_size, block_count) = default_geometry();
    let bd = RamBlockDevice::new(block_size, block_count);

    let read_size = 16u32;
    let prog_size = 16u32;
    let buf_len = read_size.max(prog_size) as usize;
    let mut buffer = vec![0u8; buf_len];

    let mut block = 1u32;
    let mut block_prev = 1u32;
    while block < block_count {
        bd.erase(block).unwrap();
        for i in (0..block_size).step_by(prog_size as usize) {
            for j in 0..prog_size {
                buffer[j as usize] = ((block as u64 + i as u64 + j as u64) % 251) as u8;
            }
            bd.prog(block, i, &buffer[..prog_size as usize]).unwrap();
        }

        for i in (0..block_size).step_by(read_size as usize) {
            bd.read(block, i, &mut buffer[..read_size as usize])
                .unwrap();
            for j in 0..read_size {
                assert_eq!(
                    buffer[j as usize],
                    ((block as u64 + i as u64 + j as u64) % 251) as u8,
                    "block {block}, offset {i}, byte {j}"
                );
            }
        }

        let next = block + block_prev;
        block_prev = block;
        block = next;
    }

    block = 1;
    block_prev = 1;
    while block < block_count {
        for i in (0..block_size).step_by(read_size as usize) {
            bd.read(block, i, &mut buffer[..read_size as usize])
                .unwrap();
            for j in 0..read_size {
                assert_eq!(
                    buffer[j as usize],
                    ((block as u64 + i as u64 + j as u64) % 251) as u8,
                    "re-read block {block}, offset {i}, byte {j}"
                );
            }
        }
        let next = block + block_prev;
        block_prev = block;
        block = next;
    }
}
