// SPDX-License-Identifier: Apache-2.0

//! Newtype wrappers for resource limits
//!
//! These types enforce validation and provide type safety for various
//! resource limits in the WACC VM.

use std::fmt;

/// Amount of fuel for WASM execution
///
/// Fuel is used to limit the computational resources consumed by WASM code.
/// This newtype ensures fuel amounts are always valid and provides a clear
/// semantic meaning.
///
/// # Examples
///
/// ```
/// # use wacc::types::FuelAmount;
/// let fuel = FuelAmount::new(1_000_000);
/// assert_eq!(fuel.as_u64(), 1_000_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FuelAmount(u64);

impl FuelAmount {
    /// Default fuel amount for VM execution (1 million)
    pub const DEFAULT: u64 = 1_000_000;

    /// Minimum recommended fuel amount
    pub const MIN_RECOMMENDED: u64 = 10_000;

    /// Maximum fuel amount (prevents resource exhaustion)
    pub const MAX: u64 = 100_000_000;

    /// Creates a new fuel amount
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::FuelAmount;
    /// let fuel = FuelAmount::new(500_000);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(amount: u64) -> Self {
        Self(amount)
    }

    /// Creates a new fuel amount with validation
    ///
    /// Returns `None` if the amount is zero or exceeds `MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::FuelAmount;
    /// assert!(FuelAmount::new_checked(1000).is_some());
    /// assert!(FuelAmount::new_checked(0).is_none());
    /// assert!(FuelAmount::new_checked(FuelAmount::MAX + 1).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new_checked(amount: u64) -> Option<Self> {
        if amount > 0 && amount <= Self::MAX {
            Some(Self(amount))
        } else {
            None
        }
    }

    /// Creates a fuel amount with the default value
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::FuelAmount;
    /// let fuel = FuelAmount::default();
    /// assert_eq!(fuel.as_u64(), FuelAmount::DEFAULT);
    /// ```
    #[inline]
    #[must_use]
    pub const fn default() -> Self {
        Self(Self::DEFAULT)
    }

    /// Returns the fuel amount as a u64
    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Attempts to subtract fuel, returning None on underflow
    #[inline]
    #[must_use]
    pub const fn checked_sub(self, amount: u64) -> Option<Self> {
        match self.0.checked_sub(amount) {
            Some(remaining) => Some(Self(remaining)),
            None => None,
        }
    }

    /// Checks if the fuel is exhausted (zero)
    #[inline]
    #[must_use]
    pub const fn is_exhausted(self) -> bool {
        self.0 == 0
    }
}

impl Default for FuelAmount {
    fn default() -> Self {
        Self::default()
    }
}

impl From<u64> for FuelAmount {
    fn from(amount: u64) -> Self {
        Self::new(amount)
    }
}

impl fmt::Display for FuelAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} fuel", self.0)
    }
}

/// Counter for check operations in a WACC script
///
/// This newtype tracks how many cryptographic check operations have been
/// performed, with a maximum limit to prevent excessive computation.
///
/// # Examples
///
/// ```
/// # use wacc::types::CheckCount;
/// let mut count = CheckCount::new(0);
/// count = count.increment().unwrap();
/// assert_eq!(count.as_usize(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckCount(usize);

impl CheckCount {
    /// Maximum number of check operations allowed in a single execution
    pub const MAX: usize = 256;

    /// Creates a new check count
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::CheckCount;
    /// let count = CheckCount::new(0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }

    /// Creates a new check count with validation
    ///
    /// Returns `None` if the count exceeds `MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::CheckCount;
    /// assert!(CheckCount::new_checked(10).is_some());
    /// assert!(CheckCount::new_checked(CheckCount::MAX + 1).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new_checked(count: usize) -> Option<Self> {
        if count <= Self::MAX {
            Some(Self(count))
        } else {
            None
        }
    }

    /// Creates a check count starting at zero
    #[inline]
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns the check count as a usize
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Increments the check count
    ///
    /// Returns `None` if incrementing would exceed `MAX`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::CheckCount;
    /// let count = CheckCount::new(5);
    /// let incremented = count.increment().unwrap();
    /// assert_eq!(incremented.as_usize(), 6);
    /// ```
    #[inline]
    #[must_use]
    pub const fn increment(self) -> Option<Self> {
        if self.0 < Self::MAX {
            Some(Self(self.0 + 1))
        } else {
            None
        }
    }

    /// Checks if the maximum has been reached
    #[inline]
    #[must_use]
    pub const fn at_max(self) -> bool {
        self.0 >= Self::MAX
    }

    /// Checks if this is zero
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl Default for CheckCount {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<usize> for CheckCount {
    fn from(count: usize) -> Self {
        Self::new(count)
    }
}

impl fmt::Display for CheckCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} checks", self.0)
    }
}

/// Size limit for the log buffer
///
/// Prevents unbounded growth of the log buffer which could lead to
/// resource exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogSize(usize);

impl LogSize {
    /// Maximum log buffer size (64 KB)
    pub const MAX: usize = 64 * 1024;

    /// Creates a new log size
    #[inline]
    #[must_use]
    pub const fn new(size: usize) -> Self {
        Self(size)
    }

    /// Creates a new log size with validation
    ///
    /// Returns `None` if the size exceeds `MAX`.
    #[inline]
    #[must_use]
    pub const fn new_checked(size: usize) -> Option<Self> {
        if size <= Self::MAX {
            Some(Self(size))
        } else {
            None
        }
    }

    /// Returns the log size as a usize
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Checks if adding more data would exceed the limit
    #[inline]
    #[must_use]
    pub const fn would_exceed(self, additional: usize) -> bool {
        match self.0.checked_add(additional) {
            Some(total) => total > Self::MAX,
            None => true,
        }
    }
}

impl fmt::Display for LogSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_amount_basic() {
        let fuel = FuelAmount::new(1000);
        assert_eq!(fuel.as_u64(), 1000);
    }

    #[test]
    fn test_fuel_amount_checked() {
        assert!(FuelAmount::new_checked(1000).is_some());
        assert!(FuelAmount::new_checked(0).is_none());
        assert!(FuelAmount::new_checked(FuelAmount::MAX + 1).is_none());
    }

    #[test]
    fn test_fuel_amount_subtract() {
        let fuel = FuelAmount::new(1000);
        assert_eq!(fuel.checked_sub(100).unwrap().as_u64(), 900);
        assert!(fuel.checked_sub(1001).is_none());
    }

    #[test]
    fn test_fuel_amount_exhausted() {
        assert!(FuelAmount::new(0).is_exhausted());
        assert!(!FuelAmount::new(1).is_exhausted());
    }

    #[test]
    fn test_check_count_basic() {
        let count = CheckCount::new(5);
        assert_eq!(count.as_usize(), 5);
    }

    #[test]
    fn test_check_count_increment() {
        let count = CheckCount::new(5);
        let incremented = count.increment().unwrap();
        assert_eq!(incremented.as_usize(), 6);

        let max_count = CheckCount::new(CheckCount::MAX);
        assert!(max_count.increment().is_none());
    }

    #[test]
    fn test_check_count_at_max() {
        assert!(!CheckCount::new(5).at_max());
        assert!(CheckCount::new(CheckCount::MAX).at_max());
    }

    #[test]
    fn test_log_size_would_exceed() {
        let size = LogSize::new(100);
        assert!(!size.would_exceed(50));

        let size = LogSize::new(LogSize::MAX - 10);
        assert!(size.would_exceed(20));
    }
}
