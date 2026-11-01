//! RangerPPTX's editor, writer and api modules in Ranger's Rust syntax: a
//! second crate over the one in src/ (the model, parser, resolver and
//! renderer), so a host that only previews decks (MFilesAPI) imports
//! src/lib.rs and leaves this code out.

pub mod pptx_shape_ink;
pub mod pptx_a11y;
pub mod pptx_text_edit;
pub mod pptx_edit;
pub mod pptx_writer;
pub mod pptx_from_evg;
pub mod pptx_css;
pub mod pptx_vega_shapes;
pub mod pptx_strip_ui;
pub mod pptx_view;
pub mod pptx_api;
pub mod pptx_chart_api;
mod rgr;
