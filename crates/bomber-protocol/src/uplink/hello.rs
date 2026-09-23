use crate::codec::{ProtoError, Result};

use super::packet::HELLO_PLAYER_ID;

/// The join packet, which may carry a name.
///
/// ```text
/// byte 0:      0xFF
/// byte 1:      0xFF
/// byte 2:      name length in bytes, 0 for none   (optional)
/// bytes 3..:   the name, UTF-8                    (optional)
/// ```
///
/// The two-byte rule governs the **per-tick action** channel, which is sent 60
/// times a second and is where the bytes actually matter. A join happens once,
/// so it can afford to be longer -- and a bare `[0xFF, 0xFF]` is still a valid
/// hello, so bots written before this existed keep working and simply get the
/// default name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hello {
    pub name: Option<String>,
}

impl Hello {
    /// Longest name accepted, in bytes. A name is shown in a UI and stored per
    /// seat, so it is bounded rather than trusted.
    pub const MAX_NAME_BYTES: usize = 24;

    pub fn anonymous() -> Self {
        Hello { name: None }
    }

    pub fn named(name: impl Into<String>) -> Self {
        Hello {
            name: Some(name.into()),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![HELLO_PLAYER_ID, 0xFF];
        if let Some(name) = &self.name {
            let bytes = truncate_on_char_boundary(name, Self::MAX_NAME_BYTES).as_bytes();
            out.push(bytes.len() as u8);
            out.extend_from_slice(bytes);
        }
        out
    }

    /// Read a hello. The caller has already established that this *is* one.
    ///
    /// An over-long name is truncated rather than refused: a name is a
    /// courtesy, and turning a bot away over one would be a poor trade.
    /// Bytes that are not valid UTF-8 are an error, so the server can say so
    /// in its log -- the bot has no channel to be told on.
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < 2 {
            return Err(ProtoError::Truncated {
                at: buf.len(),
                need: 2 - buf.len(),
            });
        }
        if buf.len() == 2 {
            return Ok(Hello::anonymous());
        }

        let declared = buf[2] as usize;
        if declared == 0 {
            return Ok(Hello::anonymous());
        }
        let available = buf.len() - 3;
        if available < declared {
            return Err(ProtoError::Truncated {
                at: 3 + available,
                need: declared - available,
            });
        }

        let raw = &buf[3..3 + declared];
        let name = std::str::from_utf8(raw)
            .map_err(|_| ProtoError::invalid("hello_name", raw.len() as u32))?;

        Ok(Hello {
            name: Some(truncate_on_char_boundary(name, Self::MAX_NAME_BYTES).to_string()),
        })
    }
}

/// Cut to at most `limit` bytes without splitting a character in half.
fn truncate_on_char_boundary(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_hello_still_works() {
        assert_eq!(Hello::decode(&[0xFF, 0xFF]), Ok(Hello::anonymous()));
        assert_eq!(Hello::anonymous().encode(), vec![0xFF, 0xFF]);
    }

    #[test]
    fn a_named_hello_round_trips() {
        let hello = Hello::named("team-rocket");
        assert_eq!(Hello::decode(&hello.encode()), Ok(hello));
    }

    #[test]
    fn a_zero_length_name_is_the_same_as_none() {
        assert_eq!(Hello::decode(&[0xFF, 0xFF, 0]), Ok(Hello::anonymous()));
    }

    #[test]
    fn an_over_long_name_is_truncated_not_refused() {
        let long = "x".repeat(200);
        let decoded = Hello::decode(&Hello::named(&long).encode()).unwrap();
        assert_eq!(decoded.name.unwrap().len(), Hello::MAX_NAME_BYTES);
    }

    /// Truncation must not leave half a character behind.
    #[test]
    fn truncation_respects_character_boundaries() {
        // Four-byte characters: the cut lands mid-character unless handled.
        let emoji = "🙂".repeat(20);
        let decoded = Hello::decode(&Hello::named(&emoji).encode()).unwrap();
        let name = decoded.name.unwrap();
        assert!(name.len() <= Hello::MAX_NAME_BYTES);
        assert_eq!(name.chars().count(), Hello::MAX_NAME_BYTES / 4);
    }

    #[test]
    fn a_name_that_is_not_utf8_is_reported() {
        let packet = [0xFF, 0xFF, 2, 0xC3, 0x28];
        assert!(Hello::decode(&packet).is_err());
    }

    #[test]
    fn a_name_shorter_than_it_claims_is_truncation() {
        assert!(matches!(
            Hello::decode(&[0xFF, 0xFF, 10, b'a', b'b']),
            Err(ProtoError::Truncated { .. })
        ));
    }

    /// Anything can arrive on a UDP port; none of it may panic.
    #[test]
    fn arbitrary_bytes_never_panic() {
        for len in 0..40usize {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37) as u8).collect();
            let _ = Hello::decode(&bytes);
        }
    }
}
