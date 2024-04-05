// SPDX-License-Identifier: FSL-1.1
use crate::{error::KeyError, Error};
use std::fmt;

/// the separator for the parts of a key
pub const KEY_SEPARATOR: char = '/';

/// The keys used to reference values in a Pairs storage. These form a path of namespaces
/// each part separated by the separator "/" and they come in two flavors: branch or leaf
/// A branch is a key-path that ends with the separator: "/foo/bar/baz/"
/// A leaf is a key-path that does not end with the separator: "/foo/bar/baz"
/// Branches identify a namespace full of leaves and a leaf identifies a single value
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Key {
    parts: Vec<String>
}

impl Key {
    /// true if this key is a branch
    pub fn is_branch(&self) -> bool {
        self.parts.last().unwrap().len() == 0
    }

    /// true if this key is a leaf
    pub fn is_leaf(&self) -> bool {
        self.parts.last().unwrap().len() > 0
    }

    /// returns the number of parts in the key
    pub fn len(&self) -> usize {
        match self.parts.len() {
            0 => 0,
            len => {
                if self.is_branch() {
                    len - 2
                } else {
                    len - 1
                }
            }
        }
    }
}

impl Default for Key {
    fn default() -> Self {
        Key {
            parts: vec!["".to_string(), "".to_string()]
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.parts.join(&KEY_SEPARATOR.to_string()))
    }
}

impl TryFrom<&str> for Key {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Key::try_from(s.to_string())
    }
}

impl TryFrom<String> for Key {
    type Error = Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() == 0 {
            return Err(KeyError::EmptyKey.into());
        }
        let filtered = {
            let mut prev = KEY_SEPARATOR;
            let mut filtered = String::default();
            for (i, c) in s.chars().enumerate() {
                match i {
                    0 => {
                        if c != KEY_SEPARATOR {
                            return Err(KeyError::MissingRootSeparator(s).into());
                        }
                        filtered.push(c);
                    }
                    // eliminate runs of the separator char '///' becomes '/'
                    _ if c == KEY_SEPARATOR => {
                        if c != prev {
                            filtered.push(c);
                            prev = c;
                        }
                    }
                    _ => {
                        filtered.push(c);
                        prev = c;
                    }
                }
            }
            filtered
        };
        Ok(Key {
            parts: filtered.split(KEY_SEPARATOR).map(|s| s.to_string()).collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_empty_key() {
        Key::try_from("").unwrap();
    }

    #[test]
    #[should_panic]
    fn test_bad_key() {
        Key::try_from("foo/bar").unwrap();
    }

    #[test]
    fn test_default() {
        let k = Key::default();
        assert!(k.is_branch());
        assert!(!k.is_leaf());
        assert_eq!(0, k.len());
        assert_eq!(format!("{}", k), "/".to_string());
    }

    #[test]
    fn test_branch() {
        let k = Key::try_from("/foo/bar/baz/").unwrap();
        assert!(k.is_branch());
        assert!(!k.is_leaf());
        assert_eq!(3, k.len());
        assert_eq!(format!("{}", k), "/foo/bar/baz/".to_string())
    }

    #[test]
    fn test_leaf() {
        let k = Key::try_from("/foo/bar/baz").unwrap();
        assert!(!k.is_branch());
        assert!(k.is_leaf());
        assert_eq!(3, k.len());
        assert_eq!(format!("{}", k), "/foo/bar/baz".to_string())
    }
}
