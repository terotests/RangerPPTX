// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxParagraph;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxTextRun;
use crate::pptx_shape_ink::PptxShapeInk;
use pptx::rgr::EVGDisplayList;
use pptx::rgr::EVGDrawCmd;
use pptx::rgr::{EVGDisplayList, EVGDrawCmd};


/// ==============================================================================
/// PptxFromEvg.rgr — an EVG display list becomes DrawingML. The other direction.
/// ==============================================================================
///
/// `PptxToEvg` turns a slide into a display list so the engine can draw it. This
/// is the inverse, and it makes the display list an OUTPUT FORMAT for slides:
/// anything in this repository that can draw into an `EVGDisplayList` — a Vela
/// chart, an EVG-laid-out document, a data grid, a flow diagram — can be put on
/// a slide as shapes without knowing that PresentationML exists.
///
/// WHY IT LIVES HERE AND NOT IN lib/evg. Because the dependency has to run
/// this way round. EVG knows nothing about Office and should go on knowing
/// nothing; the .pptx side already imports EVG. A file under lib/evg that
/// imported PptxModel would make the layout engine depend on a document format,
/// which is the direction the gallery is built to avoid.
///
/// HOW THIS DIFFERS FROM PptxVegaShapes, AND WHY BOTH EXIST. A display list is
/// POST-FLATTENING: by the time a drawing reaches it a rounded bar is a polygon,
/// a pie wedge is a polygon, and a label's position has already been resolved.
/// Everything still comes out as vectors — that is the point — but the SEMANTICS
/// are gone, so more of it lands as `custGeom` and less as a preset an author can
/// drag. Converting from Vela's command list instead keeps a bar a rectangle and
/// a scatter point an ellipse, which is smaller on disk and better to edit.
///
/// So: `PptxVegaShapes` is the specialisation, used when the source is a Vega
/// specification, and this is the universal one, used when the source is
/// whatever else drew. Both emit through `PptxShapeInk`, so a colour, a ring and
/// a label mean the same thing in either.
///
/// WHAT IS DROPPED, AND SAID SO. Clips (`PUSH_CLIP` / `POP_CLIP`) have no
/// DrawingML spelling short of putting every clipped shape in its own group with
/// a mask, which PowerPoint does not have either; content outside a clip is
/// therefore drawn rather than hidden, and `clipped` reports how many clips were
/// ignored so a caller can tell. Gradients become their first stop. A shadow
/// becomes a shadow, because the model has one.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxFromEvg {
    /// What "sans-serif" is called on a slide, for a command that named a family
    /// the deck does not have. A display list normally names a real face.
    pub sansFamily: String,
    /// Below this a label is a smudge at any zoom.
    pub minFontPt: f64,
    /// Points per display-list unit. A display list is in CSS pixels and a slide
    /// is in points, so a drawing laid out at 96 dpi is 0.75 here — which is the
    /// default, and is why a 960x540 page lands exactly on a 720x540 slide.
    pub unit: f64,
    pub shapeCount: i64,
    pub textCount: i64,
    /// How many clip commands were ignored. Not an error and not nothing: a
    /// drawing that relied on a clip to hide something comes out showing it.
    pub clipped: i64,
    /// Pictures are the one thing here that stays a picture, and a display list
    /// names them by `src` rather than carrying the bytes. Counted so a caller
    /// knows the slide is missing them.
    pub unresolvedImages: i64,
    /// …unless somebody hands over the store the list was built against, in
    /// which case they are resolved rather than counted. Empty by default,
    /// because most callers of this file are drawing a diagram and a diagram
    /// has no pictures in it.
    images: Rc<RefCell<Vfs>>,
    pub haveImages: bool,
    pub placedImages: i64,
}

