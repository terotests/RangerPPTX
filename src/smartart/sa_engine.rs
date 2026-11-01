// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaEngine — a SmartArt data model in, a drawn diagram out
//! ==============================================================================
//!
//!   def r:SaResult (SaEngine.layout(xml (new SaOptions()) 480.0 270.0))
//!   r.ok        the file was a diagram
//!   r.error     why not, when it was not
//!   r.shape     a PptxShape group (isDiagram), its children in the box's own
//!               space: the same thing PptxParser makes from the drawing part
//!               PowerPoint saves, so one representation is drawn, edited and
//!               exported whoever laid the diagram out
//!   r.warnings  what was drawn other than asked: an unknown layout or colour
//!               set, text that did not fit
//!
//! The layout is the one the data's document point names (`loTypeId`), or
//! `SaOptions.layout` when the host's Markdown overrides it, read from the
//! built-in definitions (`builtin/`, embedded in SaBuiltins) — or one passed in
//! whole (`layoutXml`), which is how a layout PowerPoint saved in a file is
//! used. An id with no definition is drawn as `default` (Basic Block List,
//! PowerPoint's own fallback) and said so.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::pptx_model::PptxColor;
use crate::pptx_model::PptxParagraph;
use crate::pptx_model::PptxShape;
use crate::pptx_model::PptxShapeKind;
use crate::pptx_model::PptxTextBody;
use crate::pptx_model::PptxTextRun;
use crate::smartart::sa_algorithms::SaAlgorithms;
use crate::smartart::sa_builtins::SaBuiltins;
use crate::smartart::sa_colors::SaColors;
use crate::smartart::sa_colors::SaColorsDef;
use crate::smartart::sa_colors::SaColorsRead;
use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_data_reader::SaDataReader;
use crate::smartart::sa_data_reader::SaReadResult;
use crate::smartart::sa_data_writer::SaDataWriter;
use crate::smartart::sa_layout_def::SaLayoutDef;
use crate::smartart::sa_layout_reader::SaLayoutReadResult;
use crate::smartart::sa_layout_reader::SaLayoutReader;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_pres_tree::SaInst;
use crate::smartart::sa_pres_tree::SaPresTree;
use crate::smartart::sa_quick_style::SaQuickStyle;
use crate::smartart::sa_quick_style::SaQuickStyleDef;
use crate::smartart::sa_quick_style::SaQuickStyleRead;
use crate::smartart::sa_style::SaPaint;
use crate::smartart::sa_style::SaStyle;
use crate::smartart::sa_text::SaLine;
use crate::smartart::sa_text::SaMeasure;
use crate::smartart::sa_text::SaText;

#[derive(Clone)]
pub struct SaOptions {
    /// overrides of what the data's document point names; a short name
    /// (`process1`) or a full id
    pub layout: String,
    pub colors: String,
    pub style: String,
    /// a whole layout definition, used instead of any id
    pub layoutXml: String,
    /// a whole colour set or quick style (a diagram's own, from its file),
    /// used instead of the id's built-in one
    pub colorsXml: String,
    pub styleXml: String,
    pub measure: Rc<RefCell<SaMeasure>>,
    /// a row of steps may wrap into rows when the box is tall (a slide host);
    /// PowerPoint never wraps them, so a deck editor leaves this off
    pub wrap: bool,
}

impl SaOptions {
    pub fn new() -> Self {
        Self {
            layout: String::new(),
            colors: String::new(),
            style: String::new(),
            layoutXml: String::new(),
            colorsXml: String::new(),
            styleXml: String::new(),
            measure: Rc::new(RefCell::new(SaMeasure::new())),
            wrap: false,
        }
    }
}

#[derive(Clone)]
pub struct SaResult {
    pub ok: bool,
    pub error: String,
    pub warnings: Vec<String>,
    pub shape: Rc<RefCell<PptxShape>>,
    /// the layout that was used, by its short name
    pub layoutUsed: String,
    pub ptree: Rc<RefCell<SaPresTree>>,
    /// what the text was measured with, which the painter must break its lines
    /// with too (SaRender.drawWith)
    pub measure: Rc<RefCell<SaMeasure>>,
}

