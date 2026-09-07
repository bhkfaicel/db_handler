use serde::{Deserialize, Deserializer};

/// MAC address format validation shared by every trust boundary.
///
/// Accepted format: six hexadecimal pairs separated by `:`
/// (case-insensitive), e.g. `AA:BB:CC:DD:EE:01` or `aa:bb:cc:dd:ee:01`.
/// Anything else (wrong pair count, wrong pair length, non-hex
/// characters, any other delimiter) is rejected so a malformed or
/// line-protocol-hostile address can never enter the system.
pub fn is_valid(mac: &str) -> bool {
    // Character-range based via the stdlib hex-digit check rather than
    // `u8::from_str_radix`, whose parser accepts a leading `+`/`-` sign
    // that is not part of a valid hexadecimal pair.
    // Split once, then require exactly six two-character hex pairs.
    let pairs: Vec<&str> = mac.split(':').collect();
    pairs.len() == 6
        && pairs
            .iter()
            .all(|pair| pair.len() == 2 && pair.chars().all(|c: char| c.is_ascii_hexdigit()))
}

/// Serde `deserialize_with` helper for MAC string fields.
///
/// Constructor checks alone are bypassable because the models carrying a
/// MAC have public fields and a derived `Deserialize`; attaching this
/// helper to those fields keeps the format invariant enforced on every
/// deserialization path as well.
pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    // Read the raw string, then reuse the very same validator the
    // constructors use, so both paths enforce one shared format rule.
    let mac = String::deserialize(deserializer)?;
    if is_valid(&mac) {
        Ok(mac)
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid MAC address format: {mac}"
        )))
    }
}
