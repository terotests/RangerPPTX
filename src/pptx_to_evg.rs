// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::pptx_chart_vela::PptxChartVela;
use crate::pptx_geom::PptxGeom;
use crate::pptx_model::PptxColor;
use crate::pptx_model::PptxGradient;
use crate::pptx_model::PptxParagraph;
use crate::pptx_model::PptxShape;
use crate::pptx_model::PptxShapeKind;
use crate::pptx_model::PptxSlide;
use crate::pptx_model::PptxTableCell;
use crate::pptx_model::PptxTableLine;
use crate::pptx_model::PptxTextBody;
use crate::pptx_model::PptxTextRun;
use crate::pptx_text_layout::PptxTextLayout;
use crate::pptx_text_layout::PptxTextLine;
use crate::rgr::EVGCodepoint;
use crate::rgr::EVGColor;
use crate::rgr::EVGDisplayList;
use crate::rgr::EVGDrawCmd;
use crate::rgr::OfficeText;
use crate::rgr::OfficeTextMeasure;
use crate::rgr::OfficeVisualRun;
use crate::rgr::PathRing;
use crate::rgr::{EVGCodepoint, EVGColor, EVGDisplayList, EVGDrawCmd, EVGElement, EVGUnit, PathRing, PptxHost};


/// ==============================================================================
/// PptxToEvg - ResolvedSlide → EVGElement tree (+ display-list painter helpers)
/// ==============================================================================
///
/// PowerPoint slides are absolute scenes — no flex/grid needed. Each shape
/// becomes a positioned View (div) with background / border / text / image.
/// Groups nest as EVG groups with the same transform box.
///
/// EMU → pt already happened in the model; here we map pt → CSS px at 96dpi
/// (1pt = 96/72 px) so SoftCanvas / WebGL paint at screen pixels.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxToEvg {
    /// extra zoom on top of pt→px
    pub scale: f64,
    /// Who says how wide a string is. A host attaches its text renderer to this
    /// and the painter, the caret and the click all measure through the one
    /// object — the whole point of the layout below.
    measure: Rc<RefCell<OfficeTextMeasure>>,
    /// Which slide is being drawn, for the `slidenum` fields on it. A deck
    /// carries the number each field had when it was saved; the number it has
    /// now is the one to draw.
    pub slideNumber: i64,
    /// The 187 preset geometries, as data.
    ///
    /// Empty until a host loads it, and a converter with an empty catalogue
    /// falls back to the hand-written table exactly as before — which is what
    /// lets this be wired in one editor at a time without a flag day.
    presets: Rc<RefCell<OfficePresetShapes>>,
    pub presetsTried: bool,
}

