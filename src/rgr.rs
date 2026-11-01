//! The .rgr classes the crate's modules use, imported once.

#[allow(dead_code, unused, non_snake_case)]
mod rgr {
    ranger::import_rgr!("../../../lib/evg/EVGCodepoint");
    ranger::import_rgr!("../../../lib/evg/EVGTextMeasurer");
    ranger::import_rgr!("../../../lib/evg/SVGPathParser");
    ranger::import_rgr!("../../xml/XmlCore");
}
pub use rgr::{EVGCodepoint, EVGTextMeasurer, PathRing, SVGPathParser, XmlNode, XmlParser};
