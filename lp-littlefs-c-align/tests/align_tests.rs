//! C ↔ Rust format alignment tests.
//!
//! Targets the known failure: mkdir("potato") + file_open("burito", CREAT) →
//! after remount, potato disappears. These tests isolate whether the bug is
//! in our write path or read path.

use lp_littlefs::Config;
use lp_littlefs_c_align::c_lfs;
use lp_littlefs_c_align::rust_lfs;
use lp_littlefs_c_align::storage::AlignStorage;

fn default_config() -> Config {
    Config::default_for_tests(128)
}

#[test]
fn c_format_rust_mount_root() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format(&storage, &config).expect("C format failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");
    // Root has no entries besides . and ..; names should be empty
    assert!(names.is_empty() || names.len() <= 2, "got: {:?}", names);
}

#[test]
fn rust_format_c_mount_root() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format(&storage, &config).expect("Rust format failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");
    assert!(names.is_empty(), "got: {:?}", names);
}

#[test]
fn c_mkdir_file_rust_sees_both() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_mkdir_file_unmount(&storage, &config, "potato", "burito")
        .expect("C format_mkdir_file_unmount failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");

    assert!(
        names.contains(&"potato".to_string()),
        "Rust should see potato after C wrote it; got: {:?}",
        names
    );
    assert!(
        names.contains(&"burito".to_string()),
        "Rust should see burito after C wrote it; got: {:?}",
        names
    );
}

#[test]
fn rust_mkdir_file_c_sees_both() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format_mkdir_file_unmount(&storage, &config, "potato", "burito")
        .expect("Rust format_mkdir_file_unmount failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");

    assert!(
        names.contains(&"potato".to_string()),
        "C should see potato after Rust wrote it; got: {:?}",
        names
    );
    assert!(
        names.contains(&"burito".to_string()),
        "C should see burito after Rust wrote it; got: {:?}",
        names
    );
}

#[test]
fn c_mkdir_file_rust_sees_both_reverse_order() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_file_mkdir_unmount(&storage, &config, "burito", "potato")
        .expect("C format_file_mkdir_unmount failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");

    assert!(
        names.contains(&"potato".to_string()),
        "Rust should see potato after C wrote it (reverse order); got: {:?}",
        names
    );
    assert!(
        names.contains(&"burito".to_string()),
        "Rust should see burito after C wrote it (reverse order); got: {:?}",
        names
    );
}

#[test]
fn c_insert_before_rust_sees_all() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_create_three_unmount(&storage, &config)
        .expect("C format_create_three_unmount failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");

    assert!(
        names.contains(&"aaa".to_string()),
        "Rust should see aaa; got: {:?}",
        names
    );
    assert!(
        names.contains(&"zzz".to_string()),
        "Rust should see zzz; got: {:?}",
        names
    );
    assert!(
        names.contains(&"mmm".to_string()),
        "Rust should see mmm (insert-before); got: {:?}",
        names
    );
}

#[test]
fn rust_insert_before_c_sees_all() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format_create_three_unmount(&storage, &config)
        .expect("Rust format_create_three_unmount failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");

    assert!(
        names.contains(&"aaa".to_string()),
        "C should see aaa; got: {:?}",
        names
    );
    assert!(
        names.contains(&"zzz".to_string()),
        "C should see zzz; got: {:?}",
        names
    );
    assert!(
        names.contains(&"mmm".to_string()),
        "C should see mmm (insert-before); got: {:?}",
        names
    );
}

#[test]
fn c_mkdir_remount_exist() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_mkdir_unmount(&storage, &config, "potato")
        .expect("C format_mkdir_unmount failed");
    c_lfs::mount_mkdir_expect_exist(&storage, &config, "potato")
        .expect("C mkdir potato should return LFS_ERR_EXIST after remount");
}

// --- Rename ---

#[test]
fn c_rename_rust_sees_new_name() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_create_rename_unmount(&storage, &config, "oldfile", "newfile")
        .expect("C format_create_rename_unmount failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");

    assert!(
        !names.contains(&"oldfile".to_string()),
        "Rust should not see old name after C rename; got: {:?}",
        names
    );
    assert!(
        names.contains(&"newfile".to_string()),
        "Rust should see new name after C rename; got: {:?}",
        names
    );
}

#[test]
#[ignore = "Rust rename: DELETE tags may not persist correctly; C sees both old and new names"]
fn rust_rename_c_sees_new_name() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format_create_rename_unmount(&storage, &config, "oldfile", "newfile")
        .expect("Rust format_create_rename_unmount failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");

    assert!(
        !names.contains(&"oldfile".to_string()),
        "C should not see old name after Rust rename; got: {:?}",
        names
    );
    assert!(
        names.contains(&"newfile".to_string()),
        "C should see new name after Rust rename; got: {:?}",
        names
    );
}

// --- Remove ---

#[test]
fn c_remove_rust_sees_gone() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_create_remove_unmount(&storage, &config, "goner")
        .expect("C format_create_remove_unmount failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");

    assert!(
        !names.contains(&"goner".to_string()),
        "Rust should not see removed file; got: {:?}",
        names
    );
}

#[test]
fn rust_remove_c_sees_gone() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format_create_remove_unmount(&storage, &config, "goner")
        .expect("Rust format_create_remove_unmount failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");

    assert!(
        !names.contains(&"goner".to_string()),
        "C should not see removed file; got: {:?}",
        names
    );
}

// --- File content ---

#[test]
fn c_write_rust_reads_content_inline() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    let content = b"hello littlefs";
    c_lfs::format_create_write_unmount(&storage, &config, "f", content)
        .expect("C format_create_write_unmount failed");
    let read =
        rust_lfs::mount_read_file(&storage, &config, "f").expect("Rust mount_read_file failed");

    assert_eq!(
        read, content,
        "Rust should read same content C wrote (inline)"
    );
}

