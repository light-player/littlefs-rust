//! File read/write integration tests.
//!
//! Per roadmap phase 04–05. test_files_simple: create, write, close, mount, read.
//! test_files_append, test_files_truncate, test_files_many.

use lp_littlefs::{
    create_inline_file, CachedBlockDevice, Config, LittleFs, OpenFlags, RamBlockDevice,
};

fn make_config() -> Config {
    Config::default_for_tests(128)
}

fn cached_bd(config: &Config) -> CachedBlockDevice<RamBlockDevice> {
    let ram = RamBlockDevice::new(config.block_size, config.block_count);
    CachedBlockDevice::new(ram, config).unwrap()
}

fn make_fs_with_hello() -> (CachedBlockDevice<RamBlockDevice>, Config, LittleFs) {
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    create_inline_file(&bd, &config, "hello", b"Hello World!\0").unwrap();

    fs.mount(&bd, &config).unwrap();

    (bd, config, fs)
}

#[test]
fn test_files_simple_read() {
    let (bd, config, mut fs) = make_fs_with_hello();

    let mut file = fs
        .file_open(&bd, &config, "hello", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();

    assert_eq!(fs.file_size(&file).unwrap(), 13);
    assert_eq!(fs.file_tell(&file).unwrap(), 0);

    let mut buf = [0u8; 32];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 13);
    assert_eq!(&buf[..13], b"Hello World!\0");

    let n2 = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n2, 0);

    fs.file_close(&bd, &config, file).unwrap();
}

#[test]
fn test_files_seek_tell() {
    let (bd, config, mut fs) = make_fs_with_hello();

    let mut file = fs
        .file_open(&bd, &config, "hello", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();

    let mut buf = [0u8; 4];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"Hell");
    assert_eq!(fs.file_tell(&file).unwrap(), 4);

    fs.file_rewind(&bd, &config, &mut file).unwrap();
    assert_eq!(fs.file_tell(&file).unwrap(), 0);

    let n2 = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n2, 4);
    assert_eq!(&buf[..4], b"Hell");

    let pos = fs
        .file_seek(&bd, &config, &mut file, 6, lp_littlefs::SeekWhence::Set)
        .unwrap();
    assert_eq!(pos, 6);
    let n3 = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n3, 4);
    assert_eq!(&buf[..4], b"Worl");

    fs.file_close(&bd, &config, file).unwrap();
}

/// Create, write "Hello World!", close, unmount, mount, read back. Per test_files_simple.
#[test]
fn test_files_simple() {
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "hello",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    let data = b"Hello World!\0";
    let n = fs.file_write(&bd, &config, &mut file, data).unwrap();
    assert_eq!(n, data.len());
    fs.file_close(&bd, &config, file).unwrap();
    fs.unmount().unwrap();

    fs.mount(&bd, &config).unwrap();
    let mut file = fs
        .file_open(&bd, &config, "hello", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 32];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, data.len());
    assert_eq!(&buf[..n], data);
    fs.file_close(&bd, &config, file).unwrap();
}

/// APPEND flag: write, close, reopen with APPEND, write more, read back.
#[test]
fn test_files_append() {
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "x",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    fs.file_write(&bd, &config, &mut file, b"aaaa").unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "x",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::APPEND),
        )
        .unwrap();
    fs.file_write(&bd, &config, &mut file, b"bbbb").unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "x", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 16];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&buf[..8], b"aaaabbbb");
    fs.file_close(&bd, &config, file).unwrap();
}

/// TRUNC: create and write, close, reopen with TRUNC, write different content.
#[test]
fn test_files_truncate() {
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "f",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    fs.file_write(&bd, &config, &mut file, b"original").unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "f",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::TRUNC),
        )
        .unwrap();
    fs.file_write(&bd, &config, &mut file, b"xyz").unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "f", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 16];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&buf[..3], b"xyz");
    fs.file_close(&bd, &config, file).unwrap();
}

