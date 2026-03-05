//! LittleFS macros: assertions (debug_assert) and logging (log crate when feature enabled).

/// Assert condition; active in debug builds, stripped in release.
#[macro_export]
macro_rules! lfs_assert {
    ($cond:expr) => {
        debug_assert!($cond)
    };
    ($cond:expr, $($arg:tt)+) => {
        debug_assert!($cond, $($arg)+)
    };
}

/// Debug-level log. No-op when `log` feature is disabled.
#[cfg(feature = "log")]
#[macro_export]
macro_rules! lfs_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[cfg(not(feature = "log"))]
#[macro_export]
macro_rules! lfs_debug {
    ($($_:tt)*) => {};
}

/// Error-level log. No-op when `log` feature is disabled.
#[cfg(feature = "log")]
#[macro_export]
macro_rules! lfs_error {
    ($($arg:tt)*) => {
        log::error!($($arg)*)
    };
}

#[cfg(not(feature = "log"))]
#[macro_export]
macro_rules! lfs_error {
    ($($_:tt)*) => {};
}

/// Trace-level log. No-op when `log` feature is disabled.
#[cfg(feature = "log")]
#[macro_export]
macro_rules! lfs_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[cfg(not(feature = "log"))]
#[macro_export]
macro_rules! lfs_trace {
    ($($_:tt)*) => {};
}

/// Produce an error value for return. When `log` feature is enabled, logs at trace level
/// (file, line, error code) before producing the value. Use as `return lfs_err!(LFS_ERR_NOSPC)`
/// or with context: `return lfs_err!(LFS_ERR_NOSPC, "off={} end={}", off, end)`.
#[cfg(feature = "log")]
#[macro_export]
macro_rules! lfs_err {
    ($e:expr) => {{
        $crate::lfs_trace!("lfs_err {} at {}:{}", $e, file!(), line!());
        $e
    }};
    ($e:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::lfs_trace!(concat!("lfs_err {} at ", file!(), ":", line!(), " ", $fmt), $e, $($arg)*);
        $e
    }};
}

#[cfg(not(feature = "log"))]
#[macro_export]
macro_rules! lfs_err {
    ($e:expr) => {
        $e
    };
    ($e:expr, $($arg:tt)+) => {
        $e
    };
}

/// Propagate an error value on return. When `log` feature is enabled, logs at trace level
/// (file, line, error code) so call chain is visible. Use as `return lfs_pass_err!(err)` or
/// with context: `return lfs_pass_err!(err, "after lfs_dir_commit")`.
#[cfg(feature = "log")]
#[macro_export]
macro_rules! lfs_pass_err {
    ($e:expr) => {{
        $crate::lfs_trace!("lfs_pass_err {} at {}:{}", $e, file!(), line!());
        $e
    }};
    ($e:expr, $fmt:expr, $($arg:tt)*) => {{
        $crate::lfs_trace!(concat!("lfs_pass_err {} at ", file!(), ":", line!(), " ", $fmt), $e, $($arg)*);
        $e
    }};
}

#[cfg(not(feature = "log"))]
#[macro_export]
macro_rules! lfs_pass_err {
    ($e:expr) => {
        $e
    };
    ($e:expr, $($arg:tt)+) => {
        $e
    };
}
