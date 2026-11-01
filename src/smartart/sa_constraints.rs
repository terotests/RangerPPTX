// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaConstraints — how big everything is, before anything is placed
//! ==============================================================================
//!
//! A constraint lives on one layout node and sets a value on itself (`for`
//! self), on its children (ch) or on everything under it (des), optionally only
//! those with a given name (`forName`) or presenting a given kind of point
//! (`ptType`). The value is a number (`val`), or another value times a factor:
//! `refType` read off `refFor`/`refForName` (`fact`, 1 when left out).
//!
//!   <dgm:constr type="h" for="ch" forName="node" refType="w" refFor="ch"
//!               refForName="node" fact="0.6"/>    a node is 0.6 as tall as wide
//!
//! `op` turns a value into a relation. `equ` on a font size makes every target
//! ONE size — the smallest any of them needs once the text is fitted — which is
//! why a process's steps all have the same type size. `gte` and `lte` are
//! bounds kept beside the value, so a later shrink (a rule) stays inside them.
//!
//! Values are resolved in program order, three times over, so a constraint
//! that reads a value another constraint sets later in the file still sees it.
//! A value nothing set is its default: a width or height is the parent's (the
//! frame's, at the root), a font size 65 pt, anything else 0.
//!
//! Each value remembers what it was derived from. `lin` needs that: a node
//! whose height is a fraction of its width shrinks with it when a row is too
//! long, and a column whose height is its parent's does not.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_layout_def::SaConstr;
use crate::smartart::sa_pres_tree::SaInst;
use crate::smartart::sa_pres_tree::SaPresTree;

#[derive(Clone)]
pub struct SaConstraints {
    pub ptree: Rc<RefCell<SaPresTree>>,
    pub frameW: f64,
    pub frameH: f64,
    pub fontGroups: i64,
    pub parents: Vec<Rc<RefCell<SaInst>>>,
    pub kids: Vec<Rc<RefCell<SaInst>>>,
}

