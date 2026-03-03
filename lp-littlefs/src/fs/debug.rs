//! Debug helpers for filesystem state inspection.
//!
//! Gated behind `trace` feature. Use `LittleFs::fs_debug_dump` to log root metadata
//! state after operations when debugging.

use crate::error::Error;
use crate::info::FileType;

use super::gstate;
use super::metadata;

/// Dump root dir state and entry list. For debugging. Called by LittleFs::fs_debug_dump.
pub(crate) fn fs_debug_dump_impl(
    dir: &metadata::MdDir,
    name_max: u32,
    gdisk: Option<&gstate::GState>,
) -> Result<::alloc::string::String, Error> {
    let mut out = ::alloc::string::String::new();
    out.push_str(&::alloc::format!(
        "root pair={:?} rev={} off={} count={} tail={:?} split={}\n",
        dir.pair,
        dir.rev,
        dir.off,
        dir.count,
        dir.tail,
        dir.split
    ));

    let start_id = if dir.pair[0] == 0 || dir.pair[0] == 1 {
        1
    } else {
        0
    };
    let mut entries = ::alloc::vec::Vec::new();
    for id in start_id..dir.count {
        match metadata::get_entry_info(dir, id, name_max, gdisk, false) {
            Ok(info) => {
                let name = info
                    .name()
                    .map(|s| ::alloc::string::String::from(s))
                    .unwrap_or_else(|_| ::alloc::string::String::from("<bad name>"));
                let typ = match info.typ {
                    FileType::Reg => "reg",
                    FileType::Dir => "dir",
                };
                entries.push(::alloc::format!("{} ({}): {}", id, typ, name));
            }
            Err(Error::Noent) => {
                entries.push(::alloc::format!("{}: <deleted>", id));
            }
            Err(e) => return Err(e),
        }
    }
    out.push_str("entries: [");
    for (i, e) in entries.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(e);
    }
    out.push_str("]\n");
    Ok(out)
}
