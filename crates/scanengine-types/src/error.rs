use thiserror::Error;

/// Errors produced while validating an inbound tick.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvalidTick {
    /// The instrument identifier was empty.
    #[error("instrument identifier must not be empty")]
    EmptyInstrument,
    /// The instrument identifier exceeded the maximum length.
    #[error("instrument identifier too long: {0} bytes")]
    InstrumentTooLong(usize),
    /// A negative price was supplied.
    #[error("price must be non-negative, got {0}")]
    NegativePrice(i64),
}

/// Errors produced while validating a rule.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvalidRule {
    /// The rule had no conditions.
    #[error("rule must have at least one condition")]
    NoConditions,
    /// The rule name was empty.
    #[error("rule name must not be empty")]
    EmptyName,
}
