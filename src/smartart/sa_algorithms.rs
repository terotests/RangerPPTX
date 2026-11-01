// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaAlgorithms — where each shape goes
//! ==============================================================================
//!
//! Run top-down from the root, whose box is the frame. Each instance's
//! algorithm places its children inside its own box; then each child's subtree
//! is sized again (its constraints re-run with its new box) and placed in turn.
//!
//!   lin        children in a row or a column (`linDir` fromL fromR fromT fromB),
//!              `sibSp` apart, centred along the line and aligned across it
//!              (`nodeVertAlign` / `nodeHorzAlign`). A line longer than the box
//!              is SHRUNK to fit: every length along it, and across it every
//!              size that was derived from a length along it — a step whose
//!              height is 0.6 of its width stays that shape; a column whose
//!              height is its parent's keeps it. Then, if something is still
//!              taller (wider) than the box, everything shrinks once more.
//!   composite  children where their own l / t / r / b / ctrX / ctrY say,
//!              relative to the box, at the w / h they were given
//!   tx, sp, conn
//!              nothing to place. tx is text in the node's own shape (SaText);
//!              sp is room; a conn among a lin's children is placed by the lin
//!              like any child, and drawn as an arrow pointing along it.
//!   snake      a grid that wraps (SaSnake)
//!   cycle      a circle, or a radial with a shape in the middle (SaCycle)
//!   hierRoot, hierChild
//!              a tree, laid out at once from its outermost hierChild (SaHier)
//!   pyra       a triangle cut into levels (SaPyra)
//!
//! A connector its parent's algorithm did not place goes between the shapes
//! it names, or the ones before and after it (SaConn.connect).
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_conn::SaConn;
use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_cycle::SaCycle;
use crate::smartart::sa_hier::SaHier;
use crate::smartart::sa_layout_def::SaConstr;
use crate::smartart::sa_pres_tree::SaInst;
use crate::smartart::sa_pyra::SaPyra;
use crate::smartart::sa_snake::SaSnake;

#[derive(Clone)]
pub struct SaAlgorithms {
    pub cons: Rc<RefCell<SaConstraints>>,
    pub warnings: Vec<String>,
}

