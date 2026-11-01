//! RangerPPTX's Rust-syntax modules: one crate. The .rgr files they replace
//! are one-line `Import "lib.rs"` files, so .rgr importers (the editor, the
//! tests, MFilesAPI, Sliqtly, RangerMarkdown) keep their paths.

pub mod smartart;
pub mod text;
mod rgr;
pub mod pptx_geom;
