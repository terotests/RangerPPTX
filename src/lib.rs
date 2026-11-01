//! RangerPPTX's Rust-syntax modules: one crate. The .rgr files they replace
//! are one-line `Import "lib.rs"` files, so .rgr importers (the editor, the
//! tests, MFilesAPI, Sliqtly, RangerMarkdown) keep their paths.

pub mod smartart;
pub mod text;
pub mod rgr;
pub mod pptx_geom;
pub mod pptx_model;
pub mod pptx_resolver;
pub mod pptx_text_layout;
pub mod pptx_chart_vela;
pub mod pptx_to_evg;
pub mod pptx_parser;
