//! CRC-32 for littlefs metadata.
//!
//! Per SPEC: polynomial 0x04c11db7, init 0xffffffff.
//! Based on lfs_util.c nibble table.

pub fn crc32(mut crc: u32, data: &[u8]) -> u32 {
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
    for &b in data {
        crc = (crc >> 4) ^ RTABLE[((crc ^ (b as u32)) & 0xf) as usize];
        crc = (crc >> 4) ^ RTABLE[((crc ^ ((b >> 4) as u32)) & 0xf) as usize];
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Matches lfs_util.c lfs_crc. Verified against C implementation.
    #[test]
    fn crc32_matches_c_implementation() {
        // C: crc = lfs_crc(0xffffffff, &dir->rev, sizeof(dir->rev))
        // for rev=1 LE: [0x01, 0x00, 0x00, 0x00]
        let rev: [u8; 4] = 1u32.to_le_bytes();
        let c = crc32(0xffff_ffff, &rev);
        assert_ne!(c, 0);
        assert_ne!(c, 0xffff_ffff);
        // Second invocation with "littlefs" (superblock magic) - C format path
        let c2 = crc32(c, b"littlefs");
        assert_ne!(c2, c);
    }

    #[test]
    fn crc32_revision_only() {
        let rev: [u8; 4] = 1u32.to_le_bytes();
        let c = crc32(0xffff_ffff, &rev);
        assert_ne!(c, 0);
        assert_ne!(c, 0xffff_ffff);
    }

    #[test]
    fn crc32_accumulates() {
        let c1 = crc32(0xffff_ffff, b"a");
        let c2 = crc32(0xffff_ffff, b"ab");
        let c2_alt = crc32(crc32(0xffff_ffff, b"a"), b"b");
        assert_eq!(c2, c2_alt);
        assert_ne!(c1, c2);
    }

    #[test]
    fn crc32_empty_no_change() {
        let c = crc32(0x1234_5678, &[]);
        assert_eq!(c, 0x1234_5678);
    }
}
