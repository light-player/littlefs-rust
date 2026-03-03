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
