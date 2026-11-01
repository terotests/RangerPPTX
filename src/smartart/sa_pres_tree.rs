// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaPresTree — a layout definition run against a data model
//! ==============================================================================
//!
//! The result is a tree of SaInst: one per layout node the program reached,
//! each with the data point it was reached FOR (its context), the points it
//! presents (`presOf`), and what the program said about it on the way — its
//! algorithm, shape, constraints, rules and variables, including whatever the
//! `choose` branches it went through added.
//!
//! THE DATA AS THE PROGRAM SEES IT. Under a parent, its children are not just
//! the nodes: each parent-of connection contributes its parent transition, the
//! node, and its sibling transition, in that order. That is what lets
//!
//!     <dgm:forEach axis="followSib" ptType="sibTrans" cnt="1">
//!
//! mean "the arrow after this step": the sibling transition is the next thing
//! after the node among its siblings. `hideLastTrans` (on by default) drops the
//! transition after the last child, so a process of three steps has two arrows.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::smartart::sa_layout_def::SaAlgDef;
use crate::smartart::sa_layout_def::SaConstr;
use crate::smartart::sa_layout_def::SaIter;
use crate::smartart::sa_layout_def::SaLayoutDef;
use crate::smartart::sa_layout_def::SaLoNode;
use crate::smartart::sa_layout_def::SaParam;
use crate::smartart::sa_layout_def::SaRule;
use crate::smartart::sa_layout_def::SaShapeDef;
use crate::smartart::sa_model::SaCxn;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPoint;

#[derive(Clone)]
pub struct SaInst {
    pub lo: Rc<RefCell<SaLoNode>>,
    pub name: String,
    pub styleLbl: String,
    /// the data point this instance was made for
    pub ctxId: String,
    pub ctxType: String,
    /// the points it presents, in order
    pub presIds: Vec<String>,
    pub hasAlg: bool,
    pub alg: Rc<RefCell<SaAlgDef>>,
    pub hasShape: bool,
    pub shape: Rc<RefCell<SaShapeDef>>,
    pub constrs: Vec<Rc<RefCell<SaConstr>>>,
    pub rules: Vec<Rc<RefCell<SaRule>>>,
    pub vars: Vec<Rc<RefCell<SaParam>>>,
    pub children: Vec<Rc<RefCell<SaInst>>>,
    pub depth: i64,
    /// position in the forEach that made it, 1-based, and how many there were
    pub pos: i64,
    pub cnt: i64,
    /// index among instances of the same name, 0-based: what colour cycling counts
    pub nameIdx: i64,
    /// geometry, filled in by the constraints and the algorithms (points, in
    /// the frame's space)
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub fontPt: f64,
    /// constraint values (SaConstraints): by type, with what each was derived
    /// from ("w", "h", "" for a number) and its bounds from gte / lte
    pub vNames: Vec<String>,
    pub vVals: Vec<f64>,
    pub vFrom: Vec<String>,
    pub bNames: Vec<String>,
    pub bLo: Vec<f64>,
    pub bHi: Vec<f64>,
    /// the instances whose primary font size must be one size with this one's
    pub fontGroup: i64,
    /// what an algorithm set for this one instance (phase 2): a rotation in
    /// degrees, clockwise; adjust values by name, in 100000ths, used instead of
    /// the shape's own; the straight segments a connector line is drawn as
    /// (x1 y1 x2 y2 …, in the frame's space) and whether the last one ends in
    /// an arrowhead; and that it was placed by an algorithm that lays out a
    /// whole subtree at once (hierarchy), so its own pass leaves it be
    pub rotDeg: f64,
    /// how much taller than its constraints a host's row made it
    /// (SaAlgorithms.stretch)
    pub stretchK: f64,
    pub adjNames: Vec<String>,
    pub adjVals: Vec<f64>,
    pub segs: Vec<f64>,
    pub arrowEnd: bool,
    pub placed: bool,
}

