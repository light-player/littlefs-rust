//! File read/write integration tests.
//!
//! Per roadmap phase 04–05. test_files_simple: create, write, close, mount, read.
//! test_files_append, test_files_truncate, test_files_many.

mod common;

use common::{fresh_fs, fs_with_hello};
use lp_littlefs::{OpenFlags, SeekWhence};

#[test]
fn test_files_simple_read() {
    let (bd, config, mut fs) = fs_with_hello();

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
    let (bd, config, mut fs) = fs_with_hello();

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
    let (bd, config, mut fs) = fresh_fs();

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

#[test]
fn test_rename_file_same_dir() {
    let (bd, config, mut fs) = fs_with_hello();
    fs.rename(&bd, &config, "hello", "world").unwrap();
    let info = fs.stat(&bd, &config, "world").unwrap();
    assert_eq!(info.name().unwrap(), "world");
    assert!(fs.stat(&bd, &config, "hello").is_err());
    let mut file = fs
        .file_open(&bd, &config, "world", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 32];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 13);
    assert_eq!(&buf[..n], b"Hello World!\0");
    fs.file_close(&bd, &config, file).unwrap();
}

#[test]
fn test_fs_gc() {
    let (bd, config, mut fs) = fresh_fs();
    fs.format(&bd, &config).unwrap();
    fs.mount(&bd, &config).unwrap();
    fs.mkdir(&bd, &config, "d").unwrap();
    let mut f = fs
        .file_open(
            &bd,
            &config,
            "d/x",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
        )
        .unwrap();
    fs.file_write(&bd, &config, &mut f, b"hello").unwrap();
    fs.file_close(&bd, &config, f).unwrap();
    fs.fs_gc(&bd, &config).unwrap();
    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();
    let info = fs.stat(&bd, &config, "d/x").unwrap();
    assert_eq!(info.name().unwrap(), "x");
}

/// APPEND flag: write, close, reopen with APPEND, write more, read back.
#[test]
fn test_files_append() {
    let (bd, config, mut fs) = fresh_fs();

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
    let (bd, config, mut fs) = fresh_fs();

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
    let (bd, config, mut fs) = fresh_fs();
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
    let (bd, config, mut fs) = fresh_fs();

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
    let (bd, config, mut fs) = fresh_fs();

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

/// Per test_truncate_simple: write LARGESIZE, close, remount, open RDWR, truncate to MEDIUMSIZE, remount, read.
#[test]
fn test_truncate_simple() {
    const LARGE_SIZE: u64 = 513;
    const MEDIUM_SIZE: u64 = 512;
    let (bd, config, mut fs) = fresh_fs();

    let mut file = fs
        .file_open(
            &bd,
            &config,
            "baldynoop",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    let chunk = b"hair";
    let mut written: u64 = 0;
    while written < LARGE_SIZE {
        let n = chunk.len().min((LARGE_SIZE - written) as usize);
        let w = fs.file_write(&bd, &config, &mut file, &chunk[..n]).unwrap();
        written += w as u64;
    }
    assert_eq!(fs.file_size(&file).unwrap(), LARGE_SIZE as i64);
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "baldynoop", OpenFlags::new(OpenFlags::RDWR))
        .unwrap();
    assert_eq!(fs.file_size(&file).unwrap(), LARGE_SIZE as i64);
    fs.file_truncate(&bd, &config, &mut file, MEDIUM_SIZE)
        .unwrap();
    assert_eq!(fs.file_size(&file).unwrap(), MEDIUM_SIZE as i64);
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "baldynoop", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    assert_eq!(fs.file_size(&file).unwrap(), MEDIUM_SIZE as i64);
    let mut buf = [0u8; 1024];
    let mut read_total: usize = 0;
    while read_total < MEDIUM_SIZE as usize {
        let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
        if n == 0 {
            break;
        }
        let chunk_len = chunk.len();
        for j in 0..n {
            assert_eq!(buf[j], chunk[j % chunk_len], "byte {}", read_total + j);
        }
        read_total += n;
    }
    assert_eq!(read_total, MEDIUM_SIZE as usize);
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 0);
    fs.file_close(&bd, &config, file).unwrap();
}

/// Per test_seek_read: SEEK_SET, SEEK_CUR, SEEK_END, rewind.
#[test]
fn test_seek_read() {
    let (bd, config, mut fs) = fresh_fs();

    let data = b"kittycatcat";
    let mut file = fs
        .file_open(
            &bd,
            &config,
            "kitty",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::APPEND),
        )
        .unwrap();
    for _ in 0..4 {
        fs.file_write(&bd, &config, &mut file, data).unwrap();
    }
    fs.file_close(&bd, &config, file).unwrap();

    fs.unmount().unwrap();
    fs.mount(&bd, &config).unwrap();

    let mut file = fs
        .file_open(&bd, &config, "kitty", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();

    let mut buf = [0u8; 32];
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);
    let pos = fs.file_tell(&file).unwrap();
    assert_eq!(pos, data.len() as i64);

    fs.file_seek(&bd, &config, &mut file, pos, SeekWhence::Set)
        .unwrap();
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);

    fs.file_rewind(&bd, &config, &mut file).unwrap();
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);

    let cur_pos = fs
        .file_seek(&bd, &config, &mut file, 0, SeekWhence::Cur)
        .unwrap();
    assert_eq!(cur_pos, data.len() as i64);
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);

    fs.file_seek(&bd, &config, &mut file, data.len() as i64, SeekWhence::Cur)
        .unwrap();
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);

    fs.file_seek(&bd, &config, &mut file, pos, SeekWhence::Set)
        .unwrap();
    fs.file_seek(
        &bd,
        &config,
        &mut file,
        -(data.len() as i64),
        SeekWhence::Cur,
    )
    .unwrap();
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);

    fs.file_seek(
        &bd,
        &config,
        &mut file,
        -(data.len() as i64),
        SeekWhence::End,
    )
    .unwrap();
    fs.file_read(&bd, &config, &mut file, &mut buf[..data.len()])
        .unwrap();
    assert_eq!(&buf[..data.len()], data);

    let size = fs.file_size(&file).unwrap();
    let end_pos = fs
        .file_seek(&bd, &config, &mut file, 0, SeekWhence::Cur)
        .unwrap();
    assert_eq!(end_pos, size);

    fs.file_close(&bd, &config, file).unwrap();
}

/// Per test_files_many_power_cycle: create file_i, unmount, mount, read — repeat N times.
#[test]
fn test_files_many_power_cycle() {
    let (bd, config, mut fs) = fresh_fs();

    for i in 0..6 {
        fs.mount(&bd, &config).unwrap();
        let name = format!("f{i}");
        let mut file = fs
            .file_open(
                &bd,
                &config,
                &name,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
            )
            .unwrap();
        let content = format!("Hi{i}");
        fs.file_write(&bd, &config, &mut file, content.as_bytes())
            .unwrap();
        fs.file_close(&bd, &config, file).unwrap();
        fs.unmount().unwrap();

        fs.mount(&bd, &config).unwrap();
        let mut file = fs
            .file_open(&bd, &config, &name, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap_or_else(|e| panic!("open {} failed: {e:?}", name));
        let mut buf = [0u8; 16];
        let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
        fs.file_close(&bd, &config, file).unwrap();
        assert_eq!(&buf[..n], content.as_bytes(), "file {name}");
        fs.unmount().unwrap();
    }
}
