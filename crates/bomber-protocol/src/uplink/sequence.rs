//! The 4-bit wrapping sequence counter carried in the high nibble of byte 1.

/// Number of distinct sequence values.
pub const SEQ_SPACE: u8 = 16;
/// Mask for extracting a sequence value.
pub const SEQ_MASK: u8 = 0x0F;

/// Is `seq` newer than `previous` in wrapping 4-bit space?
///
/// Half the space counts as "ahead". A bot sending one packet per tick wraps
/// every 16 ticks, so anything more than 8 ticks stale is indistinguishable
/// from fresh -- which is harmless, because the server only ever cares about
/// the newest packet inside the current tick's window.
pub fn seq_is_newer(seq: u8, previous: u8) -> bool {
    let diff = seq.wrapping_sub(previous) & SEQ_MASK;
    diff != 0 && diff < SEQ_SPACE / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_wraps_around_the_end_of_the_space() {
        assert!(seq_is_newer(1, 0));
        assert!(seq_is_newer(0, 15), "wrapped");
        assert!(!seq_is_newer(0, 0), "duplicate");
        assert!(!seq_is_newer(15, 0), "stale or reordered");
    }
}
