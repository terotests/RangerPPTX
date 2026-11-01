// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaModel — a SmartArt diagram's data: the points and how they are connected
//! ==============================================================================
//!
//! `dgm:dataModel` is a graph, not a tree. Its points are the document, the
//! nodes a person typed, assistants, and three kinds the person never sees:
//! a parent transition and a sibling transition for every parent-of connection
//! (the connector shapes are drawn FOR them), and presentation points (one per
//! shape the layout made). Its connections say which node is under which
//! (`parOf`), and which shape presents which point (`presOf`, `presParOf`).
//!
//! A file written by PowerPoint has all of that. A file written by anyone else
//! usually has the document, the nodes and the parent-of connections, and
//! nothing more — so `complete` makes the transitions a file left out, with ids
//! derived from the connection's own, never random: the same file gives the
//! same model on every target and every run.
//!
//! Presentation points are the layout's business (phase 1). They are kept when
//! a file has them and never made here.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;


/// One run of text: what `a:r` says, as much of it as a diagram needs.
#[derive(Clone)]
pub struct SaRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// points, 0 when the run does not say
    pub sizePt: f64,
    /// `RRGGBB` from `a:solidFill/a:srgbClr`, "" when the run does not say
    pub color: String,
}

impl SaRun {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            bold: false,
            italic: false,
            underline: false,
            sizePt: 0.0,
            color: String::new(),
        }
    }
}

/// One paragraph of a point's text. `level` is `a:pPr/@lvl`: a bullet under the
/// node's first line in the layouts that show sub-points inside the shape.
#[derive(Clone)]
pub struct SaPara {
    pub runs: Vec<Rc<RefCell<SaRun>>>,
    pub level: i64,
}

impl SaPara {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            level: 0,
        }
    }
    pub fn plainText(&self) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (self.runs.len() as i64) {
            let mut r: Rc<RefCell<SaRun>> = self.runs[i as usize].clone();
            out = format!("{}{}", out, r.borrow().text);
            i += 1;
        }
        out
    }
}

#[derive(Clone)]
pub struct SaPoint {
    pub id: String,
    pub r#type: String,
    /// for a transition: the parent-of connection it belongs to
    pub cxnId: String,
    pub paras: Vec<Rc<RefCell<SaPara>>>,
    /// `dgm:t` was there at all — an empty node and a node with no text
    /// element are written back the way they came
    pub hasText: bool,
    /// prSet. On the document point the three ids name the diagram's layout,
    /// quick style and colours; on a presentation point the rest say which
    /// layout node it was made by.
    pub layoutId: String,
    pub styleId: String,
    pub colorsId: String,
    pub presName: String,
    pub presStyleLbl: String,
    pub presStyleIdx: i64,
    pub presStyleCnt: i64,
    pub presAssocId: String,
}

impl SaPoint {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            r#type: "node".to_string(),
            cxnId: String::new(),
            paras: Vec::new(),
            hasText: false,
            layoutId: String::new(),
            styleId: String::new(),
            colorsId: String::new(),
            presName: String::new(),
            presStyleLbl: String::new(),
            presStyleIdx: -1,
            presStyleCnt: -1,
            presAssocId: String::new(),
        }
    }
    pub fn plainText(&self) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (self.paras.len() as i64) {
            let mut p: Rc<RefCell<SaPara>> = self.paras[i as usize].clone();
            if i > 0 {
                out = format!("{}\n", out);
            }
            out = format!("{}{}", out, p.borrow().plainText());
            i += 1;
        }
        out
    }
    /// A point a person typed: the document's children and theirs.
    pub fn isContent(&self) -> bool {
        self.r#type == "node" || self.r#type == "asst"
    }
}

#[derive(Clone)]
pub struct SaCxn {
    pub id: String,
    pub r#type: String,
    pub srcId: String,
    pub destId: String,
    pub srcOrd: i64,
    pub destOrd: i64,
    /// for parOf: the transition points drawn for it
    pub parTransId: String,
    pub sibTransId: String,
    /// for presOf/presParOf
    pub presId: String,
}