impl PptxFromEvg {
    pub fn new() -> Self {
        Self {
            sansFamily: "Calibri".to_string(),
            minFontPt: 5.0,
            unit: 0.75,
            shapeCount: 0,
            textCount: 0,
            clipped: 0,
            unresolvedImages: 0,
            images: Rc::new(RefCell::new(Vfs::new())),
            haveImages: false,
            placedImages: 0,
        }
    }
    fn setImages(&mut self, v: Rc<RefCell<Vfs>>) {
        self.images = v.clone();
        self.haveImages = true;
    }
    /// The whole list as one group, placed at (x, y) in points.
    ///
    /// The group is sized from the drawing's own extent rather than from a box
    /// the caller states, because a display list has no page: it is whatever was
    /// drawn into it. Scale it by setting `unit` before calling.
    fn build(&mut self, dl: &Rc<RefCell<EVGDisplayList>>, x: f64, y: f64) -> Rc<RefCell<PptxShape>> {
        self.shapeCount = 0;
        self.textCount = 0;
        self.clipped = 0;
        self.unresolvedImages = 0;
        let mut group: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        group.borrow_mut().kind = PptxShapeKind::group();
        group.borrow_mut().name = "Drawing".to_string();
        group.borrow_mut().hasXfrm = true;
        group.borrow_mut().x = x;
        group.borrow_mut().y = y;
        let n: i64 = dl.borrow().cmds.len() as i64;
        let mut i: i64 = 0;
        while i < n {
            let mut c: Rc<RefCell<EVGDrawCmd>> = dl.borrow().cmds[i as usize].clone();
            self.emit(&group, &c, x, y);
            i += 1;
        }
        // The box is the union of what was drawn. A group whose ext does not
        // cover its children is one PowerPoint clips on the first edit.
        self.fitTo(&group, x, y);
        group
    }
    pub fn fitTo(&self, group: &Rc<RefCell<PptxShape>>, x: f64, y: f64) {
        let mut w: f64 = 0.0;
        let mut h: f64 = 0.0;
        let mut i: i64 = 0;
        while i < (group.borrow().children.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = group.borrow().children[i as usize].clone();
            let right: f64 = (sh.borrow().x + sh.borrow().width) - x;
            let bottom: f64 = (sh.borrow().y + sh.borrow().height) - y;
            if right > w {
                w = right;
            }
            if bottom > h {
                h = bottom;
            }
            i += 1;
        }
        if w < 1.0 {
            w = 1.0;
        }
        if h < 1.0 {
            h = 1.0;
        }
        group.borrow_mut().width = w;
        group.borrow_mut().height = h;
        // Children hold slide points, so the child space is the group's own box
        // and nothing is scaled twice.
        group.borrow_mut().chX = x;
        group.borrow_mut().chY = y;
        group.borrow_mut().chW = w;
        group.borrow_mut().chH = h;
    }
    /// --- one command ----------------------------------------------------------
    fn emit(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, ox: f64, oy: f64) {
        if c.borrow().a <= 0.0 {
            if c.borrow().kind != 3 {
                return;
            }
        }
        if c.borrow().kind == 0 {
            self.emitRect(group, c, ox, oy);
            return;
        }
        if c.borrow().kind == 1 {
            self.emitBorder(group, c, ox, oy);
            return;
        }
        if c.borrow().kind == 2 {
            // A display list names a picture; it does not carry it. The bytes
            // belong to whoever built the list — so with the store that built
            // it, this places the picture, and without one it counts what it
            // cannot place rather than inventing an empty frame.
            if self.emitPicture(group, c, ox, oy) {
                return;
            }
            self.unresolvedImages += 1;
            return;
        }
        if c.borrow().kind == 3 {
            self.emitText(group, c, ox, oy);
            return;
        }
        if c.borrow().kind == 4 {
            self.clipped += 1;
            return;
        }
        if c.borrow().kind == 5 {
            return;
        }
        if c.borrow().kind == 6 {
            self.emitRings(group, c, ox, oy, true);
            return;
        }
        self.emitRings(group, c, ox, oy, false);
    }
    /// A filled box. Rounded corners are a `roundRect`, which is a preset an
    /// author can drag rather than a polygon they cannot.
    fn emitRect(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, ox: f64, oy: f64) {
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().preset = "rect".to_string();
        sh.borrow_mut().x = ox + c.borrow().x * self.unit;
        sh.borrow_mut().y = oy + c.borrow().y * self.unit;
        sh.borrow_mut().width = c.borrow().w * self.unit;
        sh.borrow_mut().height = c.borrow().h * self.unit;
        if c.borrow().radius > 0.0 {
            sh.borrow_mut().preset = "roundRect".to_string();
            let mut shortest: f64 = sh.borrow().width;
            if sh.borrow().height < shortest {
                shortest = sh.borrow().height;
            }
            if shortest > 0.0 {
                let mut adj: f64 = (c.borrow().radius * self.unit) / shortest;
                if adj > 0.5 {
                    adj = 0.5;
                }
                sh.borrow_mut().adj = adj;
            }
        }
        if c.borrow().hasGrad {
            // The model carries a two-stop linear gradient and the display list
            // carries the same, so this one survives rather than being flattened.
            sh.borrow_mut().fillGrad.borrow_mut().isSet = true;
            sh.borrow_mut().fillGrad.borrow_mut().dir = c.borrow().gradDir;
            let mut _tmp_5238 = c.borrow().r.clone();
            let mut _tmp_5239 = c.borrow().g.clone();
            let mut _tmp_5240 = c.borrow().b.clone();
            let _tmp_5242 = PptxShapeInk::rgbaOf(_tmp_5238, _tmp_5239, _tmp_5240, c.borrow().a.clone());
            sh.borrow_mut().fillGrad.borrow_mut().c1 = _tmp_5242;
            let mut _tmp_5243 = c.borrow().r2.clone();
            let mut _tmp_5244 = c.borrow().g2.clone();
            let mut _tmp_5245 = c.borrow().b2.clone();
            let _tmp_5247 = PptxShapeInk::rgbaOf(_tmp_5243, _tmp_5244, _tmp_5245, c.borrow().a2.clone());
            sh.borrow_mut().fillGrad.borrow_mut().c2 = _tmp_5247;
        } else {
            let mut _tmp_5248 = c.borrow().r.clone();
            let mut _tmp_5249 = c.borrow().g.clone();
            let mut _tmp_5250 = c.borrow().b.clone();
            let _tmp_5252 = PptxShapeInk::rgbaOf(_tmp_5248, _tmp_5249, _tmp_5250, c.borrow().a.clone());
            sh.borrow_mut().fill = _tmp_5252;
        }
        sh.borrow_mut().noLine = true;
        if c.borrow().hasShadow {
            sh.borrow_mut().shadow.borrow_mut().isSet = true;
            sh.borrow_mut().shadow.borrow_mut().offsetXPt = c.borrow().shadowX * self.unit;
            sh.borrow_mut().shadow.borrow_mut().offsetYPt = c.borrow().shadowY * self.unit;
            sh.borrow_mut().shadow.borrow_mut().blurPt = c.borrow().shadowBlur * self.unit;
            let mut _tmp_5256 = c.borrow().shadowR.clone();
            let mut _tmp_5257 = c.borrow().shadowG.clone();
            let _tmp_5259 = PptxShapeInk::rgbaOf(_tmp_5256, _tmp_5257, c.borrow().shadowB.clone(), 1.0);
            sh.borrow_mut().shadow.borrow_mut().color = _tmp_5259;
            sh.borrow_mut().shadow.borrow_mut().alpha = c.borrow().shadowA;
        }
        sh.borrow_mut().name = format!("Box {}", (group.borrow().children.len() as i64) + 1);
        self.attach(group, sh.clone());
    }
    /// An outline. One rectangle with a line and no fill rather than four
    /// segments: it is a quarter of the XML and it stays one thing to select.
    fn emitBorder(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, ox: f64, oy: f64) {
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().preset = "rect".to_string();
        if c.borrow().radius > 0.0 {
            sh.borrow_mut().preset = "roundRect".to_string();
        }
        sh.borrow_mut().x = ox + c.borrow().x * self.unit;
        sh.borrow_mut().y = oy + c.borrow().y * self.unit;
        sh.borrow_mut().width = c.borrow().w * self.unit;
        sh.borrow_mut().height = c.borrow().h * self.unit;
        sh.borrow_mut().noFill = true;
        let mut _tmp_5267 = c.borrow().r.clone();
        let mut _tmp_5268 = c.borrow().g.clone();
        let mut _tmp_5269 = c.borrow().b.clone();
        let _tmp_5271 = PptxShapeInk::rgbaOf(_tmp_5267, _tmp_5268, _tmp_5269, c.borrow().a.clone());
        sh.borrow_mut().line = _tmp_5271;
        sh.borrow_mut().lineWidthPt = c.borrow().thickness * self.unit;
        if sh.borrow().lineWidthPt < 0.25 {
            sh.borrow_mut().lineWidthPt = 0.25;
        }
        sh.borrow_mut().name = format!("Border {}", (group.borrow().children.len() as i64) + 1);
        self.attach(group, sh.clone());
    }
    /// A path or a stroke. `ringEnds` says where each ring stops, and one ring
    /// becomes one shape — so an even-odd hole is FILLED rather than cut, which
    /// is the one thing this conversion cannot say.
    fn emitRings(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, ox: f64, oy: f64, filled: bool) {
        let total: i64 = c.borrow().pts.len() as i64;
        if total < 4 {
            return;
        }
        let mut moved: Vec<f64> = Vec::new();
        let mut k: i64 = 0;
        while k < total {
            let px: f64 = c.borrow().pts[k as usize];
            let py: f64 = c.borrow().pts[(k + 1) as usize];
            moved.push(ox + px * self.unit);
            moved.push(oy + py * self.unit);
            k += 2;
        }
        let mut ends: Vec<i64> = c.borrow().ringEnds.clone();
        let count: i64 = ends.len() as i64;
        if filled && count > 1 {
            self.emitIslands(group, c, &moved, &ends);
            return;
        }
        let mut from: i64 = 0;
        let mut i: i64 = 0;
        while i < count {
            let upto: i64 = ends[i as usize];
            if upto - from >= 4 {
                let mut sh: Rc<RefCell<PptxShape>> = PptxShapeInk::ring(&moved, from, upto, filled);
                if filled {
                    let mut _tmp_5274 = c.borrow().r.clone();
                    let mut _tmp_5275 = c.borrow().g.clone();
                    let mut _tmp_5276 = c.borrow().b.clone();
                    let _tmp_5278 = PptxShapeInk::rgbaOf(_tmp_5274, _tmp_5275, _tmp_5276, c.borrow().a.clone());
                    sh.borrow_mut().fill = _tmp_5278;
                    sh.borrow_mut().noLine = true;
                } else {
                    sh.borrow_mut().noFill = true;
                    let mut _tmp_5279 = c.borrow().r.clone();
                    let mut _tmp_5280 = c.borrow().g.clone();
                    let mut _tmp_5281 = c.borrow().b.clone();
                    let _tmp_5283 = PptxShapeInk::rgbaOf(_tmp_5279, _tmp_5280, _tmp_5281, c.borrow().a.clone());
                    sh.borrow_mut().line = _tmp_5283;
                    sh.borrow_mut().lineWidthPt = c.borrow().thickness * self.unit;
                    if sh.borrow().lineWidthPt < 0.25 {
                        sh.borrow_mut().lineWidthPt = 0.25;
                    }
                }
                sh.borrow_mut().name = format!("Path {}", (group.borrow().children.len() as i64) + 1);
                self.attach(group, sh.clone());
            }
            from = upto;
            i += 1;
        }
        // A command with points and no ring ends is one ring.
        if count == 0 {
            let mut only: Rc<RefCell<PptxShape>> = PptxShapeInk::ring(&moved, 0, total, filled);
            if filled {
                let mut _tmp_5286 = c.borrow().r.clone();
                let mut _tmp_5287 = c.borrow().g.clone();
                let mut _tmp_5288 = c.borrow().b.clone();
                let _tmp_5290 = PptxShapeInk::rgbaOf(_tmp_5286, _tmp_5287, _tmp_5288, c.borrow().a.clone());
                only.borrow_mut().fill = _tmp_5290;
                only.borrow_mut().noLine = true;
            } else {
                only.borrow_mut().noFill = true;
                let mut _tmp_5291 = c.borrow().r.clone();
                let mut _tmp_5292 = c.borrow().g.clone();
                let mut _tmp_5293 = c.borrow().b.clone();
                let _tmp_5295 = PptxShapeInk::rgbaOf(_tmp_5291, _tmp_5292, _tmp_5293, c.borrow().a.clone());
                only.borrow_mut().line = _tmp_5295;
                only.borrow_mut().lineWidthPt = c.borrow().thickness * self.unit;
                if only.borrow().lineWidthPt < 0.25 {
                    only.borrow_mut().lineWidthPt = 0.25;
                }
            }
            only.borrow_mut().name = format!("Path {}", (group.borrow().children.len() as i64) + 1);
            self.attach(group, only.clone());
        }
    }
    /// A fill of several rings, as shapes that keep their holes: each ring that
    /// no other ring encloses is an outline, and the rings directly inside it
    /// go into the same shape as further contours (`pathEnds`). A glyph's
    /// counter — the inside of an o — is such a ring, and as a shape of its own
    /// it was drawn solid over the letter. Enclosure is by bounding box, which
    /// is what a glyph or a donut needs; a ring inside a hole is an outline
    /// again.
    fn emitIslands(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, moved: &[f64], ends: &[i64]) {
        let count: i64 = ends.len() as i64;
        let mut x0: Vec<f64> = Vec::new();
        let mut y0: Vec<f64> = Vec::new();
        let mut x1: Vec<f64> = Vec::new();
        let mut y1: Vec<f64> = Vec::new();
        let mut starts: Vec<i64> = Vec::new();
        let mut from: i64 = 0;
        let mut i: i64 = 0;
        while i < count {
            let upto: i64 = ends[i as usize];
            starts.push(from);
            let mut ax: f64 = 0.0;
            let mut ay: f64 = 0.0;
            let mut bx: f64 = 0.0;
            let mut by: f64 = 0.0;
            let mut k: i64 = from;
            while k + 1 < upto {
                let px: f64 = moved[k as usize];
                let py: f64 = moved[(k + 1) as usize];
                if k == from || px < ax {
                    ax = px;
                }
                if k == from || py < ay {
                    ay = py;
                }
                if k == from || px > bx {
                    bx = px;
                }
                if k == from || py > by {
                    by = py;
                }
                k += 2;
            }
            x0.push(ax);
            y0.push(ay);
            x1.push(bx);
            y1.push(by);
            from = upto;
            i += 1;
        }
        // the innermost ring around each, and how deep it sits
        let mut parent: Vec<i64> = Vec::new();
        let mut depth: Vec<i64> = Vec::new();
        i = 0;
        while i < count {
            let mut best: i64 = -1;
            let mut best_area: f64 = 0.0;
            let mut d: i64 = 0;
            let mut j: i64 = 0;
            while j < count {
                if j != i {
                    let inside: bool = (x0[j as usize] <= x0[i as usize] && y0[j as usize] <= y0[i as usize]) && (x1[j as usize] >= x1[i as usize] && y1[j as usize] >= y1[i as usize]);
                    let same: bool = (x0[j as usize] == x0[i as usize] && y0[j as usize] == y0[i as usize]) && (x1[j as usize] == x1[i as usize] && y1[j as usize] == y1[i as usize]);
                    if inside && false == same {
                        d += 1;
                        let area: f64 = (x1[j as usize] - x0[j as usize]) * (y1[j as usize] - y0[j as usize]);
                        if best < 0 || area < best_area {
                            best = j;
                            best_area = area;
                        }
                    }
                }
                j += 1;
            }
            parent.push(best);
            depth.push(d);
            i += 1;
        }
        i = 0;
        while i < count {
            if depth[i as usize] % 2 == 0 {
                let mut pts: Vec<f64> = Vec::new();
                let mut pe: Vec<i64> = Vec::new();
                self.appendRing(&mut pts,
                    moved,
                    starts[i as usize],
                    ends[i as usize]
                );
                pe.push(pts.len() as i64);
                let mut h: i64 = 0;
                while h < count {
                    if parent[h as usize] == i && depth[h as usize] % 2 == 1 {
                        self.appendRing(&mut pts,
                            moved,
                            starts[h as usize],
                            ends[h as usize]
                        );
                        pe.push(pts.len() as i64);
                    }
                    h += 1;
                }
                if (pts.len() as i64) >= 4 {
                    let mut sh: Rc<RefCell<PptxShape>> = PptxShapeInk::ring(&pts, 0, pts.len() as i64, true);
                    if (pe.len() as i64) > 1 {
                        sh.borrow_mut().pathEnds = pe.clone();
                    }
                    let mut _tmp_5298 = c.borrow().r.clone();
                    let mut _tmp_5299 = c.borrow().g.clone();
                    let mut _tmp_5300 = c.borrow().b.clone();
                    let _tmp_5302 = PptxShapeInk::rgbaOf(_tmp_5298, _tmp_5299, _tmp_5300, c.borrow().a.clone());
                    sh.borrow_mut().fill = _tmp_5302;
                    sh.borrow_mut().noLine = true;
                    sh.borrow_mut().name = format!("Path {}", (group.borrow().children.len() as i64) + 1);
                    self.attach(group, sh.clone());
                }
            }
            i += 1;
        }
    }
    pub fn appendRing(&self, mut out: &mut Vec<f64>, pts: &[f64], from: i64, upto: i64) {
        let mut k: i64 = from;
        while k < upto {
            out.push(pts[k as usize]);
            k += 1;
        }
    }
    /// A run of text.
    ///
    /// The display list's convention is that `y` is the TOP of the line box and
    /// `textAlign` names the horizontal anchor — and, by a quirk worth knowing
    /// before reading the display list, `textAlign == "italic"` is how a slanted
    /// run is marked. So the baseline word handed to `PptxShapeInk.label` is
    /// "top", and the alignment is read out separately.
    /// A picture, when the bytes can be found. `false` means they could not,
    /// and the caller counts it.
    fn emitPicture(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, ox: f64, oy: f64) -> bool {
        if false == self.haveImages {
            return false;
        }
        if c.borrow().src.is_empty() {
            return false;
        }
        let mut bytes: Vec<u8> = Vfs::read(&self.images, &c.borrow().src.clone());
        if bytes.is_empty() {
            return false;
        }
        let mut _st: Rc<RefCell<VfsStat>> = Vfs::stat(&self.images, &c.borrow().src);
        let mut pic: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        pic.borrow_mut().kind = PptxShapeKind::picture();
        pic.borrow_mut().name = format!("Picture {}", (group.borrow().children.len() as i64) + 1);
        pic.borrow_mut().hasXfrm = true;
        pic.borrow_mut().x = ox + c.borrow().x * self.unit;
        pic.borrow_mut().y = oy + c.borrow().y * self.unit;
        pic.borrow_mut().width = c.borrow().w * self.unit;
        pic.borrow_mut().height = c.borrow().h * self.unit;
        pic.borrow_mut().imageBytes = bytes.clone();
        pic.borrow_mut().imagePart = c.borrow().src.clone();
        pic.borrow_mut().imageOk = true;
        // a picture shown in part (a crop window over its pixels)
        if c.borrow().hasCrop {
            pic.borrow_mut().cropL = c.borrow().cropX;
            pic.borrow_mut().cropT = c.borrow().cropY;
            pic.borrow_mut().cropR = (1.0 - c.borrow().cropX) - c.borrow().cropW;
            pic.borrow_mut().cropB = (1.0 - c.borrow().cropY) - c.borrow().cropH;
        }
        // Alt text, because a picture is the one shape where the pixels tell a
        // screen reader nothing — `PptxA11y` says "No alt text" for exactly
        // this case, and a deck built from a document that HAD alt text should
        // not be the one that loses it.
        pic.borrow_mut().altText = c.borrow().text.clone();
        group.borrow_mut().children.push(pic.clone());
        self.shapeCount += 1;
        self.placedImages += 1;
        true
    }
    fn emitText(&mut self, group: &Rc<RefCell<PptxShape>>, c: &Rc<RefCell<EVGDrawCmd>>, ox: f64, oy: f64) {
        let body: String = c.borrow().text.clone();
        if body.is_empty() {
            return;
        }
        let size: f64 = c.borrow().fontSize * self.unit;
        if size < self.minFontPt {
            return;
        }
        let mut align: String = "left";
        if c.borrow().textAlign == "center" {
            align = "center";
        }
        if c.borrow().textAlign == "right" {
            align = "right";
        }
        let mut family: String = c.borrow().fontFamily.clone();
        if family.is_empty() {
            family = self.sansFamily.clone();
        }
        if family == "sans-serif" {
            family = self.sansFamily.clone();
        }
        let bold: bool = c.borrow().fontWeight == "bold";
        let mut _tmp_5318 = c.borrow().r.clone();
        let mut _tmp_5319 = c.borrow().g.clone();
        let mut _tmp_5320 = c.borrow().b.clone();
        let mut _tmp_5321 = c.borrow().a.clone();
        let mut ink: Rc<RefCell<PptxColor>> = PptxShapeInk::rgbaOf(
            _tmp_5318,
            _tmp_5319,
            _tmp_5320,
            _tmp_5321
        );
        let ax: f64 = ox + c.borrow().x * self.unit;
        let ay: f64 = oy + c.borrow().y * self.unit;
        // A turned label turns about the point the DOCUMENT said, which is
        // `transform-origin` and is rarely where the ink starts: an axis label
        // is pinned where it meets its tick. Rotating about the ink's own
        // corner instead swung the category names up into the plot, where the
        // bars drew over them.
        let mut px: f64 = ax;
        let mut py: f64 = ay;
        if c.borrow().hasRotOrigin {
            px = ox + c.borrow().rotOriginX * self.unit;
            py = oy + c.borrow().rotOriginY * self.unit;
        }
        let mut _tmp_5322 = c.borrow().rotate.clone();
        let mut sh: Rc<RefCell<PptxShape>> = PptxShapeInk::labelAbout(
            body.clone(),
            size,
            family.clone(),
            bold,
            align,
            "top",
            _tmp_5322,
            ax,
            ay,
            px,
            py,
            ink.clone()
        );
        if c.borrow().textAlign == "italic" {
            let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow().text.borrow().paragraphs[0].clone();
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[0].clone();
            run.borrow_mut().italicSet = true;
            run.borrow_mut().italic = true;
        }
        sh.borrow_mut().name = format!("Text {}", (group.borrow().children.len() as i64) + 1);
        self.textCount += 1;
        self.attach(group, sh.clone());
    }
    pub fn attach(&mut self, group: &Rc<RefCell<PptxShape>>, sh: Rc<RefCell<PptxShape>>) {
        group.borrow_mut().children.push(sh.clone());
        self.shapeCount += 1;
    }
}
