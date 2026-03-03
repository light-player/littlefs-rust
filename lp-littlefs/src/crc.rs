//! CRC-32. Per lfs_util.c lfs_crc.
//! Polynomial = 0x04c11db7, small lookup table.

const RTABLE: [u32; 16] = [
    0x0000_0000,
    0x1db7_1064,
    0x3b6e_20c8,
    0x26d9_30ac,
    0x76dc_4190,
    0x6b6b_51f4,
    0x4db2_6158,
    0x5005_713c,
    0xedb8_8320,
    0xf00f_9344,
    0xd6d6_a3e8,
    0xcb61_b38c,
    0x9b64_c2b0,
    0x86d3_d2d4,
    0xa00a_e278,
    0xbdbd_f21c,
];

/// Per lfs_util.c lfs_crc
#[inline(always)]
pub fn lfs_crc(crc: u32, buffer: *const u8, size: usize) -> u32 {
    let mut crc = crc;
    let data = buffer;
    unsafe {
        for i in 0..size {
            let byte = *data.add(i);
            let idx = ((crc ^ (byte >> 0) as u32) & 0xf) as usize;
            crc = (crc >> 4) ^ RTABLE[idx];
            let idx = ((crc ^ (byte >> 4) as u32) & 0xf) as usize;
            crc = (crc >> 4) ^ RTABLE[idx];
        }
    }
    crc
}
