#![feature(fundamental, alloc_layout_extra, slice_from_raw_parts, allocator_api, custom_attribute, optin_builtin_traits, async_await, arbitrary_self_types, alloc_error_hook, trivial_bounds, in_band_lifetimes)]
#![feature(label_break_value)]
//! HyperVec is a highly-experimental primitive


#![deny(
missing_docs,
trivial_numeric_casts,
unused_extern_crates,
unused_import_braces,
variant_size_differences,
unused_features,
unused_results,
warnings
)]

/// Import everything herein to gain access to the HyperVec and all its associated structures, subroutines, and implementations
pub mod prelude {
    pub use crate::hypervec::{Endianness, HyperVec};
    pub use crate::impls::*;
    pub use crate::results::*;
}

/// A memory primitive
pub mod hypervec;

pub(crate) mod results;

#[macro_use]
extern crate hyperbuf_derive;

pub(crate) mod util;

/// provides useful implementations for HyperVec
pub mod impls;

/// Low-level memory tracking system that removes the necessity to store a single type to a vector by keeping track of sizes
pub mod partition_map;