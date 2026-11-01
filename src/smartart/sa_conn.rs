// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaConn — connectors: the lines and arrows between two shapes
//! ==============================================================================
//!
//! A connector is a layout node with the `conn` algorithm. Who draws it
//! depends on its parent's algorithm:
//!
//!   lin      places it in the line like any child, an arrow pointing the way
//!            the line goes (SaAlgorithms.lin)
//!   cycle    an arrow on the circle between two nodes, or a line from the
//!            centre to a node (SaCycle)
//!   hierRoot elbow lines from a box to the boxes under it (SaHier)
//!   anything else
//!            between the shapes it names (`srcNode` / `dstNode`), else the
//!            sibling before it and the one after it (`connect` here)
//!
//! Its parameters:
//!
//!   dim      2D (default): an arrow shape between the two, along the line
//!            from one to the other; 1D: a line, edge to edge
//!   endSty   arr / noArr / auto: an arrowhead at a 1D line's end; auto is
//!            one for a sibling transition, none for a parent transition
//!   connRout stra (default) or bend: bend makes the line an elbow (vertical,
//!            across, vertical)
//!   begPts / endPts
//!            auto (default): where the line from one centre to the other
//!            leaves the shape; ctr, tCtr, bCtr, midL, midR: that point of
//!            the box
//!
//! A line is kept as segments (SaInst.segs) and drawn as one `line` shape
//! per segment (SaEngine); its box is the segments' bounds.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_constraints::SaConstraints;
use crate::smartart::sa_pres_tree::SaInst;

#[derive(Clone)]
pub struct SaConn {
}