impl SaAlgorithms {
    pub fn new() -> Self {
        Self {
            cons: Rc::new(RefCell::new(SaConstraints::new())),
            warnings: Vec::new(),
        }
    }
    pub fn run(c: Rc<RefCell<SaConstraints>>) -> Rc<RefCell<SaAlgorithms>> {
        let mut a: Rc<RefCell<SaAlgorithms>> = Rc::new(RefCell::new(SaAlgorithms::new()));
        a.borrow_mut().cons = c.clone();
        let mut r: Rc<RefCell<SaInst>> = c.borrow().ptree.borrow().root.clone().clone();
        r.borrow_mut().x = 0.0;
        r.borrow_mut().y = 0.0;
        r.borrow_mut().w = c.borrow().frameW;
        r.borrow_mut().h = c.borrow().frameH;
        a.borrow().place(&r);
        a
    }
    pub fn dirCode(dir: &str) -> f64 {
        if dir == "fromR" {
            return 180.0;
        }
        if dir == "fromT" {
            return 90.0;
        }
        if dir == "fromB" {
            return 270.0;
        }
        0.0
    }
    pub fn sum(xs: &[f64]) -> f64 {
        let mut s: f64 = 0.0;
        let mut i: i64 = 0;
        while i < (xs.len() as i64) {
            s += xs[i as usize];
            i += 1;
        }
        s
    }
    pub fn max(xs: &[f64]) -> f64 {
        let mut m: f64 = 0.0;
        let mut i: i64 = 0;
        while i < (xs.len() as i64) {
            if xs[i as usize] > m {
                m = xs[i as usize];
            }
            i += 1;
        }
        m
    }
    pub fn warnOnce(&mut self, msg: &str) {
        let mut i: i64 = 0;
        while i < (self.warnings.len() as i64) {
            if self.warnings[i as usize].clone() == msg {
                return;
            }
            i += 1;
        }
        self.warnings.push(msg.to_string());
    }
    pub fn place(&self, p: &Rc<RefCell<SaInst>>) {
        let t: String = p.borrow().alg.borrow().r#type.clone();
        if !p.borrow().children.is_empty() && !p.borrow().placed {
            if t == "lin" || t.is_empty() {
                self.lin(p);
            }
            if (t == "composite" || t == "tx") || (t == "sp" || t == "conn") {
                self.composite(p);
                SaConn::connect(&self.cons.clone(), p);
            }
            if t == "snake" {
                SaSnake::place(&self.cons.clone(), p);
            }
            if t == "cycle" {
                SaCycle::place(&self.cons.clone(), p);
            }
            if t == "hierChild" || t == "hierRoot" {
                SaHier::place(&self.cons.clone(), p);
            }
            if t == "pyra" {
                SaPyra::place(&self.cons.clone(), p);
            }
        }
        let mut i: i64 = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            self.resize(&c);
            self.place(&c);
            i += 1;
        }
    }
    /// A child's subtree sized again for the box its parent gave it: its own
    /// constraints, and its descendants', re-run with its w and h fixed.
    pub fn resize(&self, c: &Rc<RefCell<SaInst>>) {
        let mut _tmp_339 = c.borrow().w.clone();
        c.borrow_mut().put("w", _tmp_339, &c.borrow().fromOf("w"));
        let mut _tmp_341 = c.borrow().h.clone();
        c.borrow_mut().put("h", _tmp_341, &c.borrow().fromOf("h"));
        let mut subtree: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        subtree.push(c.clone());
        c.borrow().collect(&mut subtree);
        let mut pass: i64 = 0;
        while pass < 2 {
            let mut i: i64 = 0;
            while i < (subtree.len() as i64) {
                let mut s: Rc<RefCell<SaInst>> = subtree[i as usize].clone();
                let mut k: i64 = 0;
                while k < (s.borrow().constrs.len() as i64) {
                    let mut con: Rc<RefCell<SaConstr>> = s.borrow().constrs[k as usize].clone();
                    // the box is the parent's to give: a node's own w and h
                    // constraints do not undo it
                    let own_size: bool = (Rc::ptr_eq(&s, &c) && con.borrow().forRel == "self") && (con.borrow().r#type == "w" || con.borrow().r#type == "h");
                    if !own_size {
                        self.cons.clone().borrow_mut().apply(&s, &con);
                    }
                    k += 1;
                }
                i += 1;
            }
            pass += 1;
        }
        let mut _tmp_345 = c.borrow().w.clone();
        c.borrow_mut().put("w", _tmp_345, &c.borrow().fromOf("w"));
        let mut _tmp_347 = c.borrow().h.clone();
        c.borrow_mut().put("h", _tmp_347, &c.borrow().fromOf("h"));
    }
    pub fn lin(&self, p: &Rc<RefCell<SaInst>>) {
        let dir: String = (p.borrow().alg).borrow().param("linDir", "fromL");
        let vertical: bool = dir == "fromT" || dir == "fromB";
        let n: i64 = p.borrow().children.len() as i64;
        let mut along: Vec<f64> = Vec::new();
        let mut across: Vec<f64> = Vec::new();
        let mut tied: Vec<bool> = Vec::new();
        let mut i: i64 = 0;
        while i < n {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            let cw: f64 = self.cons.borrow().get(&c, "w");
            let chh: f64 = self.cons.borrow().get(&c, "h");
            if vertical {
                along.push(chh);
                across.push(cw);
                tied.push(c.borrow().fromOf("w") == "h");
            } else {
                along.push(cw);
                across.push(chh);
                tied.push(c.borrow().fromOf("h") == "w");
            }
            i += 1;
        }
        let mut sp: f64 = self.cons.borrow().get(p, "sibSp");
        let room: f64 = if vertical { p.borrow().h } else { p.borrow().w };
        let room_across: f64 = if vertical { p.borrow().w } else { p.borrow().h };
        let mut total: f64 = SaAlgorithms::sum(&along) + sp * ((n - 1) as f64);
        // too long: shrink along the line, and what is tied to it
        if total > room && total > 0.0 {
            let s: f64 = room / total;
            i = 0;
            while i < n {
                along[i as usize] = along[i as usize] * s;
                if tied[i as usize] {
                    across[i as usize] = across[i as usize] * s;
                }
                i += 1;
            }
            sp *= s;
            total = room;
        }
        // still too big across: shrink everything
        let max_across: f64 = SaAlgorithms::max(&across);
        if max_across > room_across && max_across > 0.0 {
            let s2: f64 = room_across / max_across;
            i = 0;
            while i < n {
                along[i as usize] = along[i as usize] * s2;
                across[i as usize] = across[i as usize] * s2;
                i += 1;
            }
            sp *= s2;
            total *= s2;
        }
        let align_v: String = (p.borrow().alg).borrow().param("nodeVertAlign", "mid");
        let align_h: String = (p.borrow().alg).borrow().param("nodeHorzAlign", "ctr");
        let mut at: f64 = (room - total) / 2.0;
        i = 0;
        while i < n {
            let mut idx: i64 = i;
            if dir == "fromR" || dir == "fromB" {
                idx = (n - 1) - i;
            }
            let mut c2: Rc<RefCell<SaInst>> = p.borrow().children[idx as usize].clone();
            let a: f64 = along[idx as usize];
            let b: f64 = across[idx as usize];
            let mut off: f64 = (room_across - b) / 2.0;
            if vertical {
                if align_h == "l" {
                    off = 0.0;
                }
                if align_h == "r" {
                    off = room_across - b;
                }
                c2.borrow_mut().x = p.borrow().x + off;
                c2.borrow_mut().y = p.borrow().y + at;
                c2.borrow_mut().w = b;
                c2.borrow_mut().h = a;
            } else {
                if align_v == "t" {
                    off = 0.0;
                }
                if align_v == "b" {
                    off = room_across - b;
                }
                c2.borrow_mut().x = p.borrow().x + at;
                c2.borrow_mut().y = p.borrow().y + off;
                c2.borrow_mut().w = a;
                c2.borrow_mut().h = b;
            }
            // a connector points the way the line goes
            if c2.borrow().alg.borrow().r#type == "conn" {
                c2.borrow_mut().put("connDir", SaAlgorithms::dirCode(&dir), "");
            }
            at = (at + a) + sp;
            i += 1;
        }
    }
    pub fn composite(&self, p: &Rc<RefCell<SaInst>>) {
        let mut i: i64 = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            let mut w: f64 = self.cons.borrow().get(&c, "w");
            let mut h: f64 = self.cons.borrow().get(&c, "h");
            let mut x: f64 = 0.0;
            let mut y: f64 = 0.0;
            if c.borrow().has("l") {
                x = c.borrow().raw("l", 0.0);
            } else {
                if c.borrow().has("ctrX") {
                    x = c.borrow().raw("ctrX", 0.0) - w / 2.0;
                } else {
                    if c.borrow().has("r") {
                        x = c.borrow().raw("r", 0.0) - w;
                    }
                }
            }
            if c.borrow().has("t") {
                y = c.borrow().raw("t", 0.0);
            } else {
                if c.borrow().has("ctrY") {
                    y = c.borrow().raw("ctrY", 0.0) - h / 2.0;
                } else {
                    if c.borrow().has("b") {
                        y = c.borrow().raw("b", 0.0) - h;
                    }
                }
            }
            // both edges given: the width is what lies between them
            if c.borrow().has("l") && c.borrow().has("r") {
                w = c.borrow().raw("r", 0.0) - x;
            }
            if c.borrow().has("t") && c.borrow().has("b") {
                h = c.borrow().raw("b", 0.0) - y;
            }
            c.borrow_mut().x = p.borrow().x + x;
            c.borrow_mut().y = p.borrow().y + y;
            c.borrow_mut().w = w;
            c.borrow_mut().h = h;
            i += 1;
        }
    }
}