impl SaResult {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            warnings: Vec::new(),
            shape: Rc::new(RefCell::new(PptxShape::new())),
            layoutUsed: String::new(),
            ptree: Rc::new(RefCell::new(SaPresTree::new())),
            measure: Rc::new(RefCell::new(SaMeasure::new())),
        }
    }
}

/// How a diagram's shapes are coloured: its colour set and quick style, and
/// how many shapes carry each style label (what a colour span runs over).
#[derive(Clone)]
pub struct SaLook {
    pub cd: Rc<RefCell<SaColorsDef>>,
    pub qs: Rc<RefCell<SaQuickStyleDef>>,
    pub names: Vec<String>,
    pub counts: Vec<i64>,
}

impl SaLook {
    pub fn new() -> Self {
        Self {
            cd: Rc::new(RefCell::new(SaColorsDef::new())),
            qs: Rc::new(RefCell::new(SaQuickStyleDef::new())),
            names: Vec::new(),
            counts: Vec::new(),
        }
    }
    pub fn count(&mut self, name: &str) {
        let mut i: i64 = 0;
        while i < (self.names.len() as i64) {
            if self.names[i as usize].clone() == name {
                self.counts[i as usize] = self.counts[i as usize] + 1;
                return;
            }
            i += 1;
        }
        self.names.push(name.to_string());
        self.counts.push(1);
    }
    pub fn countOf(&self, name: &str) -> i64 {
        let mut i: i64 = 0;
        while i < (self.names.len() as i64) {
            if self.names[i as usize].clone() == name {
                return self.counts[i as usize];
            }
            i += 1;
        }
        1
    }
    pub fn paint(&self, s: &Rc<RefCell<SaInst>>) -> Rc<RefCell<SaPaint>> {
        let mut _tmp_680 = s.borrow().styleLbl.clone();
        let mut _tmp_681 = self.cd.clone();
        let mut _tmp_682 = self.qs.clone();
        let mut _tmp_683 = s.borrow().nameIdx.clone();
        let mut _tmp_685 = self.countOf(&s.borrow().name.clone());
        return SaStyle::paint(&_tmp_680, &_tmp_681, &_tmp_682, _tmp_683, _tmp_685);
    }
}

#[derive(Clone)]
pub struct SaEngine {
}

