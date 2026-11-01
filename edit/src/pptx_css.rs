// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxParagraph;
use pptx::pptx_model::PptxPresentation;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxSlide;
use pptx::pptx_model::PptxTextBody;
use pptx::pptx_model::PptxTextRun;
use crate::pptx_shape_ink::PptxShapeInk;
use pptx::rgr::EVGColor;
use pptx::rgr::{EVGColor};


/// ==============================================================================
/// PptxCss.rgr — a stylesheet over the deck, resolved to DrawingML
/// ==============================================================================
///
/// The fluent API says where a box IS and what it LOOKS LIKE in the same breath:
///
///     slide.addTextBox(70, 150, 820, 110, "Quarterly review")
///          .run(0, 0).font("Calibri", 44).bold().color("#1F3864");
///
/// Geometry belongs there — a slide has no layout to work it out from, and a
/// program placing a box knows where it wants it. Appearance does not. Written
/// that way, changing the house font means finding every call that named one,
/// and a deck built in a loop cannot say "the second line is smaller" without
/// putting an `if` in the middle of its own layout code.
///
/// So a sheet, resolved by the same core EVG's stylesheet is built on:
///
///     deck.addStyleSheet(".title { font-family: Calibri; font-size: 44pt; " +
///                        "font-weight: bold; color: #1F3864 }")
///     slide.addTextBox(70, 150, 820, 110, "Quarterly review").addClass("title")
///
/// ==============================================================================
/// WHAT THIS FILE DECIDES, AND WHAT CssCore DECIDED ALREADY
/// ==============================================================================
///
/// `CssCore` read the text, matched the selectors, ranked them and handed back
/// the properties that apply. It has no idea what any of them MEAN. Everything
/// below is the meaning:
///
///   * which element names a selector can use — `slide`, `shape`, `textBox`,
///     `picture`, `group`, `table`, `chart`, `paragraph`, `run`;
///   * which properties inherit — the text ones, and only those;
///   * which properties this format cannot honour at all;
///   * and where each one lands in the model.
///
/// INHERITANCE. `color`, `font-family`, `font-size`, `font-weight` and
/// `font-style` flow from a shape down through its paragraphs to its runs, which
/// is what makes `.title { font-size: 44pt }` do the obvious thing to a box
/// whose text is three runs long. `fill`, `stroke` and `stroke-width` do not
/// inherit, because a run has no fill and a paragraph inside a red box is not a
/// red paragraph. Those are CSS's own rules and they are the right ones here for
/// CSS's own reason: the inheriting properties describe TEXT, which is nested,
/// and the others describe a BOX, which is not.
///
/// WHAT IS REPORTED RATHER THAN APPLIED. `display: flex`, `display: grid`, every
/// `flex-*` and `grid-*`, and `gap`. The core parses them because the same text
/// is read by EVG, which implements them. A slide has explicit geometry and no
/// layout pass at all, so honouring them here would mean inventing a layout
/// engine and a child hierarchy that `addShape(x, y, w, h)` does not have. They
/// go into `unsupported` — named, counted, and visible to a caller — rather than
/// being dropped in a silence that reads as "applied".
///
/// WHY INLINE ALWAYS WINS, once more. `shape.addClass("warning").fill("blue")`
/// is blue, even against `.warning { fill: red !important }`. A fluent call is
/// the author stating it in the code in front of them; a sheet that could beat
/// it would make every existing call in every existing program conditional on a
/// file it cannot see.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxCss {
    sheet: Rc<RefCell<CssSheet>>,
    /// Property names the sheet used that a slide cannot honour, each once.
    pub unsupported: Vec<String>,
    /// How many objects the last pass touched, and how many declarations landed.
    pub styled: i64,
    pub applied: i64,
}

