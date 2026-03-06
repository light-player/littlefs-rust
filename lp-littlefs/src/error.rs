use core::fmt;
use lp_littlefs_core::{
    LFS_ERR_CORRUPT, LFS_ERR_EXIST, LFS_ERR_INVAL, LFS_ERR_IO, LFS_ERR_ISDIR, LFS_ERR_NAMETOOLONG,
    LFS_ERR_NOATTR, LFS_ERR_NOENT, LFS_ERR_NOMEM, LFS_ERR_NOSPC, LFS_ERR_NOTDIR, LFS_ERR_NOTEMPTY,
};

/// LittleFS operation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Io,
    Corrupt,
    NoEntry,
    Exists,
    NotDir,
    IsDir,
    NotEmpty,
    Invalid,
    NoSpace,
    NoMemory,
    NoAttribute,
    NameTooLong,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io => write!(f, "I/O error"),
            Error::Corrupt => write!(f, "filesystem corrupt"),
            Error::NoEntry => write!(f, "no such file or directory"),
            Error::Exists => write!(f, "file or directory already exists"),
            Error::NotDir => write!(f, "not a directory"),
            Error::IsDir => write!(f, "is a directory"),
            Error::NotEmpty => write!(f, "directory not empty"),
            Error::Invalid => write!(f, "invalid parameter"),
            Error::NoSpace => write!(f, "no space left on device"),
            Error::NoMemory => write!(f, "out of memory"),
            Error::NoAttribute => write!(f, "no such attribute"),
            Error::NameTooLong => write!(f, "name too long"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl From<i32> for Error {
    fn from(code: i32) -> Self {
        match code {
            c if c == LFS_ERR_IO => Error::Io,
            c if c == LFS_ERR_CORRUPT => Error::Corrupt,
            c if c == LFS_ERR_NOENT => Error::NoEntry,
            c if c == LFS_ERR_EXIST => Error::Exists,
            c if c == LFS_ERR_NOTDIR => Error::NotDir,
            c if c == LFS_ERR_ISDIR => Error::IsDir,
            c if c == LFS_ERR_NOTEMPTY => Error::NotEmpty,
            c if c == LFS_ERR_INVAL => Error::Invalid,
            c if c == LFS_ERR_NOSPC => Error::NoSpace,
            c if c == LFS_ERR_NOMEM => Error::NoMemory,
            c if c == LFS_ERR_NOATTR => Error::NoAttribute,
            c if c == LFS_ERR_NAMETOOLONG => Error::NameTooLong,
            _ => panic!("unknown LFS error code: {}", code),
        }
    }
}

pub(crate) fn from_lfs_result(code: i32) -> Result<(), Error> {
    if code == 0 {
        Ok(())
    } else {
        Err(Error::from(code))
    }
}

pub(crate) fn from_lfs_size(code: i32) -> Result<u32, Error> {
    if code >= 0 {
        Ok(code as u32)
    } else {
        Err(Error::from(code))
    }
}
