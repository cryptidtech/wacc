// SPDX-License-Identifier: FSL-1.1

/// The interface to a blockstore
pub mod blocks;

/// The interface to a key-value pairs store
pub mod pairs;

/// The interface to a value stack
pub mod stack;

pub use blocks::Blocks;
pub use pairs::Pairs;
pub use stack::Stack;