impl PptxCss {
    pub fn new() -> Self {
        let mut me = Self {
            sheet: Rc::new(RefCell::new(CssSheet::new())),
            unsupported: Vec::new(),
            styled: 0,
            applied: 0,
        };
        let mut u: Vec<String> = Vec::new();
        me.unsupported = u.clone();
        me
    }
    /// --- helpers ---------------------------------------------------------------
    /// The element name a selector writes for this shape. `shape` matches every
    /// one of them, so `.divider { fill: … }` and `shape { … }` both work while
    /// `textBox run { … }` can still be narrower.
    pub fn tagOf(shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().kind == PptxShapeKind::picture() {
            return "picture".to_string();
        }
        if shape.borrow().kind == PptxShapeKind::group() {
            return "group".to_string();
        }
        if shape.borrow().kind == PptxShapeKind::connector() {
            return "connector".to_string();
        }
        if shape.borrow().kind == PptxShapeKind::table() {
            return "table".to_string();
        }
        if shape.borrow().kind == PptxShapeKind::chart() {
            return "chart".to_string();
        }
        if shape.borrow().hasText {
            return "textBox".to_string();
        }
        "shape".to_string()
    }
    /// The five that flow down. See the banner: they describe TEXT, which is
    /// nested, and everything else describes a BOX, which is not.
    pub fn isTextProperty(name: &str) -> bool {
        if name == "color" {
            return true;
        }
        if name == "font-family" {
            return true;
        }
        if name == "font-size" {
            return true;
        }
        if name == "font-weight" {
            return true;
        }
        if name == "font-style" {
            return true;
        }
        false
    }
    fn inherit(into: &Rc<RefCell<CssComputed>>, from: &Rc<RefCell<CssComputed>>) {
        let mut i: i64 = 0;
        while i < CssComputed::count(&from) {
            let name: String = from.borrow().names[i as usize].clone();
            if PptxCss::isTextProperty(&name) {
                CssComputed::set(&into, &name, &from.borrow().values[i as usize].clone());
            }
            if name == "text-align" {
                // Not a CSS text property in the inheriting sense, but it IS
                // inherited in CSS and it is the one alignment a box has.
                CssComputed::set(&into, &name, &from.borrow().values[i as usize].clone());
            }
            i += 1;
        }
    }
    fn copyAll(into: &Rc<RefCell<CssComputed>>, from: &Rc<RefCell<CssComputed>>) {
        let mut i: i64 = 0;
        while i < CssComputed::count(&from) {
            CssComputed::set(
                &into,
                &from.borrow().names[i as usize].clone(),
                &from.borrow().values[i as usize].clone()
            );
            i += 1;
        }
    }
    fn copyInto(node: &Rc<RefCell<CssNode>>, classes: &[String], names: &[String], values: &[String]) {
        let mut i: i64 = 0;
        while i < (classes.len() as i64) {
            CssNode::addClass(&node, &classes[i as usize].clone());
            i += 1;
        }
        let mut k: i64 = 0;
        while k < (names.len() as i64) {
            if k < (values.len() as i64) {
                CssNode::setInline(&node, &names[k as usize].clone(), &values[k as usize].clone());
            }
            k += 1;
        }
    }
    pub fn colorOf(value: &str) -> Rc<RefCell<PptxColor>> {
        let mut col: Rc<RefCell<EVGColor>> = CssValue::color(value);
        PptxShapeInk::colorOf(&col, 1.0)
    }
    pub fn alignOf(value: &str) -> String {
        let v: String = CssSheet::lower(&CssSheet::tidy(value));
        if v == "left" {
            return "l".to_string();
        }
        if v == "start" {
            return "l".to_string();
        }
        if v == "center" {
            return "ctr".to_string();
        }
        if v == "centre" {
            return "ctr".to_string();
        }
        if v == "right" {
            return "r".to_string();
        }
        if v == "end" {
            return "r".to_string();
        }
        if v == "justify" {
            return "just".to_string();
        }
        String::new()
    }
    /// --- the identity a caller sets --------------------------------------------
    ///
    /// Static because the model has no methods on it — it is a struct the parser
    /// fills — and the API facade is where the fluent spelling lives.
    pub fn addShapeClass(shape: &Rc<RefCell<PptxShape>>, name: &str) {
        let mut i: i64 = 0;
        while i < (shape.borrow().styleClasses.len() as i64) {
            if shape.borrow().styleClasses[i as usize].clone() == name {
                return;
            }
            i += 1;
        }
        shape.borrow_mut().styleClasses.push(name.to_string());
    }
    pub fn removeShapeClass(shape: &Rc<RefCell<PptxShape>>, name: &str) {
        let mut kept: Vec<String> = Vec::new();
        let mut i: i64 = 0;
        while i < (shape.borrow().styleClasses.len() as i64) {
            let had: String = shape.borrow().styleClasses[i as usize].clone();
            if had != name {
                kept.push(had.clone());
            }
            i += 1;
        }
        shape.borrow_mut().styleClasses = kept.clone();
    }
    pub fn hasShapeClass(shape: &Rc<RefCell<PptxShape>>, name: &str) -> bool {
        let mut i: i64 = 0;
        while i < (shape.borrow().styleClasses.len() as i64) {
            if shape.borrow().styleClasses[i as usize].clone() == name {
                return true;
            }
            i += 1;
        }
        false
    }
    pub fn setShapeStyle(shape: &Rc<RefCell<PptxShape>>, name: &str, value: &str) {
        let key: String = CssSheet::lower(&CssSheet::tidy(name));
        let mut i: i64 = 0;
        while i < (shape.borrow().inlineStyleNames.len() as i64) {
            if shape.borrow().inlineStyleNames[i as usize].clone() == key {
                shape.borrow_mut().inlineStyleValues[i as usize] = value.to_string();
                return;
            }
            i += 1;
        }
        shape.borrow_mut().inlineStyleNames.push(key);
        shape.borrow_mut().inlineStyleValues.push(value.to_string());
    }
    pub fn addSlideClass(slide: &Rc<RefCell<PptxSlide>>, name: &str) {
        let mut i: i64 = 0;
        while i < (slide.borrow().styleClasses.len() as i64) {
            if slide.borrow().styleClasses[i as usize].clone() == name {
                return;
            }
            i += 1;
        }
        slide.borrow_mut().styleClasses.push(name.to_string());
    }
    pub fn setSlideStyle(slide: &Rc<RefCell<PptxSlide>>, name: &str, value: &str) {
        let key: String = CssSheet::lower(&CssSheet::tidy(name));
        let mut i: i64 = 0;
        while i < (slide.borrow().inlineStyleNames.len() as i64) {
            if slide.borrow().inlineStyleNames[i as usize].clone() == key {
                slide.borrow_mut().inlineStyleValues[i as usize] = value.to_string();
                return;
            }
            i += 1;
        }
        slide.borrow_mut().inlineStyleNames.push(key);
        slide.borrow_mut().inlineStyleValues.push(value.to_string());
    }
    pub fn addStyleSheet(&self, css: &str) {
        CssSheet::parse(&self.sheet.clone(), css);
    }
    pub fn errorCount(&self) -> i64 {
        return CssSheet::errorCount(&self.sheet);
    }
    pub fn errorAt(&self, i: i64) -> String {
        return CssSheet::errorAt(&self.sheet, i);
    }
    pub fn unsupportedCount(&self) -> i64 {
        self.unsupported.len() as i64
    }
    /// --- the walk -------------------------------------------------------------
    /// Resolve the sheet over every slide, shape, paragraph and run.
    pub fn applyTo(&mut self, pres: &Rc<RefCell<PptxPresentation>>) {
        self.styled = 0;
        self.applied = 0;
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = pres.borrow().slides[i as usize].clone();
            self.applyToSlide(&slide);
            i += 1;
        }
    }
    pub fn applyToSlide(&mut self, slide: &Rc<RefCell<PptxSlide>>) {
        let mut node: Rc<RefCell<CssNode>> = Rc::new(RefCell::new(CssNode::new()));
        node.borrow_mut().tag = "slide".to_string();
        node.borrow_mut().id = slide.borrow().styleId.clone();
        let mut _tmp_5299 = slide.borrow().styleClasses.clone();
        let mut _tmp_5300 = slide.borrow().inlineStyleNames.clone();
        PptxCss::copyInto(&node, &_tmp_5299, &_tmp_5300, &slide.borrow().inlineStyleValues.clone());
        let mut got: Rc<RefCell<CssComputed>> = CssCascade::resolve(&self.sheet.clone(), &node);
        self.styled += 1;
        self.paintSlide(slide, &got);
        // The slide's own text properties are the top of the inheritance chain,
        // so they travel down with the walk rather than being applied here.
        let mut inherited: Rc<RefCell<CssComputed>> = Rc::new(RefCell::new(CssComputed::new()));
        PptxCss::inherit(&inherited, &got);
        let mut i: i64 = 0;
        while i < (slide.borrow().shapes.len() as i64) {
            let mut shape: Rc<RefCell<PptxShape>> = slide.borrow().shapes[i as usize].clone();
            self.applyToShape(&shape, node.clone(), &inherited);
            i += 1;
        }
    }
    fn applyToShape(&mut self, shape: &Rc<RefCell<PptxShape>>, parent: Rc<RefCell<CssNode>>, inherited: &Rc<RefCell<CssComputed>>) {
        let mut node: Rc<RefCell<CssNode>> = Rc::new(RefCell::new(CssNode::new()));
        node.borrow_mut().tag = PptxCss::tagOf(shape);
        node.borrow_mut().id = shape.borrow().styleId.clone();
        node.borrow_mut().parent = Some(parent.clone());
        let mut _tmp_5305 = shape.borrow().styleClasses.clone();
        let mut _tmp_5306 = shape.borrow().inlineStyleNames.clone();
        PptxCss::copyInto(&node, &_tmp_5305, &_tmp_5306, &shape.borrow().inlineStyleValues.clone());
        let mut own: Rc<RefCell<CssComputed>> = CssCascade::resolve(&self.sheet.clone(), &node);
        self.styled += 1;
        // What the shape itself says, over what came down to it. Own wins,
        // which is what inheritance means.
        let mut here: Rc<RefCell<CssComputed>> = Rc::new(RefCell::new(CssComputed::new()));
        PptxCss::copyAll(&here, inherited);
        PptxCss::copyAll(&here, &own);
        self.paintShape(shape, &here);
        let mut down: Rc<RefCell<CssComputed>> = Rc::new(RefCell::new(CssComputed::new()));
        PptxCss::inherit(&down, &here);
        if shape.borrow().hasText {
            self.applyToBody(&shape.borrow().text.clone(), node.clone(), &down);
        }
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            let mut child: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
            self.applyToShape(&child, node.clone(), &down);
            i += 1;
        }
    }
    fn applyToBody(&mut self, body: &Rc<RefCell<PptxTextBody>>, parent: Rc<RefCell<CssNode>>, inherited: &Rc<RefCell<CssComputed>>) {
        let mut p: i64 = 0;
        while p < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[p as usize].clone();
            let mut pnode: Rc<RefCell<CssNode>> = Rc::new(RefCell::new(CssNode::new()));
            pnode.borrow_mut().tag = "paragraph".to_string();
            pnode.borrow_mut().parent = Some(parent.clone());
            let mut own: Rc<RefCell<CssComputed>> = CssCascade::resolve(&self.sheet.clone(), &pnode);
            self.styled += 1;
            let mut here: Rc<RefCell<CssComputed>> = Rc::new(RefCell::new(CssComputed::new()));
            PptxCss::copyAll(&here, inherited);
            PptxCss::copyAll(&here, &own);
            self.paintParagraph(&para, &here);
            let mut down: Rc<RefCell<CssComputed>> = Rc::new(RefCell::new(CssComputed::new()));
            PptxCss::inherit(&down, &here);
            let mut r: i64 = 0;
            while r < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[r as usize].clone();
                let mut rnode: Rc<RefCell<CssNode>> = Rc::new(RefCell::new(CssNode::new()));
                rnode.borrow_mut().tag = "run".to_string();
                rnode.borrow_mut().parent = Some(pnode.clone());
                let mut rown: Rc<RefCell<CssComputed>> = CssCascade::resolve(&self.sheet.clone(), &rnode);
                self.styled += 1;
                let mut mine: Rc<RefCell<CssComputed>> = Rc::new(RefCell::new(CssComputed::new()));
                PptxCss::copyAll(&mine, &down);
                PptxCss::copyAll(&mine, &rown);
                self.paintRun(&run, &mine);
                r += 1;
            }
            p += 1;
        }
    }
    /// --- what each property means ---------------------------------------------
    fn paintSlide(&mut self, slide: &Rc<RefCell<PptxSlide>>, got: &Rc<RefCell<CssComputed>>) {
        let mut i: i64 = 0;
        while i < CssComputed::count(&got) {
            let name: String = got.borrow().names[i as usize].clone();
            let value: String = got.borrow().values[i as usize].clone();
            let mut took: bool = false;
            if name == "background-color" {
                slide.borrow_mut().bg = PptxCss::colorOf(&value);
                took = true;
            }
            if name == "background" {
                slide.borrow_mut().bg = PptxCss::colorOf(&value);
                took = true;
            }
            self.note(&name, took);
            i += 1;
        }
    }
    fn paintShape(&mut self, shape: &Rc<RefCell<PptxShape>>, got: &Rc<RefCell<CssComputed>>) {
        let mut i: i64 = 0;
        while i < CssComputed::count(&got) {
            let name: String = got.borrow().names[i as usize].clone();
            let value: String = got.borrow().values[i as usize].clone();
            let mut took: bool = false;
            if name == "fill" || name == "background-color" {
                if CssValue::isNone(&value) {
                    shape.borrow_mut().noFill = true;
                    shape.borrow_mut().fill.borrow_mut().isSet = false;
                } else {
                    shape.borrow_mut().fill = PptxCss::colorOf(&value);
                    shape.borrow_mut().noFill = false;
                }
                took = true;
            }
            if name == "stroke" || name == "border-color" {
                if CssValue::isNone(&value) {
                    shape.borrow_mut().noLine = true;
                    shape.borrow_mut().line.borrow_mut().isSet = false;
                } else {
                    shape.borrow_mut().line = PptxCss::colorOf(&value);
                    shape.borrow_mut().noLine = false;
                }
                took = true;
            }
            if name == "stroke-width" || name == "border-width" {
                let _tmp_5317 = CssValue::points(&value, shape.borrow().lineWidthPt.clone());
                shape.borrow_mut().lineWidthPt = _tmp_5317;
                took = true;
            }
            if name == "opacity" {
                // A slide has no shape-level alpha, only a colour's. Folding it
                // into the fill is the closest true thing; a shape with no fill
                // has nothing to fold it into and says so.
                if shape.borrow().fill.borrow().isSet {
                    let a: f64 = CssValue::fontPoints(&value, 1.0);
                    shape.borrow_mut().fill.borrow_mut().alpha = a;
                    took = true;
                }
            }
            if name == "border-radius" {
                let r: f64 = CssValue::points(&value, 0.0);
                let mut shortest: f64 = shape.borrow().width;
                if shape.borrow().height < shortest {
                    shortest = shape.borrow().height;
                }
                if shortest > 0.0 {
                    shape.borrow_mut().preset = "roundRect".to_string();
                    let mut adj: f64 = r / shortest;
                    if adj > 0.5 {
                        adj = 0.5;
                    }
                    shape.borrow_mut().adj = adj;
                }
                took = true;
            }
            if name == "text-align" {
                took = true;
            }
            if PptxCss::isTextProperty(&name) {
                // Carried down to the runs rather than applied to the box. Not
                // unsupported — the opposite: it is where the whole point of a
                // stylesheet on a deck lives.
                took = true;
            }
            self.note(&name, took);
            i += 1;
        }
    }
    fn paintParagraph(&mut self, para: &Rc<RefCell<PptxParagraph>>, got: &Rc<RefCell<CssComputed>>) {
        let align: String = CssComputed::get(&got, "text-align");
        if !align.is_empty() {
            let a: String = PptxCss::alignOf(&align);
            if !a.is_empty() {
                para.borrow_mut().align = a.clone();
                para.borrow_mut().alignSet = true;
                self.applied += 1;
            }
        }
    }
    fn paintRun(&mut self, run: &Rc<RefCell<PptxTextRun>>, got: &Rc<RefCell<CssComputed>>) {
        let family: String = CssComputed::get(&got, "font-family");
        if !family.is_empty() {
            run.borrow_mut().fontFamily = CssValue::firstFamily(&family);
            self.applied += 1;
        }
        let size: String = CssComputed::get(&got, "font-size");
        if !size.is_empty() {
            let _tmp_5320 = CssValue::fontPoints(&size, run.borrow().fontSizePt.clone());
            run.borrow_mut().fontSizePt = _tmp_5320;
            run.borrow_mut().sizeSet = true;
            self.applied += 1;
        }
        let weight: String = CssComputed::get(&got, "font-weight");
        if !weight.is_empty() {
            run.borrow_mut().bold = CssValue::isBold(&weight);
            run.borrow_mut().boldSet = true;
            self.applied += 1;
        }
        let style: String = CssComputed::get(&got, "font-style");
        if !style.is_empty() {
            run.borrow_mut().italic = CssValue::isItalic(&style);
            run.borrow_mut().italicSet = true;
            self.applied += 1;
        }
        let colour: String = CssComputed::get(&got, "color");
        if !colour.is_empty() {
            run.borrow_mut().color = PptxCss::colorOf(&colour);
            self.applied += 1;
        }
    }
    /// A property the sheet stated and this format has nowhere to put. Recorded
    /// once each — a sheet that names `display: flex` on forty boxes is one
    /// thing a caller needs to be told, not forty.
    pub fn note(&mut self, name: &str, took: bool) {
        if took {
            self.applied += 1;
            return;
        }
        let mut i: i64 = 0;
        while i < (self.unsupported.len() as i64) {
            if self.unsupported[i as usize].clone() == name {
                return;
            }
            i += 1;
        }
        self.unsupported.push(name.to_string());
    }
}
