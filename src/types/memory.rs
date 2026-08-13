// SPDX-License-Identifier: Apache-2.0

//! Newtype wrappers for WASM linear memory addresses and sizes
//!
//! These types provide compile-time safety by distinguishing between
//! memory pointers and sizes, preventing common mistakes like passing
//! a size where a pointer is expected.

use std::fmt;

/// A pointer to a location in WASM linear memory
///
/// This newtype prevents accidentally mixing up pointers with sizes or other integers.
/// All memory addresses are represented as u32 in WASM.
///
/// # Examples
///
/// ```
/// # use wacc::types::WasmPtr;
/// let ptr = WasmPtr::new(0x1000);
/// assert_eq!(ptr.as_usize(), 0x1000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WasmPtr(u32);

impl WasmPtr {
    /// Creates a new WASM memory pointer
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::WasmPtr;
    /// let ptr = WasmPtr::new(0x1000);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(addr: u32) -> Self {
        Self(addr)
    }

    /// Returns the pointer value as a u32
    #[inline]
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the pointer value as a usize
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Attempts to add an offset to this pointer, checking for overflow
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::WasmPtr;
    /// let ptr = WasmPtr::new(0x1000);
    /// let new_ptr = ptr.checked_add(0x100).unwrap();
    /// assert_eq!(new_ptr.as_u32(), 0x1100);
    /// ```
    #[inline]
    #[must_use]
    pub const fn checked_add(self, offset: u32) -> Option<Self> {
        match self.0.checked_add(offset) {
            Some(addr) => Some(Self(addr)),
            None => None,
        }
    }

    /// Checks if this pointer plus a size would overflow
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::{WasmPtr, WasmSize};
    /// let ptr = WasmPtr::new(u32::MAX - 10);
    /// let size = WasmSize::new(20);
    /// assert!(ptr.would_overflow(size)); // Would overflow
    ///
    /// let ptr = WasmPtr::new(100);
    /// let size = WasmSize::new(50);
    /// assert!(!ptr.would_overflow(size)); // Would not overflow
    /// ```
    #[inline]
    #[must_use]
    pub const fn would_overflow(self, size: WasmSize) -> bool {
        self.0.checked_add(size.as_u32()).is_none()
    }
}

impl From<u32> for WasmPtr {
    fn from(addr: u32) -> Self {
        Self::new(addr)
    }
}

impl From<i32> for WasmPtr {
    fn from(addr: i32) -> Self {
        Self::new(addr as u32)
    }
}

impl fmt::Display for WasmPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08x}", self.0)
    }
}

/// A size value for WASM linear memory operations
///
/// This newtype prevents accidentally using a size value where a pointer is expected.
///
/// # Examples
///
/// ```
/// # use wacc::types::WasmSize;
/// let size = WasmSize::new(1024);
/// assert_eq!(size.as_usize(), 1024);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WasmSize(u32);

impl WasmSize {
    /// Maximum allowed size for a single memory operation (16 MB)
    pub const MAX_SIZE: u32 = 16 * 1024 * 1024;

    /// Creates a new WASM memory size
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::WasmSize;
    /// let size = WasmSize::new(1024);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new(size: u32) -> Self {
        Self(size)
    }

    /// Creates a new WASM memory size with validation
    ///
    /// Returns `None` if the size exceeds `MAX_SIZE`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::WasmSize;
    /// let size = WasmSize::new_checked(1024).unwrap();
    /// assert_eq!(size.as_u32(), 1024);
    ///
    /// // Too large
    /// assert!(WasmSize::new_checked(WasmSize::MAX_SIZE + 1).is_none());
    /// ```
    #[inline]
    #[must_use]
    pub const fn new_checked(size: u32) -> Option<Self> {
        if size <= Self::MAX_SIZE {
            Some(Self(size))
        } else {
            None
        }
    }

    /// Returns the size value as a u32
    #[inline]
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Returns the size value as a usize
    #[inline]
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Checks if this size is zero
    #[inline]
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<u32> for WasmSize {
    fn from(size: u32) -> Self {
        Self::new(size)
    }
}

impl From<i32> for WasmSize {
    fn from(size: i32) -> Self {
        Self::new(size as u32)
    }
}

impl fmt::Display for WasmSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bytes", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_ptr_basic() {
        let ptr = WasmPtr::new(0x1000);
        assert_eq!(ptr.as_u32(), 0x1000);
        assert_eq!(ptr.as_usize(), 0x1000);
    }

    #[test]
    fn test_wasm_ptr_checked_add() {
        let ptr = WasmPtr::new(100);
        assert_eq!(ptr.checked_add(50).unwrap().as_u32(), 150);

        let ptr = WasmPtr::new(u32::MAX);
        assert!(ptr.checked_add(1).is_none());
    }

    #[test]
    fn test_wasm_ptr_overflow_check() {
        let ptr = WasmPtr::new(u32::MAX - 10);
        let size = WasmSize::new(20);
        assert!(ptr.would_overflow(size));

        let ptr = WasmPtr::new(100);
        let size = WasmSize::new(50);
        assert!(!ptr.would_overflow(size));
    }

    #[test]
    fn test_wasm_size_basic() {
        let size = WasmSize::new(1024);
        assert_eq!(size.as_u32(), 1024);
        assert_eq!(size.as_usize(), 1024);
    }

    #[test]
    fn test_wasm_size_checked() {
        assert!(WasmSize::new_checked(1024).is_some());
        assert!(WasmSize::new_checked(WasmSize::MAX_SIZE).is_some());
        assert!(WasmSize::new_checked(WasmSize::MAX_SIZE + 1).is_none());
    }

    #[test]
    fn test_wasm_size_is_zero() {
        assert!(WasmSize::new(0).is_zero());
        assert!(!WasmSize::new(1).is_zero());
    }
}
