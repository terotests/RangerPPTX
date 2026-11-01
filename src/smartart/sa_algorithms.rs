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
//!              relative to the box, at the w / h they were given; with `ar`
//!              (width over height) in the largest box of that shape, centred
//!   tx, sp, conn
//!              nothing to place. tx is text in the node's own shape (SaText);
//!              sp is room; a conn among a lin's children is placed by the lin
//!              like any child, and drawn as an arrow pointing along it.
//!              With `wrap` (SaOptions.wrap, a host that is not PowerPoint), a
//!              row of shapes that keep their proportions is broken into rows
//!              when that makes them clearly larger: eight steps in a tall box
//!              are two rows of four, not one strip a tenth of its height.
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
    /// break a lin row into rows when the box is tall enough (linRows)
    pub wrap: bool,
    /// a host's taller row (SaEngine.taller): the cells of a horizontal lin
    /// whose height follows their width are made this much taller, and
    /// `stretched` says some were
    pub stretch: f64,
    pub stretched: bool,
}

impl SaAlgorithms {
    pub fn new() -> Self {
        Self {
            cons: Rc::new(RefCell::new(SaConstraints::new())),
            warnings: Vec::new(),
            wrap: false,
            stretch: 1.0,
            stretched: false,
        }
    }
    pub fn run(c: Rc<RefCell<SaConstraints>>) -> Rc<RefCell<SaAlgorithms>> {
        SaAlgorithms::runWith(c.clone(), false)
    }
    pub fn runWith(c: Rc<RefCell<SaConstraints>>, wrap: bool) -> Rc<RefCell<SaAlgorithms>> {
        SaAlgorithms::runStretched(c.clone(), wrap, 1.0)
    }
    pub fn runStretched(c: Rc<RefCell<SaConstraints>>, wrap: bool, stretch: f64) -> Rc<RefCell<SaAlgorithms>> {
        let mut a: Rc<RefCell<SaAlgorithms>> = Rc::new(RefCell::new(SaAlgorithms::new()));
        a.borrow_mut().cons = c.clone();
        a.borrow_mut().wrap = wrap;
        a.borrow_mut().stretch = stretch;
        let mut r: Rc<RefCell<SaInst>> = c.borrow().ptree.borrow().root.clone().clone();
        r.borrow_mut().x = 0.0;
        r.borrow_mut().y = 0.0;
        r.borrow_mut().w = c.borrow().frameW;
        r.borrow_mut().h = c.borrow().frameH;
        a.borrow_mut().place(&r);
        a
    }
    /// The scale a lin row's shapes get in a box of room × roomAcross: as
    /// large as fits along and across, never larger than they are.
    pub fn rowScale(__len: f64, high: f64, room: f64, room_across: f64) -> f64 {
        if __len <= 0.0 || high <= 0.0 {
            return 0.0;
        }
        let mut s: f64 = room / __len;
        if room_across / high < s {
            s = room_across / high;
        }
        if s > 1.0 {
            s = 1.0;
        }
        s
    }
    /// Where row `r` of `rows` ends: the index after the last child it holds.
    /// Each row takes `per` shapes and the connectors that follow them, so an
    /// arrow at a row's end says the steps go on in the next.
    pub fn rowEnds(p: &Rc<RefCell<SaInst>>, per: i64) -> Vec<i64> {
        let mut ends: Vec<i64> = Vec::new();
        let n: i64 = p.borrow().children.len() as i64;
        let mut cells: i64 = 0;
        let mut i: i64 = 0;
        while i < n {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if SaConn::isCell(&c) {
                if cells == per {
                    ends.push(i);
                    cells = 0;
                }
                cells += 1;
            }
            i += 1;
        }
        ends.push(n);
        ends
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
    pub fn place(&mut self, p: &Rc<RefCell<SaInst>>) {
        let t: String = p.borrow().alg.borrow().r#type.clone();
        if !p.borrow().children.is_empty() && !p.borrow().placed {
            if t == "lin" || t.is_empty() {
                self.lin(p);
            }
            if (t == "composite" || t == "tx") || (t == "sp" || t == "conn") {
                if t == "composite" {
                    self.keepAspect(p);
                }
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
        let mut _tmp_348 = c.borrow().w.clone();
        c.borrow_mut().put("w", _tmp_348, &c.borrow().fromOf("w"));
        let mut _tmp_350 = c.borrow().h.clone();
        c.borrow_mut().put("h", _tmp_350, &c.borrow().fromOf("h"));
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
        let mut _tmp_354 = c.borrow().w.clone();
        c.borrow_mut().put("w", _tmp_354, &c.borrow().fromOf("w"));
        let mut _tmp_356 = c.borrow().h.clone();
        c.borrow_mut().put("h", _tmp_356, &c.borrow().fromOf("h"));
    }
    pub fn lin(&mut self, p: &Rc<RefCell<SaInst>>) {
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
        if self.stretch > 1.0 && !vertical {
            i = 0;
            while i < n {
                let mut sc: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
                if tied[i as usize] && SaConn::isCell(&sc) {
                    across[i as usize] = across[i as usize] * self.stretch;
                    sc.borrow_mut().stretchK = self.stretch;
                    self.stretched = true;
                }
                i += 1;
            }
        }
        let mut sp: f64 = self.cons.borrow().get(p, "sibSp");
        let room: f64 = if vertical { p.borrow().h } else { p.borrow().w };
        let room_across: f64 = if vertical { p.borrow().w } else { p.borrow().h };
        let mut total: f64 = SaAlgorithms::sum(&along) + sp * ((n - 1) as f64);
        if self.wrap && dir == "fromL" {
            if self.linRows(p, &along, &across, &tied, sp) {
                return;
            }
        }
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
    /// A horizontal lin's children in rows of three or more, when rows make
    /// its shapes at least a fifth larger than one row does; false leaves it
    /// to the one row.
    /// Only shapes whose height follows their width wrap: one as tall as the
    /// box (a list's column) has no row to share.
    pub fn linRows(&self, p: &Rc<RefCell<SaInst>>, along: &[f64], across: &[f64], tied: &[bool], sp: f64) -> bool {
        let n: i64 = p.borrow().children.len() as i64;
        let mut cell_count: i64 = 0;
        let mut i: i64 = 0;
        while i < n {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if SaConn::isCell(&c) {
                cell_count += 1;
                if !(tied[i as usize]) {
                    return false;
                }
            }
            i += 1;
        }
        if cell_count < 2 {
            return false;
        }
        let room: f64 = p.borrow().w;
        let room_across: f64 = p.borrow().h;
        let high: f64 = SaAlgorithms::max(across);
        // rows a quarter of a shape apart
        let gap: f64 = high * 0.25;
        let single: f64 = SaAlgorithms::rowScale(SaAlgorithms::sum(along) + sp * ((n - 1) as f64), high, room, room_across);
        let mut best: f64 = single * 1.2;
        let mut best_per: i64 = 0;
        let mut rows: i64 = 2;
        while rows <= cell_count {
            let per: i64 = ((((cell_count + rows) - 1) as f64) / (rows as f64)).floor() as i64;
            // the same rows as one row fewer would give; and a row of fewer
            // than three steps no longer reads as a process
            if per * (rows - 1) < cell_count && per >= 3 {
                let mut ends: Vec<i64> = SaAlgorithms::rowEnds(p, per);
                let mut longest: f64 = 0.0;
                let mut from: i64 = 0;
                let mut r: i64 = 0;
                while r < (ends.len() as i64) {
                    let to: i64 = ends[r as usize];
                    let mut __len: f64 = 0.0;
                    let mut k: i64 = from;
                    while k < to {
                        __len += along[k as usize];
                        k += 1;
                    }
                    __len += sp * (((to - from) - 1) as f64);
                    if __len > longest {
                        longest = __len;
                    }
                    from = to;
                    r += 1;
                }
                let tall: f64 = ((ends.len() as i64) as f64) * high + gap * (((ends.len() as i64) - 1) as f64);
                let s: f64 = SaAlgorithms::rowScale(longest, tall, room, room_across);
                if s > best {
                    best = s;
                    best_per = per;
                }
            }
            rows += 1;
        }
        if best_per == 0 {
            return false;
        }
        // placed: every row starts where the longest does, the rows centred
        // in the box together
        let mut ends2: Vec<i64> = SaAlgorithms::rowEnds(p, best_per);
        let mut longest2: f64 = 0.0;
        let mut from2: i64 = 0;
        let mut r2: i64 = 0;
        while r2 < (ends2.len() as i64) {
            let to2: i64 = ends2[r2 as usize];
            let mut len2: f64 = 0.0;
            let mut k2: i64 = from2;
            while k2 < to2 {
                len2 += along[k2 as usize] * best;
                k2 += 1;
            }
            len2 += (sp * best) * (((to2 - from2) - 1) as f64);
            if len2 > longest2 {
                longest2 = len2;
            }
            from2 = to2;
            r2 += 1;
        }
        let row_h: f64 = high * best;
        let row_gap: f64 = gap * best;
        let count: i64 = ends2.len() as i64;
        let mut y: f64 = p.borrow().y + (room_across - ((count as f64) * row_h + row_gap * ((count - 1) as f64))) / 2.0;
        let x0: f64 = p.borrow().x + (room - longest2) / 2.0;
        let mut start: i64 = 0;
        let mut row: i64 = 0;
        while row < count {
            let stop: i64 = ends2[row as usize];
            let mut at: f64 = x0;
            let mut j: i64 = start;
            while j < stop {
                let mut c2: Rc<RefCell<SaInst>> = p.borrow().children[j as usize].clone();
                let a: f64 = along[j as usize] * best;
                let b: f64 = across[j as usize] * best;
                c2.borrow_mut().x = at;
                c2.borrow_mut().y = y + (row_h - b) / 2.0;
                c2.borrow_mut().w = a;
                c2.borrow_mut().h = b;
                if c2.borrow().alg.borrow().r#type == "conn" {
                    c2.borrow_mut().put("connDir", 0.0, "");
                }
                at = (at + a) + sp * best;
                j += 1;
            }
            y = (y + row_h) + row_gap;
            start = stop;
            row += 1;
        }
        true
    }
    /// A composite with `ar` (its width over its height) is drawn in the
    /// largest box of that shape inside the one it was given, centred: a
    /// drawing whose parts are placed by fractions of a curve's own frame
    /// (arrow2's points along its arrow) stays on the curve in any frame.
    pub fn keepAspect(&self, p: &Rc<RefCell<SaInst>>) {
        let ar: Option<f64> = (p.borrow().alg).borrow().param("ar", "")
            .trim()
            .to_string()
            .parse::<f64>()
            .ok();
        if ar.is_none() {
            return;
        }
        let want: f64 = ar.unwrap();
        if (want <= 0.0 || p.borrow().w <= 0.0) || p.borrow().h <= 0.0 {
            return;
        }
        let mut w: f64 = p.borrow().w;
        let mut h: f64 = p.borrow().h;
        if w / h > want {
            w = h * want;
        } else {
            h = w / want;
        }
        let hoist_1 = p.borrow().x + (p.borrow().w - w) / 2.0;
        p.borrow_mut().x = hoist_1;
        let hoist_2 = p.borrow().y + (p.borrow().h - h) / 2.0;
        p.borrow_mut().y = hoist_2;
        p.borrow_mut().w = w;
        p.borrow_mut().h = h;
        self.resize(p);
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
