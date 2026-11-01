// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaPyra — a triangle cut into levels
//! ==============================================================================
//!
//! The children that take a place (or only those named `pyraLvlNode`, when
//! it is given) are the levels of one triangle, the first at the apex
//! (`linDir` fromT, the default) or at the base (fromB). Every level is as
//! tall as the others; each is a trapezoid whose sloping sides are the
//! triangle's, so the apex level comes out a triangle.
//!
//! The triangle is as tall as the box and its base 1.15 times its height,
//! unless the box is too narrow for that, when it is as wide as the box and
//! its height 1/1.15 of that; centred in the box.
//!
//! A level's text is kept off the slopes: its left and right margins are set
//! to half the slope's run across the level, so the text stays inside the
//! shape at its middle; the apex's text goes in its lower part (top margin
//! 0.35 of the level, sides 0.65 of the run), where the triangle is wide
//! enough for it. The algorithm sets all four margins: a level's own margin
//! constraints would undo them.
//!
//! Not here yet: the accent text beside the levels (`pyraAcctPos`,
//! `pyraAcctRatio`, `pyraAcctBkgdNode`, `pyraAcctTxNode`), which pyramid2
//! and pyramid3 use.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_conn::SaConn;
use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_pres_tree::SaInst;

#[derive(Clone)]
pub struct SaPyra {
}

impl SaPyra {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn aspect() -> f64 {
        1.15
    }
    pub fn place(cons: &Rc<RefCell<SaConstraints>>, p: &Rc<RefCell<SaInst>>) {
        let lvl_name: String = (p.borrow().alg).borrow().param("pyraLvlNode", "");
        let mut levels: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            let mut is_level: bool = SaConn::isCell(&c);
            if !lvl_name.is_empty() {
                is_level = c.borrow().name == lvl_name;
            }
            if is_level {
                levels.push(c.clone());
            } else {
                SaConn::hide(&c, p);
            }
            i += 1;
        }
        let n: i64 = levels.len() as i64;
        if n == 0 {
            return;
        }
        let mut H: f64 = p.borrow().h;
        let mut W: f64 = H * SaPyra::aspect();
        if W > p.borrow().w {
            W = p.borrow().w;
            H = W / SaPyra::aspect();
        }
        let top: f64 = p.borrow().y + (p.borrow().h - H) / 2.0;
        let cx: f64 = p.borrow().x + p.borrow().w / 2.0;
        let lh: f64 = H / (n as f64);
        // how far in a slope comes over one level
        let run: f64 = W / (2.0 * (n as f64));
        let from_b: bool = (p.borrow().alg).borrow().param("linDir", "fromT") == "fromB";
        i = 0;
        while i < n {
            let mut lvl: i64 = i;
            if from_b {
                lvl = (n - 1) - i;
            }
            let mut s: Rc<RefCell<SaInst>> = levels[i as usize].clone();
            let wb: f64 = (W * ((lvl + 1) as f64)) / (n as f64);
            s.borrow_mut().x = cx - wb / 2.0;
            s.borrow_mut().y = top + (lvl as f64) * lh;
            s.borrow_mut().w = wb;
            s.borrow_mut().h = lh;
            // the trapezoid's top corners `run` in from the bottom ones
            let mut ss: f64 = wb;
            if lh < ss {
                ss = lh;
            }
            let mut none: Vec<String> = Vec::new();
            let mut no_vals: Vec<f64> = Vec::new();
            s.borrow_mut().adjNames = none.clone();
            s.borrow_mut().adjVals = no_vals.clone();
            if ss > 0.0 {
                s.borrow_mut().adjNames.push("adj".to_string().to_string());
                s.borrow_mut().adjVals.push((run * 100000.0) / ss);
            }
            // the text off the slopes; in the apex, in its lower part, where
            // there is room
            let mut side: f64 = run / 2.0;
            let mut top_m: f64 = lh * 0.08;
            if lvl == 0 {
                side = run * 0.65;
                top_m = lh * 0.35;
            }
            s.borrow_mut().put("lMarg", side, "");
            s.borrow_mut().put("rMarg", side, "");
            s.borrow_mut().put("tMarg", top_m, "");
            s.borrow_mut().put("bMarg", lh * 0.08, "");
            i += 1;
        }
    }
}
