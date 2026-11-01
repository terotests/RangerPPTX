// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaText — the text a shape shows, and the size it is set at
//! ==============================================================================
//!
//! A shape shows the points its `presOf` walked to. The first is its own text;
//! the ones under it (presOf axis="desOrSelf") are bullets, one level per level
//! of depth below it — that is how a vertical bullet list's box holds a heading
//! and its points.
//!
//! The size starts at the shape's primary font size (65 pt unless a constraint
//! says otherwise, and never above an `lte` bound) and comes down, half a point
//! at a time, until the text fits the box inside its margins — as far as a
//! `primFontSz` rule allows (its `val` is the floor). Shapes whose font size
//! was made one by an `equ` constraint then all take the smallest of them.
//!
//! Width is measured by an SaMeasure. A host with fonts hands it the measurer
//! its own text is laid out with and the family the diagram is drawn in
//! (`SaMeasure.over`): the engine fits the text with those widths, the painter
//! breaks the lines with them (SaRender) and every run names that family, so
//! what was measured is what is drawn. With no host measurer it is the
//! average-width guess, 0.52 em per character (counted as characters, not
//! bytes), which is what a headless test has. A subclass needs a
//! `Constructor () {}` of its own: without one the C++ writer stops with
//! "Cannot read properties of undefined (reading 'params')".
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_layout_def::SaRule;
use crate::smartart::sa_model::SaPara;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_model::SaRun;
use crate::smartart::sa_pres_tree::SaInst;
use crate::smartart::sa_pres_tree::SaPresTree;
use crate::rgr::{EVGCodepoint, EVGTextMeasurer};

pub struct SaMeasure {
    pub em: f64,
    /// the host's measurer and the family it measures in; unset, the guess
    pub hasHost: bool,
    host: Rc<RefCell<EVGTextMeasurer>>,
    pub family: String,
}

impl SaMeasure {
    pub fn new() -> Self {
        Self {
            em: 0.52,
            hasHost: false,
            host: Rc::new(RefCell::new(EVGTextMeasurer::new())),
            family: String::new(),
        }
    }
    /// widths from `m`, in `fam` (its bold cut is `fam-Bold`, the name EVG's
    /// layout and the slide's own text use)
    fn over(m: Rc<RefCell<EVGTextMeasurer>>, fam: String) -> Rc<RefCell<SaMeasure>> {
        let mut r: Rc<RefCell<SaMeasure>> = Rc::new(RefCell::new(SaMeasure::new()));
        r.borrow_mut().host = m.clone();
        r.borrow_mut().family = fam.clone();
        r.borrow_mut().hasHost = true;
        r
    }
    pub fn faceOf(&self, bold: bool) -> String {
        if bold {
            return format!("{}-Bold", self.family);
        }
        self.family.clone()
    }
    pub fn widthOf(&self, text: &str, size_pt: f64, bold: bool) -> f64 {
        if self.hasHost {
            if text.is_empty() {
                return 0.0;
            }
            let _tmp_370 = self.host.clone();
            return _tmp_370.borrow_mut().measureTextWidth(text.to_string(), self.faceOf(bold), size_pt);
        }
        let n: f64 = (EVGCodepoint::charCount(text)) as f64;
        let mut f: f64 = self.em;
        if bold {
            f = self.em * 1.08;
        }
        (n * size_pt) * f
    }
}

#[derive(Clone)]
pub struct SaLine {
    pub text: String,
    pub level: i64,
    pub bold: bool,
    /// the data point the line is the text of
    pub ptId: String,
}

