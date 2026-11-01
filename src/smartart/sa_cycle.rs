// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaCycle — shapes on a circle
//! ==============================================================================
//!
//! The children that take a place go round a circle, the first at `stAng`
//! degrees (0 is twelve o'clock, clockwise), the rest `spanAng` (360 by
//! default) shared out between them: a full circle in equal steps, a part of
//! one from its first angle to its last.
//!
//!   ctrShpMap  none (default); fNode: the first shape goes in the middle and
//!              the others round it, as a radial diagram has them
//!   rotPath    none (default); alongPath: each shape turned to its angle
//!
//! A negative `spanAng` goes round anticlockwise.
//!
//! The circle is as small as keeps neighbours `sibSp` apart (and clear of a
//! shape in the middle); then the whole of it is made smaller, the shapes with
//! it, until it fits the box, never larger, and centred in it.
//!
//! Connectors: a sibling transition is an arrow on the circle between the
//! shape before it and the one after it (the last back to the first on a full
//! circle), pointing the way round, no longer than 80 % of the gap; a parent
//! transition is a line from the shape in the middle to its own shape (the
//! one after it in the program) — `dim="1D"` makes either a line.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_conn::SaConn;
use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_pres_tree::SaInst;

#[derive(Clone)]
pub struct SaCycle {
}

