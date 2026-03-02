//! Error types for lp_littlefs.
//!
//! Maps to littlefs error codes (lfs.h enum lfs_error).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// No error
    Ok,
    /// Error during device operation (I/O)
    Io,
    /// Corrupted filesystem
    Corrupt,
    /// No directory entry
    Noent,
    /// Entry already exists
    Exist,
    /// Entry is not a directory
    NotDir,
    /// Entry is a directory
    IsDir,
    /// Directory is not empty
    NotEmpty,
    /// Bad file number
    Badf,
    /// File too large
    Fbig,
    /// Invalid parameter
    Inval,
    /// No space left on device
    Nospc,
    /// No more memory available
    Nomem,
    /// No data/attr available
    Noattr,
    /// File name too long
    Nametoolong,
}
