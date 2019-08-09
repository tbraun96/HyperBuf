#![feature(fundamental, alloc_layout_extra, slice_from_raw_parts, allocator_api, custom_attribute, optin_builtin_traits, async_await, arbitrary_self_types, alloc_error_hook)]
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

/// A memory primitive
pub mod hypervec;

pub(crate) mod results;

#[macro_use]
extern crate hyxe_derive;