// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaFile — a SmartArt file as a host meets it: bytes with a name and a type
//! ==============================================================================
//!
//! A host (Sliqtly's editor, its MCP server) keeps a diagram the way it keeps a
//! picture: a file in the deck, `media/process.xml`, named from the Markdown.
//! This is what such a host asks about the file before anything is laid out:
//! is it a diagram, what is its text, and how large is it before a box says.
//! ==============================================================================

use ranger::prelude::*;



#[derive(Clone)]
pub struct SaFile {
}

impl SaFile {
    pub fn new() -> Self {
        Self {
        }
    }
    /// what OOXML calls a diagram's data part, and so the type a host stores
    /// the file under
    pub fn TYPE() -> String {
        "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml".to_string()
    }
    /// The natural size a host gives the file before its Markdown says
    /// otherwise: 16:9, larger than any column, so a layout fits it down to the
    /// column (a picture is never scaled up) and `{width}` / `{height}` work.
    pub fn naturalW() -> i64 {
        1600
    }
    pub fn naturalH() -> i64 {
        900
    }
    pub fn endsWith(s: &str, tail: &str) -> bool {
        let n: i64 = s.as_bytes().len() as i64;
        let t: i64 = tail.as_bytes().len() as i64;
        if t > n {
            return false;
        }
        s[(n - t) as usize..(n) as usize].to_string() == tail
    }
    /// A diagram: stored under its own type, or an .xml file that is not a
    /// picture. An .xml file is never a picture a host can draw, so one that
    /// holds something other than a diagram is still taken as one: the engine
    /// then refuses it with the reason ("the root element is <x>"), which the
    /// host shows in its place, instead of a broken picture that draws
    /// nothing. Only SVG kept under an .xml name, and a file stored under a
    /// picture's type, stay pictures.
    pub fn isSmartArt(name: &str, r#type: &str, head: &str) -> bool {
        if r#type == SaFile::TYPE() {
            return true;
        }
        let lower: String = name.to_lowercase();
        if !SaFile::endsWith(&lower, ".xml") {
            return false;
        }
        if (r#type.as_bytes().len() as i64) > 0 && ((r#type != "application/xml" && r#type != "text/xml") && r#type != "application/octet-stream") {
            return false;
        }
        // the data alone, or a whole diagram as a Flat OPC package (SaWhole)
        if PptxText::index_of(&head, "dataModel") >= 0 {
            return true;
        }
        if PptxText::index_of(&head, "http://schemas.microsoft.com/office/2006/xmlPackage") >= 0 {
            return true;
        }
        PptxText::index_of(&head, "<svg") < 0
    }
    /// the bytes as text: UTF-8, so a Finnish or Greek label is itself
    pub fn textOf(data: &[u8]) -> String {
        if data.is_empty() {
            return String::new();
        }
        Utf8::decode(&String::from_utf8_lossy(&data).to_string())
    }
    /// the first bytes, as text, for isSmartArt. Only those are decoded: a
    /// host asks this of every picture it is handed, and decoding a whole
    /// PNG or JPEG as text to read its first characters was most of what
    /// handing a deck's pictures over cost.
    pub fn headOf(data: &[u8]) -> String {
        let n: i64 = data.len() as i64;
        if n <= 2048 {
            return SaFile::textOf(data);
        }
        // not cut inside a character: back off its continuation bytes
        let mut end: i64 = 2048;
        while end > 0 && ((data[end as usize] as i64) >= 128 && (data[end as usize] as i64) < 192) {
            end -= 1;
        }
        SaFile::textOf(&data[0..(end) as usize].to_vec())
    }
}
