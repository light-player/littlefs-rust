use lp_littlefs::{Config, Filesystem, RamStorage};

fn main() {
    let block_size = 512;
    let block_count = 128;
    let mut storage = RamStorage::new(block_size, block_count);
    let config = Config::new(block_size, block_count);

    Filesystem::format(&mut storage, &config).expect("format failed");
    let fs = Filesystem::mount(storage, config).expect("mount failed");

    fs.write_file("/hello.txt", b"Hello, littlefs!")
        .expect("write failed");

    let data = fs.read_to_vec("/hello.txt").expect("read failed");
    println!("{}", core::str::from_utf8(&data).unwrap());

    let storage = fs.unmount().expect("unmount failed");
    println!(
        "Storage: {} blocks x {} bytes = {} bytes total",
        storage.block_count(),
        storage.block_size(),
        storage.data().len()
    );
}