/// Large file (chunked write) to exercise CTZ. Per test_files_large subset.
#[test]
fn test_files_large() {
    // First verify 65 bytes works (inline->CTZ transition, single CTZ block)
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();
    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();
    let mut file = fs
        .file_open(
            &bd,
            &config,
            "small",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    let data_65: Vec<u8> = (0..65).collect();
    fs.file_write(&bd, &config, &mut file, &data_65).unwrap();
    fs.file_close(&bd, &config, file).unwrap();
    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();
    let mut file = fs
        .file_open(&bd, &config, "small", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 128];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 65, "65-byte file (inline->CTZ) should read back");
    assert_eq!(&buf[..65], &data_65[..]);
    fs.file_close(&bd, &config, file).unwrap();
    fs.unmount().unwrap();

    // Now test 1024 bytes (multi-block CTZ) - reuse bd, fresh format
    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "big",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    let size = 513usize; // two blocks - first full, second has 1 byte
    let chunk = 64;
    for i in 0..size.div_ceil(chunk) {
        let start = i * chunk;
        let end = (start + chunk).min(size);
        let _len = end - start;
        let data: Vec<u8> = (start..end).map(|j| (j % 256) as u8).collect();
        fs.file_write(&bd, &config, &mut file, &data).unwrap();
    }
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "big", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    assert_eq!(fs.file_size(&file).unwrap(), size as i64);
    let mut buf = [0u8; 64];
    let mut pos = 0;
    while pos < size {
        let n = fs
            .file_read(&bd, &config, &mut file, &mut buf)
            .unwrap_or_else(|e| panic!("read at pos {} failed: {:?}", pos, e));
        if n == 0 {
            break;
        }
        for (j, &b) in buf.iter().take(n).enumerate() {
            assert_eq!(b, ((pos + j) % 256) as u8, "at pos {}", pos + j);
        }
        pos += n;
    }
    assert_eq!(pos, size);
    fs.file_close(&bd, &config, file).unwrap();
}

/// Overwrite existing file with different size. Per test_files_rewrite.
#[test]
fn test_files_rewrite() {
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();

    // Write initial content
    let mut file = fs
        .file_open(
            &bd,
            &config,
            "f",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    let data1: Vec<u8> = (0..64).map(|i| i as u8).collect();
    fs.file_write(&bd, &config, &mut file, &data1).unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    // Reopen for overwrite (no TRUNC), write smaller then larger
    let mut file = fs
        .file_open(&bd, &config, "f", OpenFlags::new(OpenFlags::WRONLY))
        .unwrap();
    let data2: Vec<u8> = (100..150).map(|i| i as u8).collect();
    fs.file_write(&bd, &config, &mut file, &data2).unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    // Read: first 50 bytes are new (100..150), bytes 50..64 are old (50..64)
    let mut file = fs
        .file_open(&bd, &config, "f", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    assert_eq!(fs.file_size(&file).unwrap(), 64);
    let mut buf = vec![0u8; 64];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 64);
    assert_eq!(&buf[..50], &data2[..], "first 50 bytes overwritten");
    assert_eq!(&buf[50..64], &data1[50..64], "bytes 50..64 preserved");
    fs.file_close(&bd, &config, file).unwrap();

    // Rewrite with more data (extend)
    let mut file = fs
        .file_open(&bd, &config, "f", OpenFlags::new(OpenFlags::WRONLY))
        .unwrap();
    let data3: Vec<u8> = (200..280).map(|i| i as u8).collect();
    fs.file_write(&bd, &config, &mut file, &data3).unwrap();
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "f", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    assert_eq!(fs.file_size(&file).unwrap(), 80);
    let mut buf = vec![0u8; 128];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 80);
    assert_eq!(&buf[..80], &data3[..]);
    fs.file_close(&bd, &config, file).unwrap();
}

/// Many small files. Per test_files_many.
#[test]
fn test_files_many() {
    let config = make_config();
    let bd = cached_bd(&config);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();

    for i in 0..8u8 {
        let name = format!("f{}", i);
        let mut file = fs
            .file_open(
                &bd,
                &config,
                &name,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
            )
            .unwrap();
        fs.file_write(&bd, &config, &mut file, &[i]).unwrap();
        fs.file_close(&bd, &config, file).unwrap();
    }

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    for i in 0..8u8 {
        let name = format!("f{}", i);
        let mut file = fs
            .file_open(&bd, &config, &name, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        let mut buf = [0u8; 1];
        let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], i);
        fs.file_close(&bd, &config, file).unwrap();
    }
}
