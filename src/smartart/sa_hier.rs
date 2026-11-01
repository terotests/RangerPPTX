// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaHier — a tree: each shape over the shapes under it
//! ==============================================================================
//!
//! Two algorithms that only make sense together:
//!
//!   hierRoot   one branch: its first shape that is not a hierChild is the
//!              branch's own box; under it, its hierChild children — the
//!              assistants' (a hierChild whose branches are `asst` points)
//!              and the others'
//!   hierChild  a row of branches (hierRoot children) and the connectors to
//!              them: the k-th connector belongs to the k-th branch
//!
//! A layout writes it as a recursion (`forEach ref`):
//!
//!   forEach "branches" axis=ch ptType=node
//!     layoutNode hierRoot       box, then
//!       layoutNode hierChild    forEach axis=ch ptType=parTrans → conn
//!                               forEach ref="branches"
//!
//! The tree is laid out at once, from the outermost hierChild (or a hierRoot
//! with none around it): every branch is as wide as its box or the row under
//! it, whichever is wider, and its box is centred over that row (`hierAlign`
//! tCtr; the other alignments are drawn as tCtr). Rows are
//! `sibSp` of their hierChild apart (0.15 of a box when not given), levels
//! `sp` of the hierRoot (half a box's height when not given). Then the whole
//! tree is made smaller, everything with it, until it fits the box (never
//! larger), and centred in it.
//!
//! How a branch's children hang (the layout's `hierBranch` variable; a
//! point's own, from its presentation properties, is not read yet):
//!
//!   std (default)  in a row under it
//!   l / r          in a column, to the left / right of a line down from it
//!   hang           in two columns either side of that line
//!   init           as std
//!
//! Assistants (a row whose branches are all `asst` points) go on the line
//! down from the box, alternately right and left, above the row. A row with
//! no branches is drawn as nothing.
//!
//! Connectors are elbows: down from the box's bottom middle, across, down to
//! the top middle of the box under it; to a shape in a column or an
//! assistant, down and across to its side.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_conn::SaConn;
use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_pres_tree::SaInst;

#[derive(Clone)]
pub struct SaHier {
}