impl SaEngine {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn layout(data_xml: &str, opts: &Rc<RefCell<SaOptions>>, w: f64, h: f64) -> Rc<RefCell<SaResult>> {
        let mut read: Rc<RefCell<SaReadResult>> = SaDataReader::read(data_xml);
        if !read.borrow().ok {
            let mut bad: Rc<RefCell<SaResult>> = Rc::new(RefCell::new(SaResult::new()));
            bad.borrow_mut().error = read.borrow().error.clone();
            return bad;
        }
        return SaEngine::layoutModel(read.borrow().model.clone(), opts, w, h);
    }
    pub fn noItems() -> String {
        "the diagram has no items: no point is connected under the document point".to_string()
    }
    pub fn fullId(s: &str, kind: &str) -> String {
        if PptxText::index_of(&s, ":") >= 0 {
            return s.to_string();
        }
        format!("urn:microsoft.com/office/officeart/2005/8/{}/{}", kind, s)
    }
    pub fn layoutModel(model: Rc<RefCell<SaModel>>, opts: &Rc<RefCell<SaOptions>>, w: f64, h: f64) -> Rc<RefCell<SaResult>> {
        let mut res: Rc<RefCell<SaResult>> = Rc::new(RefCell::new(SaResult::new()));
        model.borrow_mut().complete();
        // A diagram with no items is laid out as nothing (an editor can empty
        // one and fill it again), and says so: a host that draws nothing
        // shows this reason in its place.
        let mut doc_pt: Rc<RefCell<SaPoint>> = model.borrow().doc();
        let mut items: Vec<Rc<RefCell<SaPoint>>> = model.borrow().children(&doc_pt.borrow().id.clone());
        if items.is_empty() {
            res.borrow_mut().warnings.push(SaEngine::noItems());
        }
        // which layout
        let mut lo: Rc<RefCell<SaLayoutDef>> = Rc::new(RefCell::new(SaLayoutDef::new()));
        let mut lo_xml: String = opts.borrow().layoutXml.clone();
        if !opts.borrow().layoutXml.is_empty() {
            let mut lr: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&opts.borrow().layoutXml.clone());
            if !lr.borrow().ok {
                res.borrow_mut().error = format!("the layout definition: {}", lr.borrow().error);
                return res;
            }
            lo = lr.borrow().layout.clone();
        } else {
            let mut want: String = model.borrow().layoutId();
            if !opts.borrow().layout.is_empty() {
                want = opts.borrow().layout.clone();
            }
            let short_want: String = SaLayoutDef::shortOf(&want);
            let mut xml: String = SaBuiltins::layoutXml(&short_want);
            if xml.is_empty() {
                if !short_want.is_empty() {
                    res.borrow_mut().warnings.push(format!("the layout \"{}\" is not one this engine has; drawn as default (it has: {})", short_want, SaBuiltins::names()));
                }
                xml = SaBuiltins::layoutXml("default");
            }
            let mut lr2: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&xml);
            if !lr2.borrow().ok {
                res.borrow_mut().error = format!("the built-in layout {}: {}", short_want, lr2.borrow().error);
                return res;
            }
            lo = lr2.borrow().layout.clone();
            lo_xml = xml.clone();
        }
        res.borrow_mut().layoutUsed = lo.borrow().shortId();
        let mut colors: String = model.borrow().doc().borrow().colorsId.clone();
        if !opts.borrow().colors.is_empty() {
            colors = SaEngine::fullId(&opts.borrow().colors, "colors");
        }
        if opts.borrow().colorsXml.is_empty() && !SaStyle::knownColors(&colors) {
            res.borrow_mut().warnings.push(format!("the colour set \"{}\" is not one this engine has; drawn as accent1_2 (it has: {})", SaStyle::short(&colors), SaColors::names()));
            colors = String::new();
        }
        // run it
        let mut ptree: Rc<RefCell<SaPresTree>> = SaPresTree::build(model.clone(), lo.clone());
        if !ptree.borrow().error.is_empty() {
            res.borrow_mut().error = ptree.borrow().error.clone();
            return res;
        }
        ptree.borrow().numberByName();
        res.borrow_mut().ptree = ptree.clone();
        res.borrow_mut().measure = Rc::clone(&opts.borrow().measure);
        let mut cons: Rc<RefCell<SaConstraints>> = SaConstraints::resolve(ptree.clone(), w, h);
        let mut algs: Rc<RefCell<SaAlgorithms>> = SaAlgorithms::runWith(cons.clone(), opts.borrow().wrap.clone());
        let mut text: Rc<RefCell<SaText>> = SaText::fit(cons.clone(), Rc::clone(&opts.borrow().measure));
        // A slide host (wrap): a row of shapes that keep their proportions
        // and fill less than half the box's height, their text broken onto
        // several lines — three long-texted chevrons across a slide — is made
        // taller, when that sets the text clearly larger. PowerPoint keeps the proportions; a slide has the
        // room.
        if (opts.borrow().wrap && SaEngine::heightUsed(&ptree) < h * 0.5) && text.borrow().breaks() {
            let mut best: f64 = SaEngine::leastFont(&ptree);
            let mut tries: Vec<f64> = vec![1.4, 1.8, 2.4];
            let mut ti: i64 = 0;
            while ti < (tries.len() as i64) {
                let mut pt2: Rc<RefCell<SaPresTree>> = SaPresTree::build(model.clone(), lo.clone());
                pt2.borrow().numberByName();
                let mut c2: Rc<RefCell<SaConstraints>> = SaConstraints::resolve(pt2.clone(), w, h);
                let mut a2: Rc<RefCell<SaAlgorithms>> = SaAlgorithms::runStretched(
                    c2.clone(),
                    true,
                    tries[ti as usize]
                );
                if !a2.borrow().stretched {
                    break;
                }
                let mut t2: Rc<RefCell<SaText>> = SaText::fit(c2.clone(), Rc::clone(&opts.borrow().measure));
                let least: f64 = SaEngine::leastFont(&pt2);
                if least >= best * 1.12 && (t2.borrow().warnings.len() as i64) <= (text.borrow().warnings.len() as i64) {
                    ptree = pt2.clone();
                    cons = c2.clone();
                    algs = a2.clone();
                    text = t2.clone();
                    best = least;
                }
                ti += 1;
            }
            res.borrow_mut().ptree = ptree.clone();
        }
        let mut k: i64 = 0;
        while k < (algs.borrow().warnings.len() as i64) {
            res.borrow_mut().warnings.push(algs.borrow().warnings[k as usize].clone());
            k += 1;
        }
        k = 0;
        while k < (text.borrow().warnings.len() as i64) {
            res.borrow_mut().warnings.push(text.borrow().warnings[k as usize].clone());
            k += 1;
        }
        // draw it
        let mut g: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        g.borrow_mut().kind = PptxShapeKind::group();
        g.borrow_mut().isDiagram = true;
        g.borrow_mut().name = "Diagram".to_string();
        g.borrow_mut().x = 0.0;
        g.borrow_mut().y = 0.0;
        g.borrow_mut().width = w;
        g.borrow_mut().height = h;
        g.borrow_mut().hasXfrm = true;
        g.borrow_mut().hasOff = true;
        g.borrow_mut().hasExt = true;
        g.borrow_mut().chX = 0.0;
        g.borrow_mut().chY = 0.0;
        g.borrow_mut().chW = w;
        g.borrow_mut().chH = h;
        g.borrow_mut().diagramText = model.borrow().outline();
        // what it was made from, with the ids it was drawn with, so the writer
        // can save it as SmartArt and the editor can lay it out again
        let mut style: String = model.borrow().doc().borrow().styleId.clone();
        if !opts.borrow().style.is_empty() {
            style = SaEngine::fullId(&opts.borrow().style, "quickstyle");
        }
        let mut look: Rc<RefCell<SaLook>> = Rc::new(RefCell::new(SaLook::new()));
        look.borrow_mut().qs = SaStyle::styleDef(&style);
        if !opts.borrow().styleXml.is_empty() {
            let mut sr: Rc<RefCell<SaQuickStyleRead>> = SaQuickStyle::read(&opts.borrow().styleXml.clone());
            if sr.borrow().ok {
                look.borrow_mut().qs = sr.borrow().def.clone();
                if !sr.borrow().def.borrow().uniqueId.is_empty() {
                    style = sr.borrow().def.borrow().uniqueId.clone();
                }
            }
        } else {
            if !SaStyle::knownStyle(&style) {
                res.borrow_mut().warnings.push(format!("the quick style \"{}\" is not one this engine has; drawn as simple1 (it has: {})", SaStyle::short(&style), SaQuickStyle::names()));
                style = String::new();
            }
        }
        if style.is_empty() {
            style = SaEngine::fullId("simple1", "quickstyle");
        }
        let mut colors_used: String = colors.clone();
        if colors_used.is_empty() {
            colors_used = SaEngine::fullId("accent1_2", "colors");
        }
        look.borrow_mut().cd = SaStyle::colorsDef(&colors_used);
        if !opts.borrow().colorsXml.is_empty() {
            let mut cr: Rc<RefCell<SaColorsRead>> = SaColors::read(&opts.borrow().colorsXml.clone());
            if cr.borrow().ok {
                look.borrow_mut().cd = cr.borrow().def.clone();
                if !cr.borrow().def.borrow().uniqueId.is_empty() {
                    colors_used = cr.borrow().def.borrow().uniqueId.clone();
                }
            }
        }
        let mut d: Rc<RefCell<SaPoint>> = model.borrow().doc();
        d.borrow_mut().layoutId = lo.borrow().uniqueId.clone();
        d.borrow_mut().styleId = style.clone();
        d.borrow_mut().colorsId = colors_used.clone();
        g.borrow_mut().diagramModelXml = SaDataWriter::write(&model);
        g.borrow_mut().diagramLayoutXml = lo_xml.clone();
        g.borrow_mut().diagramLayoutId = lo.borrow().uniqueId.clone();
        g.borrow_mut().diagramStyleId = style.clone();
        g.borrow_mut().diagramColorsId = colors_used.clone();
        let mut every: Vec<Rc<RefCell<SaInst>>> = ptree.borrow().all();
        let mut ci: i64 = 0;
        while ci < (every.len() as i64) {
            let mut cs: Rc<RefCell<SaInst>> = every[ci as usize].clone();
            look.borrow_mut().count(&cs.borrow().name);
            ci += 1;
        }
        // the points whose text some shape shows
        let mut shown: Vec<String> = Vec::new();
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            if (s.borrow().segs.len() as i64) >= 4 {
                SaEngine::linesOf(&s, &look, &mut g.borrow_mut().children);
            } else {
                let mut made: Option<Rc<RefCell<PptxShape>>> = SaEngine::shapeOf(&s, &text, &look);
                if made.is_some() {
                    g.borrow_mut().children.push(made.clone().unwrap());
                    let mut lines: Vec<Rc<RefCell<SaLine>>> = text.borrow().linesOf(&s);
                    let mut li: i64 = 0;
                    while li < (lines.len() as i64) {
                        let mut ln: Rc<RefCell<SaLine>> = lines[li as usize].clone();
                        shown.push(ln.borrow().ptId.clone());
                        li += 1;
                    }
                }
            }
            i += 1;
        }
        let left: String = SaEngine::notShown(&model, &shown, &res.borrow().layoutUsed.clone());
        if !left.is_empty() {
            res.borrow_mut().warnings.push(left.clone());
        }
        // what the shapes say now: a diagram whose shapes were changed by
        // hand no longer matches it, and is saved as those shapes
        let hoist_24 = g.borrow().diagramSignature();
        g.borrow_mut().diagramSig = hoist_24;
        g.borrow_mut().diagramStyleXml = opts.borrow().styleXml.clone();
        g.borrow_mut().diagramColorsXml = opts.borrow().colorsXml.clone();
        res.borrow_mut().shape = g.clone();
        res.borrow_mut().ok = true;
        res
    }
    /// the smallest size any shape's text was set at; 0 when none has text
    pub fn leastFont(ptree: &Rc<RefCell<SaPresTree>>) -> f64 {
        let mut every: Vec<Rc<RefCell<SaInst>>> = ptree.borrow().all();
        let mut least: f64 = 0.0;
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            if s.borrow().fontPt > 0.0 && (least == 0.0 || s.borrow().fontPt < least) {
                least = s.borrow().fontPt;
            }
            i += 1;
        }
        least
    }
    /// how tall the drawn shapes are together, top of the highest to the
    /// bottom of the lowest
    pub fn heightUsed(ptree: &Rc<RefCell<SaPresTree>>) -> f64 {
        let mut every: Vec<Rc<RefCell<SaInst>>> = ptree.borrow().all();
        let mut top: f64 = 0.0;
        let mut bottom: f64 = 0.0;
        let mut any: bool = false;
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            if ((s.borrow().hasShape && s.borrow().shape.borrow().r#type != "none") && s.borrow().w > 0.0) && s.borrow().h > 0.0 {
                if !any || s.borrow().y < top {
                    top = s.borrow().y;
                }
                if !any || s.borrow().y + s.borrow().h > bottom {
                    bottom = s.borrow().y + s.borrow().h;
                }
                any = true;
            }
            i += 1;
        }
        bottom - top
    }
    /// What the layout left out: the items with text that no shape shows (a
    /// layout holds as many as it has places for — gear1 three, cycle4 four),
    /// named, so whoever wrote the file knows what the slide does not say.
    /// "" when every item is drawn.
    pub fn notShown(model: &Rc<RefCell<SaModel>>, shown: &[String], layout: &str) -> String {
        let mut all: Vec<Rc<RefCell<SaPoint>>> = Vec::new();
        SaEngine::contentUnder(model, &model.borrow().doc().borrow().id.clone(), &mut all);
        let mut missing: Vec<String> = Vec::new();
        let mut total: i64 = 0;
        let mut i: i64 = 0;
        while i < (all.len() as i64) {
            let mut p: Rc<RefCell<SaPoint>> = all[i as usize].clone();
            let t: String = p.borrow().plainText().trim().to_string();
            if !t.is_empty() {
                total += 1;
                if (shown.iter().position( |__r| __r.clone() == (p.borrow().id).clone() ).map(|__i| __i as i64).unwrap_or(-1)) < 0 {
                    missing.push(format!("\"{}\"", t));
                }
            }
            i += 1;
        }
        let n: i64 = missing.len() as i64;
        if n == 0 {
            return String::new();
        }
        format!("the layout {} has a place for {} of the {} items; not drawn: {}", layout, total - n, total, missing.join(&*", ".to_string()))
    }
    pub fn contentUnder(model: &Rc<RefCell<SaModel>>, id: &str, mut out: &mut Vec<Rc<RefCell<SaPoint>>>) {
        let mut kids: Vec<Rc<RefCell<SaPoint>>> = model.borrow().children(id);
        let mut i: i64 = 0;
        while i < (kids.len() as i64) {
            let mut k: Rc<RefCell<SaPoint>> = kids[i as usize].clone();
            if k.borrow().isContent() {
                out.push(k.clone());
                SaEngine::contentUnder(model, &k.borrow().id.clone(), &mut out);
            }
            i += 1;
        }
    }
    /// A connector line: one `line` shape per segment, its box the segment's
    /// bounds, flipped when the segment runs up or left; an arrowhead at the
    /// end of the last when the connector has one.
    pub fn linesOf(s: &Rc<RefCell<SaInst>>, look: &Rc<RefCell<SaLook>>, mut out: &mut Vec<Rc<RefCell<PptxShape>>>) {
        let mut paint: Rc<RefCell<SaPaint>> = look.borrow().paint(s);
        let mut col: Rc<RefCell<PptxColor>> = paint.borrow().line.clone().clone();
        if paint.borrow().noLine {
            col = paint.borrow().fill.clone();
        }
        let n: i64 = s.borrow().segs.len() as i64;
        let mut i: i64 = 0;
        while i + 3 < n {
            let x1: f64 = s.borrow().segs[i as usize];
            let y1: f64 = s.borrow().segs[(i + 1) as usize];
            let x2: f64 = s.borrow().segs[(i + 2) as usize];
            let y2: f64 = s.borrow().segs[(i + 3) as usize];
            let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            sh.borrow_mut().kind = PptxShapeKind::shape();
            sh.borrow_mut().name = format!("{} {}.{}", s.borrow().name, s.borrow().nameIdx + 1, (((i as f64) / 4.0).floor() as i64) + 1);
            sh.borrow_mut().x = x1;
            if x2 < x1 {
                sh.borrow_mut().x = x2;
            }
            sh.borrow_mut().y = y1;
            if y2 < y1 {
                sh.borrow_mut().y = y2;
            }
            sh.borrow_mut().width = (x2 - x1).abs();
            sh.borrow_mut().height = (y2 - y1).abs();
            sh.borrow_mut().flipH = x2 < x1;
            sh.borrow_mut().flipV = y2 < y1;
            sh.borrow_mut().hasXfrm = true;
            sh.borrow_mut().hasOff = true;
            sh.borrow_mut().hasExt = true;
            sh.borrow_mut().preset = "line".to_string();
            sh.borrow_mut().noFill = true;
            sh.borrow_mut().line = col.clone();
            sh.borrow_mut().lineWidthPt = paint.borrow().lineWidthPt;
            if sh.borrow().lineWidthPt < 1.0 {
                sh.borrow_mut().lineWidthPt = 1.0;
            }
            if s.borrow().arrowEnd && i + 4 >= n {
                sh.borrow_mut().tailEnd = "triangle".to_string();
            }
            out.push(sh.clone());
            i += 4;
        }
    }
    /// the preset a connector is drawn as, by the way its line goes
    pub fn arrowFor(dir: f64) -> f64 {
        dir
    }
    pub fn connPreset(dir: f64) -> String {
        if dir == 180.0 {
            return "leftArrow".to_string();
        }
        if dir == 90.0 {
            return "downArrow".to_string();
        }
        if dir == 270.0 {
            return "upArrow".to_string();
        }
        "rightArrow".to_string()
    }
    /// what a preset calls its adjustments, in order
    pub fn adjName(preset: &str, idx: i64) -> String {
        let two: String = " rightArrow leftArrow upArrow downArrow leftRightArrow upDownArrow notchedRightArrow stripedRightArrow ";
        if PptxText::index_of(&two, &format!(" {} ", preset)) >= 0 {
            return format!("adj{}", idx);
        }
        if idx == 1 {
            return "adj".to_string();
        }
        format!("adj{}", idx)
    }
    pub fn shapeOf(s: &Rc<RefCell<SaInst>>, text: &Rc<RefCell<SaText>>, look: &Rc<RefCell<SaLook>>) -> Option<Rc<RefCell<PptxShape>>> {
        let mut none: Option<Rc<RefCell<PptxShape>>> = None;
        let mut lines: Vec<Rc<RefCell<SaLine>>> = text.borrow().linesOf(s);
        let geometry: bool = (s.borrow().hasShape && s.borrow().shape.borrow().r#type != "none") && !s.borrow()
            .shape.borrow()
            .hideGeom;
        if !geometry && lines.is_empty() {
            return none;
        }
        if s.borrow().w <= 0.0 || s.borrow().h <= 0.0 {
            return none;
        }
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().name = format!("{} {}", s.borrow().name, s.borrow().nameIdx + 1);
        sh.borrow_mut().x = s.borrow().x;
        sh.borrow_mut().y = s.borrow().y;
        sh.borrow_mut().width = s.borrow().w;
        sh.borrow_mut().height = s.borrow().h;
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().hasOff = true;
        sh.borrow_mut().hasExt = true;
        sh.borrow_mut().rotation = s.borrow().shape.borrow().rot + s.borrow().rotDeg;
        let mut preset: String = s.borrow().shape.borrow().r#type.clone();
        if preset == "conn" || s.borrow().alg.borrow().r#type == "conn" {
            preset = SaEngine::connPreset(s.borrow().raw("connDir", 0.0));
        }
        if preset == "none" || preset.is_empty() {
            preset = "rect".to_string();
        }
        sh.borrow_mut().preset = preset.clone();
        let mut a: i64 = 0;
        if !s.borrow().adjNames.is_empty() {
            // an algorithm's own, for this one shape
            while a < (s.borrow().adjNames.len() as i64) {
                let an: String = s.borrow().adjNames[a as usize].clone();
                let av: f64 = s.borrow().adjVals[a as usize];
                sh.borrow_mut().adjNames.push(an.clone());
                sh.borrow_mut().adjValues.push(av);
                if an == "adj" {
                    sh.borrow_mut().adj = av / 100000.0;
                }
                a += 1;
            }
            a = s.borrow().shape.borrow().adj.len() as i64;
        }
        while a < (s.borrow().shape.borrow().adj.len() as i64) {
            if s.borrow().shape.borrow().adjSet[a as usize] {
                let v: f64 = s.borrow().shape.borrow().adj[a as usize];
                let nm: String = SaEngine::adjName(&preset, a + 1);
                sh.borrow_mut().adjNames.push(nm.clone());
                sh.borrow_mut().adjValues.push(v * 100000.0);
                if nm == "adj" {
                    sh.borrow_mut().adj = v;
                }
            }
            a += 1;
        }
        // a shape a host's row made taller keeps its point as deep as it was
        // (the depth is a fraction of the shorter side, which grew)
        if s.borrow().stretchK > 1.0 && (preset == "chevron" || preset == "homePlate") {
            let mut side: f64 = s.borrow().w;
            if s.borrow().h < side {
                side = s.borrow().h;
            }
            let f: f64 = (s.borrow().h / s.borrow().stretchK) / side;
            let mut found: bool = false;
            let mut ai: i64 = 0;
            while ai < (sh.borrow().adjNames.len() as i64) {
                if sh.borrow().adjNames[ai as usize].clone() == "adj" {
                    sh.borrow_mut().adjValues[ai as usize] = sh.borrow().adjValues[ai as usize] * f;
                    let hoist_25 = sh.borrow().adj * f;
                    sh.borrow_mut().adj = hoist_25;
                    found = true;
                }
                ai += 1;
            }
            if !found {
                sh.borrow_mut().adjNames.push("adj".to_string().to_string());
                sh.borrow_mut().adjValues.push(50000.0 * f);
                sh.borrow_mut().adj = 0.5 * f;
            }
        }
        let mut paint: Rc<RefCell<SaPaint>> = look.borrow().paint(s);
        if !geometry {
            paint.borrow_mut().noFill = true;
            paint.borrow_mut().noLine = true;
        }
        if paint.borrow().noFill {
            sh.borrow_mut().noFill = true;
        } else {
            sh.borrow_mut().fill = paint.borrow().fill.clone();
        }
        if paint.borrow().noLine {
            sh.borrow_mut().noLine = true;
        } else {
            sh.borrow_mut().line = paint.borrow().line.clone();
            sh.borrow_mut().lineWidthPt = paint.borrow().lineWidthPt;
        }
        if paint.borrow().shadow {
            sh.borrow_mut().shadow.borrow_mut().isSet = true;
            sh.borrow_mut().shadow.borrow_mut().offsetXPt = 0.0;
            sh.borrow_mut().shadow.borrow_mut().offsetYPt = paint.borrow().shadowDistPt;
            sh.borrow_mut().shadow.borrow_mut().blurPt = paint.borrow().shadowBlurPt;
            sh.borrow_mut().shadow.borrow_mut().color = PptxColor::srgbOf("000000".to_string());
            sh.borrow_mut().shadow.borrow_mut().alpha = paint.borrow().shadowAlpha;
        }
        if !lines.is_empty() {
            sh.borrow_mut().hasText = true;
            let mut body: Rc<RefCell<PptxTextBody>> = sh.borrow().text.clone().clone();
            let size: f64 = s.borrow().fontPt;
            body.borrow_mut().insetL = text.borrow().margin(s, "lMarg", size);
            body.borrow_mut().insetR = text.borrow().margin(s, "rMarg", size);
            body.borrow_mut().insetT = text.borrow().margin(s, "tMarg", size);
            body.borrow_mut().insetB = text.borrow().margin(s, "bMarg", size);
            body.borrow_mut().insetsSet = true;
            let anchor: String = (s.borrow().alg).borrow().param("txAnchorVert", "mid");
            body.borrow_mut().anchor = "ctr".to_string();
            if anchor == "t" {
                body.borrow_mut().anchor = "t".to_string();
            }
            if anchor == "b" {
                body.borrow_mut().anchor = "b".to_string();
            }
            body.borrow_mut().anchorSet = true;
            body.borrow_mut().wrap = true;
            let align: String = (s.borrow().alg).borrow().param("parTxLTRAlign", "ctr");
            let mut k: i64 = 0;
            while k < (lines.len() as i64) {
                let mut ln: Rc<RefCell<SaLine>> = lines[k as usize].clone();
                let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
                para.borrow_mut().level = ln.borrow().level;
                para.borrow_mut().alignSet = true;
                para.borrow_mut().lineSpacingPct = 90.0;
                if ln.borrow().level > 0 {
                    para.borrow_mut().align = "l".to_string();
                    para.borrow_mut().hasBullet = true;
                    para.borrow_mut().bulletSet = true;
                    para.borrow_mut().bulletChar = "•".to_string();
                    para.borrow_mut().marginLeftPt = ((ln.borrow().level as f64) * size) * 0.6;
                    para.borrow_mut().indentPt = 0.0 - size * 0.6;
                    para.borrow_mut().marginSet = true;
                    para.borrow_mut().indentSet = true;
                } else {
                    para.borrow_mut().align = align.clone();
                }
                let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                run.borrow_mut().text = ln.borrow().text.clone();
                run.borrow_mut().fontSizePt = size;
                run.borrow_mut().sizeSet = true;
                run.borrow_mut().bold = ln.borrow().bold;
                run.borrow_mut().boldSet = ln.borrow().bold;
                run.borrow_mut().color = paint.borrow().text.clone();
                run.borrow_mut().colorStated = true;
                if text.borrow().measure.borrow().hasHost {
                    run.borrow_mut().fontFamily = text.borrow().measure.borrow().family.clone();
                }
                para.borrow_mut().runs.push(run.clone());
                body.borrow_mut().paragraphs.push(para.clone());
                sh.borrow_mut().diagramParaPts.push(ln.borrow().ptId.clone());
                k += 1;
            }
        }
        let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(sh.clone());
        hit
    }
}
