#[macro_export]
macro_rules! g_info {
    ($id:expr, $($arg:tt)*) => {
        info!("[{}] {}", $id, format!($($arg)*))
    };
}

#[macro_export]
macro_rules! g_warn {
    ($id:expr, $($arg:tt)*) => {
        warn!("[{}] {}", $id, format!($($arg)*))
    };
}
