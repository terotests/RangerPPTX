// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::rgr::{PathRing, SVGPathParser};


/// ==============================================================================
/// PptxGeom - DrawingML geometry → flat polygon points (unit box)
/// ==============================================================================
///
/// Returns [x0,y0,x1,y1,…] in unit space [0,1]×[0,1]; `scaleToBox` puts that in
/// the shape's rectangle and SoftCanvas / WebGL fill it.
///
/// The shapes are written as **SVG path data** and flattened by EVG's own vector
/// stack (`SVGPathParser`), rather than as tables of points typed out here. That
/// is the difference between a circle and a twelve-sided polygon: `parse` turns
/// an `A` arc into cubics the way SVG says to, and `flatten` walks the curve at
/// whatever resolution it is asked for. It also means a preset is one readable
/// line — the same line a designer would write — instead of twenty pushes.
///
/// What it deliberately does NOT do is holes: `flatten` welds every subpath into
/// one ring, and the display list's polygon primitive has no even-odd rule, so a
/// donut would come out solid. Single-contour presets only, until the painter
/// grows rings.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxGeom {
}

impl PptxGeom {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn pushPt(pts: &mut Vec<f64>, x: f64, y: f64) {
        pts.push(x);
        pts.push(y);
    }
    /// Segments per curve. 16 puts an ellipse's error under a thousandth of its
    /// box, which is finer than a pixel on any slide anyone will project.
    pub fn curveSteps() -> i64 {
        16
    }
    /// A preset as SVG path data in the unit box — the hand-written thirty-four.
    ///
    /// They are gone. `OfficePresetShapes` resolves all 187 from the
    /// specification's own definitions, so keeping these would be thirty-four
    /// standing opportunities to disagree with it — and nine of them already
    /// did. Each of those nine is an adjustable shape whose proportion somebody
    /// had to guess: `plus` was drawn with its corners inset a third of the box
    /// whatever the box was, and the file says a quarter of the SHORTER side,
    /// which is why the hand-drawn one stretched when the box was wide.
    ///
    /// What is left is the empty answer, because the painter still asks: a
    /// converter with no catalogue loaded falls back through here, and a host
    /// that has not loaded one draws rectangles rather than nothing. `rect` and
    /// `roundRect` are drawn by the rounded-rect primitive, which knows about
    /// gradients, shadows and corner radii in a way a polygon does not.
    ///
    /// The comparison that had to pass before this could be deleted is in
    /// PptxGeomTest: every shape here drawn both ways, the thirteen with no
    /// adjustment agreeing to four thousandths of the box.
    pub fn presetPath(preset: &str) -> String {
        String::new()
    }
    /// A path that is a line rather than an area: filling it would draw a solid
    /// blob where the deck asked for a stroke.
    pub fn isOpenPath(preset: &str) -> bool {
        if preset == "leftBracket" {
            return true;
        }
        if preset == "rightBracket" {
            return true;
        }
        if preset == "leftBrace" {
            return true;
        }
        if preset == "rightBrace" {
            return true;
        }
        if preset == "line" {
            return true;
        }
        if preset == "straightConnector1" {
            return true;
        }
        false
    }
    /// Flatten path data into the unit-space point list the painter consumes.
    /// The same flattening, but keeping the contours apart.
    ///
    /// `flattenData` returns one flat list, which is right for a shape that is
    /// one ring and wrong for one that is several: an outline with holes comes
    /// back with every contour joined to the next by a line that is not in the
    /// shape. Callers that can draw more than one ring ask for this instead.
    fn flattenDataRings(data: &str) -> Vec<Rc<RefCell<PathRing>>> {
        let mut parser: Rc<RefCell<SVGPathParser>> = Rc::new(RefCell::new(SVGPathParser::new()));
        SVGPathParser::parse(&parser, data.to_string());
        return SVGPathParser::flattenRingsPlain(&parser, PptxGeom::curveSteps());
    }
    pub fn flattenData(data: &str) -> Vec<f64> {
        let mut parser: Rc<RefCell<SVGPathParser>> = Rc::new(RefCell::new(SVGPathParser::new()));
        SVGPathParser::parse(&parser, data.to_string());
        return SVGPathParser::flatten(&parser, PptxGeom::curveSteps());
    }
    pub fn unitPreset(preset: &str) -> Vec<f64> {
        let data: String = PptxGeom::presetPath(preset);
        if !data.is_empty() {
            return PptxGeom::flattenData(&data);
        }
        let mut pts: Vec<f64> = Vec::new();
        PptxGeom::pushPt(&mut pts, 0.0, 0.0);
        PptxGeom::pushPt(&mut pts, 1.0, 0.0);
        PptxGeom::pushPt(&mut pts, 1.0, 1.0);
        PptxGeom::pushPt(&mut pts, 0.0, 1.0);
        pts
    }
    /// A number in path data, without an exponent: `1e-7` is a perfectly good
    /// double and not a number any path parser has to accept.
    pub fn pathNum(v: f64) -> String {
        // Rounded on the magnitude and the sign put back, because `to_int`
        // FLOORS: `(v - 0.5)` on a negative rounds away from zero twice and
        // -0.25 comes out -0.250001. The same shape of bug the JPEG encoder
        // had in its quantizer.
        let neg: bool = v < 0.0;
        let mut mag: f64 = v;
        if neg {
            mag = 0.0 - v;
        }
        let r: i64 = (mag * 1000000.0 + 0.5).floor() as i64;
        let mut back: f64 = (r as f64) / 1000000.0;
        if neg {
            back = 0.0 - back;
        }
        back.to_string()
    }
    pub fn scaleToBox(unit: &[f64], x: f64, y: f64, w: f64, h: f64) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        let mut i: i64 = 0;
        while i < (unit.len() as i64) {
            let ux: f64 = unit[i as usize];
            let uy: f64 = unit[(i + 1) as usize];
            out.push(x + ux * w);
            out.push(y + uy * h);
            i += 2;
        }
        out
    }
    pub fn needsPath(preset: &str) -> bool {
        (PptxGeom::presetPath(preset).as_bytes().len() as i64) > 0
    }
}