impl PptxToEvg {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            measure: Rc::new(RefCell::new(OfficeTextMeasure::new())),
            slideNumber: 0,
            presets: Rc::new(RefCell::new(OfficePresetShapes::new())),
            presetsTried: false,
        }
    }
    pub fn num(v: f64) -> String {
        let scaled: i64 = (v * 100.0 + 0.5).floor() as i64;
        let whole: i64 = scaled / 100;
        let mut frac: i64 = scaled - whole * 100;
        if frac < 0 {
            frac = 0 - frac;
        }
        let mut f: String = frac.to_string();
        if frac < 10 {
            f = format!("0{}", f);
        }
        format!("{}.{}", whole.to_string(), f)
    }
    /// A point turned about another one. Both backends rotate an element about
    /// the element's OWN centre — the shader says so and so does the software
    /// canvas — which is right for a shape's box and wrong for everything drawn
    /// inside it: a line of text turned twenty degrees about its own middle is
    /// not the same as the same line turned with the shape it sits in.
    pub fn rotateAbout(px: f64, py: f64, cx: f64, cy: f64, degrees: f64) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        let rad: f64 = degrees * 0.017453292519943295;
        let cs: f64 = rad.cos();
        let sn: f64 = rad.sin();
        let dx: f64 = px - cx;
        let dy: f64 = py - cy;
        out.push(cx + (dx * cs - dy * sn));
        out.push(cy + (dx * sn + dy * cs));
        out
    }
    /// Every point of a path, turned about the shape's centre. A filled polygon
    /// carries no rotation of its own in the display list, so a preset shape
    /// that had been rotated drew straight in BOTH backends until now.
    pub fn rotatePoints(pts: &[f64], cx: f64, cy: f64, degrees: f64) -> Vec<f64> {
        if degrees > 0.0 - 0.001 && degrees < 0.001 {
            return pts.to_vec();
        }
        let mut out: Vec<f64> = Vec::new();
        let rad: f64 = degrees * 0.017453292519943295;
        let cs: f64 = rad.cos();
        let sn: f64 = rad.sin();
        let mut i: i64 = 0;
        while i < (pts.len() as i64) {
            let dx: f64 = pts[i as usize] - cx;
            let dy: f64 = pts[(i + 1) as usize] - cy;
            out.push(cx + (dx * cs - dy * sn));
            out.push(cy + (dx * sn + dy * cs));
            i += 2;
        }
        out
    }
    /// A mirrored shape is its path read from the other side of its own box.
    pub fn mirrorUnit(unit: &[f64], flip_h: bool, flip_v: bool) -> Vec<f64> {
        if !flip_h {
            if !flip_v {
                return unit.to_vec();
            }
        }
        let mut out: Vec<f64> = Vec::new();
        let mut i: i64 = 0;
        while i < (unit.len() as i64) {
            let mut ux: f64 = unit[i as usize];
            let mut uy: f64 = unit[(i + 1) as usize];
            if flip_h {
                ux = 1.0 - ux;
            }
            if flip_v {
                uy = 1.0 - uy;
            }
            out.push(ux);
            out.push(uy);
            i += 2;
        }
        out
    }
    /// A preset path's own fill instruction, applied to the shape's colour.
    ///
    /// A preset states several paths and tells each how to be painted relative
    /// to the fill: `darken`, `darkenLess`, `lighten`, `lightenLess`. That is
    /// the whole of how a flat polygon reads as a solid — a cube is one colour
    /// with one face darker and one lighter, and drawing all three the same
    /// draws a hexagon.
    ///
    /// The specification defines these as transforms in a colour space we do
    /// not carry; these are the proportions every other reader uses, and they
    /// are approximations said out loud rather than pretended exact.
    fn shadeColor(col: &Rc<RefCell<EVGColor>>, mode: &str) -> Rc<RefCell<EVGColor>> {
        let mut f: f64 = 1.0;
        if mode == "darken" {
            f = 0.6;
        }
        if mode == "darkenLess" {
            f = 0.8;
        }
        if mode == "lighten" {
            f = 1.4;
        }
        if mode == "lightenLess" {
            f = 1.2;
        }
        if f == 1.0 {
            return col;
        }
        return EVGColor::rgba(
            PptxToEvg::clamp255(((EVGColor::red(&col)) as f64) * f),
            PptxToEvg::clamp255(((EVGColor::green(&col)) as f64) * f),
            PptxToEvg::clamp255(((EVGColor::blue(&col)) as f64) * f),
            EVGColor::alpha(&col)
        ).clone();
    }
    pub fn clamp255(v: f64) -> i64 {
        if v < 0.0 {
            return 0;
        }
        if v > 255.0 {
            return 255;
        }
        v.floor() as i64
    }
    /// One preset, drawn from the specification's own definition.
    ///
    /// The guides are evaluated in the shape's real proportions — `ss` is the
    /// shorter side, so a preset resolved in a square box and then stretched is
    /// a different shape from the one the file asked for — and the points come
    /// back in the unit box, where `mirrorUnit`, `scaleToBox` and
    /// `rotatePoints` take over exactly as they do for a custGeom.
    ///
    /// Each path carries its own instructions. `stroke="false"` means the path
    /// is not outlined even when the shape has a line; `fill="none"` means it
    /// is a drawn line and not an area; the shading modes are how a flat
    /// polygon reads as a solid. Returns false when the catalogue could not
    /// resolve the shape, so the caller can fall back rather than draw nothing.
    /// The two the rounded-rectangle primitive draws better than a polygon can.
    pub fn drawnAsBox(preset: &str) -> bool {
        if preset == "rect" {
            return true;
        }
        if preset == "roundRect" {
            return true;
        }
        false
    }
    /// The corner radius a rounded rectangle actually asks for: `adj` times the
    /// shorter side. DrawingML's default is 16667 (a sixth), which is what a
    /// shape that states no adjustment gets — not the fixed 12 pixels every
    /// rounded rectangle used to be drawn with whatever its size.
    pub fn cornerRadius(shape: &Rc<RefCell<PptxShape>>, w: f64, h: f64) -> f64 {
        let mut adj: f64 = shape.borrow().adj;
        if adj < 0.0 {
            adj = 0.16667;
        }
        let mut short: f64 = w;
        if h < w {
            short = h;
        }
        let mut r: f64 = adj * short;
        if r < 0.0 {
            r = 0.0;
        }
        if r > short * 0.5 {
            r = short * 0.5;
        }
        r
    }
    /// The outline of a rounded rectangle as points, so it can be STROKED. The
    /// display list's border primitive draws square corners, and a rounded box
    /// with only an outline came out square.
    pub fn roundRectPoints(x: f64, y: f64, w: f64, h: f64, r: f64) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        if r <= 0.5 {
            out.push(x);
            out.push(y);
            out.push(x + w);
            out.push(y);
            out.push(x + w);
            out.push(y + h);
            out.push(x);
            out.push(y + h);
            return out;
        }
        let steps: i64 = 6;
        let mut cx: Vec<f64> = vec![((x + w) - r), ((x + w) - r), (x + r), (x + r)];
        let mut cy: Vec<f64> = vec![(y + r), ((y + h) - r), ((y + h) - r), (y + r)];
        // Corner order: top-right, bottom-right, bottom-left, top-left.
        let mut start: Vec<f64> = vec![-90.0, 0.0, 90.0, 180.0];
        let mut c: i64 = 0;
        while c < 4 {
            let a0: f64 = start[c as usize];
            let mut s: i64 = 0;
            while s <= steps {
                let a: f64 = a0 + (90.0 * (s as f64)) / (steps as f64);
                let rad: f64 = (a * 3.14159265358979) / 180.0;
                out.push(cx[c as usize] + r * rad.cos());
                out.push(cy[c as usize] + r * rad.sin());
                s += 1;
            }
            c += 1;
        }
        out
    }
    pub fn isArrow(kind: &str) -> bool {
        if kind.is_empty() {
            return false;
        }
        if kind == "none" {
            return false;
        }
        true
    }
    pub fn setScale(&mut self, s: f64) {
        self.scale = s;
    }
    /// 1 typographic point → CSS px
    pub fn ptToPx(&self, pt: f64) -> f64 {
        (pt * (96.0 / 72.0)) * self.scale
    }
    fn colorFrom(&self, c: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<EVGColor>> {
        if !c.borrow().isSet {
            return EVGColor::rgba(0, 0, 0, 0.0);
        }
        let hex: String = format!("#{}", c.borrow().srgb);
        let mut col: Rc<RefCell<EVGColor>> = EVGColor::parse(&hex);
        if c.borrow().alpha < 0.999 {
            let mut _tmp_4167 = c.borrow().alpha.clone();
            return EVGColor::create(
                (EVGColor::red(&col)) as f64,
                (EVGColor::green(&col)) as f64,
                (EVGColor::blue(&col)) as f64,
                _tmp_4167
            ).clone();
        }
        col
    }
    pub fn hexCss(&self, c: &Rc<RefCell<PptxColor>>) -> String {
        if !c.borrow().isSet {
            return "transparent".to_string();
        }
        return format!("#{}", c.borrow().srgb);
    }
    pub fn alignCss(&self, algn: &str) -> String {
        if algn == "ctr" {
            return "center".to_string();
        }
        if algn == "r" {
            return "right".to_string();
        }
        if algn == "just" {
            return "justify".to_string();
        }
        "left".to_string()
    }
    pub fn alignToken(&self, algn: &str) -> String {
        if algn == "ctr" {
            return "center".to_string();
        }
        if algn == "r" {
            return "right".to_string();
        }
        "left".to_string()
    }
    fn applyBox(&self, el: &Rc<RefCell<EVGElement>>, shape: &Rc<RefCell<PptxShape>>) {
        let mut _tmp_4169 = self.ptToPx(shape.borrow().x.clone());
        el.borrow_mut().left = Some(EVGUnit::px(_tmp_4169));
        let mut _tmp_4173 = self.ptToPx(shape.borrow().y.clone());
        el.borrow_mut().top = Some(EVGUnit::px(_tmp_4173));
        let mut _tmp_4177 = self.ptToPx(shape.borrow().width.clone());
        el.borrow_mut().width = Some(EVGUnit::px(_tmp_4177));
        let mut _tmp_4181 = self.ptToPx(shape.borrow().height.clone());
        el.borrow_mut().height = Some(EVGUnit::px(_tmp_4181));
        el.borrow_mut().position = "absolute".to_string();
        if shape.borrow().rotation != 0.0 {
            el.borrow_mut().rotate = shape.borrow().rotation;
        }
        if shape.borrow().fill.borrow().isSet {
            let _tmp_4186 = self.colorFrom(&shape.borrow().fill.clone());
            el.borrow_mut().backgroundColor = Some(_tmp_4186);
        }
        if shape.borrow().line.borrow().isSet && shape.borrow().lineWidthPt > 0.0 {
            let mut _tmp_4188 = self.ptToPx(shape.borrow().lineWidthPt.clone());
            el.borrow_mut().borderWidth = Some(EVGUnit::px(_tmp_4188));
            let _tmp_4192 = self.colorFrom(&shape.borrow().line.clone());
            el.borrow_mut().borderColor = Some(_tmp_4192);
        }
        if shape.borrow().preset == "ellipse" {
            // Approximate ellipse with a large border-radius
            let rx: f64 = self.ptToPx(shape.borrow().width * 0.5);
            EVGElement::setAttribute(
                &el,
                "border-radius",
                format!("{}px", (rx.floor() as i64).to_string())
            );
        }
        if shape.borrow().preset == "roundRect" {
            EVGElement::setAttribute(&el, "border-radius", "12px".to_string());
        }
    }
    fn textElement(&self, shape: &Rc<RefCell<PptxShape>>) -> Rc<RefCell<EVGElement>> {
        let mut r#box: Rc<RefCell<EVGElement>> = EVGElement::createDiv();
        self.applyBox(&r#box, shape);
        let mut body: Rc<RefCell<PptxTextBody>> = shape.borrow().text.clone().clone();
        let pad_l: f64 = self.ptToPx(body.borrow().insetL.clone());
        let pad_t: f64 = self.ptToPx(body.borrow().insetT.clone());
        r#box.borrow_mut().paddingLeft = Some(EVGUnit::px(pad_l));
        r#box.borrow_mut().paddingTop = Some(EVGUnit::px(pad_t));
        let mut _tmp_4198 = self.ptToPx(body.borrow().insetR.clone());
        r#box.borrow_mut().paddingRight = Some(EVGUnit::px(_tmp_4198));
        let mut _tmp_4202 = self.ptToPx(body.borrow().insetB.clone());
        r#box.borrow_mut().paddingBottom = Some(EVGUnit::px(_tmp_4202));
        let plain: String = body.borrow().plainText();
        r#box.borrow_mut().textContent = plain.clone();
        if !body.borrow().paragraphs.is_empty() {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[0].clone();
            let _tmp_4206 = self.alignCss(&para.borrow().align.clone());
            r#box.borrow_mut().textAlign = _tmp_4206;
            if !para.borrow().runs.is_empty() {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[0].clone();
                let mut _tmp_4208 = self.ptToPx(run.borrow().fontSizePt.clone());
                r#box.borrow_mut().fontSize = Some(EVGUnit::px(_tmp_4208));
                if !run.borrow().fontFamily.is_empty() {
                    r#box.borrow_mut().fontFamily = run.borrow().fontFamily.clone();
                }
                if run.borrow().bold {
                    r#box.borrow_mut().fontWeight = "bold".to_string();
                }
                if run.borrow().color.borrow().isSet {
                    let _tmp_4213 = self.colorFrom(&run.borrow().color.clone());
                    r#box.borrow_mut().color = Some(_tmp_4213);
                }
            }
        }
        r#box.clone()
    }
    pub fn mapGroupChild(&self, shape: &Rc<RefCell<PptxShape>>, child: &Rc<RefCell<PptxShape>>) -> Rc<RefCell<PptxShape>> {
        let mut mapped: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        mapped.borrow_mut().kind = child.borrow().kind;
        mapped.borrow_mut().name = child.borrow().name.clone();
        mapped.borrow_mut().preset = child.borrow().preset.clone();
        mapped.borrow_mut().fill = child.borrow().fill.clone();
        mapped.borrow_mut().line = child.borrow().line.clone();
        mapped.borrow_mut().lineWidthPt = child.borrow().lineWidthPt;
        mapped.borrow_mut().hasText = child.borrow().hasText;
        mapped.borrow_mut().text = child.borrow().text.clone();
        mapped.borrow_mut().imageRid = child.borrow().imageRid.clone();
        mapped.borrow_mut().imagePart = child.borrow().imagePart.clone();
        mapped.borrow_mut().imageBytes = child.borrow().imageBytes.clone();
        mapped.borrow_mut().imageOk = child.borrow().imageOk;
        mapped.borrow_mut().rotation = child.borrow().rotation + shape.borrow().rotation;
        mapped.borrow_mut().children = child.borrow().children.clone();
        mapped.borrow_mut().chX = child.borrow().chX;
        mapped.borrow_mut().chY = child.borrow().chY;
        mapped.borrow_mut().chW = child.borrow().chW;
        mapped.borrow_mut().chH = child.borrow().chH;
        mapped.borrow_mut().isPlaceholder = child.borrow().isPlaceholder;
        mapped.borrow_mut().phType = child.borrow().phType.clone();
        mapped.borrow_mut().phIdx = child.borrow().phIdx;
        mapped.borrow_mut().inherited = child.borrow().inherited;
        mapped.borrow_mut().adj = child.borrow().adj;
        mapped.borrow_mut().noFill = child.borrow().noFill;
        mapped.borrow_mut().noLine = child.borrow().noLine;
        mapped.borrow_mut().headEnd = child.borrow().headEnd.clone();
        mapped.borrow_mut().tailEnd = child.borrow().tailEnd.clone();
        mapped.borrow_mut().flipH = child.borrow().flipH;
        mapped.borrow_mut().flipV = child.borrow().flipV;
        mapped.borrow_mut().fillGrad = child.borrow().fillGrad.clone();
        mapped.borrow_mut().shadow = child.borrow().shadow.clone();
        mapped.borrow_mut().hasPath = child.borrow().hasPath;
        mapped.borrow_mut().pathUnit = child.borrow().pathUnit.clone();
        mapped.borrow_mut().pathEnds = child.borrow().pathEnds.clone();
        mapped.borrow_mut().pathClosed = child.borrow().pathClosed;
        let mut sx: f64 = 1.0;
        let mut sy: f64 = 1.0;
        if shape.borrow().chW > 0.0 && shape.borrow().width > 0.0 {
            sx = shape.borrow().width / shape.borrow().chW;
        }
        if shape.borrow().chH > 0.0 && shape.borrow().height > 0.0 {
            sy = shape.borrow().height / shape.borrow().chH;
        }
        mapped.borrow_mut().x = (child.borrow().x - shape.borrow().chX) * sx;
        mapped.borrow_mut().y = (child.borrow().y - shape.borrow().chY) * sy;
        mapped.borrow_mut().width = child.borrow().width * sx;
        mapped.borrow_mut().height = child.borrow().height * sy;
        // A turned group carries its children ROUND, it does not spin each of
        // them where it stands.
        //
        // The line above gives the child the group's angle on top of its own,
        // which is right for how the child itself sits — but nothing moved it,
        // so turning a group left every child exactly where it was and rotated
        // each in place. The frame turned and the contents did not follow it,
        // which is what it was reported as.
        //
        // So the child's CENTRE orbits the group's centre, and its own angle
        // stays the sum. The two together are what a rotation about a point
        // is: everything moves, and everything keeps its orientation relative
        // to the thing it moved with.
        if shape.borrow().rotation > 0.001 || shape.borrow().rotation < -0.001 {
            let gcx: f64 = shape.borrow().width * 0.5;
            let gcy: f64 = shape.borrow().height * 0.5;
            let ccx: f64 = mapped.borrow().x + mapped.borrow().width * 0.5;
            let ccy: f64 = mapped.borrow().y + mapped.borrow().height * 0.5;
            let mut moved: Vec<f64> = PptxToEvg::rotateAbout(ccx, ccy, gcx, gcy, shape.borrow().rotation.clone());
            let hoist_20 = moved[0] - mapped.borrow().width * 0.5;
            mapped.borrow_mut().x = hoist_20;
            let hoist_21 = moved[1] - mapped.borrow().height * 0.5;
            mapped.borrow_mut().y = hoist_21;
        }
        mapped.borrow_mut().hasXfrm = true;
        mapped
    }
    fn shapeElement(&self, shape: &Rc<RefCell<PptxShape>>) -> Rc<RefCell<EVGElement>> {
        if shape.borrow().kind == PptxShapeKind::picture() {
            let mut img: Rc<RefCell<EVGElement>> = EVGElement::createImg();
            self.applyBox(&img, shape);
            if !shape.borrow().imagePart.is_empty() {
                img.borrow_mut().src = shape.borrow().imagePart.clone();
            }
            return img;
        }
        if shape.borrow().kind == PptxShapeKind::group() {
            let mut grp: Rc<RefCell<EVGElement>> = EVGElement::createDiv();
            self.applyBox(&grp, shape);
            let mut i: i64 = 0;
            while i < (shape.borrow().children.len() as i64) {
                let mut child: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
                let mut mapped: Rc<RefCell<PptxShape>> = self.mapGroupChild(shape,
                    &child
                );
                // Nested absolute coords are page-relative after mapping — but
                // as EVG children of the group we want group-local coords.
                let hoist_22 = mapped.borrow().x;
                mapped.borrow_mut().x = hoist_22;
                let hoist_23 = mapped.borrow().y;
                mapped.borrow_mut().y = hoist_23;
                EVGElement::addChild(&grp, self.shapeElement(&mapped));
                i += 1;
            }
            return grp;
        }
        // A shape with its own outline is a PATH, not a box.
        //
        // `PptxFromEvg` writes a drawing's strokes as `custGeom`, the parser
        // flattens them into `pathUnit`, and this turned every one of them into
        // an empty div — so a deck's diagram read back as its labels floating
        // over nothing, and a generated deck could not be checked by looking at
        // it. The geometry was in the file the whole time.
        if shape.borrow().hasPath {
            let mut drawn: Option<Rc<RefCell<EVGElement>>> = self.pathElement(shape);
            if !drawn.is_none() {
                return drawn.clone().unwrap();
            }
        }
        if shape.borrow().hasText {
            return self.textElement(shape);
        }
        let mut el: Rc<RefCell<EVGElement>> = EVGElement::createDiv();
        self.applyBox(&el, shape);
        el
    }
    /// The outline, as an EVG path.
    ///
    /// `pathUnit` is in the shape's own 0..1 square — `PptxApp` says so where it
    /// writes one — so the points are scaled by the box and the `viewBox` is the
    /// box, which keeps the numbers readable and the transform out of it.
    ///
    /// `pathEnds` separates contours and is empty for the ordinary one-ring
    /// case. Joining them would weld the end of each ring to the start of the
    /// next, which is how a letter O fills solid.
    fn pathElement(&self, shape: &Rc<RefCell<PptxShape>>) -> Option<Rc<RefCell<EVGElement>>> {
        let n: i64 = shape.borrow().pathUnit.len() as i64;
        if n < 4 {
            let mut none: Option<Rc<RefCell<EVGElement>>> = None;
            return none;
        }
        let mut w: f64 = self.ptToPx(shape.borrow().width.clone());
        let mut h: f64 = self.ptToPx(shape.borrow().height.clone());
        if w <= 0.0 {
            w = 1.0;
        }
        if h <= 0.0 {
            h = 1.0;
        }
        let mut d: String = String::new();
        let mut i: i64 = 0;
        let mut ring: i64 = 0;
        let mut started: bool = false;
        while i + 1 < n {
            let x: f64 = shape.borrow().pathUnit[i as usize] * w;
            let y: f64 = shape.borrow().pathUnit[(i + 1) as usize] * h;
            if !started {
                d = format!("{}M {}", d, PptxToEvg::num(x));
                d = format!("{} {}", d, PptxToEvg::num(y));
                started = true;
            } else {
                d = format!("{} L {}", d, PptxToEvg::num(x));
                d = format!("{} {}", d, PptxToEvg::num(y));
            }
            i += 2;
            // A contour ends where `pathEnds` says, and the next one starts
            // with its own move rather than a line from here.
            if ring < (shape.borrow().pathEnds.len() as i64) {
                if i >= shape.borrow().pathEnds[ring as usize] {
                    if shape.borrow().pathClosed {
                        d = format!("{} Z", d);
                    }
                    started = false;
                    ring += 1;
                }
            }
        }
        if started && shape.borrow().pathClosed {
            d = format!("{} Z", d);
        }
        let mut el: Rc<RefCell<EVGElement>> = EVGElement::createPath();
        self.applyBox(&el, shape);
        // The box's own paint would draw a rectangle behind the outline, which
        // is not what a stroke is. The path takes the colours instead.
        el.borrow_mut().backgroundColor = Some(EVGColor::noColor());
        el.borrow_mut().svgPath = d.clone();
        el.borrow_mut().viewBox = format!("0 0 {} {}", PptxToEvg::num(w), PptxToEvg::num(h));
        if shape.borrow().fill.borrow().isSet {
            if shape.borrow().pathClosed {
                let _tmp_4265 = self.colorFrom(&shape.borrow().fill.clone());
                el.borrow_mut().fillColor = Some(_tmp_4265);
            }
        }
        if shape.borrow().line.borrow().isSet && shape.borrow().lineWidthPt > 0.0 {
            let _tmp_4267 = self.colorFrom(&shape.borrow().line.clone());
            el.borrow_mut().strokeColor = Some(_tmp_4267);
            let _tmp_4269 = self.ptToPx(shape.borrow().lineWidthPt.clone());
            el.borrow_mut().strokeWidth = _tmp_4269;
        } else {
            // An open path with no line is a shape nobody can see. A drawing's
            // connectors arrive that way when the writer put the colour in the
            // fill, so the fill becomes the stroke rather than nothing.
            if !shape.borrow().pathClosed {
                if shape.borrow().fill.borrow().isSet {
                    let _tmp_4271 = self.colorFrom(&shape.borrow().fill.clone());
                    el.borrow_mut().strokeColor = Some(_tmp_4271);
                    el.borrow_mut().strokeWidth = 1.0;
                }
            }
        }
        let mut out: Option<Rc<RefCell<EVGElement>>> = Some(el.clone());
        out
    }
    fn slideToEvg(&self, slide: &Rc<RefCell<PptxSlide>>) -> Rc<RefCell<EVGElement>> {
        let mut page: Rc<RefCell<EVGElement>> = EVGElement::createDiv();
        let mut _tmp_4273 = self.ptToPx(slide.borrow().width.clone());
        page.borrow_mut().width = Some(EVGUnit::px(_tmp_4273));
        let mut _tmp_4277 = self.ptToPx(slide.borrow().height.clone());
        page.borrow_mut().height = Some(EVGUnit::px(_tmp_4277));
        if slide.borrow().bg.borrow().isSet {
            let _tmp_4281 = self.colorFrom(&slide.borrow().bg.clone());
            page.borrow_mut().backgroundColor = Some(_tmp_4281);
        } else {
            page.borrow_mut().backgroundColor = Some(EVGColor::parse("#FFFFFF"));
        }
        page.borrow_mut().position = "relative".to_string();
        let mut i: i64 = 0;
        while i < (slide.borrow().shapes.len() as i64) {
            EVGElement::addChild(&page, self.shapeElement(&slide.borrow().shapes[i as usize].clone()));
            i += 1;
        }
        page
    }
    /// --------------------------------------------------------------------------
    /// Direct display-list emission for the SoftCanvas / WebGL viewer path.
    /// --------------------------------------------------------------------------
    fn pushRect(&self, dl: &Rc<RefCell<EVGDisplayList>>, x: f64, y: f64, w: f64, h: f64, col: &Rc<RefCell<EVGColor>>) {
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 0;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = w;
        c.borrow_mut().h = h;
        c.borrow_mut().r = EVGColor::red(&col);
        c.borrow_mut().g = EVGColor::green(&col);
        c.borrow_mut().b = EVGColor::blue(&col);
        c.borrow_mut().a = EVGColor::alpha(&col);
        dl.borrow_mut().cmds.push(c.clone());
    }
    fn pushBorder(&self, dl: &Rc<RefCell<EVGDisplayList>>, x: f64, y: f64, w: f64, h: f64, thickness: f64, col: &Rc<RefCell<EVGColor>>) {
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 1;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = w;
        c.borrow_mut().h = h;
        c.borrow_mut().thickness = thickness;
        c.borrow_mut().r = EVGColor::red(&col);
        c.borrow_mut().g = EVGColor::green(&col);
        c.borrow_mut().b = EVGColor::blue(&col);
        c.borrow_mut().a = EVGColor::alpha(&col);
        dl.borrow_mut().cmds.push(c.clone());
    }
    fn pushTextCmd(&self, dl: &Rc<RefCell<EVGDisplayList>>, text: &str, x: f64, y: f64, size: f64, col: &Rc<RefCell<EVGColor>>, font_family: String, rotate: f64, align: String, box_w: f64, bold: bool, max_w: f64) {
        if text.is_empty() {
            return;
        }
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 3;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = box_w;
        c.borrow_mut().h = size;
        // Reordered and shaped on the way into the display list, and nowhere
        // after: every backend — the software painter, the GL painter, the
        // browser — draws the string it is handed, so a second pass anywhere
        // below would reverse a right-to-left line back again.
        c.borrow_mut().text = OfficeText::forDisplay(text);
        c.borrow_mut().fontFamily = font_family.clone();
        c.borrow_mut().fontSize = size;
        c.borrow_mut().textAlign = align.clone();
        c.borrow_mut().maxWidth = max_w;
        if bold {
            c.borrow_mut().fontWeight = "bold".to_string();
        }
        c.borrow_mut().r = EVGColor::red(&col);
        c.borrow_mut().g = EVGColor::green(&col);
        c.borrow_mut().b = EVGColor::blue(&col);
        c.borrow_mut().a = EVGColor::alpha(&col);
        c.borrow_mut().rotate = rotate;
        dl.borrow_mut().cmds.push(c.clone());
    }
    /// The command itself lives in `EVGDisplayList` now: the .odp painter emits
    /// one too, and a second copy of a five-field struct is how the two drift.
    fn pushImage(&self, dl: &Rc<RefCell<EVGDisplayList>>, src: &str, x: f64, y: f64, w: f64, h: f64, flip_h: bool, flip_v: bool, rotate: f64) {
        EVGDisplayList::addImage(&dl, src.to_string(), x, y, w, h, flip_h, flip_v, rotate);
    }
    /// The lines of a shape's text, in painter pixels. One layout, asked for by
    /// the painter below and by the host that draws the caret — which is what
    /// makes a caret land on the glyphs of a WRAPPED line and not just on the
    /// first line of the paragraph.
    pub fn lineBoxes(&self, shape: &Rc<RefCell<PptxShape>>, x: f64, y: f64, w: f64, h: f64) -> Vec<Rc<RefCell<PptxTextLine>>> {
        let s: f64 = (96.0 / 72.0) * self.scale;
        let mut _tmp_4297 = shape.borrow().text.clone();
        let fit: f64 = self.substituteFit(&_tmp_4297, w - self.ptToPx(shape.borrow().text.borrow().insetL + shape.borrow().text.borrow().insetR), s);
        let mut _tmp_4299 = shape.borrow().text.clone();
        return PptxTextLayout::layout(&_tmp_4299, &self.measure.clone(), s * fit, x, y, w, h);
    }
    pub fn loadPresets(&mut self, text: &str) {
        OfficePresetShapes::load(&self.presets.clone(), text);
        self.presetsTried = true;
    }
    /// The catalogue, found on its own if nobody handed one over.
    ///
    /// A host that loads it is the normal case and this never runs. A converter
    /// built directly — a test, an embedder — used to draw every preset as a
    /// rectangle instead, silently, which is the exact failure this whole piece
    /// of work exists to end and would have been a fine way to reintroduce it
    /// one call site at a time.
    ///
    /// Once only, and a failure is not an error: a browser has no filesystem
    /// and gets its catalogue from a fetch before anything is drawn.
    pub fn ensurePresets(&mut self) {
        if self.presetsTried {
            return;
        }
        self.presetsTried = true;
        let mut buf: Vec<u8> = PptxHost::readBytes("gallery/office/geom/assets".to_string(), "presets.txt".to_string());
        if !buf.is_empty() {
            OfficePresetShapes::load(&self.presets.clone(), &String::from_utf8_lossy(&buf).to_string());
        }
    }
    pub fn presetsReady(&self) -> bool {
        self.presets.borrow().ready
    }
    /// How much to shrink a body whose font we do not have.
    ///
    /// A deck names Calibri; we draw Open Sans, which is wider; a label the
    /// author sized to fit spills onto a second line. Nothing in the file is
    /// wrong and nothing in the layout is wrong — the font is simply not the
    /// one the box was measured against.
    ///
    /// The file does say how wide the text was meant to be, though: the author
    /// made the box fit it. So a paragraph that overflows its box by a LITTLE
    /// is taken as our metrics being off by that little, and the body is scaled
    /// to fit — which is what PowerPoint's own autofit does, for the same
    /// reason. A paragraph that overflows by a lot is a paragraph that really
    /// wraps, and `floor` is what keeps the two apart: nothing here can shrink
    /// text by more than a sixth, so a real body of prose is left alone.
    ///
    /// Only when the face was substituted. A deck whose fonts we have is
    /// measured correctly already, and second-guessing it would be a bug.
    pub fn substituteFit(&self, body: &Rc<RefCell<PptxTextBody>>, content_w: f64, s: f64) -> f64 {
        if content_w <= 1.0 {
            return 1.0;
        }
        let floor: f64 = 0.84;
        let mut fit: f64 = 1.0;
        let mut substituted: bool = false;
        let mut pi: i64 = 0;
        while pi < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[pi as usize].clone();
            let mut natural: f64 = 0.0;
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                if !OfficeTextMeasure::hasFamily(&self.measure, &run.borrow().fontFamily.clone()) {
                    substituted = true;
                }
                natural += OfficeTextMeasure::widthOfIn(
                    &self.measure,
                    &run.borrow().text,
                    run.borrow().fontSizePt * s,
                    run.borrow().bold,
                    run.borrow().italic,
                    &run.borrow().fontFamily
                );
                ri += 1;
            }
            // The indent this paragraph's level pushes it in by counts against
            // the width it has, or a bulleted line is judged against a box it
            // never had.
            let avail: f64 = content_w - self.ptToPx(para.borrow().marginLeftPt.clone());
            if avail > 1.0 {
                if natural > avail {
                    let need: f64 = avail / natural;
                    if need < fit {
                        fit = need;
                    }
                }
            }
            pi += 1;
        }
        if !substituted {
            return 1.0;
        }
        if fit < floor {
            return 1.0;
        }
        if fit >= 1.0 {
            return 1.0;
        }
        // A hair under, so a line that measures exactly its box does not sit
        // on the boundary and wrap anyway.
        fit * 0.999
    }
    fn emitPresetPaths(&self, dl: &Rc<RefCell<EVGDisplayList>>, shape: &Rc<RefCell<PptxShape>>, x: f64, y: f64, w: f64, h: f64) -> bool {
        let mut _tmp_4309 = shape.borrow().preset.clone();
        let mut _tmp_4310 = shape.borrow().width.clone();
        let mut _tmp_4311 = shape.borrow().height.clone();
        let mut _tmp_4312 = shape.borrow().adjNames.clone();
        let mut _tmp_4313 = shape.borrow().adjValues.clone();
        let mut paths: Vec<Rc<RefCell<GeomPath>>> = OfficePresetShapes::pathsFor(
            &self.presets,
            &_tmp_4309,
            _tmp_4310,
            _tmp_4311,
            &_tmp_4312,
            &_tmp_4313
        );
        if paths.is_empty() {
            return false;
        }
        let cx: f64 = x + w * 0.5;
        let cy: f64 = y + h * 0.5;
        let mut base_fill: Rc<RefCell<EVGColor>> = EVGColor::rgb(180, 180, 190);
        if shape.borrow().fill.borrow().isSet {
            let _tmp_4315 = self.colorFrom(&shape.borrow().fill.clone());
            base_fill = _tmp_4315;
        }
        let mut line_col: Rc<RefCell<EVGColor>> = EVGColor::rgb(60, 60, 70);
        if shape.borrow().line.borrow().isSet {
            let _tmp_4317 = self.colorFrom(&shape.borrow().line.clone());
            line_col = _tmp_4317;
        }
        let mut lw: f64 = self.ptToPx(shape.borrow().lineWidthPt.clone());
        if lw < 1.0 {
            lw = 1.0;
        }
        let has_line: bool = shape.borrow()
            .line.borrow()
            .isSet && shape.borrow()
            .lineWidthPt > 0.0;
        let mut i: i64 = 0;
        while i < (paths.len() as i64) {
            let mut gp: Rc<RefCell<GeomPath>> = paths[i as usize].clone();
            // Each contour transformed on its own, because they have to STAY
            // separate: a chart marker's plus is two strokes and a block arc
            // is an outer and an inner sweep, and joining them draws a line
            // that is not in the file.
            let mut rings: Vec<Rc<RefCell<PathRing>>> = Vec::new();
            let mut ri: i64 = 0;
            while ri < (gp.borrow().rings.len() as i64) {
                let mut src: Rc<RefCell<PathRing>> = gp.borrow().rings[ri as usize].clone();
                let mut _tmp_4319 = src.borrow().pts.clone();
                let mut _tmp_4320 = shape.borrow().flipH.clone();
                let mut unit: Vec<f64> = PptxToEvg::mirrorUnit(&_tmp_4319, _tmp_4320, shape.borrow().flipV.clone());
                let mut flat: Vec<f64> = PptxGeom::scaleToBox(&unit, x, y, w, h);
                let mut out: Rc<RefCell<PathRing>> = Rc::new(RefCell::new(PathRing::new()));
                let _tmp_4323 = PptxToEvg::rotatePoints(&flat, cx, cy, shape.borrow().rotation.clone());
                out.borrow_mut().pts = _tmp_4323;
                rings.push(out.clone());
                ri += 1;
            }
            // Filled unless the file said otherwise — and `<a:noFill/>` on the
            // shape beats anything the preset says, because a deck that asks
            // for no fill is asking about the whole shape.
            let mut want_fill: bool = gp.borrow().fill != "none";
            if shape.borrow().noFill {
                want_fill = false;
            }
            if !gp.borrow().closed {
                want_fill = false;
            }
            if want_fill {
                let mut shade: Rc<RefCell<EVGColor>> = PptxToEvg::shadeColor(&base_fill, &gp.borrow().fill.clone());
                if (rings.len() as i64) > 1 {
                    // Even-odd, which is what makes the inner contour a HOLE
                    // rather than a second disc — a donut, a block arc, the
                    // ring of `noSmoking`.
                    EVGDisplayList::addPolyRings(&dl, &rings, &shade, true);
                } else {
                    let mut only: Rc<RefCell<PathRing>> = rings[0].clone();
                    self.pushPathFill(dl, &only.borrow().pts.clone(), &shade);
                }
            }
            // Stroked only when the SHAPE has a line. `stroke="true"` on a
            // path is permission, not an instruction — it says this path
            // participates in the shape's outline, and a shape with no outline
            // has none to participate in. Drawing one anyway puts a black box
            // round every text frame in a deck, which is what it did.
            if gp.borrow().stroke {
                if has_line {
                    let mut si: i64 = 0;
                    while si < (rings.len() as i64) {
                        let mut sr: Rc<RefCell<PathRing>> = rings[si as usize].clone();
                        let mut _tmp_4326 = sr.borrow().pts.clone();
                        self.pushPathStroke(dl, &_tmp_4326, lw, &line_col, gp.borrow().closed.clone());
                        si += 1;
                    }
                }
            }
            i += 1;
        }
        true
    }
    fn emitTextBody(&self, dl: &Rc<RefCell<EVGDisplayList>>, shape: &Rc<RefCell<PptxShape>>, x: f64, y: f64, w: f64, h: f64) {
        let mut lines: Vec<Rc<RefCell<PptxTextLine>>> = self.lineBoxes(shape,
            x,
            y,
            w,
            h
        );
        let mut body: Rc<RefCell<PptxTextBody>> = shape.borrow().text.clone().clone();
        // The SAME scale the lines were laid out at. Shrinking the layout and
        // drawing at full size would put the right number of lines on the
        // slide with the wrong glyphs in them.
        let mut _tmp_4328 = self.ptToPx(body.borrow().insetL + body.borrow().insetR);
        let s: f64 = ((96.0 / 72.0) * self.scale) * self.substituteFit(&body,
            w - _tmp_4328,
            (96.0 / 72.0) * self.scale
        );
        // The shape's centre, which is what its text turns about.
        let rot_deg: f64 = shape.borrow().rotation;
        let mut has_rot: bool = false;
        if rot_deg > 0.001 || rot_deg < -0.001 {
            has_rot = true;
        }
        let pivot_x: f64 = x + w * 0.5;
        let pivot_y: f64 = y + h * 0.5;
        let mut li: i64 = 0;
        while li < (lines.len() as i64) {
            let mut line: Rc<RefCell<PptxTextLine>> = lines[li as usize].clone();
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[(line.borrow().para) as usize].clone();
            if !line.borrow().bullet.is_empty() {
                let mut _tmp_4330 = line.borrow().bullet.clone();
                let mut _tmp_4331 = line.borrow().bulletSize.clone();
                let bw: f64 = OfficeTextMeasure::widthOf(
                    &self.measure,
                    &_tmp_4330,
                    _tmp_4331,
                    false,
                    false
                );
                let mut bx: f64 = line.borrow().bulletX;
                let mut by: f64 = line.borrow().y;
                if has_rot {
                    let bcx: f64 = bx + bw * 0.5;
                    let bcy: f64 = by + line.borrow().h * 0.5;
                    let mut bp: Vec<f64> = PptxToEvg::rotateAbout(bcx, bcy, pivot_x, pivot_y, rot_deg);
                    bx = bp[0] - bw * 0.5;
                    by = bp[1] - line.borrow().h * 0.5;
                }
                // A bullet has its own colour, which is not the text's: a deck
                // states it per level, and drawing it in one grey for every
                // level throws away half of what a list style says.
                let mut bcol: Rc<RefCell<EVGColor>> = EVGColor::rgb(40, 40, 50);
                if para.borrow().bulletColor.borrow().isSet {
                    let _tmp_4333 = self.colorFrom(&para.borrow().bulletColor.clone());
                    bcol = _tmp_4333;
                } else {
                    if !para.borrow().runs.is_empty() {
                        let mut first_run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[0].clone();
                        if first_run.borrow().color.borrow().isSet {
                            let _tmp_4335 = self.colorFrom(&first_run.borrow().color.clone());
                            bcol = _tmp_4335;
                        }
                    }
                }
                let mut bfont: String = "Calibri".to_string();
                if !para.borrow().bulletFont.is_empty() {
                    bfont = para.borrow().bulletFont.clone();
                }
                let mut _tmp_4336 = line.borrow().bullet.clone();
                let mut _tmp_4337 = line.borrow().bulletSize.clone();
                let mut _tmp_4338 = shape.borrow().rotation.clone();
                self.pushTextCmd(dl,
                    &_tmp_4336,
                    bx,
                    by,
                    _tmp_4337,
                    &bcol,
                    bfont.clone(),
                    _tmp_4338,
                    "left".to_string(),
                    0.0,
                    false,
                    0.0
                );
            }
            // The whole line, then its pieces — in that order, because both
            // the bidirectional algorithm and Arabic joining are defined on a
            // LINE and not on a run.
            //
            // This used to walk the runs and hand each one to the reorderer on
            // its own, then lay the results out left to right. Every letter
            // came out right and every WORD came out backwards: a three-run
            // Arabic sentence drew its first run at the far left, where the
            // last one belongs, and the full stop that ends the line sat in
            // the middle of it, because the run holding it had been reordered
            // as though it were the whole line. The notes strip was right the
            // whole time and the slide was wrong, which is what gave it away —
            // the notes go into the list as one string.
            let mut line_text: String = String::new();
            let mut style_ids: Vec<i64> = Vec::new();
            let mut at: i64 = 0;
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                let __len: i64 = run.borrow().text.as_bytes().len() as i64;
                let s0: i64 = at;
                let s1: i64 = at + __len;
                let mut a: i64 = line.borrow().startCol;
                let mut b: i64 = line.borrow().endCol;
                if a < s0 {
                    a = s0;
                }
                if b > s1 {
                    b = s1;
                }
                if b > a {
                    let mut piece: String = (run.borrow().text)[(a - s0) as usize..(b - s0) as usize].to_string();
                    if run.borrow().field == "slidenum" {
                        if self.slideNumber > 0 {
                            piece = self.slideNumber.to_string();
                        }
                    }
                    line_text = format!("{}{}", line_text, piece);
                    // One id per CODEPOINT, not per code unit: a string is
                    // UTF-16 on JavaScript and bytes in C++, and the reorderer
                    // works in codepoints on both.
                    let mut pcps: Vec<i64> = EVGCodepoint::toArray(&piece);
                    let mut pi: i64 = 0;
                    while pi < (pcps.len() as i64) {
                        style_ids.push(ri);
                        pi += 1;
                    }
                }
                at = s1;
                ri += 1;
            }
            let mut _tmp_4339 = para.borrow().rtl.clone();
            let mut vruns: Vec<Rc<RefCell<OfficeVisualRun>>> = OfficeText::visualRuns(
                &line_text,
                _tmp_4339,
                &style_ids
            );
            let mut pen_x: f64 = line.borrow().x;
            let mut vi: i64 = 0;
            while vi < (vruns.len() as i64) {
                let mut vr: Rc<RefCell<OfficeVisualRun>> = vruns[vi as usize].clone();
                let mut run_1: Rc<RefCell<PptxTextRun>> = para.borrow().runs[(vr.borrow().style) as usize].clone();
                let piece_1: String = vr.borrow().text.clone();
                let font_size: f64 = run_1.borrow().fontSizePt * s;
                let mut _tmp_4341 = run_1.borrow().bold.clone();
                let mut _tmp_4342 = run_1.borrow().italic.clone();
                let mut _tmp_4343 = run_1.borrow().fontFamily.clone();
                let piece_w: f64 = OfficeTextMeasure::widthOfIn(
                    &self.measure,
                    &piece_1,
                    font_size,
                    _tmp_4341,
                    _tmp_4342,
                    &_tmp_4343
                );
                let mut draw_x: f64 = pen_x;
                let mut draw_y: f64 = line.borrow().y;
                if has_rot {
                    let pcx: f64 = pen_x + piece_w * 0.5;
                    let pcy: f64 = line.borrow().y + line.borrow().h * 0.5;
                    let mut rp: Vec<f64> = PptxToEvg::rotateAbout(pcx, pcy, pivot_x, pivot_y, rot_deg);
                    draw_x = rp[0] - piece_w * 0.5;
                    draw_y = rp[1] - line.borrow().h * 0.5;
                }
                // The family the run is DRAWN in, not the one the deck
                // named. Those are rarely the same — Calibri is not shipped
                // here and every width above came from whatever the alias map
                // sent it to — and this line used to carry the deck's name
                // through to the backends unchanged.
                //
                // The software painter ignores it and paints Open Sans, so the
                // desktop never showed the disagreement. A browser does not
                // ignore it: `ctx.font` was handed "Calibri", found no such
                // face registered, and drew the system sans at the same size —
                // 292px of title where the layout had measured 311. The text
                // came out narrow, the caret did not, and the two met a fifth
                // of a letter apart at 12pt and most of one at 40.
                let mut _tmp_4345 = run_1.borrow().fontFamily.clone();
                let mut _tmp_4346 = run_1.borrow().bold.clone();
                let mut _tmp_4347 = run_1.borrow().italic.clone();
                let font_family: String = OfficeTextMeasure::drawnFamily(
                    &self.measure,
                    &_tmp_4345,
                    _tmp_4346,
                    _tmp_4347
                );
                let mut col: Rc<RefCell<EVGColor>> = EVGColor::rgb(0, 0, 0);
                if run_1.borrow().color.borrow().isSet {
                    let _tmp_4349 = self.colorFrom(&run_1.borrow().color.clone());
                    col = _tmp_4349;
                }
                // A highlight is drawn behind the run, over the line it
                // sits on — a marker pen, which is what a deck uses it as.
                if run_1.borrow().highlight.borrow().isSet {
                    let mut _tmp_4350 = line.borrow().h.clone();
                    let mut _tmp_4352 = self.colorFrom(&run_1.borrow().highlight.clone());
                    self.pushRect(dl, draw_x, draw_y, piece_w, _tmp_4350, &_tmp_4352);
                }
                // How a display list says bold and slanted, and it is not
                // one field: every backend reads `fontWeight == "bold"`
                // for the weight and `textAlign == "italic"` for the
                // slant, which is the convention the document viewer set
                // and both painters implement.
                //
                // This wrote "bolditalic" into the weight and the
                // paragraph's ALIGNMENT into the slant field, so a run
                // that was both came out as neither — no backend has ever
                // drawn a pptx italic — and one that was only italic came
                // out upright too.
                let mut weight: String = "";
                if run_1.borrow().bold {
                    weight = "bold";
                }
                let mut slant: String = "left";
                if run_1.borrow().italic {
                    slant = "italic";
                }
                let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
                c.borrow_mut().kind = 3;
                c.borrow_mut().x = draw_x;
                c.borrow_mut().y = draw_y;
                c.borrow_mut().w = 0.0;
                c.borrow_mut().h = font_size;
                c.borrow_mut().text = piece_1.clone();
                c.borrow_mut().fontFamily = font_family.clone();
                c.borrow_mut().fontSize = font_size;
                c.borrow_mut().textAlign = slant.to_string();
                c.borrow_mut().maxWidth = 0.0;
                c.borrow_mut().fontWeight = weight.to_string();
                c.borrow_mut().r = EVGColor::red(&col);
                c.borrow_mut().g = EVGColor::green(&col);
                c.borrow_mut().b = EVGColor::blue(&col);
                c.borrow_mut().a = EVGColor::alpha(&col);
                c.borrow_mut().rotate = shape.borrow().rotation;
                dl.borrow_mut().cmds.push(c.clone());
                pen_x += piece_w;
                vi += 1;
            }
            li += 1;
        }
    }
    /// `addPolygon` rather than a command built here: it is the one place that
    /// also closes the ring list and derives the box from the points. Built by
    /// hand, this shape reached the software canvas — which reads `pts` — and
    /// nothing else, because a path with no rings serialises to a path with no
    /// points.
    fn pushPathFill(&self, dl: &Rc<RefCell<EVGDisplayList>>, pts: &[f64], col: &Rc<RefCell<EVGColor>>) {
        EVGDisplayList::addPolygon(&dl, pts, col);
    }
    /// A path of several contours, filled together. `ends` are indices into
    /// `pts`, as `PptxShape.pathEnds` states them, and the transforms that got
    /// the points here are all point-wise so the boundaries still line up.
    fn pushPathRings(&self, dl: &Rc<RefCell<EVGDisplayList>>, pts: &[f64], ends: &[i64], col: &Rc<RefCell<EVGColor>>) {
        let mut rings: Vec<Rc<RefCell<PathRing>>> = Vec::new();
        let mut start: i64 = 0;
        let mut i: i64 = 0;
        while i < (ends.len() as i64) {
            let upto: i64 = ends[i as usize];
            if upto - start >= 6 {
                let mut ring: Rc<RefCell<PathRing>> = Rc::new(RefCell::new(PathRing::new()));
                let mut k: i64 = start;
                while k < upto {
                    ring.borrow_mut().pts.push(pts[k as usize]);
                    k += 1;
                }
                rings.push(ring.clone());
            }
            start = upto;
            i += 1;
        }
        if rings.is_empty() {
            return;
        }
        EVGDisplayList::addPolyRings(&dl, &rings, col, true);
    }
    fn pushGradRect(&self, dl: &Rc<RefCell<EVGDisplayList>>, x: f64, y: f64, w: f64, h: f64, grad: &Rc<RefCell<PptxGradient>>) {
        let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        c.borrow_mut().kind = 0;
        c.borrow_mut().x = x;
        c.borrow_mut().y = y;
        c.borrow_mut().w = w;
        c.borrow_mut().h = h;
        c.borrow_mut().hasGrad = true;
        c.borrow_mut().gradDir = grad.borrow().dir;
        let mut c1: Rc<RefCell<EVGColor>> = self.colorFrom(&grad.borrow().c1.clone());
        let mut c2: Rc<RefCell<EVGColor>> = self.colorFrom(&grad.borrow().c2.clone());
        c.borrow_mut().r = EVGColor::red(&c1);
        c.borrow_mut().g = EVGColor::green(&c1);
        c.borrow_mut().b = EVGColor::blue(&c1);
        c.borrow_mut().a = EVGColor::alpha(&c1);
        c.borrow_mut().r2 = EVGColor::red(&c2);
        c.borrow_mut().g2 = EVGColor::green(&c2);
        c.borrow_mut().b2 = EVGColor::blue(&c2);
        c.borrow_mut().a2 = EVGColor::alpha(&c2);
        dl.borrow_mut().cmds.push(c.clone());
    }
    fn applyShadowToCmd(&self, c: &Rc<RefCell<EVGDrawCmd>>, shape: &Rc<RefCell<PptxShape>>) {
        if !shape.borrow().shadow.borrow().isSet {
            return;
        }
        c.borrow_mut().hasShadow = true;
        let _tmp_4371 = self.ptToPx(shape.borrow().shadow.borrow().offsetXPt.clone());
        c.borrow_mut().shadowX = _tmp_4371;
        let _tmp_4373 = self.ptToPx(shape.borrow().shadow.borrow().offsetYPt.clone());
        c.borrow_mut().shadowY = _tmp_4373;
        let _tmp_4375 = self.ptToPx(shape.borrow().shadow.borrow().blurPt.clone());
        c.borrow_mut().shadowBlur = _tmp_4375;
        let mut sc: Rc<RefCell<EVGColor>> = self.colorFrom(&shape.borrow().shadow.borrow().color.clone());
        c.borrow_mut().shadowR = EVGColor::red(&sc);
        c.borrow_mut().shadowG = EVGColor::green(&sc);
        c.borrow_mut().shadowB = EVGColor::blue(&sc);
        c.borrow_mut().shadowA = shape.borrow().shadow.borrow().alpha;
    }
    /// The grid the file states, scaled to the frame the table is actually in —
    /// the two disagree whenever a table has been resized, and the columns are
    /// the file's proportions rather than equal shares.
    pub fn tableTracks(&self, stated: &[f64], count: i64, total: f64) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        let mut sum: f64 = 0.0;
        let mut i: i64 = 0;
        while i < (stated.len() as i64) {
            sum += stated[i as usize];
            i += 1;
        }
        if count < 1 {
            return out;
        }
        if (stated.len() as i64) != count {
            sum = 0.0;
        }
        if sum <= 0.0 {
            let even: f64 = total / (count as f64);
            let mut k: i64 = 0;
            while k < count {
                out.push(even);
                k += 1;
            }
            return out;
        }
        let f: f64 = total / sum;
        let mut j: i64 = 0;
        while j < count {
            out.push(stated[j as usize] * f);
            j += 1;
        }
        out
    }
    /// How tall a cell's words are in the width it has. The layout that answers
    /// this is the same one that will draw them, so the two cannot disagree.
    pub fn cellHeight(&self, cell: &Rc<RefCell<PptxTableCell>>, width: f64) -> f64 {
        if cell.borrow().text.is_empty() {
            return 0.0;
        }
        let mut inner: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        inner.borrow_mut().kind = PptxShapeKind::shape();
        inner.borrow_mut().hasText = true;
        inner.borrow_mut().text = cell.borrow().body.clone();
        let ml: f64 = self.ptToPx(cell.borrow().marLPt.clone());
        let mr: f64 = self.ptToPx(cell.borrow().marRPt.clone());
        let mt: f64 = self.ptToPx(cell.borrow().marTPt.clone());
        let mb: f64 = self.ptToPx(cell.borrow().marBPt.clone());
        let bw: f64 = (width - ml) - mr;
        if bw <= 0.0 {
            return 0.0;
        }
        let mut lines: Vec<Rc<RefCell<PptxTextLine>>> = self.lineBoxes(&inner,
            0.0,
            0.0,
            bw,
            100000.0
        );
        if lines.is_empty() {
            return 0.0;
        }
        let mut last: Rc<RefCell<PptxTextLine>> = lines[((lines.len() as i64) - 1) as usize].clone();
        return ((last.borrow().y + last.borrow().h) + mt) + mb;
    }
    fn emitTable(&self, dl: &Rc<RefCell<EVGDisplayList>>, shape: &Rc<RefCell<PptxShape>>, x: f64, y: f64, w: f64, h: f64) {
        let mut rows: i64 = shape.borrow().tableRows;
        let mut cols: i64 = shape.borrow().tableCols;
        if rows < 1 {
            rows = 1;
        }
        if cols < 1 {
            cols = 1;
        }
        let mut col_w: Vec<f64> = self.tableTracks(&shape.borrow().tableColW.clone(), cols, w);
        let mut row_h: Vec<f64> = self.tableTracks(&shape.borrow().tableRowH.clone(), rows, h);
        // A row is at least as tall as its words: a stated height is a minimum
        // in this format, and a row that ignores it draws its second line over
        // the row underneath.
        let mut grown: bool = false;
        let mut gi: i64 = 0;
        while gi < (shape.borrow().tableCells.len() as i64) {
            let mut gcell: Rc<RefCell<PptxTableCell>> = shape.borrow().tableCells[gi as usize].clone();
            let gr: i64 = gi / cols;
            let gc: i64 = gi - gr * cols;
            if gr < rows {
                if !gcell.borrow().merged {
                    if gcell.borrow().rowSpan == 1 {
                        let mut avail: f64 = col_w[gc as usize];
                        let mut sp: i64 = 1;
                        while sp < gcell.borrow().gridSpan {
                            if gc + sp < cols {
                                avail += col_w[(gc + sp) as usize];
                            }
                            sp += 1;
                        }
                        let need: f64 = self.cellHeight(&gcell, avail);
                        if need > row_h[gr as usize] {
                            row_h[gr as usize] = need;
                            grown = true;
                        }
                    }
                }
            }
            gi += 1;
        }
        // Where each track starts, so a cell that spans several knows how far
        // it reaches.
        let mut col_x: Vec<f64> = Vec::new();
        let mut cx0: f64 = x;
        let mut c0: i64 = 0;
        while c0 < cols {
            col_x.push(cx0);
            cx0 += col_w[c0 as usize];
            c0 += 1;
        }
        let mut row_y: Vec<f64> = Vec::new();
        let mut ry0: f64 = y;
        let mut r0: i64 = 0;
        while r0 < rows {
            row_y.push(ry0);
            ry0 += row_h[r0 as usize];
            r0 += 1;
        }
        let mut i: i64 = 0;
        while i < (shape.borrow().tableCells.len() as i64) {
            let mut cell: Rc<RefCell<PptxTableCell>> = shape.borrow().tableCells[i as usize].clone();
            let ri: i64 = i / cols;
            let ci: i64 = i - ri * cols;
            if ri >= rows {
                i += 1;
                continue;
            }
            if cell.borrow().merged {
                i += 1;
                continue;
            }
            let cx: f64 = col_x[ci as usize];
            let cy: f64 = row_y[ri as usize];
            let mut cw: f64 = 0.0;
            let mut sc2: i64 = 0;
            while sc2 < cell.borrow().gridSpan {
                if ci + sc2 < cols {
                    cw += col_w[(ci + sc2) as usize];
                }
                sc2 += 1;
            }
            let mut ch: f64 = 0.0;
            let mut sr: i64 = 0;
            while sr < cell.borrow().rowSpan {
                if ri + sr < rows {
                    ch += row_h[(ri + sr) as usize];
                }
                sr += 1;
            }
            // A cell is only filled when it says so. Painting an unfilled cell
            // white put a white table on every dark slide in the deck.
            if cell.borrow().fill.borrow().isSet {
                let mut _tmp_4390 = self.colorFrom(&cell.borrow().fill.clone());
                self.pushRect(dl, cx, cy, cw, ch, &_tmp_4390);
            }
            self.emitCellEdges(dl, &cell, cx, cy, cw, ch);
            self.emitCellText(dl, &cell, cx, cy, cw, ch);
            i += 1;
        }
    }
    /// Each edge on its own, because that is how a table states them: a header
    /// rule under the first row and nothing else is a very common table, and a
    /// box around every cell is a different picture entirely.
    fn emitCellEdges(&self, dl: &Rc<RefCell<EVGDisplayList>>, cell: &Rc<RefCell<PptxTableCell>>, cx: f64, cy: f64, cw: f64, ch: f64) {
        self.emitCellEdge(dl, &cell.borrow().lnT.clone(), cx, cy, cw, 0.0);
        self.emitCellEdge(dl, &cell.borrow().lnB.clone(), cx, cy + ch, cw, 0.0);
        self.emitCellEdge(dl, &cell.borrow().lnL.clone(), cx, cy, 0.0, ch);
        self.emitCellEdge(dl, &cell.borrow().lnR.clone(), cx + cw, cy, 0.0, ch);
    }
    fn emitCellEdge(&self, dl: &Rc<RefCell<EVGDisplayList>>, line: &Rc<RefCell<PptxTableLine>>, x: f64, y: f64, w: f64, h: f64) {
        if !line.borrow().isSet {
            return;
        }
        if !line.borrow().color.borrow().isSet {
            return;
        }
        let mut col: Rc<RefCell<EVGColor>> = self.colorFrom(&line.borrow().color.clone());
        if EVGColor::alpha(&col) <= 0.004 {
            return;
        }
        let mut t: f64 = self.ptToPx(line.borrow().widthPt.clone());
        if t < 1.0 {
            t = 1.0;
        }
        let mut rw: f64 = w;
        let mut rh: f64 = h;
        if rw <= 0.0 {
            rw = t;
        }
        if rh <= 0.0 {
            rh = t;
        }
        self.pushRect(dl, x - t * 0.5, y - t * 0.5, rw, rh, &col);
    }
    /// A cell's words go through the same layout as a shape's: it IS a text body
    /// in a box, and giving it its own one-line painter is how a table ends up
    /// with text running out the side of every cell.
    fn emitCellText(&self, dl: &Rc<RefCell<EVGDisplayList>>, cell: &Rc<RefCell<PptxTableCell>>, cx: f64, cy: f64, cw: f64, ch: f64) {
        if cell.borrow().text.is_empty() {
            return;
        }
        let mut inner: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        inner.borrow_mut().kind = PptxShapeKind::shape();
        inner.borrow_mut().hasText = true;
        inner.borrow_mut().text = cell.borrow().body.clone();
        inner.borrow_mut().text.borrow_mut().anchor = cell.borrow().anchor.clone();
        inner.borrow_mut().text.borrow_mut().anchorSet = true;
        let ml: f64 = self.ptToPx(cell.borrow().marLPt.clone());
        let mr: f64 = self.ptToPx(cell.borrow().marRPt.clone());
        let mt: f64 = self.ptToPx(cell.borrow().marTPt.clone());
        let mb: f64 = self.ptToPx(cell.borrow().marBPt.clone());
        let bx: f64 = cx + ml;
        let by: f64 = cy + mt;
        let bw: f64 = (cw - ml) - mr;
        let bh: f64 = (ch - mt) - mb;
        if bw <= 0.0 {
            return;
        }
        EVGDisplayList::addClip(&dl, cx, cy, cw, ch);
        self.emitTextBody(dl, &inner, bx, by, bw, bh);
        EVGDisplayList::addClipEnd(&dl);
    }
    /// A path drawn as a LINE rather than as an area: each segment is a quad,
    /// and the joins are square, which is what `a:round` on a thin bracket looks
    /// like at any size a slide is shown at.
    fn pushPathStroke(&self, dl: &Rc<RefCell<EVGDisplayList>>, pts: &[f64], width: f64, col: &Rc<RefCell<EVGColor>>, closed: bool) {
        let n: i64 = (pts.len() as i64) / 2;
        if n < 2 {
            return;
        }
        let mut hw: f64 = width * 0.5;
        if hw < 0.5 {
            hw = 0.5;
        }
        let mut segs: i64 = n - 1;
        if closed {
            segs = n;
        }
        let mut i: i64 = 0;
        while i < segs {
            let a: i64 = i * 2;
            let b: i64 = ((i + 1) % n) * 2;
            let x0: f64 = pts[a as usize];
            let y0: f64 = pts[(a + 1) as usize];
            let x1: f64 = pts[b as usize];
            let y1: f64 = pts[(b + 1) as usize];
            let dx: f64 = x1 - x0;
            let dy: f64 = y1 - y0;
            let __len: f64 = (dx * dx + dy * dy).sqrt();
            if __len > 0.0001 {
                let nx: f64 = (0.0 - dy / __len) * hw;
                let ny: f64 = (dx / __len) * hw;
                let mut quad: Vec<f64> = vec![(x0 + nx), (y0 + ny), (x1 + nx), (y1 + ny), (x1 - nx), (y1 - ny), (x0 - nx), (y0 - ny)];
                self.pushPathFill(dl, &quad, col);
                // A square patch at the join, so a corner is not a notch.
                let mut join: Vec<f64> = vec![(x1 - hw), (y1 - hw), (x1 + hw), (y1 - hw), (x1 + hw), (y1 + hw), (x1 - hw), (y1 + hw)];
                // EVERY vertex of a CLOSED ring gets one, the last included.
                //
                // The guard below is there so an open polyline does not grow a
                // square cap past its final point, and on an open path it is
                // right. On a closed one it skipped the join where the ring
                // comes back to the vertex it started from — leaving one notch
                // per shape, and always in the same place.
                //
                // For a rounded rectangle that place is not arbitrary:
                // `roundRectPoints` starts at the top-right corner with the arc
                // at -90 degrees, which is the junction of the top edge and the
                // corner. So every rounded box on every slide had a single
                // unfilled pixel at its top right, and it survived the edge
                // snapping in `PptxView` because it was never the same bug —
                // that one was the fill falling short, this one is the OUTLINE
                // with a hole in it.
                let mut want_join: bool = i < segs - 1;
                if closed {
                    want_join = true;
                }
                if want_join {
                    self.pushPathFill(dl, &join, col);
                }
            }
            i += 1;
        }
    }
    /// --- connectors -----------------------------------------------------------
    fn emitConnector(&self, dl: &Rc<RefCell<EVGDisplayList>>, shape: &Rc<RefCell<PptxShape>>, x: f64, y: f64, w: f64, h: f64) {
        let mut col: Rc<RefCell<EVGColor>> = EVGColor::rgb(60, 60, 70);
        if shape.borrow().line.borrow().isSet {
            let _tmp_4405 = self.colorFrom(&shape.borrow().line.clone());
            col = _tmp_4405;
        }
        let mut width: f64 = self.ptToPx(shape.borrow().lineWidthPt.clone());
        if width < 1.0 {
            width = 1.0;
        }
        let mut x0: f64 = x;
        let mut y0: f64 = y;
        let mut x1: f64 = x + w;
        let mut y1: f64 = y + h;
        if shape.borrow().flipH {
            x0 = x + w;
            x1 = x;
        }
        if shape.borrow().flipV {
            y0 = y + h;
            y1 = y;
        }
        let cx: f64 = x + w * 0.5;
        let cy: f64 = y + h * 0.5;
        if shape.borrow().rotation != 0.0 {
            let mut ends: Vec<f64> = vec![x0, y0, x1, y1];
            let mut turned: Vec<f64> = PptxToEvg::rotatePoints(&ends, cx, cy, shape.borrow().rotation.clone());
            x0 = turned[0];
            y0 = turned[1];
            x1 = turned[2];
            y1 = turned[3];
        }
        // The line itself, as a quad — a display list has no stroke primitive,
        // and a polygon is what both backends already draw.
        let dx: f64 = x1 - x0;
        let dy: f64 = y1 - y0;
        let __len: f64 = (dx * dx + dy * dy).sqrt();
        if __len < 0.0001 {
            return;
        }
        let ux: f64 = dx / __len;
        let uy: f64 = dy / __len;
        let nx: f64 = 0.0 - uy;
        let ny: f64 = ux;
        let hw: f64 = width * 0.5;
        // The line stops short of an arrowhead, so the point is the end and not
        // a blunt overhang past it.
        let head_len: f64 = width * 3.5;
        let mut sx: f64 = x0;
        let mut sy: f64 = y0;
        let mut ex: f64 = x1;
        let mut ey: f64 = y1;
        let has_head: bool = PptxToEvg::isArrow(&shape.borrow().headEnd.clone());
        let has_tail: bool = PptxToEvg::isArrow(&shape.borrow().tailEnd.clone());
        if has_head {
            sx = x0 + ux * (head_len * 0.6);
            sy = y0 + uy * (head_len * 0.6);
        }
        if has_tail {
            ex = x1 - ux * (head_len * 0.6);
            ey = y1 - uy * (head_len * 0.6);
        }
        let mut quad: Vec<f64> = vec![(sx + nx * hw), (sy + ny * hw), (ex + nx * hw), (ey + ny * hw), (ex - nx * hw), (ey - ny * hw), (sx - nx * hw), (sy - ny * hw)];
        self.pushPathFill(dl, &quad, &col);
        if has_head {
            self.pushArrowHead(dl,
                x0,
                y0,
                0.0 - ux,
                0.0 - uy,
                head_len,
                width * 1.6,
                &col
            );
        }
        if has_tail {
            self.pushArrowHead(dl, x1, y1, ux, uy, head_len, width * 1.6, &col);
        }
    }
    /// A triangle with its point at the end of the line, pointing along it.
    fn pushArrowHead(&self, dl: &Rc<RefCell<EVGDisplayList>>, tip_x: f64, tip_y: f64, ux: f64, uy: f64, __len: f64, half_w: f64, col: &Rc<RefCell<EVGColor>>) {
        let nx: f64 = 0.0 - uy;
        let ny: f64 = ux;
        let bx: f64 = tip_x - ux * __len;
        let by: f64 = tip_y - uy * __len;
        let mut tri: Vec<f64> = vec![tip_x, tip_y, (bx + nx * half_w), (by + ny * half_w), (bx - nx * half_w), (by - ny * half_w)];
        self.pushPathFill(dl, &tri, col);
    }
    fn emitShape(&mut self, dl: &Rc<RefCell<EVGDisplayList>>, shape: &Rc<RefCell<PptxShape>>, ox: f64, oy: f64) {
        let x: f64 = self.ptToPx(shape.borrow().x + ox);
        let y: f64 = self.ptToPx(shape.borrow().y + oy);
        let w: f64 = self.ptToPx(shape.borrow().width.clone());
        let h: f64 = self.ptToPx(shape.borrow().height.clone());
        if shape.borrow().kind == PptxShapeKind::group() {
            let mut i: i64 = 0;
            while i < (shape.borrow().children.len() as i64) {
                let mut child: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
                let mut mapped: Rc<RefCell<PptxShape>> = self.mapGroupChild(shape,
                    &child
                );
                let __arg_0 = ox + shape.borrow().x;
                self.emitShape(dl, &mapped, __arg_0, oy + shape.borrow().y);
                i += 1;
            }
            return;
        }
        if shape.borrow().kind == PptxShapeKind::picture() {
            if !shape.borrow().imagePart.is_empty() {
                let mut _tmp_4412 = shape.borrow().imagePart.clone();
                let mut _tmp_4413 = shape.borrow().flipH.clone();
                let mut _tmp_4414 = shape.borrow().flipV.clone();
                let mut _tmp_4415 = shape.borrow().rotation.clone();
                self.pushImage(dl,
                    &_tmp_4412,
                    x,
                    y,
                    w,
                    h,
                    _tmp_4413,
                    _tmp_4414,
                    _tmp_4415
                );
            } else {
                self.pushRect(dl, x, y, w, h, &EVGColor::rgb(200, 210, 230).clone());
            }
            return;
        }
        if shape.borrow().kind == PptxShapeKind::table() {
            self.emitTable(dl, shape, x, y, w, h);
            return;
        }
        if shape.borrow().kind == PptxShapeKind::chart() {
            PptxChartVela::emitToDisplayList(dl, &shape.borrow().chart.clone(), x, y, w, h);
            return;
        }
        // A connector is a LINE across its box, not a rectangle: the box states
        // the two ends, and `flipH` / `flipV` say which corner is which. Drawn
        // as a rectangle it was invisible — no fill, and an outline around a
        // box nobody can see.
        if shape.borrow().kind == PptxShapeKind::connector() {
            self.emitConnector(dl, shape, x, y, w, h);
            return;
        }
        // Fill: gradient, path preset, or rect
        let mut filled: bool = false;
        // An open path has already been stroked with the shape's own line, and
        // must not also get a rectangle drawn round it.
        let mut stroked_path: bool = false;
        // The outline of a shape that is a PATH has to follow that path.
        //
        // A closed preset — a triangle, a diamond, a funnel stage — was filled
        // as a polygon and then outlined as its BOUNDING RECTANGLE, so every
        // one of them wore a box it did not have. On a seven-stage funnel that
        // is seven rectangles drawn over the thing they are supposed to be
        // tracing, and the deck looks like a stack of boxes with triangles
        // inside them.
        let mut outline_pts: Vec<f64> = Vec::new();
        let mut has_outline: bool = false;
        if shape.borrow().fillGrad.borrow().isSet {
            // unused:  let mut gc: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
            // use pushGradRect path via temporary
            self.pushGradRect(dl, x, y, w, h, &shape.borrow().fillGrad.clone());
            let mut last: Rc<RefCell<EVGDrawCmd>> = dl.borrow().cmds[((dl.borrow().cmds.len() as i64) - 1) as usize].clone();
            last.borrow_mut().rotate = shape.borrow().rotation;
            self.applyShadowToCmd(&last, shape);
            filled = true;
        } else {
            // A preset the catalogue knows is drawn from the SPECIFICATION,
            // guides and all, rather than from a polygon somebody typed in.
            // It comes back as one path or several, each with its own fill and
            // stroke, which is what a cube and a donut need and what the hand
            // table could not express.
            let mut used_catalogue: bool = false;
            if !shape.borrow().hasPath {
                self.ensurePresets();
                // A plain rectangle and a rounded one stay on the rounded-rect
                // primitive, which knows about gradients, shadows and corner
                // radii in a way a polygon does not — and between them they
                // are most of the shapes in most decks, so this is also the
                // path that must not get slower.
                if PptxToEvg::drawnAsBox(&shape.borrow().preset.clone()) {
                } else {
                    if OfficePresetShapes::has(&self.presets, &shape.borrow().preset) {
                        used_catalogue = self.emitPresetPaths(dl, shape, x, y, w, h);
                    }
                }
            }
            if used_catalogue {
                filled = true;
                stroked_path = true;
            } else {
                let mut use_path: bool = shape.borrow().hasPath;
                if !use_path {
                    use_path = PptxGeom::needsPath(&shape.borrow().preset);
                }
                if use_path {
                    let mut unit: Vec<f64> = shape.borrow().pathUnit.clone();
                    if !shape.borrow().hasPath {
                        unit = PptxGeom::unitPreset(&shape.borrow().preset);
                    }
                    unit = PptxToEvg::mirrorUnit(&unit, shape.borrow().flipH, shape.borrow().flipV);
                    let path_cx: f64 = x + w * 0.5;
                    let path_cy: f64 = y + h * 0.5;
                    let mut flat: Vec<f64> = PptxGeom::scaleToBox(&unit, x, y, w, h);
                    let mut pts: Vec<f64> = PptxToEvg::rotatePoints(&flat, path_cx, path_cy, shape.borrow().rotation.clone());
                    // The grey is a placeholder for a shape that states no fill
                    // and no `<a:noFill/>` either — better a visible shape than a
                    // missing one. A shape that DID state `<a:noFill/>` is not
                    // that case and must not be filled at all: it is the shape a
                    // deck uses for a drawn line.
                    let mut col: Rc<RefCell<EVGColor>> = EVGColor::rgb(180, 180, 190);
                    if shape.borrow().fill.borrow().isSet {
                        let _tmp_4422 = self.colorFrom(&shape.borrow().fill.clone());
                        col = _tmp_4422;
                    }
                    let do_fill: bool = !shape.borrow().noFill;
                    // shadow as offset dark polygon first
                    if shape.borrow().shadow.borrow().isSet {
                        let mut _tmp_4424 = self.ptToPx(shape.borrow().shadow.borrow().offsetXPt.clone());
                        let mut _tmp_4426 = self.ptToPx(shape.borrow().shadow.borrow().offsetYPt.clone());
                        let mut sflat: Vec<f64> = PptxGeom::scaleToBox(
                            &unit,
                            x + _tmp_4424,
                            y + _tmp_4426,
                            w,
                            h
                        );
                        let mut spts: Vec<f64> = PptxToEvg::rotatePoints(&sflat, path_cx, path_cy, shape.borrow().rotation.clone());
                        let mut sc: Rc<RefCell<EVGColor>> = self.colorFrom(&shape.borrow().shadow.borrow().color.clone());
                        let mut _tmp_4431 = shape.borrow().shadow.borrow().alpha.clone();
                        let mut shc: Rc<RefCell<EVGColor>> = EVGColor::rgba(
                            EVGColor::red(&sc),
                            EVGColor::green(&sc),
                            EVGColor::blue(&sc),
                            _tmp_4431
                        );
                        EVGDisplayList::addPolygon(&dl, &spts, &shc);
                    }
                    // An open path — a bracket, a brace, a hand-drawn stroke — is
                    // a stroke and not an area: filling it draws a solid blob
                    // where the deck asked for a rule.
                    //
                    // This asked the PRESET whether it was open and left a custom
                    // path out of the question entirely, so every freeform line in
                    // every deck was filled. A drawn line is the commonest custom
                    // path there is, and it is exactly the one that does not
                    // close: an `<a:path>` with no `<a:close/>` under it. The
                    // parser records that now, because flattening the path to
                    // points loses it.
                    let mut open_path: bool = false;
                    if shape.borrow().hasPath {
                        open_path = !shape.borrow().pathClosed;
                    } else {
                        open_path = PptxGeom::isOpenPath(&shape.borrow().preset);
                    }
                    if open_path {
                        let mut lw: f64 = self.ptToPx(shape.borrow().lineWidthPt.clone());
                        if lw < 1.0 {
                            lw = 1.0;
                        }
                        let mut lc: Rc<RefCell<EVGColor>> = EVGColor::rgb(60, 60, 70);
                        if shape.borrow().line.borrow().isSet {
                            let _tmp_4434 = self.colorFrom(&shape.borrow().line.clone());
                            lc = _tmp_4434;
                        }
                        self.pushPathStroke(dl, &pts, lw, &lc, false);
                        stroked_path = true;
                    } else {
                        // A closed path with `<a:noFill/>` is an outline: the
                        // points are still the shape the border follows, they are
                        // simply not filled in.
                        if do_fill {
                            // A shape with several contours is filled as ONE path
                            // under the even-odd rule, so its holes stay holes.
                            // Filling each contour separately - which is what
                            // `pushPathFill` does - paints the holes in.
                            if (shape.borrow().pathEnds.len() as i64) > 1 {
                                self.pushPathRings(dl, &pts, &shape.borrow().pathEnds.clone(), &col);
                            } else {
                                self.pushPathFill(dl, &pts, &col);
                            }
                        }
                        outline_pts = pts.clone();
                        has_outline = true;
                    }
                    filled = true;
                } else {
                    // A ROUNDED BOX WITH AN OUTLINE IS FILLED AS THE SAME RING
                    // ITS OUTLINE IS STROKED FROM, and that is the only way the
                    // two can be guaranteed to meet.
                    //
                    // Otherwise the fill goes out as a rectangle command and the
                    // rasteriser snaps it to whole pixels, while the outline goes
                    // out as a polygon and is rasterised at float precision. Two
                    // approximations of two slightly different rectangles, and
                    // where they disagree the paper shows through. Snapping the
                    // rectangle's edges rather than its size fixed the long seam
                    // down the straight side; it could not fix the corner, because
                    // at the corner the two are not even approximating the same
                    // curve — one is an integer-radius arc, the other six chords.
                    //
                    // Sharing the ring removes the question. Only when there is an
                    // outline to disagree with, and only for the plain case: a
                    // gradient, a shadow or a rotation all need the rectangle
                    // command, and none of them has a hairline to lose.
                    let mut ring_filled: bool = false;
                    if shape.borrow().preset == "roundRect" {
                        if shape.borrow().line.borrow().isSet && shape.borrow().lineWidthPt > 0.0 {
                            if !shape.borrow().shadow.borrow().isSet {
                                if shape.borrow().rotation > -0.001 && shape.borrow().rotation < 0.001 {
                                    if shape.borrow().fill.borrow().isSet {
                                        if !shape.borrow().fillGrad.borrow().isSet {
                                            let rr: f64 = PptxToEvg::cornerRadius(shape, w, h);
                                            let mut rring: Vec<f64> = PptxToEvg::roundRectPoints(x, y, w, h, rr);
                                            let mut _tmp_4437 = self.colorFrom(&shape.borrow().fill.clone());
                                            self.pushPathFill(dl, &rring, &_tmp_4437);
                                            ring_filled = true;
                                            filled = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if ring_filled {
                    } else {
                        if shape.borrow().fill.borrow().isSet {
                            let mut rc: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
                            rc.borrow_mut().kind = 0;
                            rc.borrow_mut().x = x;
                            rc.borrow_mut().y = y;
                            rc.borrow_mut().w = w;
                            rc.borrow_mut().h = h;
                            let mut col2: Rc<RefCell<EVGColor>> = self.colorFrom(&shape.borrow().fill.clone());
                            rc.borrow_mut().r = EVGColor::red(&col2);
                            rc.borrow_mut().g = EVGColor::green(&col2);
                            rc.borrow_mut().b = EVGColor::blue(&col2);
                            rc.borrow_mut().a = EVGColor::alpha(&col2);
                            if shape.borrow().preset == "roundRect" {
                                rc.borrow_mut().radius = PptxToEvg::cornerRadius(shape, w, h);
                            }
                            rc.borrow_mut().rotate = shape.borrow().rotation;
                            self.applyShadowToCmd(&rc, shape);
                            dl.borrow_mut().cmds.push(rc.clone());
                            filled = true;
                        } else {
                            if shape.borrow().shadow.borrow().isSet {
                                let mut rc2: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
                                rc2.borrow_mut().kind = PptxShapeKind::shape();
                                rc2.borrow_mut().x = x;
                                rc2.borrow_mut().y = y;
                                rc2.borrow_mut().w = w;
                                rc2.borrow_mut().h = h;
                                rc2.borrow_mut().r = 255;
                                rc2.borrow_mut().g = 255;
                                rc2.borrow_mut().b = 255;
                                rc2.borrow_mut().a = 0.0;
                                rc2.borrow_mut().rotate = shape.borrow().rotation;
                                self.applyShadowToCmd(&rc2, shape);
                                dl.borrow_mut().cmds.push(rc2.clone());
                            }
                        }
                    }
                }
            }
        }
        if shape.borrow().line.borrow().isSet && shape.borrow().lineWidthPt > 0.0 {
            let lw2: f64 = self.ptToPx(shape.borrow().lineWidthPt.clone());
            let mut lcol: Rc<RefCell<EVGColor>> = self.colorFrom(&shape.borrow().line.clone());
            if shape.borrow().preset == "roundRect" {
                // A rounded box's outline follows its corners. The border
                // primitive only knows square ones.
                let r2: f64 = PptxToEvg::cornerRadius(shape, w, h);
                let mut ring: Vec<f64> = PptxToEvg::roundRectPoints(x, y, w, h, r2);
                let mut _tmp_4449 = shape.borrow().rotation.clone();
                let mut turned: Vec<f64> = PptxToEvg::rotatePoints(
                    &ring,
                    x + w * 0.5,
                    y + h * 0.5,
                    _tmp_4449
                );
                self.pushPathStroke(dl, &turned, lw2, &lcol, true);
            } else {
                if has_outline {
                    // Round the shape it actually is.
                    self.pushPathStroke(dl, &outline_pts, lw2, &lcol, true);
                    stroked_path = true;
                }
                if !stroked_path {
                    self.pushBorder(dl, x, y, w, h, lw2, &lcol);
                    let mut border: Rc<RefCell<EVGDrawCmd>> = dl.borrow().cmds[((dl.borrow().cmds.len() as i64) - 1) as usize].clone();
                    border.borrow_mut().rotate = shape.borrow().rotation;
                }
            }
        }
        if shape.borrow().hasText {
            self.emitTextBody(dl, shape, x, y, w, h);
        }
    }
    fn slideToDisplayList(&mut self, slide: &Rc<RefCell<PptxSlide>>) -> Rc<RefCell<EVGDisplayList>> {
        let mut dl: Rc<RefCell<EVGDisplayList>> = Rc::new(RefCell::new(EVGDisplayList::new()));
        let pw: f64 = self.ptToPx(slide.borrow().width.clone());
        let ph: f64 = self.ptToPx(slide.borrow().height.clone());
        let mut bg: Rc<RefCell<EVGColor>> = EVGColor::parse("#FFFFFF");
        if slide.borrow().bg.borrow().isSet {
            let _tmp_4454 = self.colorFrom(&slide.borrow().bg.clone());
            bg = _tmp_4454;
        }
        self.pushRect(&dl, 0.0, 0.0, pw, ph, &bg);
        let mut i: i64 = 0;
        while i < (slide.borrow().shapes.len() as i64) {
            self.emitShape(&dl, &slide.borrow().shapes[i as usize].clone().clone(), 0.0, 0.0);
            i += 1;
        }
        dl
    }
}
