//! Debug dump: raw blocks with parsed interpretation.
//!
//! Use `dump_fs(&ram.data, block_size, block_count)` to inspect filesystem
//! state. Outputs hex + structured parse to stderr.

use std::io::Write;

const MAGIC: &[u8; 8] = b"littlefs";

/// Parse u32 little-endian from slice.
fn u32_le(b: &[u8]) -> u32 {
    let a: [u8; 4] = b[..4].try_into().unwrap();
    u32::from_le_bytes(a)
}

/// Parse u32 big-endian.
fn u32_be(b: &[u8]) -> u32 {
    let a: [u8; 4] = b[..4].try_into().unwrap();
    u32::from_be_bytes(a)
}

/// Tag type names for human-readable dump.
fn tag_type_name(type3: u32) -> &'static str {
    match type3 {
        0x001 => "REG",
        0x002 => "DIR",
        0x000 => "NAME",
        0x200 => "STRUCT",
        0x201 => "INLINESTRUCT",
        0x202 => "CTZSTRUCT",
        0x0ff => "SUPERBLOCK",
        0x401 => "CREATE",
        0x4ff => "DELETE",
        0x400 => "SPLICE",
        0x600 => "SOFTTAIL",
        0x601 => "HARDTAIL",
        0x500 => "CCRC",
        0x5ff => "FCRC",
        0x7ff => "MOVESTATE",
        _ => "?",
    }
}

/// Dump a single block: hex on left, parsed hints on right.
pub fn dump_block(block: &[u8], block_id: u32, block_size: u32, out: &mut impl Write) {
    let _ = writeln!(out, "\n=== Block {} ({} bytes) ===", block_id, block_size);
    let len = block_size as usize;

    // Rev at 0
    if block.len() >= 4 {
        let rev = u32_le(block);
        let _ = writeln!(out, "  [0..4]   rev = {} (0x{:08x})", rev, rev);
    }

    // Magic at 8 or 12
    if block.len() >= 20 {
        let s8 = std::str::from_utf8(&block[8..16]).unwrap_or("?");
        let s12 = std::str::from_utf8(&block[12..20]).unwrap_or("?");
        if s8 == "littlefs" || s12 == "littlefs" {
            let _ = writeln!(out, "  magic 'littlefs' at 8 or 12");
        }
    }

    // Hex dump with byte offsets
    for (i, chunk) in block[..len.min(block.len())].chunks(16).enumerate() {
        let hex: String = chunk
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        let _ = writeln!(out, "  {:04x}  {}  |{}|", i * 16, hex, ascii);
    }

    // Try to parse as directory commit: tags are XOR chain, first tag XOR 0xffffffff
    let mut off = 4u32;
    let mut ptag: u32 = 0xffffffff;
    let mut tag_count = 0;
    while off + 4 <= len as u32 && tag_count < 20 {
        let tag_raw = u32_be(&block[off as usize..]);
        let tag = tag_raw ^ ptag;
        let valid = (tag & 0x8000_0000) == 0;
        if !valid {
            let _ = writeln!(out, "  [tag chain end at off {}]", off);
            break;
        }
        let type3 = (tag >> 20) & 0x7ff;
        let id = (tag >> 10) & 0x3ff;
        let size = tag & 0x3ff;
        let dsize = 4 + size;
        if off + dsize > len as u32 {
            break;
        }
        let _ = writeln!(
            out,
            "  [{}] off={:4} tag=0x{:08x} {} id={} size={}",
            tag_count,
            off,
            tag,
            tag_type_name(type3),
            id,
            size
        );
        if type3 == 0x000 {
            // NAME: next `size` bytes are the name
            let name_start = (off + 4) as usize;
            let name_end = (name_start + size as usize).min(block.len());
            let name = &block[name_start..name_end];
            let s = std::str::from_utf8(name).unwrap_or("?");
            let _ = writeln!(out, "      name = \"{}\"", s.escape_default());
        }
        if type3 == 0x600 || type3 == 0x601 {
            // TAIL: 8 bytes = pair[2]
            if off + 12 <= len as u32 {
                let t0 = u32_le(&block[(off + 4) as usize..]);
                let t1 = u32_le(&block[(off + 8) as usize..]);
                let _ = writeln!(out, "      tail = [{}, {}]", t0, t1);
            }
        }
        if type3 == 0x201 {
            // INLINESTRUCT: can be CTZ head/size (8 bytes) or inline data
            if size >= 8 && off + 12 <= len as u32 {
                let v0 = u32_le(&block[(off + 4) as usize..]);
                let v1 = u32_le(&block[(off + 8) as usize..]);
                let _ = writeln!(out, "      struct = [{}, {}] (ctz head/size?)", v0, v1);
            }
        }
        ptag = tag;
        off += dsize;
        tag_count += 1;
    }
}

/// Dump full filesystem: all blocks with hex + parse.
pub fn dump_fs(data: &[u8], block_size: u32, block_count: u32) {
    let mut out = std::io::stderr().lock();
    let _ = writeln!(
        out,
        "\n========== LITTLEFS DUMP ({} blocks x {} bytes) ==========",
        block_count, block_size
    );
    for b in 0..block_count {
        let base = (b as usize) * (block_size as usize);
        if base + block_size as usize <= data.len() {
            let block = &data[base..base + block_size as usize];
            dump_block(block, b, block_size, &mut out);
        }
    }
    let _ = writeln!(out, "\n========== END DUMP ==========\n");
}
