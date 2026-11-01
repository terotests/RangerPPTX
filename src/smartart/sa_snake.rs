// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaSnake — shapes in rows that wrap, as text does
//! ==============================================================================
//!
//! The children that take a place (not connectors, not spaces) are laid out
//! in a grid: along a row (`flowDir` row) or down a column (col), `sibSp`
//! apart, the rows `secSibSp` apart (`sibSp` when not given).
//!
//!   grDir      tL (default) tR bL bR: the corner the first shape is in
//!   contDir    sameDir (default): every row runs the same way; revDir: every
//!              other row runs back, as a snake does
//!   bkpt       endCnv (default): the shapes as large as they can be, and
//!              as many to a row as fit at that size, to the end of the box;
//!              bal: the same number of rows, the shapes spread evenly over
//!              them; fixed: `bkPtFixedVal` to a row
//!   off        ctr (default): a short last row is centred; off: it starts
//!              where the others do
//!
//! The shapes keep the proportions their constraints give them and are made
//! smaller, with the gaps, until the grid fits the box; never larger. The
//! grid is centred in the box.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_conn::SaConn;
use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_pres_tree::SaInst;

#[derive(Clone)]
pub struct SaSnake {
}

impl SaSnake {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn place(cons: &Rc<RefCell<SaConstraints>>, p: &Rc<RefCell<SaInst>>) {
        let mut cells: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if SaConn::isCell(&c) {
                cells.push(c.clone());
            } else {
                SaConn::hide(&c, p);
            }
            i += 1;
        }
        let n: i64 = cells.len() as i64;
        if n == 0 {
            return;
        }
        let mut w0: f64 = 0.0;
        let mut h0: f64 = 0.0;
        i = 0;
        while i < n {
            let mut c2: Rc<RefCell<SaInst>> = cells[i as usize].clone();
            let cw: f64 = cons.borrow().get(&c2, "w");
            let chh: f64 = cons.borrow().get(&c2, "h");
            if cw > w0 {
                w0 = cw;
            }
            if chh > h0 {
                h0 = chh;
            }
            i += 1;
        }
        if w0 <= 0.0 || h0 <= 0.0 {
            return;
        }
        let mut sp: f64 = cons.borrow().get(p, "sibSp");
        let mut ssp: f64 = sp;
        if p.borrow().has("secSibSp") {
            ssp = cons.borrow().get(p, "secSibSp");
        }
        let by_col: bool = (p.borrow().alg).borrow().param("flowDir", "row") == "col";
        let bkpt: String = (p.borrow().alg).borrow().param("bkpt", "endCnv");
        // along a row: the shape's length and the gap; across: the row's
        let mut la: f64 = if by_col { h0 } else { w0 };
        let mut lc: f64 = if by_col { w0 } else { h0 };
        let room_a: f64 = if by_col { p.borrow().h } else { p.borrow().w };
        let room_c: f64 = if by_col { p.borrow().w } else { p.borrow().h };
        let mut k: i64 = 1;
        if bkpt == "fixed" {
            let fv: Option<i64> = (p.borrow().alg).borrow().param("bkPtFixedVal", "1")
                .parse::<i64>()
                .ok();
            k = if fv.is_some() { fv.unwrap() } else { 1 };
            if k < 1 {
                k = 1;
            }
            if k > n {
                k = n;
            }
        } else {
            // the largest the shapes can be (never larger than they are),
            // then as many to a row as still allows that
            let mut best: f64 = -1.0;
            let mut try_k: i64 = 1;
            while try_k <= n {
                let mut s: f64 = SaSnake::scaleFor(n, try_k, la, lc, sp, ssp, room_a, room_c);
                if s > 1.0 {
                    s = 1.0;
                }
                if s >= best - 0.000001 {
                    if s > best + 0.000001 {
                        best = s;
                    }
                    k = try_k;
                }
                try_k += 1;
            }
            if bkpt == "bal" {
                let lines0: i64 = SaSnake::linesOf(n, k);
                k = SaSnake::linesOf(n, lines0);
            }
        }
        let lines: i64 = SaSnake::linesOf(n, k);
        let mut sc: f64 = SaSnake::scaleFor(n, k, la, lc, sp, ssp, room_a, room_c);
        if sc > 1.0 {
            sc = 1.0;
        }
        la *= sc;
        lc *= sc;
        sp *= sc;
        ssp *= sc;
        let total_a: f64 = (k as f64) * la + ((k - 1) as f64) * sp;
        let total_c: f64 = (lines as f64) * lc + ((lines - 1) as f64) * ssp;
        let origin_a: f64 = (room_a - total_a) / 2.0;
        let origin_c: f64 = (room_c - total_c) / 2.0;
        let gr: String = (p.borrow().alg).borrow().param("grDir", "tL");
        let flip_x: bool = gr == "tR" || gr == "bR";
        let flip_y: bool = gr == "bL" || gr == "bR";
        let rev: bool = (p.borrow().alg).borrow().param("contDir", "sameDir") == "revDir";
        let centre: bool = (p.borrow().alg).borrow().param("off", "ctr") == "ctr";
        i = 0;
        while i < n {
            let line: i64 = ((i as f64) / (k as f64)).floor() as i64;
            let mut idx: i64 = i % k;
            let mut in_line: i64 = k;
            if line == lines - 1 {
                in_line = n - line * k;
            }
            if rev && line % 2 == 1 {
                idx = (in_line - 1) - idx;
            }
            let mut a: f64 = origin_a + (idx as f64) * (la + sp);
            if centre && in_line < k {
                a += (((k - in_line) as f64) * (la + sp)) / 2.0;
            }
            let cpos: f64 = origin_c + (line as f64) * (lc + ssp);
            let mut cell: Rc<RefCell<SaInst>> = cells[i as usize].clone();
            let mut lx: f64 = if by_col { cpos } else { a };
            let mut ly: f64 = if by_col { a } else { cpos };
            let cw2: f64 = if by_col { lc } else { la };
            let ch2: f64 = if by_col { la } else { lc };
            if flip_x {
                lx = (p.borrow().w - lx) - cw2;
            }
            if flip_y {
                ly = (p.borrow().h - ly) - ch2;
            }
            cell.borrow_mut().x = p.borrow().x + lx;
            cell.borrow_mut().y = p.borrow().y + ly;
            cell.borrow_mut().w = cw2;
            cell.borrow_mut().h = ch2;
            i += 1;
        }
    }
    /// rows (lines) for n shapes, k to a row
    pub fn linesOf(n: i64, k: i64) -> i64 {
        if k <= 0 {
            return n;
        }
        (((n + (k - 1)) as f64) / (k as f64)).floor() as i64
    }
    /// how much a grid of n, k to a row, must be scaled to fit
    pub fn scaleFor(n: i64, k: i64, la: f64, lc: f64, sp: f64, ssp: f64, room_a: f64, room_c: f64) -> f64 {
        let lines: i64 = SaSnake::linesOf(n, k);
        let need_a: f64 = (k as f64) * la + ((k - 1) as f64) * sp;
        let need_c: f64 = (lines as f64) * lc + ((lines - 1) as f64) * ssp;
        let mut s: f64 = 1000000000.0;
        if need_a > 0.0 {
            s = room_a / need_a;
        }
        if need_c > 0.0 {
            let s2: f64 = room_c / need_c;
            if s2 < s {
                s = s2;
            }
        }
        s
    }
}