impl SaConstraints {
    pub fn new() -> Self {
        Self {
            ptree: Rc::new(RefCell::new(SaPresTree::new())),
            frameW: 0.0,
            frameH: 0.0,
            fontGroups: 0,
            parents: Vec::new(),
            kids: Vec::new(),
        }
    }
    pub fn resolve(t: Rc<RefCell<SaPresTree>>, w: f64, h: f64) -> Rc<RefCell<SaConstraints>> {
        let mut c: Rc<RefCell<SaConstraints>> = Rc::new(RefCell::new(SaConstraints::new()));
        c.borrow_mut().ptree = t.clone();
        c.borrow_mut().frameW = w;
        c.borrow_mut().frameH = h;
        c.borrow_mut().indexParents();
        t.borrow().root.clone().borrow_mut().put("w", w, "");
        t.borrow().root.clone().borrow_mut().put("h", h, "");
        let mut pass: i64 = 0;
        while pass < 3 {
            c.borrow_mut().fontGroups = 0;
            c.borrow_mut().applyAll();
            pass += 1;
        }
        c
    }
    pub fn indexParents(&mut self) {
        let mut every: Vec<Rc<RefCell<SaInst>>> = self.ptree.borrow().all();
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut p: Rc<RefCell<SaInst>> = every[i as usize].clone();
            let mut k: i64 = 0;
            while k < (p.borrow().children.len() as i64) {
                self.kids.push(p.borrow().children[k as usize].clone());
                self.parents.push(p.clone());
                k += 1;
            }
            i += 1;
        }
    }
    pub fn parentOf(&self, s: &Rc<RefCell<SaInst>>) -> Option<Rc<RefCell<SaInst>>> {
        let mut i: i64 = 0;
        while i < (self.kids.len() as i64) {
            if Rc::ptr_eq(&self.kids[i as usize], &s) {
                let mut hit: Option<Rc<RefCell<SaInst>>> = Some(self.parents[i as usize].clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaInst>>> = None;
        none
    }
    /// A value as anything reading it sees it: what was set, or the default.
    pub fn get(&self, s: &Rc<RefCell<SaInst>>, r#type: &str) -> f64 {
        if s.borrow().has(r#type) {
            return s.borrow().raw(r#type, 0.0);
        }
        if r#type == "w" || r#type == "h" {
            let mut p: Option<Rc<RefCell<SaInst>>> = self.parentOf(s);
            if p.is_none() {
                return if (r#type == "w") { self.frameW } else { self.frameH };
            }
            return self.get(&p.clone().unwrap(), r#type);
        }
        if r#type == "primFontSz" || r#type == "secFontSz" {
            return 65.0;
        }
        0.0
    }
    pub fn matches(&self, s: &Rc<RefCell<SaInst>>, name: &str, pt_type: &str) -> bool {
        if !name.is_empty() && s.borrow().name != name {
            return false;
        }
        if pt_type == "all" {
            return true;
        }
        if pt_type == "node" {
            return s.borrow().ctxType == "node" || s.borrow().ctxType == "asst";
        }
        if pt_type == "norm" {
            return s.borrow().ctxType == "node";
        }
        if pt_type == "nonNorm" {
            return s.borrow().ctxType != "node";
        }
        if pt_type == "nonAsst" {
            return s.borrow().ctxType != "asst";
        }
        return s.borrow().ctxType == pt_type;
    }
    pub fn targets(&self, s: &Rc<RefCell<SaInst>>, rel: &str, name: &str, pt_type: &str) -> Vec<Rc<RefCell<SaInst>>> {
        let mut out: Vec<Rc<RefCell<SaInst>>> = Vec::new();
        if rel == "self" {
            out.push(s.clone());
            return out;
        }
        let mut pool: Vec<Rc<RefCell<SaInst>>> = s.borrow().children.clone();
        if rel == "des" {
            pool = s.borrow().descendants();
        }
        let mut i: i64 = 0;
        while i < (pool.len() as i64) {
            let mut c: Rc<RefCell<SaInst>> = pool[i as usize].clone();
            if self.matches(&c, name, pt_type) {
                out.push(c.clone());
            }
            i += 1;
        }
        out
    }
    pub fn applyAll(&mut self) {
        let mut every: Vec<Rc<RefCell<SaInst>>> = self.ptree.borrow().all();
        let mut i: i64 = 0;
        while i < (every.len() as i64) {
            let mut s: Rc<RefCell<SaInst>> = every[i as usize].clone();
            let mut k: i64 = 0;
            while k < (s.borrow().constrs.len() as i64) {
                self.apply(&s, &s.borrow().constrs[k as usize].clone().clone());
                k += 1;
            }
            i += 1;
        }
    }
    pub fn apply(&mut self, s: &Rc<RefCell<SaInst>>, k: &Rc<RefCell<SaConstr>>) {
        let mut _tmp_296 = k.borrow().forRel.clone();
        let mut _tmp_297 = k.borrow().forName.clone();
        let mut _tmp_298 = k.borrow().ptType.clone();
        let mut ts: Vec<Rc<RefCell<SaInst>>> = self.targets(s,
            &_tmp_296,
            &_tmp_297,
            &_tmp_298
        );
        if ts.is_empty() {
            return;
        }
        // the value, once: references are read from `s`'s point of view
        let mut has: bool = false;
        let mut v: f64 = 0.0;
        let mut from: String = String::new();
        if k.borrow().refType != "none" {
            let mut _tmp_299 = k.borrow().refFor.clone();
            let mut _tmp_300 = k.borrow().refForName.clone();
            let mut _tmp_301 = k.borrow().refPtType.clone();
            let mut refs: Vec<Rc<RefCell<SaInst>>> = self.targets(s,
                &_tmp_299,
                &_tmp_300,
                &_tmp_301
            );
            if !refs.is_empty() {
                let mut r: Rc<RefCell<SaInst>> = refs[0].clone();
                v = self.get(&r, &k.borrow().refType) * k.borrow().fact;
                has = true;
                from = k.borrow().refType.clone();
                // a reference to the target's own other dimension, or to one
                // that was itself derived from it, carries the tie along
                let rf: String = r.borrow().fromOf(&k.borrow().refType.clone());
                if !rf.is_empty() {
                    from = rf.clone();
                }
            }
        } else {
            if k.borrow().hasVal {
                v = k.borrow().val;
                if k.borrow().hasFact {
                    v *= k.borrow().fact;
                }
                has = true;
            }
        }
        if (k.borrow().r#type == "primFontSz" || k.borrow().r#type == "secFontSz") && k.borrow().op == "equ" {
            self.fontGroups += 1;
        }
        let mut i: i64 = 0;
        while i < (ts.len() as i64) {
            let mut t: Rc<RefCell<SaInst>> = ts[i as usize].clone();
            if (k.borrow().r#type == "primFontSz" || k.borrow().r#type == "secFontSz") && k.borrow().op == "equ" {
                t.borrow_mut().fontGroup = self.fontGroups;
            }
            if has {
                if k.borrow().op == "none" || k.borrow().op == "equ" {
                    t.borrow_mut().put(&k.borrow().r#type.clone(), v, &from);
                }
                if k.borrow().op == "gte" {
                    t.borrow_mut().bound(&k.borrow().r#type.clone(), v, 1000000000.0);
                    if self.get(&t, &k.borrow().r#type) < v {
                        t.borrow_mut().put(&k.borrow().r#type.clone(), v, &from);
                    }
                }
                if k.borrow().op == "lte" {
                    t.borrow_mut().bound(&k.borrow().r#type.clone(), -1000000000.0, v);
                    if self.get(&t, &k.borrow().r#type) > v {
                        t.borrow_mut().put(&k.borrow().r#type.clone(), v, &from);
                    }
                }
            }
            i += 1;
        }
    }
}
