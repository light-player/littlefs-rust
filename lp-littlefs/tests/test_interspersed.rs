//! Files + dirs mixed ops. Per upstream test_interspersed.toml.

mod common;

use common::{dir_entry_names, fresh_fs, init_log};
use lp_littlefs::{FileType, OpenFlags};
use rstest::rstest;

#[rstest]
#[case(10, 4)]
#[case(100, 10)]
fn test_interspersed_files(#[case] size: usize, #[case] files: usize) {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();
    let alphas = b"abcdefghijklmnopqrstuvwxyz";

    let mut open_files = Vec::new();
    for j in 0..files {
        let path = format!("{}", alphas[j] as char);
        let file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
            )
            .unwrap();
        open_files.push(file);
    }
    for _i in 0..size {
        for j in 0..files {
            lfs.file_write(&bd, &config, &mut open_files[j], &[alphas[j]])
                .unwrap();
        }
    }
    for file in open_files {
        lfs.file_close(&bd, &config, file).unwrap();
    }

    let names = dir_entry_names(&mut lfs, &bd, &config, "/").unwrap();
    assert_eq!(names.len(), files);
    let mut names_sorted = names.clone();
    names_sorted.sort();
    for j in 0..files {
        let name = format!("{}", alphas[j] as char);
        assert!(names_sorted.contains(&name));
        let info = lfs.stat(&bd, &config, &name).unwrap();
        assert_eq!(info.size, size as u32);
        assert!(matches!(info.typ, FileType::Reg));
    }

    let mut open_files = Vec::new();
    for j in 0..files {
        let path = format!("{}", alphas[j] as char);
        let file = lfs
            .file_open(&bd, &config, &path, OpenFlags::new(OpenFlags::RDONLY))
            .unwrap();
        open_files.push(file);
    }
    for _ in 0..size {
        for j in 0..files {
            let mut buf = [0u8; 1];
            let n = lfs
                .file_read(&bd, &config, &mut open_files[j], &mut buf)
                .unwrap();
            assert_eq!(n, 1);
            assert_eq!(buf[0], alphas[j]);
        }
    }
    for file in open_files {
        lfs.file_close(&bd, &config, file).unwrap();
    }
}

#[rstest]
#[case(10, 4)]
fn test_interspersed_remove_files(#[case] size: usize, #[case] files: usize) {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();
    let alphas = b"abcdefghijklmnopqrstuvwxyz";

    for j in 0..files {
        let path = format!("{}", alphas[j] as char);
        let mut file = lfs
            .file_open(
                &bd,
                &config,
                &path,
                OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT | OpenFlags::EXCL),
            )
            .unwrap();
        for _ in 0..size {
            lfs.file_write(&bd, &config, &mut file, &[alphas[j]])
                .unwrap();
        }
        lfs.file_close(&bd, &config, file).unwrap();
    }
    lfs.unmount(&bd, &config).unwrap();
    lfs.mount(&bd, &config).unwrap();

    let mut file = lfs
        .file_open(
            &bd,
            &config,
            "zzz",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    for j in 0..files {
        lfs.file_write(&bd, &config, &mut file, b"~").unwrap();
        lfs.file_sync(&bd, &config, &mut file).unwrap();
        let path = format!("{}", alphas[j] as char);
        lfs.remove(&bd, &config, &path).unwrap();
    }
    lfs.file_close(&bd, &config, file).unwrap();

    let names = dir_entry_names(&mut lfs, &bd, &config, "/").unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], "zzz");
    let info = lfs.stat(&bd, &config, "zzz").unwrap();
    assert_eq!(info.size, files as u32);

    let mut file = lfs
        .file_open(&bd, &config, "zzz", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    for _ in 0..files {
        let mut buf = [0u8; 1];
        let n = lfs.file_read(&bd, &config, &mut file, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], b'~');
    }
    lfs.file_close(&bd, &config, file).unwrap();
}

#[rstest]
#[case(10)]
#[case(20)]
fn test_interspersed_remove_inconveniently(#[case] size: usize) {
    init_log();
    let (bd, config, mut lfs) = fresh_fs();

    let mut f0 = lfs
        .file_open(
            &bd,
            &config,
            "e",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    let mut f1 = lfs
        .file_open(
            &bd,
            &config,
            "f",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();
    let mut f2 = lfs
        .file_open(
            &bd,
            &config,
            "g",
            OpenFlags::new(OpenFlags::WRONLY | OpenFlags::CREAT),
        )
        .unwrap();

    for _ in 0..size / 2 {
        lfs.file_write(&bd, &config, &mut f0, b"e").unwrap();
        lfs.file_write(&bd, &config, &mut f1, b"f").unwrap();
        lfs.file_write(&bd, &config, &mut f2, b"g").unwrap();
    }
    lfs.remove(&bd, &config, "f").unwrap();
    for _ in 0..size / 2 {
        lfs.file_write(&bd, &config, &mut f0, b"e").unwrap();
        lfs.file_write(&bd, &config, &mut f1, b"f").unwrap();
        lfs.file_write(&bd, &config, &mut f2, b"g").unwrap();
    }
    lfs.file_close(&bd, &config, f0).unwrap();
    lfs.file_close(&bd, &config, f1).unwrap();
    lfs.file_close(&bd, &config, f2).unwrap();

    let names = dir_entry_names(&mut lfs, &bd, &config, "/").unwrap();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"e".to_string()));
    assert!(names.contains(&"g".to_string()));

    let mut f0 = lfs
        .file_open(&bd, &config, "e", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut f1 = lfs
        .file_open(&bd, &config, "g", OpenFlags::new(OpenFlags::RDONLY))
        .unwrap();
    let mut buf = [0u8; 1];
    for _ in 0..size {
        let n = lfs.file_read(&bd, &config, &mut f0, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], b'e');
        let n = lfs.file_read(&bd, &config, &mut f1, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf[0], b'g');
    }
    lfs.file_close(&bd, &config, f0).unwrap();
    lfs.file_close(&bd, &config, f1).unwrap();
}

#[test]
#[ignore = "powerloss runner not implemented"]
fn test_interspersed_reentrant_files() {}
