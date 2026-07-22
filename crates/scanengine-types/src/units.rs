use serde::{Deserialize, Serialize};

use crate::error::InvalidTick;

/// Maximum length of an instrument identifier in bytes.
pub const MAX_INSTRUMENT_LEN: usize = 64;

/// A validated instrument identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct InstrumentId(String);

impl InstrumentId {
    /// Create a validated instrument identifier.
    ///
    /// # Errors
    /// Returns [`InvalidTick::EmptyInstrument`] or
    /// [`InvalidTick::InstrumentTooLong`] on failure.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidTick> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(InvalidTick::EmptyInstrument);
        }
        if raw.len() > MAX_INSTRUMENT_LEN {
            return Err(InvalidTick::InstrumentTooLong(raw.len()));
        }
        Ok(Self(raw))
    }

    /// Borrow the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for InstrumentId {
    type Error = InvalidTick;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<InstrumentId> for String {
    fn from(value: InstrumentId) -> Self {
        value.0
    }
}

impl std::fmt::Display for InstrumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A non-negative integer price expressed in exchange ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Price(i64);

impl Price {
    /// Create a non-negative price.
    ///
    /// # Errors
    /// Returns [`InvalidTick::NegativePrice`] when `ticks < 0`.
    pub const fn new(ticks: i64) -> Result<Self, InvalidTick> {
        if ticks < 0 {
            return Err(InvalidTick::NegativePrice(ticks));
        }
        Ok(Self(ticks))
    }

    /// The price value in ticks.
    #[must_use]
    pub const fn ticks(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrument_validation() {
        assert!(InstrumentId::new("").is_err());
        assert!(InstrumentId::new("x".repeat(65)).is_err());
        assert_eq!(InstrumentId::new("NSE:TCS").unwrap().as_str(), "NSE:TCS");
    }

    #[test]
    fn price_rejects_negative() {
        assert!(Price::new(-1).is_err());
        assert_eq!(Price::new(10).unwrap().ticks(), 10);
    }

    #[test]
    fn instrument_serde_roundtrip() {
        let id = InstrumentId::new("NSE:INFY").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let back: InstrumentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