impl SaHier {
    pub fn new() -> Self {
        Self {
        }
    }
    /// the branch's own box
    pub fn boxOf(r: &Rc<RefCell<SaInst>>) -> Option<Rc<RefCell<SaInst>>> {
        let mut i: i64 = 0;
        while i < (r.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = r.borrow().children[i as usize].clone();
            if (c.borrow().alg.borrow().r#type != "hierChild" && SaConn::isCell(&c)) && c.borrow()
                .alg.borrow()
                .r#type != "hierRoot" {
                let mut hit: Option<Rc<RefCell<SaInst>>> = Some(c.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaInst>>> = None;
        none
    }
    pub fn branchesOf(hc: &Rc<RefCell<SaInst>>) -> Vec<Rc<RefCell<SaInst>>> {
        let mut out: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (hc.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = hc.borrow().children[i as usize].clone();
            if c.borrow().alg.borrow().r#type == "hierRoot" {
                out.push(c.clone());
            }
            i += 1;
        }
        out
    }
    pub fn connsOf(hc: &Rc<RefCell<SaInst>>) -> Vec<Rc<RefCell<SaInst>>> {
        let mut out: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (hc.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = hc.borrow().children[i as usize].clone();
            if SaConn::isConn(&c) {
                out.push(c.clone());
            }
            i += 1;
        }
        out
    }
    /// a hierChild whose branches are all assistants
    pub fn isAsstRow(hc: &Rc<RefCell<SaInst>>) -> bool {
        let mut bs: Vec<Rc<RefCell<SaInst>>> = SaHier::branchesOf(hc);
        if bs.is_empty() {
            return false;
        }
        let mut i: i64 = 0;
        while i < (bs.len() as i64) {
            let mut b: Rc<RefCell<SaInst>> = bs[i as usize].clone();
            if b.borrow().ctxType != "asst" {
                return false;
            }
            i += 1;
        }
        true
    }
    pub fn rowsOf(r: &Rc<RefCell<SaInst>>) -> Vec<Rc<RefCell<SaInst>>> {
        let mut out: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (r.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = r.borrow().children[i as usize].clone();
            if c.borrow().alg.borrow().r#type == "hierChild" {
                out.push(c.clone());
            }
            i += 1;
        }
        out
    }
    /// The whole tree under `top` (a hierChild, or a lone hierRoot), laid out
    /// and fitted to top's box.
    pub fn place(cons: &Rc<RefCell<SaConstraints>>, top: &Rc<RefCell<SaInst>>) {
        let mut all: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        all.push(top.clone());
        top.borrow().collect(&mut all);
        let keep_x: f64 = top.borrow().x;
        let keep_y: f64 = top.borrow().y;
        let keep_w: f64 = top.borrow().w;
        let keep_h: f64 = top.borrow().h;
        if top.borrow().alg.borrow().r#type == "hierChild" {
            SaHier::row(cons, top);
        } else {
            SaHier::branch(cons, top);
        }
        // everything was laid out with top's corner at 0, 0; fit it
        let bw: f64 = top.borrow().w;
        let bh: f64 = top.borrow().h;
        let mut s: f64 = 1.0;
        if bw > 0.0 && keep_w / bw < s {
            s = keep_w / bw;
        }
        if bh > 0.0 && keep_h / bh < s {
            s = keep_h / bh;
        }
        let ox: f64 = keep_x + (keep_w - bw * s) / 2.0;
        let oy: f64 = keep_y + (keep_h - bh * s) / 2.0;
        let mut i: i64 = 0;
        while i < (all.len() as i64) {
            let mut a: Rc<RefCell<SaInst>> = all[i as usize].clone();
            if !Rc::ptr_eq(&a, &top) {
                let hoist_7 = ox + a.borrow().x * s;
                a.borrow_mut().x = hoist_7;
                let hoist_8 = oy + a.borrow().y * s;
                a.borrow_mut().y = hoist_8;
                let hoist_9 = a.borrow().w * s;
                a.borrow_mut().w = hoist_9;
                let hoist_10 = a.borrow().h * s;
                a.borrow_mut().h = hoist_10;
                let mut k: i64 = 0;
                while k + 1 < (a.borrow().segs.len() as i64) {
                    a.borrow_mut().segs[k as usize] = ox + a.borrow().segs[k as usize] * s;
                    a.borrow_mut().segs[(k + 1) as usize] = oy + a.borrow().segs[(k + 1) as usize] * s;
                    k += 2;
                }
                if a.borrow()
                    .alg.borrow()
                    .r#type == "hierRoot" || a.borrow()
                    .alg.borrow()
                    .r#type == "hierChild" {
                    a.borrow_mut().placed = true;
                }
                if SaConn::isConn(&a) {
                    a.borrow_mut().placed = true;
                }
            }
            i += 1;
        }
        top.borrow_mut().x = keep_x;
        top.borrow_mut().y = keep_y;
        top.borrow_mut().w = keep_w;
        top.borrow_mut().h = keep_h;
        top.borrow_mut().placed = true;
    }
    /// how far apart a row's branches are
    pub fn hGap(cons: &Rc<RefCell<SaConstraints>>, hc: &Rc<RefCell<SaInst>>, box_w: f64) -> f64 {
        if hc.borrow().has("sibSp") {
            return cons.borrow().get(hc, "sibSp");
        }
        box_w * 0.15
    }
    /// how far a branch's box is above what hangs from it
    pub fn vGap(cons: &Rc<RefCell<SaConstraints>>, r: &Rc<RefCell<SaInst>>, box_h: f64) -> f64 {
        if r.borrow().has("sp") {
            return cons.borrow().get(r, "sp");
        }
        box_h * 0.5
    }
    pub fn shift(s: &Rc<RefCell<SaInst>>, dx: f64, dy: f64) {
        let mut all: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        all.push(s.clone());
        s.borrow().collect(&mut all);
        let mut i: i64 = 0;
        while i < (all.len() as i64) {
            let mut a: Rc<RefCell<SaInst>> = all[i as usize].clone();
            let hoist_11 = a.borrow().x + dx;
            a.borrow_mut().x = hoist_11;
            let hoist_12 = a.borrow().y + dy;
            a.borrow_mut().y = hoist_12;
            let mut k: i64 = 0;
            while k + 1 < (a.borrow().segs.len() as i64) {
                a.borrow_mut().segs[k as usize] = a.borrow().segs[k as usize] + dx;
                a.borrow_mut().segs[(k + 1) as usize] = a.borrow().segs[(k + 1) as usize] + dy;
                k += 2;
            }
            i += 1;
        }
    }
    /// A row of branches side by side, tops level, at 0, 0; the row's box is
    /// their bounds.
    pub fn row(cons: &Rc<RefCell<SaConstraints>>, hc: &Rc<RefCell<SaInst>>) {
        let mut bs: Vec<Rc<RefCell<SaInst>>> = SaHier::branchesOf(hc);
        let n: i64 = bs.len() as i64;
        hc.borrow_mut().x = 0.0;
        hc.borrow_mut().y = 0.0;
        hc.borrow_mut().w = 0.0;
        hc.borrow_mut().h = 0.0;
        let mut cs: Vec<Rc<RefCell<SaInst>>> = SaHier::connsOf(hc);
        let mut ci: i64 = 0;
        while ci < (cs.len() as i64) {
            SaConn::hide(&cs[ci as usize].clone(), hc);
            ci += 1;
        }
        if n == 0 {
            return;
        }
        let mut at: f64 = 0.0;
        let mut gap: f64 = 0.0;
        let mut tallest: f64 = 0.0;
        let mut i: i64 = 0;
        while i < n {
            let mut b: Rc<RefCell<SaInst>> = bs[i as usize].clone();
            SaHier::branch(cons, &b);
            if i == 0 {
                let mut bx: Option<Rc<RefCell<SaInst>>> = SaHier::boxOf(&b);
                let mut bw0: f64 = 0.0;
                if bx.is_some() {
                    let mut bxx: Rc<RefCell<SaInst>> = bx.clone().unwrap();
                    bw0 = bxx.borrow().w;
                }
                gap = SaHier::hGap(cons, hc, bw0);
            }
            SaHier::shift(&b, at, 0.0);
            at = (at + b.borrow().w) + gap;
            if b.borrow().h > tallest {
                tallest = b.borrow().h;
            }
            i += 1;
        }
        hc.borrow_mut().w = at - gap;
        hc.borrow_mut().h = tallest;
    }
    /// One branch at 0, 0: its box over what hangs from it. Its w and h are
    /// the branch's bounds.
    pub fn branch(cons: &Rc<RefCell<SaConstraints>>, r: &Rc<RefCell<SaInst>>) {
        r.borrow_mut().x = 0.0;
        r.borrow_mut().y = 0.0;
        let mut bx: Option<Rc<RefCell<SaInst>>> = SaHier::boxOf(r);
        let mut bw: f64 = 0.0;
        let mut bh: f64 = 0.0;
        if bx.is_some() {
            let mut b0: Rc<RefCell<SaInst>> = bx.clone().unwrap();
            bw = cons.borrow().get(&b0, "w");
            bh = cons.borrow().get(&b0, "h");
            b0.borrow_mut().w = bw;
            b0.borrow_mut().h = bh;
            b0.borrow_mut().x = 0.0;
            b0.borrow_mut().y = 0.0;
        }
        let mut rows: Vec<Rc<RefCell<SaInst>>> = SaHier::rowsOf(r);
        let mut kids: Option<Rc<RefCell<SaInst>>> = None;
        let mut assts: Option<Rc<RefCell<SaInst>>> = None;
        let mut i: i64 = 0;
        while i < (rows.len() as i64) {
            let mut hc: Rc<RefCell<SaInst>> = rows[i as usize].clone();
            let many: i64 = SaHier::branchesOf(&hc).len() as i64;
            if SaHier::isAsstRow(&hc) {
                if assts.is_none() {
                    assts = Some(hc.clone());
                }
            } else {
                if kids.is_none() && many > 0 {
                    kids = Some(hc.clone());
                }
            }
            i += 1;
        }
        // rows other than the first of each kind are not drawn
        i = 0;
        while i < (rows.len() as i64) {
            let mut hc2: Rc<RefCell<SaInst>> = rows[i as usize].clone();
            let mut used: bool = false;
            if kids.is_some() && Rc::ptr_eq(&hc2, &kids.clone().unwrap()) {
                used = true;
            }
            if assts.is_some() && Rc::ptr_eq(&hc2, &assts.clone().unwrap()) {
                used = true;
            }
            if !used {
                SaHier::hideAll(&hc2);
            }
            i += 1;
        }
        let vgap: f64 = SaHier::vGap(cons, r, bh);
        let mode: String = r.borrow().varValue("hierBranch", "std");
        // trunk: the line down from the middle of the box
        let trunk_x: f64 = bw / 2.0;
        let mut below: f64 = bh + vgap;
        let mut min_x: f64 = 0.0;
        let mut max_x: f64 = bw;
        let mut max_y: f64 = bh;
        // assistants, alternately right and left of the trunk
        if assts.is_some() {
            let mut ar: Rc<RefCell<SaInst>> = assts.clone().unwrap();
            let mut abs: Vec<Rc<RefCell<SaInst>>> = SaHier::branchesOf(&ar);
            let mut acs: Vec<Rc<RefCell<SaInst>>> = SaHier::connsOf(&ar);
            let gap_a: f64 = SaHier::hGap(cons, &ar, bw);
            let mut row_top: f64 = below;
            let mut row_h: f64 = 0.0;
            let mut k: i64 = 0;
            while k < (abs.len() as i64) {
                let mut a: Rc<RefCell<SaInst>> = abs[k as usize].clone();
                SaHier::branch(cons, &a);
                let right: bool = k % 2 == 0;
                let mut ax: f64 = trunk_x + gap_a / 2.0;
                if !right {
                    ax = (trunk_x - gap_a / 2.0) - a.borrow().w;
                }
                SaHier::shift(&a, ax, row_top);
                if a.borrow().h > row_h {
                    row_h = a.borrow().h;
                }
                if a.borrow().x < min_x {
                    min_x = a.borrow().x;
                }
                if a.borrow().x + a.borrow().w > max_x {
                    max_x = a.borrow().x + a.borrow().w;
                }
                if a.borrow().y + a.borrow().h > max_y {
                    max_y = a.borrow().y + a.borrow().h;
                }
                if k < (acs.len() as i64) {
                    let mut ab: Option<Rc<RefCell<SaInst>>> = SaHier::boxOf(&a);
                    if ab.is_some() {
                        let mut abx: Rc<RefCell<SaInst>> = ab.clone().unwrap();
                        let mut side_x: f64 = abx.borrow().x;
                        if !right {
                            side_x = abx.borrow().x + abx.borrow().w;
                        }
                        SaConn::hang(
                            &acs[k as usize].clone(),
                            trunk_x,
                            bh,
                            side_x,
                            abx.borrow().y + abx.borrow().h / 2.0
                        );
                    }
                }
                if !right {
                    row_top = (row_top + row_h) + vgap / 2.0;
                    row_h = 0.0;
                }
                k += 1;
            }
            if row_h > 0.0 {
                row_top = (row_top + row_h) + vgap / 2.0;
            }
            below = row_top + vgap / 2.0;
            SaHier::boundsOf(&ar);
        }
        if kids.is_some() {
            let mut kr: Rc<RefCell<SaInst>> = kids.clone().unwrap();
            let mut bs: Vec<Rc<RefCell<SaInst>>> = SaHier::branchesOf(&kr);
            let mut cs: Vec<Rc<RefCell<SaInst>>> = SaHier::connsOf(&kr);
            if !bs.is_empty() {
                if mode == "l" || (mode == "r" || mode == "hang") {
                    SaHier::column(cons, &kr, &bs, &cs, &mode, trunk_x, bh, below, bw);
                } else {
                    SaHier::row(cons, &kr);
                    let row_x: f64 = trunk_x - kr.borrow().w / 2.0;
                    SaHier::shift(&kr, row_x, below);
                    let mut mid_y: f64 = bh + (below - bh) / 2.0;
                    if assts.is_some() {
                        mid_y = below - vgap / 2.0;
                    }
                    let mut k2: i64 = 0;
                    while k2 < (bs.len() as i64) {
                        let mut b: Rc<RefCell<SaInst>> = bs[k2 as usize].clone();
                        if k2 < (cs.len() as i64) {
                            let mut bb: Option<Rc<RefCell<SaInst>>> = SaHier::boxOf(&b);
                            if bb.is_some() {
                                let mut bbx: Rc<RefCell<SaInst>> = bb.clone().unwrap();
                                let mut _tmp_335 = bbx.borrow().y.clone();
                                SaConn::elbow(
                                    &cs[k2 as usize].clone(),
                                    trunk_x,
                                    bh,
                                    bbx.borrow().x + bbx.borrow().w / 2.0,
                                    _tmp_335,
                                    mid_y
                                );
                            }
                        }
                        k2 += 1;
                    }
                }
                SaHier::boundsOf(&kr);
                if kr.borrow().x < min_x {
                    min_x = kr.borrow().x;
                }
                if kr.borrow().x + kr.borrow().w > max_x {
                    max_x = kr.borrow().x + kr.borrow().w;
                }
                if kr.borrow().y + kr.borrow().h > max_y {
                    max_y = kr.borrow().y + kr.borrow().h;
                }
            } else {
                SaHier::hideAll(&kr);
            }
        }
        // the branch's corner at 0, 0
        r.borrow_mut().w = max_x - min_x;
        r.borrow_mut().h = max_y;
        if min_x < 0.0 {
            SaHier::shift(r, 0.0 - min_x, 0.0);
        }
        r.borrow_mut().x = 0.0;
        r.borrow_mut().y = 0.0;
    }
    /// branches in a column (l, r) or two (hang) beside the trunk
    pub fn column(cons: &Rc<RefCell<SaConstraints>>, kr: &Rc<RefCell<SaInst>>, bs: &Vec<Rc<RefCell<SaInst>>>, cs: &Vec<Rc<RefCell<SaInst>>>, mode: &str, trunk_x: f64, top: f64, below: f64, bw: f64) {
        let gap: f64 = SaHier::hGap(cons, kr, bw);
        let row_gap: f64 = (below - top) / 2.0;
        let mut yl: f64 = below;
        let mut yr: f64 = below;
        let mut k: i64 = 0;
        while k < (bs.len() as i64) {
            let mut b: Rc<RefCell<SaInst>> = bs[k as usize].clone();
            SaHier::branch(cons, &b);
            let mut right: bool = mode == "r";
            if mode == "hang" {
                right = k % 2 == 1;
            }
            let y: f64 = if right { yr } else { yl };
            let mut x: f64 = trunk_x + gap / 2.0;
            if !right {
                x = (trunk_x - gap / 2.0) - b.borrow().w;
            }
            SaHier::shift(&b, x, y);
            if right {
                yr = (yr + b.borrow().h) + row_gap;
            } else {
                yl = (yl + b.borrow().h) + row_gap;
            }
            if k < (cs.len() as i64) {
                let mut bb: Option<Rc<RefCell<SaInst>>> = SaHier::boxOf(&b);
                if bb.is_some() {
                    let mut bbx: Rc<RefCell<SaInst>> = bb.clone().unwrap();
                    let mut side_x: f64 = bbx.borrow().x;
                    if !right {
                        side_x = bbx.borrow().x + bbx.borrow().w;
                    }
                    SaConn::hang(
                        &cs[k as usize].clone(),
                        trunk_x,
                        top,
                        side_x,
                        bbx.borrow().y + bbx.borrow().h / 2.0
                    );
                }
            }
            k += 1;
        }
    }
    /// a row's box: the bounds of its branches
    pub fn boundsOf(hc: &Rc<RefCell<SaInst>>) {
        let mut bs: Vec<Rc<RefCell<SaInst>>> = SaHier::branchesOf(hc);
        if bs.is_empty() {
            return;
        }
        let mut b0: Rc<RefCell<SaInst>> = bs[0].clone();
        let mut min_x: f64 = b0.borrow().x;
        let mut min_y: f64 = b0.borrow().y;
        let mut max_x: f64 = b0.borrow().x + b0.borrow().w;
        let mut max_y: f64 = b0.borrow().y + b0.borrow().h;
        let mut i: i64 = 1;
        while i < (bs.len() as i64) {
            let mut b: Rc<RefCell<SaInst>> = bs[i as usize].clone();
            if b.borrow().x < min_x {
                min_x = b.borrow().x;
            }
            if b.borrow().y < min_y {
                min_y = b.borrow().y;
            }
            if b.borrow().x + b.borrow().w > max_x {
                max_x = b.borrow().x + b.borrow().w;
            }
            if b.borrow().y + b.borrow().h > max_y {
                max_y = b.borrow().y + b.borrow().h;
            }
            i += 1;
        }
        hc.borrow_mut().x = min_x;
        hc.borrow_mut().y = min_y;
        hc.borrow_mut().w = max_x - min_x;
        hc.borrow_mut().h = max_y - min_y;
    }
    /// a subtree drawn as nothing
    pub fn hideAll(s: &Rc<RefCell<SaInst>>) {
        let mut all: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        all.push(s.clone());
        s.borrow().collect(&mut all);
        let mut i: i64 = 0;
        while i < (all.len() as i64) {
            let mut a: Rc<RefCell<SaInst>> = all[i as usize].clone();
            let mut none: Vec<f64> = Vec::new();
            a.borrow_mut().segs = none.clone();
            a.borrow_mut().w = 0.0;
            a.borrow_mut().h = 0.0;
            i += 1;
        }
    }
}
