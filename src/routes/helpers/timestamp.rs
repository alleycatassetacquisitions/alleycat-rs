use jiff::Timestamp;

/// Devices report UTC milliseconds since the Unix epoch.
pub(crate) fn from_device_millis(ms: i64) -> Option<Timestamp> {
    if ms < 0 {
        return None;
    }
    // Jiff's millisecond constructor stops at the start of its last second.
    Timestamp::new(ms / 1000, ((ms % 1000) * 1_000_000) as i32).ok()
}
