//! Power-loss simulation tests.
//!
//! Per upstream test_powerloss.toml.
//! test_powerloss_only_rev: partial write (rev only); mount still works.
//!
//! Run with: RUST_LOG=trace cargo test test_powerloss --features trace

mod common;

use common::{default_config, uncached_bd};
use lp_littlefs::{BlockDevice, LittleFs, OpenFlags};

// --- test_powerloss_only_rev ---
// Upstream: write rev+1 to one block of dir pair; mount picks higher rev, read/write still works.
#[test]
fn test_powerloss_only_rev() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let bd = uncached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "notebook").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::APPEND),
        )
        .unwrap();
    let buf = b"hello";
    for _ in 0..5 {
        lfs.file_write(&bd, &config, &mut file, buf).unwrap();
        lfs.file_sync(&bd, &config, &mut file).unwrap();
    }
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    let mut rbuf = [0u8; 256];
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..5])
            .unwrap();
        assert_eq!(n, 5);
        assert_eq!(&rbuf[..5], b"hello");
    }
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let dir = lfs.dir_open(&bd, &config, "notebook").unwrap();
    let pair = dir.pair();
    let rev = dir.revision();
    drop(dir);
    lfs.unmount(&bd, &config).unwrap();

    let mut block_buf = vec![0u8; config.block_size as usize];
    bd.read(pair[1], 0, &mut block_buf).unwrap();
    block_buf[0..4].copy_from_slice(&(rev + 1).to_le_bytes());
    bd.erase(pair[1]).unwrap();
    bd.prog(pair[1], 0, &block_buf).unwrap();

    lfs.mount(&bd, &config).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..5])
            .unwrap();
        assert_eq!(n, 5);
        assert_eq!(&rbuf[..5], b"hello");
    }
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::APPEND),
        )
        .unwrap();
    let buf2 = b"goodbye";
    for _ in 0..5 {
        lfs.file_write(&bd, &config, &mut file, buf2).unwrap();
        lfs.file_sync(&bd, &config, &mut file).unwrap();
    }
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..5])
            .unwrap();
        assert_eq!(n, 5);
        assert_eq!(&rbuf[..5], b"hello");
    }
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..7])
            .unwrap();
        assert_eq!(n, 7);
        assert_eq!(&rbuf[..7], b"goodbye");
    }
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}

// --- test_powerloss_partial_prog ---
// Upstream: simulate partial prog (tweak one byte in metadata); mount should still work.
// Requires prog_size < block_size (we have 16 < 512).
#[test]
fn test_powerloss_partial_prog() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let bd = uncached_bd(&config);
    let mut lfs = LittleFs::new();
    lfs.format(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    lfs.mkdir(&bd, &config, "notebook").unwrap();
    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::APPEND),
        )
        .unwrap();
    let buf = b"hello";
    for _ in 0..5 {
        lfs.file_write(&bd, &config, &mut file, buf).unwrap();
        lfs.file_sync(&bd, &config, &mut file).unwrap();
    }
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();

    lfs.mount(&bd, &config).unwrap();
    let dir = lfs.dir_open(&bd, &config, "notebook").unwrap();
    let block = dir.pair()[0];
    let off = dir.off() as u32;
    drop(dir);
    lfs.unmount(&bd, &config).unwrap();

    let mut block_buf = vec![0u8; config.block_size as usize];
    bd.read(block, 0, &mut block_buf).unwrap();
    let byte_off = 0usize;
    let byte_value = 0x33u8;
    block_buf[off as usize + byte_off] = byte_value;

    bd.erase(block).unwrap();
    bd.prog(block, 0, &block_buf).unwrap();

    lfs.mount(&bd, &config).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    let mut rbuf = [0u8; 256];
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..5])
            .unwrap();
        assert_eq!(n, 5);
        assert_eq!(&rbuf[..5], b"hello");
    }
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::APPEND),
        )
        .unwrap();
    let buf2 = b"goodbye";
    for _ in 0..5 {
        lfs.file_write(&bd, &config, &mut file, buf2).unwrap();
        lfs.file_sync(&bd, &config, &mut file).unwrap();
    }
    lfs.file_close(&bd, &config, file).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "notebook/paper",
            OpenFlags::new(OpenFlags::RDONLY),
        )
        .unwrap();
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..5])
            .unwrap();
        assert_eq!(n, 5);
        assert_eq!(&rbuf[..5], b"hello");
    }
    for _ in 0..5 {
        let n = lfs
            .file_read(&bd, &config, &mut file, &mut rbuf[..7])
            .unwrap();
        assert_eq!(n, 7);
        assert_eq!(&rbuf[..7], b"goodbye");
    }
    lfs.file_close(&bd, &config, file).unwrap();
    lfs.unmount(&bd, &config).unwrap();
}