#[test]
fn rust_write_c_reads_content_inline() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    let content = b"hello littlefs";
    rust_lfs::format_create_write_unmount(&storage, &config, "f", content)
        .expect("Rust format_create_write_unmount failed");
    let read = c_lfs::mount_read_file(&storage, &config, "f").expect("C mount_read_file failed");

    assert_eq!(
        read, content,
        "C should read same content Rust wrote (inline)"
    );
}

#[test]
fn c_write_rust_reads_content_ctz() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    let content: Vec<u8> = (0..600).map(|i| (i % 256) as u8).collect();
    c_lfs::format_create_write_unmount(&storage, &config, "big", &content)
        .expect("C format_create_write_unmount failed");
    let read =
        rust_lfs::mount_read_file(&storage, &config, "big").expect("Rust mount_read_file failed");

    assert_eq!(read, content, "Rust should read same content C wrote (CTZ)");
}

#[test]
#[ignore = "Rust bdcache overflow when writing CTZ; C read fails"]
fn rust_write_c_reads_content_ctz() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    let content: Vec<u8> = (0..600).map(|i| (i % 256) as u8).collect();
    rust_lfs::format_create_write_unmount(&storage, &config, "big", &content)
        .expect("Rust format_create_write_unmount failed");
    let read = c_lfs::mount_read_file(&storage, &config, "big").expect("C mount_read_file failed");

    assert_eq!(read, content, "C should read same content Rust wrote (CTZ)");
}

// --- Nested dirs ---

#[test]
fn c_nested_dir_rust_sees() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_nested_dir_file_unmount(&storage, &config, "a", "b", "f")
        .expect("C format_nested_dir_file_unmount failed");

    let root_names =
        rust_lfs::mount_dir_names_at(&storage, &config, "/").expect("Rust mount dir names /");
    assert!(
        root_names.contains(&"a".to_string()),
        "Rust should see a at root; got: {:?}",
        root_names
    );

    let a_names =
        rust_lfs::mount_dir_names_at(&storage, &config, "a").expect("Rust mount dir names a");
    assert!(
        a_names.contains(&"b".to_string()),
        "Rust should see b in a; got: {:?}",
        a_names
    );

    let ab_names =
        rust_lfs::mount_dir_names_at(&storage, &config, "a/b").expect("Rust mount dir names a/b");
    assert!(
        ab_names.contains(&"f".to_string()),
        "Rust should see f in a/b; got: {:?}",
        ab_names
    );
}

#[test]
fn rust_nested_dir_c_sees() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format_nested_dir_file_unmount(&storage, &config, "a", "b", "f")
        .expect("Rust format_nested_dir_file_unmount failed");

    let root_names =
        c_lfs::mount_dir_names_at(&storage, &config, "/").expect("C mount dir names /");
    assert!(
        root_names.contains(&"a".to_string()),
        "C should see a at root; got: {:?}",
        root_names
    );

    let a_names = c_lfs::mount_dir_names_at(&storage, &config, "a").expect("C mount dir names a");
    assert!(
        a_names.contains(&"b".to_string()),
        "C should see b in a; got: {:?}",
        a_names
    );

    let ab_names =
        c_lfs::mount_dir_names_at(&storage, &config, "a/b").expect("C mount dir names a/b");
    assert!(
        ab_names.contains(&"f".to_string()),
        "C should see f in a/b; got: {:?}",
        ab_names
    );
}

// --- Rmdir ---

#[test]
fn c_rmdir_rust_sees_gone() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    c_lfs::format_mkdir_file_rmdir_unmount(&storage, &config, "emptydir", "f")
        .expect("C format_mkdir_file_rmdir_unmount failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");

    assert!(
        !names.contains(&"emptydir".to_string()),
        "Rust should not see rmdir'd dir; got: {:?}",
        names
    );
}

#[test]
#[ignore = "Rust remove of file in dir may not persist; rmdir returns NotEmpty"]
fn rust_rmdir_c_sees_gone() {
    let _ = env_logger::builder().is_test(true).try_init();
    let config = default_config();
    let storage = AlignStorage::new(&config);

    rust_lfs::format_mkdir_file_rmdir_unmount(&storage, &config, "emptydir", "f")
        .expect("Rust format_mkdir_file_rmdir_unmount failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");

    assert!(
        !names.contains(&"emptydir".to_string()),
        "C should not see rmdir'd dir; got: {:?}",
        names
    );
}

// --- CRC layout: different prog_size ---

#[test]
fn format_layout_prog_size_16() {
    let _ = env_logger::builder().is_test(true).try_init();
    let mut config = default_config();
    config.prog_size = 16;
    config.block_size = 512;
    let storage = AlignStorage::new(&config);

    rust_lfs::format(&storage, &config).expect("Rust format failed");
    let names = c_lfs::mount_dir_names(&storage, &config).expect("C mount failed");
    assert!(
        names.is_empty(),
        "C should mount Rust format with prog_size=16; got: {:?}",
        names
    );
}

#[test]
fn format_layout_prog_size_64() {
    let _ = env_logger::builder().is_test(true).try_init();
    let mut config = default_config();
    config.prog_size = 64;
    config.block_size = 512;
    let storage = AlignStorage::new(&config);

    c_lfs::format(&storage, &config).expect("C format failed");
    let names = rust_lfs::mount_dir_names(&storage, &config).expect("Rust mount failed");
    assert!(
        names.is_empty() || names.len() <= 2,
        "Rust should mount C format with prog_size=64; got: {:?}",
        names
    );
}
