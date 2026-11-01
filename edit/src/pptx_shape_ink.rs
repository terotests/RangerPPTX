// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxParagraph;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxTextBody;
use pptx::pptx_model::PptxTextRun;
use pptx::rgr::EVGColor;
use pptx::rgr::{EVGColor};

/// ==============================================================================
/// PptxShapeInk.rgr — the four shapes every drawing turns into
/// ==============================================================================
///
/// Two converters now write DrawingML from something that is not a .pptx:
/// `PptxVegaShapes` from Vela's command list, and `PptxFromEvg` from an EVG
/// display list. They have different inputs and the same output, and the
/// interesting decisions are all in the output half:
///
///   * a colour is an EVG colour and a slide wants six hexadecimal digits;
///   * a filled ring is a `custGeom` in the shape's OWN unit square, so the box
///     has to be the ring's bounding box and a flat ring has to be given room;
///   * a straight segment is `prstGeom "line"` with flips, which is the one
///     spelling where a zero-height box is legal;
///   * a label has an anchor POINT and an alignment, and a slide has a BOX.
///
/// That last one is the reason this file exists rather than being copied twice.
/// The box is built around the anchor wide enough that the text cannot reach
/// either end, wrapping is turned off, and the alignment inside the box does the
/// placing — which puts the glyphs where the anchor says without either
/// converter owning a font metric. And because a drawing rotates a label about
/// its ANCHOR while DrawingML rotates a shape about its CENTRE, the box's centre
/// is itself rotated about the anchor first. Get that wrong and every rotated
/// axis title sits a box-width away from where it belongs, further out the wider
/// the label — which reads as a layout bug rather than a rotation one, and cost
/// an afternoon the first time.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxShapeInk {
}