impl SaCxn {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            r#type: "parOf".to_string(),
            srcId: String::new(),
            destId: String::new(),
            srcOrd: 0,
            destOrd: 0,
            parTransId: String::new(),
            sibTransId: String::new(),
            presId: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct SaModel {
    pub points: Vec<Rc<RefCell<SaPoint>>>,
    pub cxns: Vec<Rc<RefCell<SaCxn>>>,
    /// `dgm:extLst/a:ext/dsp:dataModelExt/@relId`: the relationship, from the
    /// SLIDE, to the drawing part PowerPoint last drew this diagram into
    pub drawingRelId: String,
}

impl SaModel {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            cxns: Vec::new(),
            drawingRelId: String::new(),
        }
    }
    pub fn pointById(&self, id: &str) -> Option<Rc<RefCell<SaPoint>>> {
        let mut i: i64 = 0;
        while i < (self.points.len() as i64) {
            let mut p: Rc<RefCell<SaPoint>> = self.points[i as usize].clone();
            if p.borrow().id == id {
                let mut hit: Option<Rc<RefCell<SaPoint>>> = Some(p.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaPoint>>> = None;
        none
    }
    pub fn has(&self, id: &str) -> bool {
        let mut p: Option<Rc<RefCell<SaPoint>>> = self.pointById(id);
        p.is_some()
    }
    pub fn cxnById(&self, id: &str) -> Option<Rc<RefCell<SaCxn>>> {
        let mut i: i64 = 0;
        while i < (self.cxns.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = self.cxns[i as usize].clone();
            if c.borrow().id == id {
                let mut hit: Option<Rc<RefCell<SaCxn>>> = Some(c.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaCxn>>> = None;
        none
    }
    pub fn doc(&self) -> Rc<RefCell<SaPoint>> {
        let mut i: i64 = 0;
        while i < (self.points.len() as i64) {
            let mut p: Rc<RefCell<SaPoint>> = self.points[i as usize].clone();
            if p.borrow().r#type == "doc" {
                return p;
            }
            i += 1;
        }
        Rc::new(RefCell::new(SaPoint::new()))
    }
    pub fn layoutId(&self) -> String {
        let mut d: Rc<RefCell<SaPoint>> = self.doc();
        return d.borrow().layoutId.clone();
    }
    /// The parent-of connections out of `id`, in `srcOrd` order.
    pub fn childCxns(&self, id: &str) -> Vec<Rc<RefCell<SaCxn>>> {
        let mut out: Vec<Rc<RefCell<SaCxn>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (self.cxns.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = self.cxns[i as usize].clone();
            if c.borrow().r#type == "parOf" && c.borrow().srcId == id {
                // insertion by srcOrd; equal orders keep file order
                let mut at: i64 = out.len() as i64;
                let mut moving: bool = true;
                while moving && at > 0 {
                    let mut before: Rc<RefCell<SaCxn>> = out[(at - 1) as usize].clone();
                    if before.borrow().srcOrd > c.borrow().srcOrd {
                        at -= 1;
                    } else {
                        moving = false;
                    }
                }
                out.insert(at as usize, c.clone());
            }
            i += 1;
        }
        out
    }
    /// The points under `id` (nodes and assistants), in order.
    pub fn children(&self, id: &str) -> Vec<Rc<RefCell<SaPoint>>> {
        let mut out: Vec<Rc<RefCell<SaPoint>>> = Vec::new();
        let mut cs: Vec<Rc<RefCell<SaCxn>>> = self.childCxns(id);
        let mut i: i64 = 0;
        while i < (cs.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = cs[i as usize].clone();
            let mut p: Option<Rc<RefCell<SaPoint>>> = self.pointById(&c.borrow().destId.clone());
            if p.is_some() {
                out.push(p.clone().unwrap());
            }
            i += 1;
        }
        out
    }
    /// The parent-of connection INTO `id`, if there is one.
    pub fn parentCxn(&self, id: &str) -> Option<Rc<RefCell<SaCxn>>> {
        let mut i: i64 = 0;
        while i < (self.cxns.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = self.cxns[i as usize].clone();
            if c.borrow().r#type == "parOf" && c.borrow().destId == id {
                let mut hit: Option<Rc<RefCell<SaCxn>>> = Some(c.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaCxn>>> = None;
        none
    }
    /// How deep `id` is: the document 0, its children 1.
    pub fn depthOf(&self, id: &str) -> i64 {
        let mut d: i64 = 0;
        let mut at: String = id.to_string();
        let mut guard: i64 = 0;
        while guard < (self.points.len() as i64) {
            let mut c: Option<Rc<RefCell<SaCxn>>> = self.parentCxn(&at);
            if c.is_none() {
                return d;
            }
            d += 1;
            at = c.as_ref().unwrap().borrow().srcId.clone();
            guard += 1;
        }
        d
    }
    /// The text tree, one line per content point, indented two spaces per
    /// level: what a test or a diagnostic compares.
    pub fn outline(&self) -> String {
        let mut out: String = String::new();
        let mut d: Rc<RefCell<SaPoint>> = self.doc();
        out = self.outlineUnder(&d.borrow().id, 0, &out);
        out
    }
    pub fn outlineUnder(&self, id: &str, level: i64, acc: &str) -> String {
        let mut out: String = acc.to_string();
        let mut kids: Vec<Rc<RefCell<SaPoint>>> = self.children(id);
        let mut i: i64 = 0;
        while i < (kids.len() as i64) {
            let mut k: Rc<RefCell<SaPoint>> = kids[i as usize].clone();
            let mut pad: String = String::new();
            let mut j: i64 = 0;
            while j < level {
                pad = format!("{}  ", pad);
                j += 1;
            }
            out = format!("{}{}{}\n", out, pad, k.borrow().plainText());
            out = self.outlineUnder(&k.borrow().id, level + 1, &out);
            i += 1;
        }
        out
    }
    pub fn countOf(&self, r#type: &str) -> i64 {
        let mut n: i64 = 0;
        let mut i: i64 = 0;
        while i < (self.points.len() as i64) {
            let mut p: Rc<RefCell<SaPoint>> = self.points[i as usize].clone();
            if p.borrow().r#type == r#type {
                n += 1;
            }
            i += 1;
        }
        n
    }
    /// Makes what a file may leave out: connection ids, and the parent and
    /// sibling transition of every parent-of connection. Ids are derived from
    /// the connection's, so `complete` twice is `complete` once.
    pub fn complete(&mut self) {
        let mut i: i64 = 0;
        while i < (self.cxns.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = self.cxns[i as usize].clone();
            if c.borrow().id.is_empty() {
                c.borrow_mut().id = self.freeId(&format!("cxn-{}-{}", c.borrow().srcId, c.borrow().destId));
            }
            i += 1;
        }
        i = 0;
        while i < (self.cxns.len() as i64) {
            let mut c2: Rc<RefCell<SaCxn>> = self.cxns[i as usize].clone();
            if c2.borrow().r#type == "parOf" {
                if c2.borrow().parTransId.is_empty() {
                    c2.borrow_mut().parTransId = self.addTransition(&c2, "parTrans".to_string());
                }
                if c2.borrow().sibTransId.is_empty() {
                    c2.borrow_mut().sibTransId = self.addTransition(&c2, "sibTrans".to_string());
                }
            }
            i += 1;
        }
    }
    pub fn addTransition(&mut self, c: &Rc<RefCell<SaCxn>>, r#type: String) -> String {
        let mut t: Rc<RefCell<SaPoint>> = Rc::new(RefCell::new(SaPoint::new()));
        t.borrow_mut().id = self.freeId(&format!("{}-{}", c.borrow().id, r#type));
        t.borrow_mut().r#type = r#type.clone();
        t.borrow_mut().cxnId = c.borrow().id.clone();
        self.points.push(t.clone());
        return t.borrow().id.clone();
    }
    /// `want`, or `want-2`, `want-3`… whichever no point or connection has.
    pub fn freeId(&self, want: &str) -> String {
        let mut id: String = want.to_string();
        let mut n: i64 = 1;
        while self.has(&id) || self.cxnTaken(&id) {
            n += 1;
            id = format!("{}-{}", want, n);
        }
        id
    }
    pub fn cxnTaken(&self, id: &str) -> bool {
        let mut c: Option<Rc<RefCell<SaCxn>>> = self.cxnById(id);
        c.is_some()
    }
}
