//! Trace macros for debugging. No-op when the `trace` feature is disabled.
//!
//! - `trace!`: General tracing. Enable with `--features trace`.
//! - `trace_cache!`: Extra cache read/prog traces. Enable with `--features trace,trace_cache`.

#[macro_export]
#[cfg(feature = "trace")]
macro_rules! trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[macro_export]
#[cfg(not(feature = "trace"))]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

#[macro_export]
#[cfg(all(feature = "trace", feature = "trace_cache"))]
macro_rules! trace_cache {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[macro_export]
#[cfg(not(all(feature = "trace", feature = "trace_cache")))]
macro_rules! trace_cache {
    ($($arg:tt)*) => {};
}