impl SaInst {
    pub fn new() -> Self {
        Self {
            lo: Rc::new(RefCell::new(SaLoNode::new())),
            name: String::new(),
            styleLbl: String::new(),
            ctxId: String::new(),
            ctxType: String::new(),
            presIds: Vec::new(),
            hasAlg: false,
            alg: Rc::new(RefCell::new(SaAlgDef::new())),
            hasShape: false,
            shape: Rc::new(RefCell::new(SaShapeDef::new())),
            constrs: Vec::new(),
            rules: Vec::new(),
            vars: Vec::new(),
            children: Vec::new(),
            depth: 0,
            pos: 1,
            cnt: 1,
            nameIdx: 0,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            fontPt: 0.0,
            vNames: Vec::new(),
            vVals: Vec::new(),
            vFrom: Vec::new(),
            bNames: Vec::new(),
            bLo: Vec::new(),
            bHi: Vec::new(),
            fontGroup: -1,
            rotDeg: 0.0,
            stretchK: 1.0,
            adjNames: Vec::new(),
            adjVals: Vec::new(),
            segs: Vec::new(),
            arrowEnd: false,
            placed: false,
        }
    }
    pub fn isLength(t: &str) -> bool {
        if (t == "primFontSz" || t == "secFontSz") || t == "pyraAcctRatio" {
            return false;
        }
        PptxText::index_of(&t, "user") != 0
    }
    pub fn slot(&self, r#type: &str) -> i64 {
        let mut i: i64 = 0;
        while i < (self.vNames.len() as i64) {
            if self.vNames[i as usize].clone() == r#type {
                return i;
            }
            i += 1;
        }
        -1
    }
    pub fn has(&self, r#type: &str) -> bool {
        self.slot(r#type) >= 0
    }
    pub fn put(&mut self, r#type: &str, v: f64, from: &str) {
        let i: i64 = self.slot(r#type);
        if i < 0 {
            self.vNames.push(r#type.to_string());
            self.vVals.push(v);
            self.vFrom.push(from.to_string());
            return;
        }
        self.vVals[i as usize] = v;
        self.vFrom[i as usize] = from.to_string();
    }
    pub fn raw(&self, r#type: &str, fallback: f64) -> f64 {
        let i: i64 = self.slot(r#type);
        if i < 0 {
            return fallback;
        }
        self.vVals[i as usize]
    }
    pub fn fromOf(&self, r#type: &str) -> String {
        let i: i64 = self.slot(r#type);
        if i < 0 {
            return String::new();
        }
        self.vFrom[i as usize].clone()
    }
    pub fn bound(&mut self, r#type: &str, lo: f64, hi: f64) {
        let mut i: i64 = 0;
        while i < (self.bNames.len() as i64) {
            if self.bNames[i as usize].clone() == r#type {
                if lo > self.bLo[i as usize] {
                    self.bLo[i as usize] = lo;
                }
                if hi < self.bHi[i as usize] {
                    self.bHi[i as usize] = hi;
                }
                return;
            }
            i += 1;
        }
        self.bNames.push(r#type.to_string());
        self.bLo.push(lo);
        self.bHi.push(hi);
    }
    pub fn lowOf(&self, r#type: &str) -> f64 {
        let mut i: i64 = 0;
        while i < (self.bNames.len() as i64) {
            if self.bNames[i as usize].clone() == r#type {
                return self.bLo[i as usize];
            }
            i += 1;
        }
        -1000000000.0
    }
    pub fn highOf(&self, r#type: &str) -> f64 {
        let mut i: i64 = 0;
        while i < (self.bNames.len() as i64) {
            if self.bNames[i as usize].clone() == r#type {
                return self.bHi[i as usize];
            }
            i += 1;
        }
        1000000000.0
    }
    /// every length this instance and those under it hold, times `s`
    pub fn scaleLengths(&mut self, s: f64) {
        let mut i: i64 = 0;
        while i < (self.vNames.len() as i64) {
            if SaInst::isLength(&self.vNames[i as usize].clone()) {
                self.vVals[i as usize] = self.vVals[i as usize] * s;
            }
            i += 1;
        }
        let mut k: i64 = 0;
        while k < (self.children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = self.children[k as usize].clone();
            c.borrow_mut().scaleLengths(s);
            k += 1;
        }
    }
    pub fn varValue(&self, name: &str, fallback: &str) -> String {
        let mut i: i64 = (self.vars.len() as i64) - 1;
        while i >= 0 {
            let mut p: Rc<RefCell<SaParam>> = self.vars[i as usize].clone();
            if p.borrow().name == name {
                return p.borrow().value.clone();
            }
            i -= 1;
        }
        fallback.to_string()
    }
    /// every instance under this one, depth first, this one excluded
    pub fn descendants(&self) -> Vec<Rc<RefCell<SaInst>>> {
        let mut out: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        self.collect(&mut out);
        out
    }
    pub fn collect(&self, mut out: &mut Vec<Rc<RefCell<SaInst>>>) {
        let mut i: i64 = 0;
        while i < (self.children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = self.children[i as usize].clone();
            out.push(c.clone());
            c.borrow().collect(&mut out);
            i += 1;
        }
    }
    pub fn childNamed(&self, n: &str) -> Option<Rc<RefCell<SaInst>>> {
        let mut i: i64 = 0;
        while i < (self.children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = self.children[i as usize].clone();
            if c.borrow().name == n {
                let mut hit: Option<Rc<RefCell<SaInst>>> = Some(c.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaInst>>> = None;
        none
    }
}

#[derive(Clone)]
pub struct SaPresTree {
    pub model: Rc<RefCell<SaModel>>,
    pub layout: Rc<RefCell<SaLayoutDef>>,
    pub root: Rc<RefCell<SaInst>>,
    pub error: String,
    /// how many instances, a guard against a layout that never ends
    pub made: i64,
    pub maxInstances: i64,
}

impl SaPresTree {
    pub fn new() -> Self {
        Self {
            model: Rc::new(RefCell::new(SaModel::new())),
            layout: Rc::new(RefCell::new(SaLayoutDef::new())),
            root: Rc::new(RefCell::new(SaInst::new())),
            error: String::new(),
            made: 0,
            maxInstances: 5000,
        }
    }
    pub fn build(m: Rc<RefCell<SaModel>>, lo: Rc<RefCell<SaLayoutDef>>) -> Rc<RefCell<SaPresTree>> {
        let mut t: Rc<RefCell<SaPresTree>> = Rc::new(RefCell::new(SaPresTree::new()));
        t.borrow_mut().model = m.clone();
        t.borrow_mut().layout = lo.clone();
        let mut r: Rc<RefCell<SaInst>> = Rc::new(RefCell::new(SaInst::new()));
        let mut d: Rc<RefCell<SaPoint>> = m.borrow().doc();
        r.borrow_mut().ctxId = d.borrow().id.clone();
        r.borrow_mut().ctxType = "doc".to_string();
        t.borrow_mut().root = r.clone();
        let mut _tmp_220 = lo.borrow().root.clone();
        t.borrow_mut().runLayoutNode(_tmp_220, &r, d.borrow().id.clone(), 1, 1);
        t
    }
    pub fn pick(list: &[String], st: i64, cnt: i64, step: i64) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let n: i64 = list.len() as i64;
        if n == 0 {
            return out;
        }
        let mut start: i64 = st - 1;
        if st < 0 {
            start = n + st;
        }
        if st == 0 {
            start = 0;
        }
        let mut stride: i64 = step;
        if stride == 0 {
            stride = 1;
        }
        let mut i: i64 = start;
        while i >= 0 && i < n {
            out.push(list[i as usize].clone());
            if cnt > 0 && (out.len() as i64) >= cnt {
                return out;
            }
            i += stride;
        }
        out
    }
    pub fn boolText(v: &str) -> String {
        if v == "true" || v == "1" {
            return "1".to_string();
        }
        "0".to_string()
    }
    pub fn varDefault(name: &str) -> String {
        if name == "dir" {
            return "norm".to_string();
        }
        if name == "hierBranch" {
            return "std".to_string();
        }
        if name == "chMax" {
            return "-1".to_string();
        }
        if name == "chPref" {
            return "-1".to_string();
        }
        if name == "bulletEnabled" {
            return "0".to_string();
        }
        if name == "orgChart" {
            return "0".to_string();
        }
        String::new()
    }
    /// Numbers compared as numbers, anything else as text.
    pub fn compare(have: &str, op: &str, want: &str) -> bool {
        let a: Option<f64> = have.trim().to_string().parse::<f64>().ok();
        let b: Option<f64> = want.trim().to_string().parse::<f64>().ok();
        if a.is_some() && b.is_some() {
            let x: f64 = a.unwrap();
            let y: f64 = b.unwrap();
            if op == "equ" {
                return x == y;
            }
            if op == "neq" {
                return x != y;
            }
            if op == "gt" {
                return x > y;
            }
            if op == "lt" {
                return x < y;
            }
            if op == "gte" {
                return x >= y;
            }
            if op == "lte" {
                return x <= y;
            }
            return false;
        }
        if op == "equ" {
            return have == want;
        }
        if op == "neq" {
            return have != want;
        }
        false
    }
    /// ------------------------------------------------------------------
    /// The data, as the program walks it
    /// ------------------------------------------------------------------
    pub fn typeOf(&self, id: &str) -> String {
        let mut p: Option<Rc<RefCell<SaPoint>>> = self.model.borrow().pointById(id);
        if p.is_none() {
            return String::new();
        }
        return p.as_ref().unwrap().borrow().r#type.clone();
    }
    /// Under `id`: for each parent-of connection, its parent transition, the
    /// node and its sibling transition.
    pub fn chList(&self, id: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cs: Vec<Rc<RefCell<SaCxn>>> = self.model.borrow().childCxns(id);
        let mut i: i64 = 0;
        while i < (cs.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = cs[i as usize].clone();
            if !c.borrow().parTransId.is_empty() {
                out.push(c.borrow().parTransId.clone());
            }
            out.push(c.borrow().destId.clone());
            if !c.borrow().sibTransId.is_empty() {
                out.push(c.borrow().sibTransId.clone());
            }
            i += 1;
        }
        out
    }
    /// the parent: a node's is the source of its parent-of connection; a
    /// transition's is the source of the connection it belongs to
    pub fn parentOf(&self, id: &str) -> String {
        let mut p: Option<Rc<RefCell<SaPoint>>> = self.model.borrow().pointById(id);
        if p.is_none() {
            return String::new();
        }
        if p.as_ref()
            .unwrap()
            .borrow()
            .r#type == "parTrans" || p.as_ref()
            .unwrap()
            .borrow()
            .r#type == "sibTrans" {
            let mut c: Option<Rc<RefCell<SaCxn>>> = self.model.borrow().cxnById(&p.as_ref().unwrap().borrow().cxnId.clone());
            if c.is_none() {
                return String::new();
            }
            return c.as_ref().unwrap().borrow().srcId.clone();
        }
        let mut pc: Option<Rc<RefCell<SaCxn>>> = self.model.borrow().parentCxn(id);
        if pc.is_none() {
            return String::new();
        }
        return pc.as_ref().unwrap().borrow().srcId.clone();
    }
    pub fn siblingsOf(&self, id: &str) -> Vec<String> {
        let par: String = self.parentOf(id);
        if par.is_empty() {
            let mut none: Vec<String> = Vec::new();
            return none;
        }
        self.chList(&par)
    }
    pub fn desOf(&self, id: &str, mut out: &mut Vec<String>) {
        let mut kids: Vec<String> = self.chList(id);
        let mut i: i64 = 0;
        while i < (kids.len() as i64) {
            let k: String = kids[i as usize].clone();
            out.push(k.clone());
            self.desOf(&k, &mut out);
            i += 1;
        }
    }
    pub fn axisFrom(&self, id: &str, axis: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if axis == "self" || axis == "none" {
            out.push(id.to_string());
            return out;
        }
        if axis == "ch" {
            return self.chList(id);
        }
        if axis == "des" {
            self.desOf(id, &mut out);
            return out;
        }
        if axis == "desOrSelf" {
            out.push(id.to_string());
            self.desOf(id, &mut out);
            return out;
        }
        if axis == "par" || (axis == "ancst" || axis == "ancstOrSelf") {
            if axis == "ancstOrSelf" {
                out.push(id.to_string());
            }
            let mut at: String = self.parentOf(id);
            let mut guard: i64 = 0;
            while !at.is_empty() && guard < 10000 {
                out.push(at.clone());
                if axis == "par" {
                    return out;
                }
                at = self.parentOf(&at);
                guard += 1;
            }
            return out;
        }
        if axis == "root" {
            let mut d: Rc<RefCell<SaPoint>> = self.model.borrow().doc();
            out.push(d.borrow().id.clone());
            return out;
        }
        if axis == "followSib" || (axis == "precedSib" || (axis == "follow" || axis == "preced")) {
            let mut sibs: Vec<String> = self.siblingsOf(id);
            let mut me: i64 = -1;
            let mut i: i64 = 0;
            while i < (sibs.len() as i64) {
                if sibs[i as usize].clone() == id {
                    me = i;
                }
                i += 1;
            }
            if me < 0 {
                return out;
            }
            if axis == "followSib" || axis == "follow" {
                let mut j: i64 = me + 1;
                while j < (sibs.len() as i64) {
                    out.push(sibs[j as usize].clone());
                    if axis == "follow" {
                        self.desOf(&sibs[j as usize].clone(), &mut out);
                    }
                    j += 1;
                }
            } else {
                let mut k: i64 = me - 1;
                while k >= 0 {
                    out.push(sibs[k as usize].clone());
                    k -= 1;
                }
            }
            return out;
        }
        out
    }
    pub fn matchesType(&self, id: &str, want: &str) -> bool {
        if want == "all" {
            return true;
        }
        let t: String = self.typeOf(id);
        if want == "node" {
            return t == "node" || t == "asst";
        }
        if want == "norm" {
            return t == "node";
        }
        if want == "nonNorm" {
            return t != "node";
        }
        if want == "nonAsst" {
            return t != "asst";
        }
        t == want
    }
    /// the transition after the last of its parent's children
    pub fn isLastTrans(&self, id: &str) -> bool {
        let mut p: Option<Rc<RefCell<SaPoint>>> = self.model.borrow().pointById(id);
        if p.is_none() {
            return false;
        }
        if p.as_ref().unwrap().borrow().r#type != "sibTrans" {
            return false;
        }
        let mut c: Option<Rc<RefCell<SaCxn>>> = self.model.borrow().cxnById(&p.as_ref().unwrap().borrow().cxnId.clone());
        if c.is_none() {
            return false;
        }
        let mut cs: Vec<Rc<RefCell<SaCxn>>> = self.model.borrow().childCxns(&c.as_ref().unwrap().borrow().srcId.clone());
        if cs.is_empty() {
            return false;
        }
        let mut last: Rc<RefCell<SaCxn>> = cs[((cs.len() as i64) - 1) as usize].clone();
        return last.borrow().id == c.as_ref().unwrap().borrow().id;
    }
    /// The points a forEach, presOf or if walks to from `id`.
    pub fn walk(&self, id: &str, it: &Rc<RefCell<SaIter>>) -> Vec<String> {
        let mut cur: Vec<String> = Vec::new();
        cur.push(id.to_string());
        let steps: i64 = it.borrow().steps();
        if steps == 0 {
            // no axis: the point itself, filtered by type
            let mut only: Vec<String> = Vec::new();
            if self.matchesType(id, &it.borrow().ptTypeAt(0)) {
                only.push(id.to_string());
            }
            return only;
        }
        let mut s: i64 = 0;
        while s < steps {
            let mut next: Vec<String> = Vec::new();
            let mut ci: i64 = 0;
            while ci < (cur.len() as i64) {
                let mut found: Vec<String> = self.axisFrom(&cur[ci as usize].clone(),
                    &it.borrow().axisAt(s)
                );
                // type filter, last transition hidden
                let mut filtered: Vec<String> = Vec::new();
                let mut fi: i64 = 0;
                while fi < (found.len() as i64) {
                    let f: String = found[fi as usize].clone();
                    if self.matchesType(&f, &it.borrow().ptTypeAt(s)) {
                        let hidden: bool = it.borrow().hideLastTransAt(s) && self.isLastTrans(&f);
                        if !hidden {
                            filtered.push(f.clone());
                        }
                    }
                    fi += 1;
                }
                // st (1-based; negative from the end), step, cnt
                let mut picked: Vec<String> = SaPresTree::pick(
                    &filtered,
                    it.borrow().stAt(s),
                    it.borrow().cntAt(s),
                    it.borrow().stepAt(s)
                );
                let mut pi: i64 = 0;
                while pi < (picked.len() as i64) {
                    next.push(picked[pi as usize].clone());
                    pi += 1;
                }
                ci += 1;
            }
            cur = next.clone();
            s += 1;
        }
        cur
    }
    /// ------------------------------------------------------------------
    /// Running the program
    /// ------------------------------------------------------------------
    pub fn runLayoutNode(&mut self, lo: Rc<RefCell<SaLoNode>>, inst: &Rc<RefCell<SaInst>>, ctx: String, pos: i64, cnt: i64) {
        inst.borrow_mut().lo = lo.clone();
        inst.borrow_mut().name = lo.borrow().name.clone();
        inst.borrow_mut().styleLbl = lo.borrow().styleLbl.clone();
        inst.borrow_mut().ctxId = ctx.clone();
        inst.borrow_mut().ctxType = self.typeOf(&ctx);
        inst.borrow_mut().pos = pos;
        inst.borrow_mut().cnt = cnt;
        self.runBody(&lo, inst, &ctx, pos, cnt);
    }
    /// What a layoutNode, forEach body, if or else says, applied to `inst`,
    /// and its children run.
    pub fn runBody(&mut self, body: &Rc<RefCell<SaLoNode>>, inst: &Rc<RefCell<SaInst>>, ctx: &str, pos: i64, cnt: i64) {
        if self.made > self.maxInstances {
            self.error = format!("the layout made more than {} shapes; it does not end", self.maxInstances);
            return;
        }
        if body.borrow().hasAlg {
            inst.borrow_mut().hasAlg = true;
            inst.borrow_mut().alg = body.borrow().alg.clone();
        }
        if body.borrow().hasShape {
            inst.borrow_mut().hasShape = true;
            inst.borrow_mut().shape = body.borrow().shape.clone();
        }
        if body.borrow().hasPresOf {
            let mut ps: Vec<String> = self.walk(ctx, &body.borrow().presOf.clone());
            let mut pi: i64 = 0;
            while pi < (ps.len() as i64) {
                inst.borrow_mut().presIds.push(ps[pi as usize].clone());
                pi += 1;
            }
        }
        let mut k: i64 = 0;
        while k < (body.borrow().constrs.len() as i64) {
            inst.borrow_mut().constrs.push(body.borrow().constrs[k as usize].clone());
            k += 1;
        }
        k = 0;
        while k < (body.borrow().rules.len() as i64) {
            inst.borrow_mut().rules.push(body.borrow().rules[k as usize].clone());
            k += 1;
        }
        k = 0;
        while k < (body.borrow().vars.len() as i64) {
            inst.borrow_mut().vars.push(body.borrow().vars[k as usize].clone());
            k += 1;
        }
        let mut i: i64 = 0;
        while i < (body.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaLoNode>> = body.borrow().children[i as usize].clone();
            if c.borrow().kind == "layoutNode" {
                let mut child: Rc<RefCell<SaInst>> = Rc::new(RefCell::new(SaInst::new()));
                self.made += 1;
                child.borrow_mut().depth = inst.borrow().depth + 1;
                // variables are inherited: a child starts with its parent's
                let mut vi: i64 = 0;
                while vi < (inst.borrow().vars.len() as i64) {
                    child.borrow_mut().vars.push(inst.borrow().vars[vi as usize].clone());
                    vi += 1;
                }
                inst.borrow_mut().children.push(child.clone());
                self.runLayoutNode(c.clone(), &child, ctx.to_string(), pos, cnt);
            }
            if c.borrow().kind == "forEach" {
                self.runForEach(&c, inst, ctx);
            }
            if c.borrow().kind == "choose" {
                let mut branch: Option<Rc<RefCell<SaLoNode>>> = self.choose(&c,
                    inst,
                    ctx,
                    pos,
                    cnt
                );
                if branch.is_some() {
                    self.runBody(&branch.clone().unwrap(), inst, ctx, pos, cnt);
                }
            }
            i += 1;
        }
    }
    pub fn runForEach(&mut self, fe: &Rc<RefCell<SaLoNode>>, inst: &Rc<RefCell<SaInst>>, ctx: &str) {
        let mut body: Rc<RefCell<SaLoNode>> = fe.clone();
        if (fe.borrow().r#ref.as_bytes().len() as i64) > 0 {
            let mut target: Option<Rc<RefCell<SaLoNode>>> = self.layout.borrow().forEachNamed(&fe.borrow().r#ref.clone());
            if target.is_none() {
                self.error = format!("forEach {}: ref \"{}\" names no forEach", fe.borrow().name, fe.borrow().r#ref);
                return;
            }
            body = target.clone().unwrap();
        }
        let mut pts: Vec<String> = self.walk(ctx, &body.borrow().iter.clone());
        let n: i64 = pts.len() as i64;
        let mut i: i64 = 0;
        while i < n {
            // a forEach body has no algorithm or shape of its own; anything it
            // holds lands on the instance it runs in
            self.runBody(&body, inst, &pts[i as usize].clone(), i + 1, n);
            i += 1;
        }
    }
    pub fn choose(&self, ch: &Rc<RefCell<SaLoNode>>, inst: &Rc<RefCell<SaInst>>, ctx: &str, pos: i64, cnt: i64) -> Option<Rc<RefCell<SaLoNode>>> {
        let mut i: i64 = 0;
        while i < (ch.borrow().children.len() as i64) {
            let mut br: Rc<RefCell<SaLoNode>> = ch.borrow().children[i as usize].clone();
            if br.borrow().kind == "else" {
                let mut e: Option<Rc<RefCell<SaLoNode>>> = Some(br.clone());
                return e;
            }
            if self.test(&br, inst, ctx, pos, cnt) {
                let mut t: Option<Rc<RefCell<SaLoNode>>> = Some(br.clone());
                return t;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaLoNode>>> = None;
        none
    }
    pub fn test(&self, br: &Rc<RefCell<SaLoNode>>, inst: &Rc<RefCell<SaInst>>, ctx: &str, pos: i64, cnt: i64) -> bool {
        let f: String = br.borrow().func.clone();
        if f == "var" {
            let mut _tmp_246 = br.borrow().arg.clone();
            let have: String = inst.borrow().varValue(&_tmp_246, &SaPresTree::varDefault(&br.borrow().arg.clone()));
            let mut _tmp_248 = br.borrow().op.clone();
            return SaPresTree::compare(&have, &_tmp_248, &br.borrow().val.clone());
        }
        let mut value: i64 = 0;
        if f == "cnt" {
            value = self.walk(ctx, &br.borrow().iter).len() as i64;
        }
        if f == "pos" {
            value = pos;
        }
        if f == "revPos" {
            value = (cnt - pos) + 1;
        }
        if f == "posEven" {
            let mut _tmp_250 = br.borrow().op.clone();
            let _tmp_251 = SaPresTree::boolText(&br.borrow().val);
            return SaPresTree::compare(
                &if (pos % 2 == 0) { "1".to_string() } else { "0".to_string() },
                &_tmp_250,
                &_tmp_251
            );
        }
        if f == "posOdd" {
            let mut _tmp_253 = br.borrow().op.clone();
            let _tmp_254 = SaPresTree::boolText(&br.borrow().val);
            return SaPresTree::compare(
                &if (pos % 2 == 1) { "1".to_string() } else { "0".to_string() },
                &_tmp_253,
                &_tmp_254
            );
        }
        if f == "depth" {
            value = self.model.borrow().depthOf(ctx);
        }
        if f == "maxDepth" {
            value = self.maxDepthUnder(ctx, 0);
        }
        let mut _tmp_256 = br.borrow().op.clone();
        return SaPresTree::compare(&value.to_string(), &_tmp_256, &br.borrow().val.clone());
    }
    pub fn maxDepthUnder(&self, id: &str, at: i64) -> i64 {
        let mut best: i64 = at;
        let mut kids: Vec<Rc<RefCell<SaPoint>>> = self.model.borrow().children(id);
        let mut i: i64 = 0;
        while i < (kids.len() as i64) {
            let mut k: Rc<RefCell<SaPoint>> = kids[i as usize].clone();
            let d: i64 = self.maxDepthUnder(&k.borrow().id.clone(), at + 1);
            if d > best {
                best = d;
            }
            i += 1;
        }
        best
    }
    /// every instance, depth first, the root first
    pub fn all(&self) -> Vec<Rc<RefCell<SaInst>>> {
        let mut out: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        out.push(self.root.clone());
        self.root.clone().borrow().collect(&mut out);
        out
    }
    /// numbers instances of the same name in order: the colour index
    pub fn numberByName(&self) {
        let mut every: Vec<Rc<RefCell<SaInst>>> = self.all();
        let mut names: Vec<String> = Vec::new();
        let mut counts: Vec<i64> = Vec::new();
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            let mut at: i64 = -1;
            let mut j: i64 = 0;
            while j < (names.len() as i64) {
                if names[j as usize].clone() == s.borrow().name {
                    at = j;
                }
                j += 1;
            }
            if at < 0 {
                names.push(s.borrow().name.clone());
                counts.push(0);
                at = (names.len() as i64) - 1;
            }
            s.borrow_mut().nameIdx = counts[at as usize];
            counts[at as usize] = counts[at as usize] + 1;
            i += 1;
        }
    }
}
