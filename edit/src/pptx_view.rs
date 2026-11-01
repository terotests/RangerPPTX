// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pptx::pptx_model::PptxPresentation;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxSlide;
use pptx::pptx_to_evg::PptxToEvg;
use pptx::rgr::EVGColor;
use pptx::rgr::EVGDisplayList;
use pptx::rgr::EVGDrawCmd;
use pptx::rgr::OfficeTextMeasure;
use crate::rgr::{EVGImageDecode, ImageBuffer};
use pptx::rgr::{EVGColor, EVGDisplayList, EVGDrawCmd};


/// ==============================================================================
/// PptxView - SoftCanvas / WebGL present for a resolved slide
/// ==============================================================================
pub struct PptxView {
    canvas: Rc<RefCell<SoftCanvas>>,
    ctx: Rc<RefCell<UIContext>>,
    pub converter: Rc<RefCell<PptxToEvg>>,
    pub pageW: i64,
    pub pageH: i64,
    /// Room the frame takes out of the window, so the fit-to-window scale
    /// leaves it clear. `PptxApp` sets it — it is the one that knows how tall
    /// the shared strip came out at this width.
    pub chromeH: i64,
    /// Room the slide panel takes out of the width, the same way `chromeH` is
    /// the room the toolbar and the status line take out of the height.
    pub chromeW: i64,
    pub fontFamily: String,
    pub status: String,
    pub deckName: String,
    pub slideIndex: i64,
    pub slideCount: i64,
    /// imagePart → decoded RGBA (filled by PptxApp after load)
    imageCache: HashMap<String,Rc<RefCell<ImageBuffer>>>,
}