impl SaConn {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn isConn(c: &Rc<RefCell<SaInst>>) -> bool {
        if c.borrow().alg.borrow().r#type == "conn" {
            return true;
        }
        return c.borrow().hasShape && c.borrow().shape.borrow().r#type == "conn";
    }
    pub fn isSpace(c: &Rc<RefCell<SaInst>>) -> bool {
        return c.borrow().alg.borrow().r#type == "sp";
    }
    /// a shape that takes a place of its own: not a connector, not a space
    pub fn isCell(c: &Rc<RefCell<SaInst>>) -> bool {
        !SaConn::isConn(c) && !SaConn::isSpace(c)
    }
    pub fn is1D(c: &Rc<RefCell<SaInst>>) -> bool {
        return (c.borrow().alg).borrow().param("dim", "2D") == "1D";
    }
    pub fn endArrow(c: &Rc<RefCell<SaInst>>) -> bool {
        let e: String = (c.borrow().alg).borrow().param("endSty", "auto");
        if e == "arr" {
            return true;
        }
        if e == "noArr" {
            return false;
        }
        return c.borrow().ctxType == "sibTrans";
    }
    pub fn rad(deg: f64) -> f64 {
        (deg * 3.141592653589793) / 180.0
    }
    pub fn deg(r: f64) -> f64 {
        (r * 180.0) / 3.141592653589793
    }
    /// no segments, no box: drawn as nothing
    pub fn hide(c: &Rc<RefCell<SaInst>>, at: &Rc<RefCell<SaInst>>) {
        let mut none: Vec<f64> = Vec::new();
        c.borrow_mut().segs = none.clone();
        c.borrow_mut().x = at.borrow().x;
        c.borrow_mut().y = at.borrow().y;
        c.borrow_mut().w = 0.0;
        c.borrow_mut().h = 0.0;
    }
    /// the box of a line's segments; a line along an axis gets no thickness,
    /// which is what a line shape is
    pub fn boxOfSegs(c: &Rc<RefCell<SaInst>>) {
        let n: i64 = c.borrow().segs.len() as i64;
        if n < 4 {
            return;
        }
        let mut min_x: f64 = c.borrow().segs[0];
        let mut min_y: f64 = c.borrow().segs[1];
        let mut max_x: f64 = min_x;
        let mut max_y: f64 = min_y;
        let mut i: i64 = 0;
        while i + 1 < n {
            let x: f64 = c.borrow().segs[i as usize];
            let y: f64 = c.borrow().segs[(i + 1) as usize];
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
            i += 2;
        }
        c.borrow_mut().x = min_x;
        c.borrow_mut().y = min_y;
        c.borrow_mut().w = max_x - min_x;
        c.borrow_mut().h = max_y - min_y;
    }
    pub fn line(c: &Rc<RefCell<SaInst>>, x1: f64, y1: f64, x2: f64, y2: f64) {
        let mut s: Vec<f64> = Vec::new();
        s.push(x1);
        s.push(y1);
        s.push(x2);
        s.push(y2);
        c.borrow_mut().segs = s.clone();
        c.borrow_mut().arrowEnd = SaConn::endArrow(c);
        SaConn::boxOfSegs(c);
    }
    /// an elbow: down from (x1, y1) to `midY`, across to x2, down to (x2, y2).
    /// Segments of no length are left out.
    pub fn elbow(c: &Rc<RefCell<SaInst>>, x1: f64, y1: f64, x2: f64, y2: f64, mid_y: f64) {
        let mut s: Vec<f64> = Vec::new();
        SaConn::seg(&mut s, x1, y1, x1, mid_y);
        SaConn::seg(&mut s, x1, mid_y, x2, mid_y);
        SaConn::seg(&mut s, x2, mid_y, x2, y2);
        c.borrow_mut().segs = s.clone();
        c.borrow_mut().arrowEnd = SaConn::endArrow(c);
        SaConn::boxOfSegs(c);
    }
    /// down from (x1, y1) to y2, then across to x2
    pub fn hang(c: &Rc<RefCell<SaInst>>, x1: f64, y1: f64, x2: f64, y2: f64) {
        let mut s: Vec<f64> = Vec::new();
        SaConn::seg(&mut s, x1, y1, x1, y2);
        SaConn::seg(&mut s, x1, y2, x2, y2);
        c.borrow_mut().segs = s.clone();
        c.borrow_mut().arrowEnd = SaConn::endArrow(c);
        SaConn::boxOfSegs(c);
    }
    pub fn seg(s: &mut Vec<f64>, x1: f64, y1: f64, x2: f64, y2: f64) {
        let dx: f64 = x2 - x1;
        let dy: f64 = y2 - y1;
        if dx * dx + dy * dy < 0.0001 {
            return;
        }
        s.push(x1);
        s.push(y1);
        s.push(x2);
        s.push(y2);
    }
    /// Where the line from b's centre towards (tx, ty) leaves b: on the
    /// ellipse for an ellipse, on the box for anything else.
    pub fn edgePoint(b: &Rc<RefCell<SaInst>>, tx: f64, ty: f64) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        let cx: f64 = b.borrow().x + b.borrow().w / 2.0;
        let cy: f64 = b.borrow().y + b.borrow().h / 2.0;
        let dx: f64 = tx - cx;
        let dy: f64 = ty - cy;
        let __len: f64 = (dx * dx + dy * dy).sqrt();
        if __len < 0.0001 || (b.borrow().w <= 0.0 || b.borrow().h <= 0.0) {
            out.push(cx);
            out.push(cy);
            return out;
        }
        let ux: f64 = dx / __len;
        let uy: f64 = dy / __len;
        let a: f64 = b.borrow().w / 2.0;
        let bb: f64 = b.borrow().h / 2.0;
        let mut t: f64 = 0.0;
        if b.borrow().hasShape && b.borrow().shape.borrow().r#type == "ellipse" {
            let qx: f64 = ux / a;
            let qy: f64 = uy / bb;
            t = 1.0 / ((qx * qx + qy * qy).sqrt());
        } else {
            let mut tx2: f64 = 1000000000.0;
            let mut ty2: f64 = 1000000000.0;
            if (ux.abs()) > 0.000001 {
                tx2 = a / (ux.abs());
            }
            if (uy.abs()) > 0.000001 {
                ty2 = bb / (uy.abs());
            }
            t = tx2;
            if ty2 < t {
                t = ty2;
            }
        }
        out.push(cx + ux * t);
        out.push(cy + uy * t);
        out
    }
    /// a named point of a box: ctr tCtr bCtr midL midR
    pub fn pointOf(b: &Rc<RefCell<SaInst>>, name: &str) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        let mut x: f64 = b.borrow().x + b.borrow().w / 2.0;
        let mut y: f64 = b.borrow().y + b.borrow().h / 2.0;
        if name == "tCtr" {
            y = b.borrow().y;
        }
        if name == "bCtr" {
            y = b.borrow().y + b.borrow().h;
        }
        if name == "midL" {
            x = b.borrow().x;
        }
        if name == "midR" {
            x = b.borrow().x + b.borrow().w;
        }
        out.push(x);
        out.push(y);
        out
    }
    /// A 2D arrow (the connector's own shape) centred at (cx, cy), pointing
    /// at `angle` degrees (0 is to the right, clockwise), `len` long and
    /// `thick` across.
    pub fn arrowAt(c: &Rc<RefCell<SaInst>>, cx: f64, cy: f64, angle: f64, __len: f64, thick: f64) {
        let mut none: Vec<f64> = Vec::new();
        c.borrow_mut().segs = none.clone();
        c.borrow_mut().w = __len;
        c.borrow_mut().h = thick;
        c.borrow_mut().x = cx - __len / 2.0;
        c.borrow_mut().y = cy - thick / 2.0;
        c.borrow_mut().put("connDir", 0.0, "");
        c.borrow_mut().rotDeg = angle;
    }
    /// From shape `a` to shape `b`: a line edge to edge (1D), or an arrow in
    /// the gap between them (2D), no longer than `maxLen` and as thick as the
    /// connector's own height asks.
    pub fn between(cons: &Rc<RefCell<SaConstraints>>, c: &Rc<RefCell<SaInst>>, a: &Rc<RefCell<SaInst>>, b: &Rc<RefCell<SaInst>>) {
        let bp: String = (c.borrow().alg).borrow().param("begPts", "auto");
        let ep: String = (c.borrow().alg).borrow().param("endPts", "auto");
        let acx: f64 = a.borrow().x + a.borrow().w / 2.0;
        let acy: f64 = a.borrow().y + a.borrow().h / 2.0;
        let bcx: f64 = b.borrow().x + b.borrow().w / 2.0;
        let bcy: f64 = b.borrow().y + b.borrow().h / 2.0;
        let mut p1: Vec<f64> = SaConn::edgePoint(a, bcx, bcy);
        if bp != "auto" {
            p1 = SaConn::pointOf(a, &bp);
        }
        let mut p2: Vec<f64> = SaConn::edgePoint(b, acx, acy);
        if ep != "auto" {
            p2 = SaConn::pointOf(b, &ep);
        }
        let x1: f64 = p1[0];
        let y1: f64 = p1[1];
        let x2: f64 = p2[0];
        let y2: f64 = p2[1];
        if SaConn::is1D(c) {
            if (c.borrow().alg).borrow().param("connRout", "stra") == "bend" {
                SaConn::elbow(c, x1, y1, x2, y2, (y1 + y2) / 2.0);
            } else {
                SaConn::line(c, x1, y1, x2, y2);
            }
            return;
        }
        let dx: f64 = x2 - x1;
        let dy: f64 = y2 - y1;
        let gap: f64 = (dx * dx + dy * dy).sqrt();
        let mut __len: f64 = cons.borrow().get(c, "w");
        let mut thick: f64 = cons.borrow().get(c, "h");
        // the gap's middle 80 %, at most
        if __len > gap * 0.8 {
            let f: f64 = (gap * 0.8) / __len;
            __len *= f;
            if c.borrow().fromOf("h") == "w" {
                thick *= f;
            }
        }
        if thick > __len {
            thick = __len;
        }
        SaConn::arrowAt(c, (x1 + x2) / 2.0, (y1 + y2) / 2.0, SaConn::deg(dy.atan2(dx)), __len, thick);
    }
    pub fn byName(p: &Rc<RefCell<SaInst>>, name: &str) -> Option<Rc<RefCell<SaInst>>> {
        let mut i: i64 = 0;
        while i < (p.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if c.borrow().name == name {
                let mut hit: Option<Rc<RefCell<SaInst>>> = Some(c.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaInst>>> = None;
        none
    }
    /// The connectors among p's children that its algorithm did not place:
    /// each between the shapes it names, else the shape before it and the
    /// shape after it.
    pub fn connect(cons: &Rc<RefCell<SaConstraints>>, p: &Rc<RefCell<SaInst>>) {
        let n: i64 = p.borrow().children.len() as i64;
        let mut i: i64 = 0;
        while i < n {
            let mut c: Rc<RefCell<SaInst>> = p.borrow().children[i as usize].clone();
            if SaConn::isConn(&c) {
                let mut src: Option<Rc<RefCell<SaInst>>> = None;
                let mut dst: Option<Rc<RefCell<SaInst>>> = None;
                let sn: String = (c.borrow().alg).borrow().param("srcNode", "");
                let dn: String = (c.borrow().alg).borrow().param("dstNode", "");
                if !sn.is_empty() {
                    src = SaConn::byName(p, &sn);
                }
                if !dn.is_empty() {
                    dst = SaConn::byName(p, &dn);
                }
                if src.is_none() {
                    let mut j: i64 = i - 1;
                    while j >= 0 && src.is_none() {
                        let mut b: Rc<RefCell<SaInst>> = p.borrow().children[j as usize].clone();
                        if SaConn::isCell(&b) {
                            src = Some(b.clone());
                        }
                        j -= 1;
                    }
                }
                if dst.is_none() {
                    let mut k: i64 = i + 1;
                    while k < n && dst.is_none() {
                        let mut b2: Rc<RefCell<SaInst>> = p.borrow().children[k as usize].clone();
                        if SaConn::isCell(&b2) {
                            dst = Some(b2.clone());
                        }
                        k += 1;
                    }
                }
                if src.is_some() && dst.is_some() {
                    SaConn::between(cons, &c, &src.clone().unwrap(), &dst.clone().unwrap());
                } else {
                    SaConn::hide(&c, p);
                }
            }
            i += 1;
        }
    }
}
