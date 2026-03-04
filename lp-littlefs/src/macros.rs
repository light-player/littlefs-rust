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
