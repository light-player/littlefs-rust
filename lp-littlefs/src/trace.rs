//! Trace macros for debugging. No-op when the `trace` feature is disabled.

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