impl SaCycle {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn num(s: &str, fallback: f64) -> f64 {
        let v: Option<f64> = s.trim().to_string().parse::<f64>().ok();
        if v.is_some() { v.unwrap() } else { fallback }
    }
    pub fn place(cons: &Rc<RefCell<SaConstraints>>, p: &Rc<RefCell<SaInst>>) {
        let mut cells: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if SaConn::isCell(&c) {
                cells.push(c.clone());
            }
            i += 1;
        }
        let n: i64 = cells.len() as i64;
        if n == 0 {
            i = 0;
            while i < (p.borrow().children.len() as i64) {
                SaConn::hide(&p.borrow().children[i as usize].clone(), p);
                i += 1;
            }
            return;
        }
        let has_ctr: bool = (p.borrow().alg).borrow().param("ctrShpMap", "none") == "fNode";
        let mut ring: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        let mut first: i64 = 0;
        if has_ctr {
            first = 1;
        }
        i = first;
        while i < n {
            ring.push(cells[i as usize].clone());
            i += 1;
        }
        let m: i64 = ring.len() as i64;
        let st_ang: f64 = SaCycle::num(&(p.borrow().alg).borrow().param("stAng", "0"), 0.0);
        let span: f64 = SaCycle::num(&(p.borrow().alg).borrow().param("spanAng", "360"), 360.0);
        // a negative span goes round anticlockwise
        let full: bool = (span.abs()) >= 359.999;
        let mut step: f64 = 0.0;
        if m > 1 {
            if full {
                step = span / (m as f64);
            } else {
                step = span / ((m - 1) as f64);
            }
        }
        // sizes, as the constraints give them
        let mut ws: Vec<f64> = Vec::new();
        let mut hs: Vec<f64> = Vec::new();
        let mut d: f64 = 0.0;
        i = 0;
        while i < m {
            let mut r0: Rc<RefCell<SaInst>> = ring[i as usize].clone();
            let rw: f64 = cons.borrow().get(&r0, "w");
            let rh: f64 = cons.borrow().get(&r0, "h");
            ws.push(rw);
            hs.push(rh);
            if rw > d {
                d = rw;
            }
            if rh > d {
                d = rh;
            }
            i += 1;
        }
        let mut cw: f64 = 0.0;
        let mut chh: f64 = 0.0;
        if has_ctr {
            let mut c0: Rc<RefCell<SaInst>> = cells[0].clone();
            cw = cons.borrow().get(&c0, "w");
            chh = cons.borrow().get(&c0, "h");
        }
        let sp: f64 = cons.borrow().get(p, "sibSp");
        // the radius that keeps neighbours apart, and clear of the middle
        let mut r: f64 = 0.0;
        if m > 1 {
            let half: f64 = SaConn::rad((step / 2.0).abs());
            let sn: f64 = half.sin();
            if sn > 0.000001 {
                r = (d + sp) / (2.0 * sn);
            }
        }
        if has_ctr && m > 0 {
            let mut cd: f64 = cw;
            if chh > cd {
                cd = chh;
            }
            let rc: f64 = (cd / 2.0 + sp) + d / 2.0;
            if rc > r {
                r = rc;
            }
        }
        // the bounds of it all, centred at 0, 0
        let mut min_x: f64 = 0.0;
        let mut max_x: f64 = 0.0;
        let mut min_y: f64 = 0.0;
        let mut max_y: f64 = 0.0;
        if has_ctr {
            min_x = 0.0 - cw / 2.0;
            max_x = cw / 2.0;
            min_y = 0.0 - chh / 2.0;
            max_y = chh / 2.0;
        }
        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();
        i = 0;
        while i < m {
            let a: f64 = SaConn::rad(st_ang + (i as f64) * step);
            let x: f64 = r * a.sin();
            let y: f64 = 0.0 - r * a.cos();
            xs.push(x);
            ys.push(y);
            let hw: f64 = ws[i as usize] / 2.0;
            let hh: f64 = hs[i as usize] / 2.0;
            if i == 0 && !has_ctr {
                min_x = x - hw;
                max_x = x + hw;
                min_y = y - hh;
                max_y = y + hh;
            }
            if x - hw < min_x {
                min_x = x - hw;
            }
            if x + hw > max_x {
                max_x = x + hw;
            }
            if y - hh < min_y {
                min_y = y - hh;
            }
            if y + hh > max_y {
                max_y = y + hh;
            }
            i += 1;
        }
        let bw: f64 = max_x - min_x;
        let bh: f64 = max_y - min_y;
        let mut s: f64 = 1.0;
        if bw > 0.0 && p.borrow().w / bw < s {
            s = p.borrow().w / bw;
        }
        if bh > 0.0 && p.borrow().h / bh < s {
            s = p.borrow().h / bh;
        }
        // the centre of the circle, so that the bounds sit in the middle
        let ox: f64 = (p.borrow().x + p.borrow().w / 2.0) - ((min_x + max_x) / 2.0) * s;
        let oy: f64 = (p.borrow().y + p.borrow().h / 2.0) - ((min_y + max_y) / 2.0) * s;
        let along: bool = (p.borrow().alg).borrow().param("rotPath", "none") == "alongPath";
        if has_ctr {
            let mut c1: Rc<RefCell<SaInst>> = cells[0].clone();
            c1.borrow_mut().w = cw * s;
            c1.borrow_mut().h = chh * s;
            let hoist_3 = ox - c1.borrow().w / 2.0;
            c1.borrow_mut().x = hoist_3;
            let hoist_4 = oy - c1.borrow().h / 2.0;
            c1.borrow_mut().y = hoist_4;
        }
        i = 0;
        while i < m {
            let mut rn: Rc<RefCell<SaInst>> = ring[i as usize].clone();
            rn.borrow_mut().w = ws[i as usize] * s;
            rn.borrow_mut().h = hs[i as usize] * s;
            let hoist_5 = (ox + xs[i as usize] * s) - rn.borrow().w / 2.0;
            rn.borrow_mut().x = hoist_5;
            let hoist_6 = (oy + ys[i as usize] * s) - rn.borrow().h / 2.0;
            rn.borrow_mut().y = hoist_6;
            if along {
                rn.borrow_mut().rotDeg = st_ang + (i as f64) * step;
            }
            i += 1;
        }
        // the connectors
        let mut sib: i64 = 0;
        let mut par: i64 = 0;
        let gap: f64 = ((2.0 * r) * SaConn::rad((step / 2.0).abs()).sin() - d) * s;
        i = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c3: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if SaConn::isConn(&c3) {
                if c3.borrow().ctxType == "parTrans" && has_ctr {
                    if par < m {
                        SaConn::between(cons, &c3, &cells[0].clone(), &ring[par as usize].clone());
                    } else {
                        SaConn::hide(&c3, p);
                    }
                    par += 1;
                } else {
                    let mut last: i64 = m - 1;
                    if full {
                        last = m;
                    }
                    if sib < last && m > 1 {
                        let mut from: Rc<RefCell<SaInst>> = ring[sib as usize].clone();
                        let mut to: Rc<RefCell<SaInst>> = ring[((sib + 1) % m) as usize].clone();
                        if SaConn::is1D(&c3) {
                            SaConn::between(cons, &c3, &from, &to);
                        } else {
                            let mid: f64 = st_ang + ((sib as f64) + 0.5) * step;
                            let ma: f64 = SaConn::rad(mid);
                            let cx: f64 = ox + (r * s) * ma.sin();
                            let cy: f64 = oy - (r * s) * ma.cos();
                            let mut __len: f64 = cons.borrow().get(&c3, "w") * s;
                            let mut thick: f64 = cons.borrow().get(&c3, "h") * s;
                            if __len > gap * 0.8 && gap > 0.0 {
                                let f: f64 = (gap * 0.8) / __len;
                                __len *= f;
                                thick *= f;
                            }
                            // the tangent: the arrow points from this shape on
                            // to the next, clockwise or, on a negative span,
                            // anticlockwise
                            let mut heading: f64 = mid;
                            if step < 0.0 {
                                heading = mid + 180.0;
                            }
                            SaConn::arrowAt(&c3, cx, cy, heading, __len, thick);
                        }
                    } else {
                        SaConn::hide(&c3, p);
                    }
                    sib += 1;
                }
            } else {
                if SaConn::isSpace(&c3) && !SaConn::isCell(&c3) {
                    SaConn::hide(&c3, p);
                }
            }
            i += 1;
        }
    }
}
