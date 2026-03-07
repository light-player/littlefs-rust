//! Directory operations: mkdir, list, rename, stat, and remove.

use littlefs_rust::{Config, Filesystem, RamStorage};

fn main() {
    let mut storage = RamStorage::new(512, 128);
    let config = Config::new(512, 128);

    Filesystem::format(&mut storage, &config).expect("format failed");
    let fs = Filesystem::mount(storage, config)
        .map_err(|(e, _)| e)
        .expect("mount failed");

    // Build a small directory tree with some files.
    fs.mkdir("/docs").expect("mkdir docs");
    fs.mkdir("/docs/drafts").expect("mkdir drafts");
    fs.write_file("/docs/readme.txt", b"Read me")
        .expect("write readme");
    fs.write_file("/docs/drafts/notes.txt", b"Draft notes")
        .expect("write notes");

    // list_dir returns an iterator of DirEntry with name, type, and size.
    println!("/ contents:");
    for entry in fs.list_dir("/").expect("list /") {
        println!(
            "  {} ({:?}, {} bytes)",
            entry.name, entry.file_type, entry.size
        );
    }

    println!("\n/docs contents:");
    for entry in fs.list_dir("/docs").expect("list /docs") {
        println!(
            "  {} ({:?}, {} bytes)",
            entry.name, entry.file_type, entry.size
        );
    }

    // Rename moves or renames a file/directory atomically.
    fs.rename("/docs/readme.txt", "/docs/README.txt")
        .expect("rename");
    println!("\nAfter rename, /docs contains:");
    for entry in fs.list_dir("/docs").expect("list /docs") {
        println!("  {}", entry.name);
    }

    // stat retrieves metadata without opening the file.
    let meta = fs.stat("/docs/README.txt").expect("stat");
    println!(
        "\n/docs/README.txt: {:?}, {} bytes",
        meta.file_type, meta.size
    );

    // Directories must be empty before they can be removed.
    fs.remove("/docs/drafts/notes.txt").expect("remove file");
    fs.remove("/docs/drafts").expect("remove dir");
    println!("\nAfter removing drafts, /docs contains:");
    for entry in fs.list_dir("/docs").expect("list /docs") {
        println!("  {}", entry.name);
    }

    // fs_size reports how many blocks are currently in use.
    let size = fs.fs_size().expect("fs_size");
    println!("\nFilesystem uses {} blocks", size);

    fs.unmount().expect("unmount");
}