impl PptxShapeInk {
    pub fn new() -> Self {
        Self {
        }
    }
    /// --- colour ---------------------------------------------------------------
    /// An EVG colour as a slide states one: six hexadecimal digits and an alpha.
    fn colorOf(col: &Rc<RefCell<EVGColor>>, alpha: f64) -> Rc<RefCell<PptxColor>> {
        let r: i64 = EVGColor::red(&col);
        let g: i64 = EVGColor::green(&col);
        let b: i64 = EVGColor::blue(&col);
        return PptxShapeInk::rgbaOf(r, g, b, EVGColor::alpha(&col) * alpha);
    }
    pub fn rgbaOf(r: i64, g: i64, b: i64, alpha: f64) -> Rc<RefCell<PptxColor>> {
        let hex: String = format!("{}{}{}", PptxShapeInk::hex2(r), PptxShapeInk::hex2(g), PptxShapeInk::hex2(b));
        let mut out: Rc<RefCell<PptxColor>> = PptxColor::srgbOf(hex.clone());
        out.borrow_mut().alpha = alpha;
        if out.borrow().alpha > 1.0 {
            out.borrow_mut().alpha = 1.0;
        }
        if out.borrow().alpha < 0.0 {
            out.borrow_mut().alpha = 0.0;
        }
        out
    }
    pub fn hex2(v: i64) -> String {
        let mut n: i64 = v;
        if n < 0 {
            n = 0;
        }
        if n > 255 {
            n = 255;
        }
        let hi: i64 = n / 16;
        let lo: i64 = n - hi * 16;
        format!("{}{}", PptxShapeInk::hexDigit(hi), PptxShapeInk::hexDigit(lo))
    }
    pub fn hexDigit(v: i64) -> String {
        if v < 10 {
            return char::from_u32((48 + v) as u32).unwrap_or('\0').to_string();
        }
        char::from_u32((65 + (v - 10)) as u32).unwrap_or('\0').to_string()
    }
    /// "none" is how a drawing says "do not paint this", and it is not a colour:
    /// parsing it would give black, which is the most visible possible way to be
    /// wrong.
    pub fn paints(colour: &str) -> bool {
        if colour.is_empty() {
            return false;
        }
        if colour == "none" {
            return false;
        }
        if colour == "transparent" {
            return false;
        }
        true
    }
    pub fn isBold(weight: &str) -> bool {
        if weight == "bold" {
            return true;
        }
        if weight == "bolder" {
            return true;
        }
        if weight == "600" {
            return true;
        }
        if weight == "700" {
            return true;
        }
        if weight == "800" {
            return true;
        }
        if weight == "900" {
            return true;
        }
        false
    }
    /// --- geometry -------------------------------------------------------------
    /// A straight segment.
    ///
    /// `prstGeom "line"` runs from the top left of the box to the bottom right,
    /// so the DIRECTION is carried by the flips rather than by the geometry —
    /// which is how PowerPoint itself writes a straight connector, and why a
    /// zero-height box is legal here and would not be for a `custGeom`.
    pub fn segment(x1: f64, y1: f64, x2: f64, y2: f64) -> Rc<RefCell<PptxShape>> {
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().preset = "line".to_string();
        sh.borrow_mut().noFill = true;
        sh.borrow_mut().x = x1;
        sh.borrow_mut().y = y1;
        sh.borrow_mut().width = x2 - x1;
        sh.borrow_mut().height = y2 - y1;
        if sh.borrow().width < 0.0 {
            sh.borrow_mut().x = x2;
            let hoist_5 = 0.0 - sh.borrow().width;
            sh.borrow_mut().width = hoist_5;
            sh.borrow_mut().flipH = true;
        }
        if sh.borrow().height < 0.0 {
            sh.borrow_mut().y = y2;
            let hoist_6 = 0.0 - sh.borrow().height;
            sh.borrow_mut().height = hoist_6;
            sh.borrow_mut().flipV = true;
        }
        sh
    }
    /// One ring of a flattened outline, as a `custGeom`.
    ///
    /// `pathUnit` is stated in the shape's own unit square and the writer
    /// multiplies it back up, so the box has to be the ring's bounding box. A
    /// DEGENERATE box is a real shape — a single-datum line series, a zero-height
    /// area — and it still has to be drawn, so the box is widened to a hundredth
    /// of a point. That moves nothing: every point is then at the same fraction
    /// of it.
    ///
    /// `closed` is the difference between a shape and a stroke. A polyline that
    /// is closed draws a segment from the last point back to the first, and if it
    /// is also filled it paints a blob over half the picture.
    pub fn ring(pts: &[f64], from: i64, upto: i64, closed: bool) -> Rc<RefCell<PptxShape>> {
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        if upto - from < 4 {
            return sh;
        }
        let mut min_x: f64 = pts[from as usize];
        let mut min_y: f64 = pts[(from + 1) as usize];
        let mut max_x: f64 = min_x;
        let mut max_y: f64 = min_y;
        let mut k: i64 = from + 2;
        while k < upto {
            let px: f64 = pts[k as usize];
            let py: f64 = pts[(k + 1) as usize];
            if px < min_x {
                min_x = px;
            }
            if px > max_x {
                max_x = px;
            }
            if py < min_y {
                min_y = py;
            }
            if py > max_y {
                max_y = py;
            }
            k += 2;
        }
        let mut bw: f64 = max_x - min_x;
        let mut bh: f64 = max_y - min_y;
        if bw < 0.01 {
            bw = 0.01;
        }
        if bh < 0.01 {
            bh = 0.01;
        }
        sh.borrow_mut().x = min_x;
        sh.borrow_mut().y = min_y;
        sh.borrow_mut().width = bw;
        sh.borrow_mut().height = bh;
        sh.borrow_mut().hasPath = true;
        sh.borrow_mut().pathClosed = closed;
        let mut unit: Vec<f64> = Vec::new();
        let mut j: i64 = from;
        while j < upto {
            let qx: f64 = pts[j as usize];
            let qy: f64 = pts[(j + 1) as usize];
            unit.push((qx - min_x) / bw);
            unit.push((qy - min_y) / bh);
            j += 2;
        }
        sh.borrow_mut().pathUnit = unit.clone();
        sh
    }
    /// --- text -----------------------------------------------------------------
    /// A label placed by its ANCHOR, in a box wide enough that the alignment can
    /// do the placing. See the banner: `angle` rotates about the anchor, not
    /// about the box.
    ///
    /// `align` is "left" / "center" / "right" and `baseline` is "top" / "middle"
    /// / "bottom" / "alphabetic", which are the words both SVG and the display
    /// list use. `alphabetic` puts the anchor ON the baseline, roughly a third of
    /// the size below the visual centre of a line of Latin text.
    pub fn label(body: &str, size_pt: f64, family: &str, bold: bool, align: &str, baseline: &str, angle: f64, anchor_x: f64, anchor_y: f64, colour: Rc<RefCell<PptxColor>>) -> Rc<RefCell<PptxShape>> {
        PptxShapeInk::labelAbout(
            body.to_string(),
            size_pt,
            family.to_string(),
            bold,
            align,
            baseline,
            angle,
            anchor_x,
            anchor_y,
            anchor_x,
            anchor_y,
            colour.clone()
        )
    }
    /// The same label, turned about a point that is NOT its anchor.
    ///
    /// A turned axis label is anchored where its text meets the tick and rotated
    /// about that same point — but a label drawn by an engine that resolved the
    /// alignment itself arrives anchored at the left edge of the INK, which is
    /// somewhere else entirely. Rotating about the wrong one of the two turns the
    /// label about a point up to its own length away, and the labels of a
    /// category axis end up standing inside the plot, behind the bars they name.
    ///
    /// PowerPoint turns a shape about its own centre, so the work here is to
    /// carry that centre round the pivot and let `rot` do the rest.
    pub fn labelAbout(body: String, size_pt: f64, family: String, bold: bool, align: &str, baseline: &str, angle: f64, anchor_x: f64, anchor_y: f64, pivot_x: f64, pivot_y: f64, colour: Rc<RefCell<PptxColor>>) -> Rc<RefCell<PptxShape>> {
        let chars: i64 = (body.as_bytes().len() as i64) + 4;
        let box_w: f64 = size_pt * (chars as f64);
        let box_h: f64 = size_pt * 1.6;
        let mut cx: f64 = 0.0;
        if align == "left" {
            cx = box_w * 0.5;
        }
        if align == "right" {
            cx = 0.0 - box_w * 0.5;
        }
        let mut cy: f64 = 0.0;
        if baseline == "top" {
            cy = box_h * 0.5;
        }
        if baseline == "bottom" {
            cy = 0.0 - box_h * 0.5;
        }
        if baseline == "alphabetic" {
            cy = 0.0 - size_pt * 0.34;
        }
        // Where the box's centre sits before anything is turned, and where the
        // turn carries it. With the pivot ON the anchor the two lines below
        // reduce to rotating the offset itself, which is what this did when a
        // label could only be turned about its own anchor.
        let bx: f64 = (anchor_x + cx) - pivot_x;
        let by: f64 = (anchor_y + cy) - pivot_y;
        let mut rx: f64 = bx;
        let mut ry: f64 = by;
        if angle > 0.001 || angle < 0.0 - 0.001 {
            let rad: f64 = (angle * 3.141592653589793) / 180.0;
            let cs: f64 = rad.cos();
            let sn: f64 = rad.sin();
            rx = bx * cs - by * sn;
            ry = bx * sn + by * cs;
        }
        rx = (pivot_x + rx) - anchor_x;
        ry = (pivot_y + ry) - anchor_y;
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().preset = "rect".to_string();
        sh.borrow_mut().x = (anchor_x + rx) - box_w * 0.5;
        sh.borrow_mut().y = (anchor_y + ry) - box_h * 0.5;
        sh.borrow_mut().width = box_w;
        sh.borrow_mut().height = box_h;
        sh.borrow_mut().rotation = angle;
        sh.borrow_mut().noFill = true;
        sh.borrow_mut().noLine = true;
        let mut tb: Rc<RefCell<PptxTextBody>> = Rc::new(RefCell::new(PptxTextBody::new()));
        tb.borrow_mut().wrap = false;
        tb.borrow_mut().anchor = "ctr".to_string();
        tb.borrow_mut().anchorSet = true;
        tb.borrow_mut().insetL = 0.0;
        tb.borrow_mut().insetR = 0.0;
        tb.borrow_mut().insetT = 0.0;
        tb.borrow_mut().insetB = 0.0;
        tb.borrow_mut().insetsSet = true;
        let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
        para.borrow_mut().align = "ctr".to_string();
        if align == "left" {
            para.borrow_mut().align = "l".to_string();
        }
        if align == "right" {
            para.borrow_mut().align = "r".to_string();
        }
        para.borrow_mut().alignSet = true;
        let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        run.borrow_mut().text = body.clone();
        run.borrow_mut().fontFamily = family.clone();
        run.borrow_mut().fontSizePt = size_pt;
        run.borrow_mut().sizeSet = true;
        run.borrow_mut().boldSet = true;
        run.borrow_mut().bold = bold;
        run.borrow_mut().color = colour.clone();
        para.borrow_mut().runs.push(run.clone());
        tb.borrow_mut().paragraphs.push(para.clone());
        sh.borrow_mut().hasText = true;
        sh.borrow_mut().text = tb.clone();
        sh
    }
}