impl PptxView {
    pub fn new() -> Self {
        Self {
            canvas: Rc::new(RefCell::new(SoftCanvas::new())),
            ctx: Rc::new(RefCell::new(UIContext::new())),
            converter: Rc::new(RefCell::new(PptxToEvg::new())),
            pageW: 960,
            pageH: 720,
            chromeH: 28,
            chromeW: 0,
            fontFamily: "Open Sans".to_string(),
            status: String::new(),
            deckName: String::new(),
            slideIndex: 0,
            slideCount: 0,
            imageCache: HashMap::new(),
        }
    }
    /// A rectangle turned about its own centre, as four points. The GPU backend
    /// rotates a quad in the shader; the CPU one has a polygon filler and had
    /// been ignoring `rotate` entirely, so a shape you turned in the editor
    /// drew straight here and turned in the browser.
    pub fn rotatedCorners(x: f64, y: f64, w: f64, h: f64, degrees: f64) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        let cx: f64 = x + w * 0.5;
        let cy: f64 = y + h * 0.5;
        let rad: f64 = degrees * 0.017453292519943295;
        let cs: f64 = rad.cos();
        let sn: f64 = rad.sin();
        let mut xs: Vec<f64> = vec![0.0, 1.0, 1.0, 0.0];
        let mut ys: Vec<f64> = vec![0.0, 0.0, 1.0, 1.0];
        let mut i: i64 = 0;
        while i < 4 {
            let px: f64 = (x + xs[i as usize] * w) - cx;
            let py: f64 = (y + ys[i as usize] * h) - cy;
            out.push(cx + (px * cs - py * sn));
            out.push(cy + (px * sn + py * cs));
            i += 1;
        }
        out
    }
    /// One edge of a turned outline, as a quad through the polygon filler.
    fn strokeSegment(ctx: &Rc<RefCell<UIContext>>, x0: f64, y0: f64, x1: f64, y1: f64, thickness: f64, r: i64, g: i64, b: i64, a: i64) {
        let dx: f64 = x1 - x0;
        let dy: f64 = y1 - y0;
        let __len: f64 = (dx * dx + dy * dy).sqrt();
        if __len < 0.0001 {
            return;
        }
        let mut t: f64 = thickness;
        if t < 1.0 {
            t = 1.0;
        }
        let nx: f64 = ((0.0 - dy) / __len) * (t * 0.5);
        let ny: f64 = (dx / __len) * (t * 0.5);
        let mut pts: Vec<f64> = Vec::new();
        pts.push(x0 + nx);
        pts.push(y0 + ny);
        pts.push(x1 + nx);
        pts.push(y1 + ny);
        pts.push(x1 - nx);
        pts.push(y1 - ny);
        pts.push(x0 - nx);
        pts.push(y0 - ny);
        UIContext::fillPolygon(&ctx, &pts, r, g, b, a);
    }
    /// --- putting a float rectangle on the pixel grid --------------------------
    ///
    /// THE EDGES ARE WHAT GETS ROUNDED, not the origin and the size separately.
    /// Those are different operations and this file did the second one for
    /// years: `fillRect((PptxView.snapEdge(c.x)) (PptxView.snapEdge(c.y)) (PptxView.snapSpan(c.x c.w)) (PptxView.snapSpan(c.y c.h)))`
    /// truncates a position and a length independently, which loses up to two
    /// pixels off the right of a shape and none off its left.
    ///
    /// The blue rounded box on slide 4 of `20-business-deck.pptx` is the case
    /// that showed it. Its command is x=208.65 w=175.95, so its true right edge
    /// is at 384.6; truncation painted it to 383. The shape's own OUTLINE is not
    /// a rectangle — a rounded box is stroked as a polygon, at full precision —
    /// so the outline landed where the geometry said, on 384 and 385, and pixel
    /// 383 was left as whatever was behind. A hairline of paper between a box
    /// and its own border, which is what it was reported as.
    ///
    /// It can only ever appear on the RIGHT and the BOTTOM, because truncation
    /// only ever loses length off the far edge. That asymmetry is how the bug
    /// was recognised in a photograph before it was found in the code.
    ///
    /// Rounding both edges also makes adjacent rectangles tile: two boxes that
    /// share an edge in float space share it on the grid, with no seam between
    /// them and no doubled pixel either.
    pub fn snapEdge(v: f64) -> i64 {
        if v < 0.0 {
            return 0 - (((0.0 - v) + 0.5).floor() as i64);
        }
        (v + 0.5).floor() as i64
    }
    /// The width a rectangle has once BOTH its edges are on the grid. Never the
    /// size rounded on its own — that is the bug above, spelled with `round`
    /// instead of `to_int`.
    pub fn snapSpan(v: f64, size: f64) -> i64 {
        PptxView::snapEdge(v + size) - PptxView::snapEdge(v)
    }
    /// Everything `init` does except read font files, which a browser cannot do.
    pub fn initBare(&mut self, w: i64, h: i64) {
        self.pageW = w;
        self.pageH = h;
        SoftCanvas::init(&self.canvas.clone(), w, h);
        let mut _tmp_5928 = self.canvas.clone();
        UIContext::attach(&self.ctx.clone(), _tmp_5928);
        self.attachMeasurer();
        self.status = "no fonts yet".to_string();
    }
    pub fn init(&mut self, w: i64, h: i64, font_dir: &str) {
        self.pageW = w;
        self.pageH = h;
        SoftCanvas::init(&self.canvas.clone(), w, h);
        let mut _tmp_5933 = self.canvas.clone();
        UIContext::attach(&self.ctx.clone(), _tmp_5933);
        UITextRenderer::addFontDir(&self.ctx.borrow().tr.clone(), font_dir);
        let mut _tmp_5936 = self.fontFamily.clone();
        let ok: bool = UITextRenderer::loadFont(
            &self.ctx.borrow().tr,
            _tmp_5936,
            "Open_Sans/OpenSans-Regular.ttf"
        );
        if !ok {
            self.status = "bitmap-font fallback".to_string();
        } else {
            self.status = "ready".to_string();
        }
        let mut _tmp_5937 = self.fontFamily.clone();
        UITextRenderer::loadFont(&self.ctx.borrow().tr.clone(), _tmp_5937, "Open_Sans/OpenSans-Bold.ttf");
        let mut _tmp_5939 = self.fontFamily.clone();
        UITextRenderer::loadFont(&self.ctx.borrow().tr.clone(), _tmp_5939, "Open_Sans/OpenSans-Italic.ttf");
        let mut _tmp_5941 = self.fontFamily.clone();
        UITextRenderer::loadFont(&self.ctx.borrow().tr.clone(), _tmp_5941, "Open_Sans/OpenSans-BoldItalic.ttf");
        // Emoji. A text face has none of them, and a codepoint the face cannot
        // draw comes out as `.notdef` — a filled circle, which is what a deck
        // with a 😆 in it was showing. `FontManager` already falls back per
        // CODEPOINT across every loaded face, so having the glyphs loaded at
        // all is the whole fix. Noto Emoji is the monochrome one: black and
        // white, which reads correctly at any size and costs nothing to
        // rasterize through the same path as every other glyph.
        UITextRenderer::addFace(&self.ctx.borrow().tr.clone(), "Noto_Emoji/NotoEmoji-Regular.ttf");
        // The same problem one block down the codepoint chart: a bullet is a
        // geometric shape — ● ○ ■ ▪ — and Open Sans has none of them, so every
        // list in every deck drew a row of empty boxes down its left edge. Noto
        // Sans carries them (and the dashes and guillemets a deck reaches for
        // when it runs out of circles), and joins the fallback pool without
        // becoming the face anything is drawn in.
        UITextRenderer::addFace(&self.ctx.borrow().tr.clone(), "Noto_Sans/NotoSans-Regular.ttf");
        // Arabic. Nothing loaded here had a single Arabic glyph in it, so a
        // slide in Arabic drew a row of .notdef boxes — and it would have
        // gone on doing that however correct the bidirectional algorithm and
        // the shaper above it were. El Messiri carries the base block AND the
        // joining forms the shaper emits, which is why it is the one chosen:
        // without a GSUB shaper here, a face with only the base block would
        // still have drawn every letter unjoined.
        //
        // It joins the fallback pool per CODEPOINT, like the emoji and the
        // bullets, so it never becomes the face Latin text is drawn in — a
        // mixed line takes its Latin from the document's own face and its
        // Arabic from here, which is what a reader expects and what Word does.
        UITextRenderer::addFace(&self.ctx.borrow().tr.clone(), "El_Messiri/ElMessiri-Regular.ttf");
        UITextRenderer::addFace(&self.ctx.borrow().tr.clone(), "El_Messiri/ElMessiri-Bold.ttf");
        self.ctx.borrow_mut().tr.borrow_mut().fontFamily = self.fontFamily.clone();
        UITextRenderer::applyFace(&self.ctx.borrow().tr.clone());
        self.attachMeasurer();
    }
    /// The painter's own text renderer, handed to the layout: from here on the
    /// wrap, the caret and the click all measure with the font that will draw.
    pub fn attachMeasurer(&self) {
        let mut _tmp_5949 = self.ctx.clone();
        let mut _tmp_5950 = self.fontFamily.clone();
        OfficeTextMeasure::attach(&self.converter.borrow().measure.clone(), _tmp_5949, _tmp_5950);
    }
    pub fn clearImages(&mut self) {
        let mut empty: HashMap<String,Rc<RefCell<ImageBuffer>>> = HashMap::new();
        self.imageCache = empty.clone();
    }
    pub fn cacheImage(&mut self, part: &str, bytes: &[u8]) {
        if part.is_empty() {
            return;
        }
        let mut existing: Option<Rc<RefCell<ImageBuffer>>> = self.imageCache.get(part)
            .cloned();
        if existing.is_some() {
            return;
        }
        let mut decoded: Option<Rc<RefCell<ImageBuffer>>> = EVGImageDecode::decode(bytes);
        if decoded.is_some() {
            self.imageCache.insert(part.to_string(), decoded.clone().unwrap());
        }
    }
    pub fn cacheShapeImages(&mut self, shape: &Rc<RefCell<PptxShape>>) {
        if shape.borrow().kind == PptxShapeKind::picture() {
            if !shape.borrow().imageBytes.is_empty() {
                let mut _tmp_5952 = shape.borrow().imagePart.clone();
                self.cacheImage(&_tmp_5952, &shape.borrow().imageBytes.clone());
                let mut hit: Option<Rc<RefCell<ImageBuffer>>> = self.imageCache.get(&shape.borrow().imagePart)
                    .cloned();
                if hit.is_some() {
                    shape.borrow_mut().imageOk = true;
                }
            }
        }
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            self.cacheShapeImages(&shape.borrow().children[i as usize].clone().clone());
            i += 1;
        }
    }
    pub fn cachePresentationImages(&mut self, pres: &Rc<RefCell<PptxPresentation>>) {
        self.clearImages();
        let mut si: i64 = 0;
        while si < (pres.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = pres.borrow().slides[si as usize].clone();
            let mut j: i64 = 0;
            while j < (slide.borrow().shapes.len() as i64) {
                self.cacheShapeImages(&slide.borrow().shapes[j as usize].clone().clone());
                j += 1;
            }
            si += 1;
        }
    }
    pub fn resize(&mut self, w: i64, h: i64) {
        self.pageW = w;
        self.pageH = h;
        // the canvas of that size is kept: making it again cleared nothing a
        // frame does not paint over, and every app that starts (initBare,
        // then resize to the same size) made two (a deck laid out on a
        // server, which never draws, spent a tenth of its time zeroing them)
        if w != SoftCanvas::getWidth(&self.canvas) || h != SoftCanvas::getHeight(&self.canvas) {
            SoftCanvas::init(&self.canvas.clone(), w, h);
            let mut _tmp_5959 = self.canvas.clone();
            UIContext::attach(&self.ctx.clone(), _tmp_5959);
        }
        UITextRenderer::clearCache(&self.ctx.borrow().tr.clone());
    }
    pub fn raw(&self) -> Vec<u8> {
        return SoftCanvas::raw(&self.canvas);
    }
    pub fn fitScale(&self, slide: &Rc<RefCell<PptxSlide>>) -> f64 {
        let mut _tmp_5963 = slide.borrow().width.clone();
        return self.fitScaleFor(_tmp_5963, slide.borrow().height.clone());
    }
    /// Fitting a page into a window is a question about the page's SIZE, not
    /// about its format. The .odp viewer asks the same question of the same
    /// window, so the arithmetic is here once rather than in each reader.
    pub fn fitScaleFor(&self, width_pt: f64, height_pt: f64) -> f64 {
        let avail_w: f64 = (self.pageW - self.chromeW) as f64;
        let avail_h: f64 = (self.pageH - self.chromeH) as f64;
        let slide_wpx: f64 = width_pt * (96.0 / 72.0);
        let slide_hpx: f64 = height_pt * (96.0 / 72.0);
        if slide_wpx <= 0.0 {
            return 1.0;
        }
        if slide_hpx <= 0.0 {
            return 1.0;
        }
        let sx: f64 = avail_w / slide_wpx;
        let sy: f64 = avail_h / slide_hpx;
        if sx < sy {
            return sx;
        }
        sy
    }
    /// The slide, and only the slide.
    ///
    /// There used to be a hand-built chrome band on the end of this — a dark
    /// rectangle and a string of the deck name, the slide numbers and a pair of
    /// arrow characters, all assembled out of raw EVGDrawCmds here. That was a
    /// third toolbar, in a third style, that nothing else could use and nothing
    /// else could fix. `PptxApp` puts the shared strip around this now, the same
    /// one the spreadsheet and the document have.
    fn buildDisplayList(&self, slide: &Rc<RefCell<PptxSlide>>) -> Rc<RefCell<EVGDisplayList>> {
        return self.buildDisplayListAt(slide, self.fitScale(slide));
    }
    /// The same scene at a scale the caller chose. A show fits the slide into
    /// whatever the window leaves — which is not what `chromeW` / `chromeH`
    /// describe — and a picture drawn at one scale under an origin computed at
    /// another is a slide that runs off the side of its own frame.
    fn buildDisplayListAt(&self, slide: &Rc<RefCell<PptxSlide>>, scale: f64) -> Rc<RefCell<EVGDisplayList>> {
        self.converter.borrow_mut().setScale(scale);
        self.converter.borrow_mut().slideNumber = self.slideIndex + 1;
        return self.converter.borrow_mut().slideToDisplayList(slide);
    }
    fn paintTextCmd(&self, c: &Rc<RefCell<EVGDrawCmd>>) {
        let mut _tmp_5970 = c.borrow().a.clone();
        let mut col: Rc<RefCell<EVGColor>> = EVGColor::create(
            c.borrow().r as f64,
            c.borrow().g as f64,
            c.borrow().b as f64,
            _tmp_5970
        );
        let prev_family: String = self.ctx.borrow().tr.borrow().fontFamily.clone();
        // The face for this command, asked of the same measurer the layout
        // used. It used to be four lines that named "Open Sans" whatever the
        // command said, which was safe only for as long as the widths above
        // came from one family too — they no longer do, and a run measured in
        // Noto Sans and painted in Open Sans is a caret that drifts across the
        // line. `faceForIn` runs the family through the same alias map and the
        // same is-it-loaded check, so the two cannot come apart.
        //
        // Two fields, not one: a bold run says so in `fontWeight`, and the
        // slant is written into `textAlign` — the convention the document
        // viewer set and every backend here reads.
        let mut want_bold: bool = false;
        let mut want_italic: bool = false;
        if c.borrow().fontWeight == "bold" {
            want_bold = true;
        }
        if c.borrow().fontWeight == "italic" {
            want_italic = true;
        }
        if c.borrow().fontWeight == "bolditalic" {
            want_bold = true;
            want_italic = true;
        }
        if c.borrow().textAlign == "italic" {
            want_italic = true;
        }
        let mut _tmp_5971 = c.borrow().fontFamily.clone();
        let _tmp_5972 = OfficeTextMeasure::faceForIn(
            &self.converter.borrow().measure,
            &_tmp_5971,
            want_bold,
            want_italic
        );
        self.ctx.borrow_mut().tr.borrow_mut().fontFamily = _tmp_5972;
        let mut x: f64 = c.borrow().x;
        let y: f64 = c.borrow().y;
        let size: f64 = c.borrow().fontSize;
        let mut turned_text: bool = false;
        if c.borrow().rotate > 0.001 || c.borrow().rotate < -0.001 {
            turned_text = true;
        }
        if !c.borrow().textAlign.is_empty() {
            if c.borrow().w > 0.0 {
                let mw: f64 = UIContext::measureWidth(&self.ctx, &c.borrow().text.clone(), size);
                if c.borrow().textAlign == "center" {
                    x = c.borrow().x + (c.borrow().w - mw) * 0.5;
                }
                if c.borrow().textAlign == "right" {
                    x = c.borrow().x + (c.borrow().w - mw);
                }
            }
        }
        if turned_text {
            let mut _tmp_5976 = c.borrow().text.clone();
            let mut _tmp_5977 = c.borrow().rotate.clone();
            let _tmp_5978 = self.ctx.clone();
            UIContext::textRotated(
                &_tmp_5978,
                &_tmp_5976,
                x.floor() as i64,
                y.floor() as i64,
                size,
                &col,
                _tmp_5977
            );
        } else {
            if c.borrow().maxWidth > 8.0 {
                let mut _tmp_5980 = c.borrow().text.clone();
                let mut _tmp_5981 = c.borrow().maxWidth.clone();
                let _tmp_5982 = self.ctx.clone();
                UIContext::textWrapped(
                    &_tmp_5982,
                    &_tmp_5980,
                    x.floor() as i64,
                    y.floor() as i64,
                    size,
                    _tmp_5981,
                    &col
                );
            } else {
                let mut _tmp_5984 = c.borrow().text.clone();
                UIContext::text(&self.ctx.clone(), &_tmp_5984, x.floor() as i64, y.floor() as i64, size, &col);
            }
        }
        self.ctx.borrow_mut().tr.borrow_mut().fontFamily = prev_family.clone();
    }
    /// THE CORNER RADIUS IS STILL TRUNCATED, and that is on purpose. Snapping
    /// the edges leaves exactly one pixel of background at the top of a rounded
    /// box's corner arc, and rounding the radius the same way looks like the
    /// obvious next move: 19.55 truncates to 19 while the outline curves on the
    /// real 19.55, so the fill's corner is a fraction tighter than the ring
    /// around it.
    ///
    /// It was tried and measured and it is WORSE — two stray pixels instead of
    /// one, because a larger integer radius pulls the arc past the polygon on
    /// the other side of the join. The remaining pixel is not an offset that a
    /// different rounding would cure; it is a filled arc of integer radius and a
    /// six-chords-per-corner polygon disagreeing about coverage where they meet,
    /// and the honest fix for that is to rasterise the corner by coverage rather
    /// than to pick a better integer. One pixel, in one corner of one shape, was
    /// not worth that change.
    fn paintSoftCanvas(&self, dl: &Rc<RefCell<EVGDisplayList>>) {
        SoftCanvas::clear(&self.canvas, 40, 44, 58);
        let mut i: i64 = 0;
        let n: i64 = EVGDisplayList::count(&dl);
        while i < n {
            let mut c: Rc<RefCell<EVGDrawCmd>> = EVGDisplayList::at(&dl, i);
            if c.borrow().kind == 0 {
                let mut turned: bool = false;
                if c.borrow().rotate > 0.001 || c.borrow().rotate < -0.001 {
                    if !c.borrow().hasGrad {
                        if c.borrow().a > 0.001 {
                            turned = true;
                        }
                    }
                }
                if turned {
                    let mut _tmp_5986 = c.borrow().x.clone();
                    let mut _tmp_5987 = c.borrow().y.clone();
                    let mut _tmp_5988 = c.borrow().w.clone();
                    let mut _tmp_5989 = c.borrow().h.clone();
                    let mut _tmp_5990 = c.borrow().rotate.clone();
                    let mut corners: Vec<f64> = PptxView::rotatedCorners(
                        _tmp_5986,
                        _tmp_5987,
                        _tmp_5988,
                        _tmp_5989,
                        _tmp_5990
                    );
                    let mut _tmp_5992 = c.borrow().r.clone();
                    let mut _tmp_5993 = c.borrow().g.clone();
                    let mut _tmp_5994 = c.borrow().b.clone();
                    let arg_0 = (c.borrow().a * 255.0).floor() as i64;
                    UIContext::fillPolygon(&self.ctx.clone(), &corners, _tmp_5992, _tmp_5993, _tmp_5994, arg_0);
                }
                if c.borrow().hasShadow {
                    let mut _tmp_5997 = c.borrow().shadowR.clone();
                    let mut _tmp_5998 = c.borrow().shadowG.clone();
                    let mut _tmp_5999 = c.borrow().shadowB.clone();
                    let mut _tmp_6000 = c.borrow().x.clone();
                    let mut _tmp_6001 = c.borrow().y.clone();
                    let mut _tmp_6002 = c.borrow().x.clone();
                    let mut _tmp_6003 = c.borrow().w.clone();
                    let mut _tmp_6004 = c.borrow().y.clone();
                    let mut _tmp_6005 = c.borrow().h.clone();
                    let arg_0 = PptxView::snapEdge(_tmp_6000);
                    let arg_1 = PptxView::snapEdge(_tmp_6001);
                    let arg_2 = PptxView::snapSpan(_tmp_6002, _tmp_6003);
                    let arg_3 = PptxView::snapSpan(_tmp_6004, _tmp_6005);
                    let arg_4 = c.borrow().radius.floor() as i64;
                    let arg_5 = c.borrow().shadowX.floor() as i64;
                    let arg_6 = c.borrow().shadowY.floor() as i64;
                    let arg_7 = c.borrow().shadowBlur.floor() as i64;
                    let arg_8 = (c.borrow().shadowA * 255.0).floor() as i64;
                    let _tmp_6006 = self.ctx.clone();
                    UIContext::shadowRoundRect(
                        &_tmp_6006,
                        arg_0,
                        arg_1,
                        arg_2,
                        arg_3,
                        arg_4,
                        arg_5,
                        arg_6,
                        arg_7,
                        _tmp_5997,
                        _tmp_5998,
                        _tmp_5999,
                        arg_8
                    );
                }
                if turned {
                    i += 1;
                    continue;
                }
                if c.borrow().hasGrad {
                    let mut _tmp_6008 = c.borrow().gradDir.clone();
                    let mut _tmp_6009 = c.borrow().r.clone();
                    let mut _tmp_6010 = c.borrow().g.clone();
                    let mut _tmp_6011 = c.borrow().b.clone();
                    let mut _tmp_6012 = c.borrow().r2.clone();
                    let mut _tmp_6013 = c.borrow().g2.clone();
                    let mut _tmp_6014 = c.borrow().b2.clone();
                    let mut _tmp_6015 = c.borrow().x.clone();
                    let mut _tmp_6016 = c.borrow().y.clone();
                    let mut _tmp_6017 = c.borrow().x.clone();
                    let mut _tmp_6018 = c.borrow().w.clone();
                    let mut _tmp_6019 = c.borrow().y.clone();
                    let mut _tmp_6020 = c.borrow().h.clone();
                    let arg_0 = PptxView::snapEdge(_tmp_6015);
                    let arg_1 = PptxView::snapEdge(_tmp_6016);
                    let arg_2 = PptxView::snapSpan(_tmp_6017, _tmp_6018);
                    let arg_3 = PptxView::snapSpan(_tmp_6019, _tmp_6020);
                    let arg_4 = c.borrow().radius.floor() as i64;
                    let arg_5 = (c.borrow().a * 255.0).floor() as i64;
                    let arg_6 = (c.borrow().a2 * 255.0).floor() as i64;
                    let _tmp_6021 = self.ctx.clone();
                    UIContext::fillRoundRectGrad(
                        &_tmp_6021,
                        arg_0,
                        arg_1,
                        arg_2,
                        arg_3,
                        arg_4,
                        _tmp_6008,
                        _tmp_6009,
                        _tmp_6010,
                        _tmp_6011,
                        arg_5,
                        _tmp_6012,
                        _tmp_6013,
                        _tmp_6014,
                        arg_6
                    );
                } else {
                    if c.borrow().a > 0.001 {
                        let mut _tmp_6022 = c.borrow().a.clone();
                        let mut col: Rc<RefCell<EVGColor>> = EVGColor::create(
                            c.borrow().r as f64,
                            c.borrow().g as f64,
                            c.borrow().b as f64,
                            _tmp_6022
                        );
                        if c.borrow().radius > 0.5 {
                            let mut _tmp_6024 = c.borrow().x.clone();
                            let mut _tmp_6025 = c.borrow().y.clone();
                            let mut _tmp_6026 = c.borrow().x.clone();
                            let mut _tmp_6027 = c.borrow().w.clone();
                            let mut _tmp_6028 = c.borrow().y.clone();
                            let mut _tmp_6029 = c.borrow().h.clone();
                            let arg_0 = PptxView::snapEdge(_tmp_6024);
                            let arg_1 = PptxView::snapEdge(_tmp_6025);
                            let arg_2 = PptxView::snapSpan(_tmp_6026, _tmp_6027);
                            let arg_3 = PptxView::snapSpan(_tmp_6028, _tmp_6029);
                            let arg_4 = c.borrow().radius.floor() as i64;
                            let _tmp_6030 = self.ctx.clone();
                            UIContext::fillRoundRect(
                                &_tmp_6030,
                                arg_0,
                                arg_1,
                                arg_2,
                                arg_3,
                                arg_4,
                                &col
                            );
                        } else {
                            let mut _tmp_6032 = c.borrow().x.clone();
                            let mut _tmp_6033 = c.borrow().y.clone();
                            let mut _tmp_6034 = c.borrow().x.clone();
                            let mut _tmp_6035 = c.borrow().w.clone();
                            let mut _tmp_6036 = c.borrow().y.clone();
                            let mut _tmp_6037 = c.borrow().h.clone();
                            let arg_0 = PptxView::snapEdge(_tmp_6032);
                            let arg_1 = PptxView::snapEdge(_tmp_6033);
                            let arg_2 = PptxView::snapSpan(_tmp_6034, _tmp_6035);
                            let arg_3 = PptxView::snapSpan(_tmp_6036, _tmp_6037);
                            UIContext::fillRect(&self.ctx.clone(), arg_0, arg_1, arg_2, arg_3, &col);
                        }
                    }
                }
            }
            if c.borrow().kind == 1 {
                let mut _tmp_6039 = c.borrow().a.clone();
                let mut col2: Rc<RefCell<EVGColor>> = EVGColor::create(
                    c.borrow().r as f64,
                    c.borrow().g as f64,
                    c.borrow().b as f64,
                    _tmp_6039
                );
                if c.borrow().rotate > 0.001 || c.borrow().rotate < -0.001 {
                    // A turned outline has no axis-aligned edges: four quads
                    // through the polygon filler, the same way the editor draws
                    // a turned selection.
                    let mut _tmp_6040 = c.borrow().x.clone();
                    let mut _tmp_6041 = c.borrow().y.clone();
                    let mut _tmp_6042 = c.borrow().w.clone();
                    let mut _tmp_6043 = c.borrow().h.clone();
                    let mut _tmp_6044 = c.borrow().rotate.clone();
                    let mut corners_1: Vec<f64> = PptxView::rotatedCorners(
                        _tmp_6040,
                        _tmp_6041,
                        _tmp_6042,
                        _tmp_6043,
                        _tmp_6044
                    );
                    let mut e: i64 = 0;
                    while e < 4 {
                        let f: i64 = (e + 1) % 4;
                        let mut _tmp_6045 = self.ctx.clone();
                        let mut _tmp_6046 = c.borrow().thickness.clone();
                        let mut _tmp_6047 = c.borrow().r.clone();
                        let mut _tmp_6048 = c.borrow().g.clone();
                        let mut _tmp_6049 = c.borrow().b.clone();
                        PptxView::strokeSegment(
                            &_tmp_6045,
                            corners_1[(e * 2) as usize],
                            corners_1[(e * 2 + 1) as usize],
                            corners_1[(f * 2) as usize],
                            corners_1[(f * 2 + 1) as usize],
                            _tmp_6046,
                            _tmp_6047,
                            _tmp_6048,
                            _tmp_6049,
                            (c.borrow().a * 255.0).floor() as i64
                        );
                        e += 1;
                    }
                } else {
                    let mut _tmp_6051 = c.borrow().x.clone();
                    let mut _tmp_6052 = c.borrow().y.clone();
                    let mut _tmp_6053 = c.borrow().x.clone();
                    let mut _tmp_6054 = c.borrow().w.clone();
                    let mut _tmp_6055 = c.borrow().y.clone();
                    let mut _tmp_6056 = c.borrow().h.clone();
                    let arg_0 = PptxView::snapEdge(_tmp_6051);
                    let arg_1 = PptxView::snapEdge(_tmp_6052);
                    let arg_2 = PptxView::snapSpan(_tmp_6053, _tmp_6054);
                    let arg_3 = PptxView::snapSpan(_tmp_6055, _tmp_6056);
                    let arg_4 = c.borrow().thickness.floor() as i64;
                    UIContext::strokeRect(&self.ctx.clone(), arg_0, arg_1, arg_2, arg_3, arg_4, &col2);
                }
            }
            if c.borrow().kind == 3 {
                self.paintTextCmd(&c);
            }
            if c.borrow().kind == 2 {
                let mut img: Option<Rc<RefCell<ImageBuffer>>> = self.imageCache.get(&c.borrow().src)
                    .cloned();
                if img.is_some() {
                    let mut pixels: Rc<RefCell<ImageBuffer>> = img.clone().unwrap();
                    if pixels.borrow().width > 0 && pixels.borrow().height > 0 {
                        let mut turned_pic: bool = false;
                        if c.borrow().rotate > 0.001 || c.borrow().rotate < -0.001 {
                            turned_pic = true;
                        }
                        if turned_pic {
                            // The rotated blit takes the pivot rather than a
                            // corner, and cannot mirror as well — a picture
                            // that is both turned and mirrored is drawn turned.
                            let pivot_x: i64 = (c.borrow().x + c.borrow().w * 0.5).floor() as i64;
                            let pivot_y: i64 = (c.borrow().y + c.borrow().h * 0.5).floor() as i64;
                            let mut _tmp_6059 = pixels.borrow().width.clone();
                            let mut _tmp_6060 = pixels.borrow().height.clone();
                            let mut _tmp_6061 = c.borrow().rotate.clone();
                            let mut _tmp_6062 = c.borrow().x.clone();
                            let mut _tmp_6063 = c.borrow().w.clone();
                            let mut _tmp_6064 = c.borrow().y.clone();
                            let mut _tmp_6065 = c.borrow().h.clone();
                            let arg_0 = PptxView::snapSpan(_tmp_6062, _tmp_6063);
                            let arg_1 = PptxView::snapSpan(_tmp_6064, _tmp_6065);
                            let _tmp_6066 = self.canvas.clone();
                            SoftCanvas::blitImageRectScaledRotated(
                                &_tmp_6066,
                                &pixels,
                                0,
                                0,
                                _tmp_6059,
                                _tmp_6060,
                                pivot_x,
                                pivot_y,
                                arg_0,
                                arg_1,
                                _tmp_6061
                            );
                        } else {
                            let mut _tmp_6068 = pixels.borrow().width.clone();
                            let mut _tmp_6069 = pixels.borrow().height.clone();
                            let mut _tmp_6070 = c.borrow().flipH.clone();
                            let mut _tmp_6071 = c.borrow().flipV.clone();
                            let mut _tmp_6072 = c.borrow().x.clone();
                            let mut _tmp_6073 = c.borrow().y.clone();
                            let mut _tmp_6074 = c.borrow().x.clone();
                            let mut _tmp_6075 = c.borrow().w.clone();
                            let mut _tmp_6076 = c.borrow().y.clone();
                            let mut _tmp_6077 = c.borrow().h.clone();
                            let arg_0 = PptxView::snapEdge(_tmp_6072);
                            let arg_1 = PptxView::snapEdge(_tmp_6073);
                            let arg_2 = PptxView::snapSpan(_tmp_6074, _tmp_6075);
                            let arg_3 = PptxView::snapSpan(_tmp_6076, _tmp_6077);
                            let _tmp_6078 = self.canvas.clone();
                            SoftCanvas::blitImageRectScaledFlipped(
                                &_tmp_6078,
                                &pixels,
                                0,
                                0,
                                _tmp_6068,
                                _tmp_6069,
                                arg_0,
                                arg_1,
                                arg_2,
                                arg_3,
                                _tmp_6070,
                                _tmp_6071
                            );
                        }
                    } else {
                        let mut _tmp_6080 = c.borrow().x.clone();
                        let mut _tmp_6081 = c.borrow().y.clone();
                        let mut _tmp_6082 = c.borrow().x.clone();
                        let mut _tmp_6083 = c.borrow().w.clone();
                        let mut _tmp_6084 = c.borrow().y.clone();
                        let mut _tmp_6085 = c.borrow().h.clone();
                        let arg_0 = PptxView::snapEdge(_tmp_6080);
                        let arg_1 = PptxView::snapEdge(_tmp_6081);
                        let arg_2 = PptxView::snapSpan(_tmp_6082, _tmp_6083);
                        let arg_3 = PptxView::snapSpan(_tmp_6084, _tmp_6085);
                        let arg_4 = EVGColor::rgb(180, 198, 230).clone();
                        UIContext::fillRect(&self.ctx.clone(), arg_0, arg_1, arg_2, arg_3, &arg_4);
                    }
                } else {
                    let mut _tmp_6088 = c.borrow().x.clone();
                    let mut _tmp_6089 = c.borrow().y.clone();
                    let mut _tmp_6090 = c.borrow().x.clone();
                    let mut _tmp_6091 = c.borrow().w.clone();
                    let mut _tmp_6092 = c.borrow().y.clone();
                    let mut _tmp_6093 = c.borrow().h.clone();
                    let arg_0 = PptxView::snapEdge(_tmp_6088);
                    let arg_1 = PptxView::snapEdge(_tmp_6089);
                    let arg_2 = PptxView::snapSpan(_tmp_6090, _tmp_6091);
                    let arg_3 = PptxView::snapSpan(_tmp_6092, _tmp_6093);
                    let arg_4 = EVGColor::rgb(180, 198, 230).clone();
                    UIContext::fillRect(&self.ctx.clone(), arg_0, arg_1, arg_2, arg_3, &arg_4);
                    let mut _tmp_6096 = c.borrow().x.clone();
                    let mut _tmp_6097 = c.borrow().y.clone();
                    let mut _tmp_6098 = c.borrow().x.clone();
                    let mut _tmp_6099 = c.borrow().w.clone();
                    let mut _tmp_6100 = c.borrow().y.clone();
                    let mut _tmp_6101 = c.borrow().h.clone();
                    let arg_0 = PptxView::snapEdge(_tmp_6096);
                    let arg_1 = PptxView::snapEdge(_tmp_6097);
                    let arg_2 = PptxView::snapSpan(_tmp_6098, _tmp_6099);
                    let arg_3 = PptxView::snapSpan(_tmp_6100, _tmp_6101);
                    let arg_4 = EVGColor::rgb(68, 114, 196).clone();
                    UIContext::strokeRect(&self.ctx.clone(), arg_0, arg_1, arg_2, arg_3, 2, &arg_4);
                    let arg_0 = (c.borrow().x + 8.0).floor() as i64;
                    let arg_1 = (c.borrow().y + 8.0).floor() as i64;
                    let arg_2 = EVGColor::rgb(30, 50, 90).clone();
                    UIContext::text(&self.ctx.clone(), "[image]", arg_0, arg_1, 14.0, &arg_2);
                }
            }
            if c.borrow().kind == 6 {
                // Through the rings, not the flat list. A shape with more than
                // one contour — an action button's face with its glyph cut
                // out, a donut, a block arc — filled as one polygon gets an
                // edge from the end of one contour to the start of the next,
                // and a wedge of it comes out unfilled.
                let mut _tmp_6106 = c.borrow().pts.clone();
                let mut _tmp_6107 = c.borrow().ringEnds.clone();
                let mut _tmp_6108 = c.borrow().r.clone();
                let mut _tmp_6109 = c.borrow().g.clone();
                let mut _tmp_6110 = c.borrow().b.clone();
                let arg_0 = (c.borrow().a * 255.0).floor() as i64;
                let _tmp_6111 = self.ctx.clone();
                UIContext::fillPolygonRings(
                    &_tmp_6111,
                    &_tmp_6106,
                    &_tmp_6107,
                    _tmp_6108,
                    _tmp_6109,
                    _tmp_6110,
                    arg_0
                );
            }
            i += 1;
        }
    }
    pub fn render(&mut self, slide: &Rc<RefCell<PptxSlide>>, index: i64, count: i64) {
        self.slideIndex = index;
        self.slideCount = count;
        let mut dl: Rc<RefCell<EVGDisplayList>> = self.buildDisplayList(slide);
        self.paintSoftCanvas(&dl);
    }
    /// Oracle / visual-diff path: paint the slide alone at 96dpi (1pt = 96/72 px),
    /// no chrome, no fit-to-window scale.
    pub fn renderOracleSlide(&mut self, slide: &Rc<RefCell<PptxSlide>>, dpi_scale: f64) {
        self.converter.borrow_mut().setScale(dpi_scale);
        self.converter.borrow_mut().slideNumber = self.slideIndex + 1;
        let mut pw: i64 = ((slide.borrow().width * (96.0 / 72.0)) * dpi_scale + 0.5).floor() as i64;
        let mut ph: i64 = ((slide.borrow().height * (96.0 / 72.0)) * dpi_scale + 0.5).floor() as i64;
        if pw < 1 {
            pw = 1;
        }
        if ph < 1 {
            ph = 1;
        }
        self.resize(pw, ph);
        let mut dl: Rc<RefCell<EVGDisplayList>> = self.converter.borrow_mut().slideToDisplayList(slide);
        self.paintSoftCanvas(&dl);
    }
    pub fn sceneJson(&mut self, slide: &Rc<RefCell<PptxSlide>>, index: i64, count: i64) -> String {
        self.slideIndex = index;
        self.slideCount = count;
        let mut dl: Rc<RefCell<EVGDisplayList>> = self.buildDisplayList(slide);
        let body: String = EVGDisplayList::toJson(&dl);
        format!("{}{},\"height\":{},\"list\":{}{}", "{\"width\":".to_string(), EVGDisplayList::num(self.pageW as f64), EVGDisplayList::num(self.pageH as f64), body, "}".to_string())
    }
}
