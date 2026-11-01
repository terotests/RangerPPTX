//! The .rgr classes the crate's modules use, imported once.

#[allow(dead_code, unused, non_snake_case)]
mod rgr {
    ranger::import_rgr!("../../../lib/evg/EVGCodepoint");
    ranger::import_rgr!("../../../lib/evg/EVGColor");
    ranger::import_rgr!("../../../lib/evg/EVGDisplayList");
    ranger::import_rgr!("../../../lib/evg/EVGElement");
    ranger::import_rgr!("../../../lib/evg/EVGTextMeasurer");
    ranger::import_rgr!("../../../lib/evg/EVGUnit");
    ranger::import_rgr!("../../../lib/evg/SVGPathParser");
    ranger::import_rgr!("../../office/drawing/OfficeColor");
    ranger::import_rgr!("../../office/geom/OfficePresetShapes");
    ranger::import_rgr!("../../office/text/OfficeText");
    ranger::import_rgr!("../../office/text/OfficeTextMeasure");
    ranger::import_rgr!("../../ooxml/OpcPackage");
    ranger::import_rgr!("../../ooxml/SourceRef");
    ranger::import_rgr!("../../pdf_writer/src/core/Utf8");
    ranger::import_rgr!("../../vela/src/VlCommand");
    ranger::import_rgr!("../../vela/src/VlCompile");
    ranger::import_rgr!("../../vela/src/VlJson");
    ranger::import_rgr!("../../vela/src/VlRuntime");
    ranger::import_rgr!("../../vela/src/VlScene");
    ranger::import_rgr!("../../xml/XmlCore");
    ranger::import_rgr!("PptxHost");
}
pub use rgr::{EVGCodepoint, EVGColor, EVGDisplayList, EVGDrawCmd, EVGElement, EVGTextMeasurer, EVGUnit, GeomPath, OfficeColor, OfficeColorMath, OfficePresetShapes, OfficeText, OfficeTextMeasure, OfficeTheme, OpcPackage, PathRing, PptxHost, SVGPathParser, SourceRef, Utf8, VlCommandList, VlCompile, VlDrawCommand, VlJson, VlJsonParser, VlRuntime, VlSceneCommands, VlSceneMark, XmlNode, XmlParser};
