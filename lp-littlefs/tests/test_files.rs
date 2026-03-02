//! File read integration tests.
//!
//! Per roadmap phase 04. Uses create_inline_file to seed inline files for validation.

use lp_littlefs::{create_inline_file, Config, LittleFs, OpenFlags, RamBlockDevice};

fn make_fs_with_hello() -> (RamBlockDevice, Config, LittleFs) {
    let config = Config::default_for_tests(128);
    let bd = RamBlockDevice::new(config.block_size, config.block_count);
    let mut fs = LittleFs::new();

    fs.format(&bd, &config).unwrap();
    create_inline_file(&bd, &config, "hello", b"Hello World!\0").unwrap();

    fs.mount(&bd, &config).unwrap();

    (bd, config, fs)
}

#[test]
fn test_files_simple_read() {
    let (bd, config, fs) = make_fs_with_hello();

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

    fs.file_close(file).unwrap();
}

#[test]
fn test_files_seek_tell() {
    let (bd, config, fs) = make_fs_with_hello();

    let mut file = fs
        .file_open(&bd, &config, "hello", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();

    let mut buf = [0u8; 4];
    let n = fs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"Hell");
    assert_eq!(fs.file_tell(&file).unwrap(), 4);

    let pos = fs
        .file_seek(&bd, &config, &mut file, 0, lp_littlefs::SeekWhence::Set)
        .unwrap();
    assert_eq!(pos, 0);

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

    fs.file_close(file).unwrap();
}