impl SaLine {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            level: 0,
            bold: false,
            ptId: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct SaText {
    pub ptree: Rc<RefCell<SaPresTree>>,
    pub cons: Rc<RefCell<SaConstraints>>,
    pub measure: Rc<RefCell<SaMeasure>>,
    /// how tall a line is, as a multiple of the size: SmartArt sets its text at
    /// 90 % line spacing, and a line is 1.2 em
    pub lineFactor: f64,
    pub warnings: Vec<String>,
}

impl SaText {
    pub fn new() -> Self {
        Self {
            ptree: Rc::new(RefCell::new(SaPresTree::new())),
            cons: Rc::new(RefCell::new(SaConstraints::new())),
            measure: Rc::new(RefCell::new(SaMeasure::new())),
            lineFactor: 1.08,
            warnings: Vec::new(),
        }
    }
    pub fn fit(c: Rc<RefCell<SaConstraints>>, m: Rc<RefCell<SaMeasure>>) -> Rc<RefCell<SaText>> {
        let mut t: Rc<RefCell<SaText>> = Rc::new(RefCell::new(SaText::new()));
        t.borrow_mut().ptree = c.borrow().ptree.clone();
        t.borrow_mut().cons = c.clone();
        t.borrow_mut().measure = Rc::clone(&m);
        t.borrow_mut().fitAll();
        t
    }
    pub fn words(s: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cur: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = s.as_bytes().len() as i64;
        while i < n {
            let c: i64 = (s.as_bytes()[i as usize] as i64);
            if c == 32 || c == 10 {
                if !cur.is_empty() {
                    out.push(cur.clone());
                    cur = String::new();
                }
            } else {
                cur = format!("{}{}", cur, s[(i) as usize..(i + 1) as usize].to_string());
            }
            i += 1;
        }
        if !cur.is_empty() {
            out.push(cur.clone());
        }
        out
    }
    /// the paragraphs a shape shows: its first point's, then the points under
    /// it as bullets
    pub fn linesOf(&self, s: &Rc<RefCell<SaInst>>) -> Vec<Rc<RefCell<SaLine>>> {
        let mut out: Vec<Rc<RefCell<SaLine>>> = Vec::new();
        if s.borrow().presIds.is_empty() {
            return out;
        }
        // levels count from the point the shape was made for: its own text
        // is level 0, the points under it 1, 2…
        let base: i64 = (self.ptree.borrow().model).borrow().depthOf(&s.borrow().ctxId.clone());
        let mut i: i64 = 0;
        while i < (s.borrow().presIds.len() as i64) {
            let id: String = s.borrow().presIds[i as usize].clone();
            let mut p: Option<Rc<RefCell<SaPoint>>> = (self.ptree.borrow().model).borrow().pointById(&id);
            if p.is_some() {
                let mut pt: Rc<RefCell<SaPoint>> = p.clone().unwrap();
                let mut lvl: i64 = (self.ptree.borrow().model).borrow().depthOf(&id) - base;
                if lvl < 0 {
                    lvl = 0;
                }
                if pt.borrow().isContent() || (pt.borrow().r#type == "parTrans" || pt.borrow().r#type == "sibTrans") {
                    let mut k: i64 = 0;
                    while k < (pt.borrow().paras.len() as i64) {
                        let mut para: Rc<RefCell<SaPara>> = pt.borrow().paras[k as usize].clone();
                        let mut ln: Rc<RefCell<SaLine>> = Rc::new(RefCell::new(SaLine::new()));
                        ln.borrow_mut().text = para.borrow().plainText();
                        ln.borrow_mut().ptId = id.clone();
                        ln.borrow_mut().level = lvl + para.borrow().level;
                        if !para.borrow().runs.is_empty() {
                            let mut r0: Rc<RefCell<SaRun>> = para.borrow().runs[0].clone();
                            ln.borrow_mut().bold = r0.borrow().bold;
                        }
                        out.push(ln.clone());
                        k += 1;
                    }
                }
            }
            i += 1;
        }
        // a shape whose points have no text at all shows nothing
        let mut any: bool = false;
        let mut j: i64 = 0;
        while j < (out.len() as i64) {
            let mut lj: Rc<RefCell<SaLine>> = out[j as usize].clone();
            if !lj.borrow().text.is_empty() {
                any = true;
            }
            j += 1;
        }
        if !any {
            let mut none: Vec<Rc<RefCell<SaLine>>> = Vec::new();
            return none;
        }
        out
    }
    /// how the shape's margins come out at `size`: a margin derived from the
    /// font size follows it
    pub fn margin(&self, s: &Rc<RefCell<SaInst>>, r#type: &str, size: f64) -> f64 {
        if !s.borrow().has(r#type) {
            return size * 0.1;
        }
        let v: f64 = s.borrow().raw(r#type, 0.0);
        if s.borrow().fromOf(r#type) == "primFontSz" {
            let nominal: f64 = self.cons.borrow().get(s, "primFontSz");
            if nominal > 0.0 {
                return (v * size) / nominal;
            }
        }
        v
    }
    /// lines once wrapped into width `w` at `size`; -1 when a word alone is
    /// wider than the box
    pub fn wrappedCount(&self, lines: &Vec<Rc<RefCell<SaLine>>>, size: f64, w: f64) -> i64 {
        let mut total: i64 = 0;
        let mut i: i64 = 0;
        while i < (lines.len() as i64) {
            let mut ln: Rc<RefCell<SaLine>> = lines[i as usize].clone();
            let indent: f64 = ((ln.borrow().level as f64) * size) * 0.6;
            let avail: f64 = w - indent;
            let mut _tmp_377 = ln.borrow().text.clone();
            let n: i64 = self.wrapOne(&_tmp_377, size, avail, ln.borrow().bold.clone());
            if n < 0 {
                return -1;
            }
            total += n;
            i += 1;
        }
        total
    }
    pub fn wrapOne(&self, text: &str, size: f64, w: f64, bold: bool) -> i64 {
        if text.is_empty() {
            return 1;
        }
        let mut words: Vec<String> = SaText::words(text);
        let mut count: i64 = 1;
        let mut cur: f64 = 0.0;
        let space: f64 = self.measure.borrow().widthOf(" ", size, bold);
        // as the painter breaks a line (PptxTextLayout.layoutFlow): a word
        // and the space after it go on the line together, so the space counts
        // against the room; the last word of the paragraph has none
        let last: i64 = (words.len() as i64) - 1;
        let mut i: i64 = 0;
        while i < (words.len() as i64) {
            let ww: f64 = self.measure.borrow().widthOf(&words[i as usize].clone(), size, bold);
            if ww > w {
                return -1;
            }
            let mut chunk: f64 = ww;
            if i < last {
                chunk = ww + space;
            }
            if cur == 0.0 {
                cur = chunk;
            } else {
                if cur + chunk <= w {
                    cur += chunk;
                } else {
                    count += 1;
                    cur = chunk;
                }
            }
            i += 1;
        }
        count
    }
    /// Does some shape's text break onto more lines than it has, at the size
    /// it was set at: text long enough that a taller shape would let it be
    /// larger (SaEngine's taller row).
    pub fn breaks(&self) -> bool {
        let mut every: Vec<Rc<RefCell<SaInst>>> = self.ptree.borrow().all();
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            let mut lines: Vec<Rc<RefCell<SaLine>>> = self.linesOf(&s);
            if !lines.is_empty() && s.borrow().fontPt > 0.0 {
                let mut _tmp_381 = s.borrow().fontPt.clone();
                let mut _tmp_382 = s.borrow().fontPt.clone();
                let w: f64 = (s.borrow().w - self.margin(&s,
                    "lMarg",
                    _tmp_381
                )) - self.margin(&s, "rMarg", _tmp_382);
                if self.wrappedCount(&lines, s.borrow().fontPt.clone(), w) > (lines.len() as i64) {
                    return true;
                }
            }
            i += 1;
        }
        false
    }
    pub fn fits(&self, s: &Rc<RefCell<SaInst>>, lines: &Vec<Rc<RefCell<SaLine>>>, size: f64) -> bool {
        let w: f64 = (s.borrow().w - self.margin(s,
            "lMarg",
            size
        )) - self.margin(s, "rMarg", size);
        let h: f64 = (s.borrow().h - self.margin(s,
            "tMarg",
            size
        )) - self.margin(s, "bMarg", size);
        if w <= 0.0 || h <= 0.0 {
            return false;
        }
        let n: i64 = self.wrappedCount(lines, size, w);
        if n < 0 {
            return false;
        }
        ((n as f64) * size) * self.lineFactor <= h
    }
    /// the smallest size a rule lets the shape's text go down to; its start
    /// size when no rule lets it shrink
    pub fn floorOf(&self, s: &Rc<RefCell<SaInst>>, start: f64) -> f64 {
        let mut i: i64 = 0;
        while i < (s.borrow().rules.len() as i64) {
            let mut r: Rc<RefCell<SaRule>> = s.borrow().rules[i as usize].clone();
            if r.borrow().r#type == "primFontSz" && r.borrow().forRel == "self" {
                if r.borrow().hasVal {
                    return r.borrow().val;
                }
                return 1.0;
            }
            i += 1;
        }
        start
    }
    pub fn fitAll(&mut self) {
        let mut every: Vec<Rc<RefCell<SaInst>>> = self.ptree.borrow().all();
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            let mut lines: Vec<Rc<RefCell<SaLine>>> = self.linesOf(&s);
            if !lines.is_empty() {
                let mut start: f64 = self.cons.borrow().get(&s, "primFontSz");
                let hi: f64 = s.borrow().highOf("primFontSz");
                if start > hi {
                    start = hi;
                }
                // on the half-point grid the sizes are stepped on
                start = (((start * 2.0 + 0.0001).floor() as i64) as f64) / 2.0;
                let floor: f64 = self.floorOf(&s, start);
                let mut size: f64 = start;
                while size > floor && !self.fits(&s, &lines, size) {
                    size -= 0.5;
                }
                if size < floor {
                    size = floor;
                }
                if !self.fits(&s, &lines, size) {
                    self.warnings.push(format!("the text of {} does not fit even at {} pt", s.borrow().name, size));
                }
                s.borrow_mut().fontPt = size;
            }
            i += 1;
        }
        // one size per equ group: the smallest any member needed
        i = 0;
        while i < (every.len() as i64) {
            let mut g: Rc<RefCell<SaInst>> = every[i as usize].clone();
            if g.borrow().fontGroup >= 0 && g.borrow().fontPt > 0.0 {
                let mut least: f64 = g.borrow().fontPt;
                let mut j: i64 = 0;
                while j < (every.len() as i64) {
                    let mut o: Rc<RefCell<SaInst>> = every[j as usize].clone();
                    if (o.borrow().fontGroup == g.borrow().fontGroup && o.borrow().fontPt > 0.0) && o.borrow().fontPt < least {
                        least = o.borrow().fontPt;
                    }
                    j += 1;
                }
                g.borrow_mut().fontPt = least;
            }
            i += 1;
        }
    }
}
