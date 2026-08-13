// SPDX-License-Identifier: Apache-2.0

//! Newtype wrappers for key-value store keys and paths
//!
//! These types provide compile-time safety by distinguishing between
//! different kinds of keys (current vs proposed state) and context paths.

use std::fmt;

/// A key for accessing values in the current state store
///
/// This newtype prevents accidentally using a proposed state key
/// where a current state key is expected, providing compile-time safety.
///
/// # Examples
///
/// ```
/// # use wacc::types::CurrentKey;
/// let key = CurrentKey::new("pubkey");
/// assert_eq!(key.as_str(), "pubkey");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CurrentKey(String);

impl CurrentKey {
    /// Maximum key length (prevents memory exhaustion)
    pub const MAX_LENGTH: usize = 256;

    /// Creates a new current state key
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::CurrentKey;
    /// let key = CurrentKey::new("pubkey");
    /// ```
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Creates a new current state key with validation
    ///
    /// Returns `None` if the key is empty or exceeds `MAX_LENGTH`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::CurrentKey;
    /// assert!(CurrentKey::new_checked("valid").is_some());
    /// assert!(CurrentKey::new_checked("").is_none());
    /// ```
    pub fn new_checked(key: impl Into<String>) -> Option<Self> {
        let key = key.into();
        if key.is_empty() || key.len() > Self::MAX_LENGTH {
            None
        } else {
            Some(Self(key))
        }
    }

    /// Returns the key as a string slice
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the newtype and returns the inner String
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for CurrentKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurrentKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A key for accessing values in the proposed state store
///
/// This newtype prevents accidentally using a current state key
/// where a proposed state key is expected.
///
/// # Examples
///
/// ```
/// # use wacc::types::ProposedKey;
/// let key = ProposedKey::new("message");
/// assert_eq!(key.as_str(), "message");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProposedKey(String);

impl ProposedKey {
    /// Maximum key length (prevents memory exhaustion)
    pub const MAX_LENGTH: usize = 256;

    /// Creates a new proposed state key
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Creates a new proposed state key with validation
    ///
    /// Returns `None` if the key is empty or exceeds `MAX_LENGTH`.
    pub fn new_checked(key: impl Into<String>) -> Option<Self> {
        let key = key.into();
        if key.is_empty() || key.len() > Self::MAX_LENGTH {
            None
        } else {
            Some(Self(key))
        }
    }

    /// Returns the key as a string slice
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the newtype and returns the inner String
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ProposedKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProposedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A context path for branching in the key-value store
///
/// Context paths allow hierarchical organization of keys. This newtype
/// ensures paths are valid and prevents path-related errors.
///
/// # Examples
///
/// ```
/// # use wacc::types::ContextPath;
/// let path = ContextPath::root();
/// let branched = path.branch("subcontext").unwrap();
/// assert_eq!(branched.as_str(), "/subcontext");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextPath(String);

impl ContextPath {
    /// Maximum path length (prevents deeply nested contexts)
    pub const MAX_LENGTH: usize = 1024;

    /// Maximum depth (number of path segments)
    pub const MAX_DEPTH: usize = 32;

    /// Path separator character
    pub const SEPARATOR: char = '/';

    /// Creates a root context path
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::ContextPath;
    /// let path = ContextPath::root();
    /// assert_eq!(path.as_str(), "");
    /// ```
    #[must_use]
    pub const fn root() -> Self {
        Self(String::new())
    }

    /// Creates a new context path
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::ContextPath;
    /// let path = ContextPath::new("/context");
    /// ```
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    /// Creates a new context path with validation
    ///
    /// Returns `None` if the path is invalid (too long, too deep, or contains invalid characters).
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::ContextPath;
    /// assert!(ContextPath::new_checked("/valid/path").is_some());
    /// assert!(ContextPath::new_checked(&"x".repeat(2000)).is_none());
    /// ```
    pub fn new_checked(path: impl Into<String>) -> Option<Self> {
        let path = path.into();

        // Check length
        if path.len() > Self::MAX_LENGTH {
            return None;
        }

        // Check depth
        let depth = path.matches(Self::SEPARATOR).count();
        if depth > Self::MAX_DEPTH {
            return None;
        }

        // Check for null bytes or other invalid characters
        if path.contains('\0') {
            return None;
        }

        Some(Self(path))
    }

    /// Creates a new context by branching from this one
    ///
    /// Returns `None` if the resulting path would be invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wacc::types::ContextPath;
    /// let root = ContextPath::root();
    /// let sub = root.branch("child").unwrap();
    /// assert_eq!(sub.as_str(), "/child");
    ///
    /// let nested = sub.branch("grandchild").unwrap();
    /// assert_eq!(nested.as_str(), "/child/grandchild");
    /// ```
    #[must_use]
    pub fn branch(&self, segment: &str) -> Option<Self> {
        if segment.is_empty() || segment.contains(Self::SEPARATOR) || segment.contains('\0') {
            return None;
        }

        let new_path = format!("{}{}{}", self.0, Self::SEPARATOR, segment);
        Self::new_checked(new_path)
    }

    /// Returns the path as a string slice
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the depth of this path (number of separators)
    #[inline]
    #[must_use]
    pub fn depth(&self) -> usize {
        if self.0.is_empty() {
            0
        } else {
            self.0.matches(Self::SEPARATOR).count()
        }
    }

    /// Checks if this is the root path
    #[inline]
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Consumes the newtype and returns the inner String
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl Default for ContextPath {
    fn default() -> Self {
        Self::root()
    }
}

impl AsRef<str> for ContextPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContextPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "/")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_key_basic() {
        let key = CurrentKey::new("test");
        assert_eq!(key.as_str(), "test");
    }

    #[test]
    fn test_current_key_checked() {
        assert!(CurrentKey::new_checked("valid").is_some());
        assert!(CurrentKey::new_checked("").is_none());
        assert!(CurrentKey::new_checked("x".repeat(1000)).is_none());
    }

    #[test]
    fn test_proposed_key_basic() {
        let key = ProposedKey::new("message");
        assert_eq!(key.as_str(), "message");
    }

    #[test]
    fn test_context_path_root() {
        let path = ContextPath::root();
        assert!(path.is_root());
        assert_eq!(path.depth(), 0);
    }

    #[test]
    fn test_context_path_branch() {
        let root = ContextPath::root();
        let child = root.branch("child").unwrap();
        assert_eq!(child.as_str(), "/child");
        assert_eq!(child.depth(), 1);

        let grandchild = child.branch("grandchild").unwrap();
        assert_eq!(grandchild.as_str(), "/child/grandchild");
        assert_eq!(grandchild.depth(), 2);
    }

    #[test]
    fn test_context_path_invalid_branch() {
        let root = ContextPath::root();
        assert!(root.branch("").is_none());
        assert!(root.branch("has/slash").is_none());
        assert!(root.branch("has\0null").is_none());
    }

    #[test]
    fn test_context_path_max_depth() {
        let mut path = ContextPath::root();
        for i in 0..ContextPath::MAX_DEPTH {
            path = path.branch(&format!("seg{i}")).unwrap();
        }
        // Next branch should fail
        assert!(path.branch("overflow").is_none());
    }
}
