// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaLayoutDef — a SmartArt layout definition, as data
//! ==============================================================================
//!
//! `dgm:layoutDef` is a small program. Its `layoutNode`s are the shapes a
//! diagram may have; `forEach` repeats what it holds once per data point it
//! walks to; `choose`/`if`/`else` pick one branch by a test on the data; each
//! layout node says which algorithm places its children (`alg`), what its own
//! geometry is (`shape`), which data point's text it shows (`presOf`), how big
//! things are (`constrLst`) and what may give way when the text does not fit
//! (`ruleLst`).
//!
//! This file is only the program's text, read into objects. Running it against
//! a data model is `SaPresTree`; sizing is `SaConstraints`; placing is the
//! algorithms.
//!
//! Attributes the specification types as LISTS — `axis`, `ptType`, `st`,
//! `cnt`, `step`, `hideLastTrans` — are lists here too: `axis="ch ch"` is two
//! steps, children of children, and `ptType="node node"` filters each step.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;


#[derive(Clone)]
pub struct SaParam {
    pub name: String,
    pub value: String,
}

impl SaParam {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            value: String::new(),
        }
    }
}

/// A walk from the current data point: `forEach`, `presOf` and `if` all use it.
#[derive(Clone)]
pub struct SaIter {
    pub axis: Vec<String>,
    pub ptType: Vec<String>,
    pub st: Vec<i64>,
    pub cnt: Vec<i64>,
    pub step: Vec<i64>,
    pub hideLastTrans: Vec<bool>,
}

impl SaIter {
    pub fn new() -> Self {
        Self {
            axis: Vec::new(),
            ptType: Vec::new(),
            st: Vec::new(),
            cnt: Vec::new(),
            step: Vec::new(),
            hideLastTrans: Vec::new(),
        }
    }
    /// the value for step i, or the default when the list is shorter
    pub fn axisAt(&self, i: i64) -> String {
        if i < (self.axis.len() as i64) {
            return self.axis[i as usize].clone();
        }
        "none".to_string()
    }
    pub fn ptTypeAt(&self, i: i64) -> String {
        if i < (self.ptType.len() as i64) {
            return self.ptType[i as usize].clone();
        }
        if !self.ptType.is_empty() {
            return self.ptType[((self.ptType.len() as i64) - 1) as usize].clone()
                .clone();
        }
        "all".to_string()
    }
    pub fn stAt(&self, i: i64) -> i64 {
        if i < (self.st.len() as i64) {
            return self.st[i as usize];
        }
        1
    }
    pub fn cntAt(&self, i: i64) -> i64 {
        if i < (self.cnt.len() as i64) {
            return self.cnt[i as usize];
        }
        0
    }
    pub fn stepAt(&self, i: i64) -> i64 {
        if i < (self.step.len() as i64) {
            return self.step[i as usize];
        }
        1
    }
    pub fn hideLastTransAt(&self, i: i64) -> bool {
        if i < (self.hideLastTrans.len() as i64) {
            return self.hideLastTrans[i as usize];
        }
        true
    }
    pub fn steps(&self) -> i64 {
        self.axis.len() as i64
    }
}

#[derive(Clone)]
pub struct SaAlgDef {
    pub r#type: String,
    pub params: Vec<Rc<RefCell<SaParam>>>,
}

impl SaAlgDef {
    pub fn new() -> Self {
        Self {
            r#type: String::new(),
            params: Vec::new(),
        }
    }
    pub fn param(&self, name: &str, fallback: &str) -> String {
        let mut i: i64 = 0;
        while i < (self.params.len() as i64) {
            let mut p: Rc<RefCell<SaParam>> = self.params[i as usize].clone();
            if p.borrow().name == name {
                return p.borrow().value.clone();
            }
            i += 1;
        }
        fallback.to_string()
    }
}

#[derive(Clone)]
pub struct SaShapeDef {
    pub r#type: String,
    pub rot: f64,
    pub zOrderOff: i64,
    pub hideGeom: bool,
    pub lkTxEntry: bool,
    /// adjust values by `idx` (1-based in the file, stored at idx-1); NaN-free:
    /// `adjSet` says which were given
    pub adj: Vec<f64>,
    pub adjSet: Vec<bool>,
}

impl SaShapeDef {
    pub fn new() -> Self {
        Self {
            r#type: "none".to_string(),
            rot: 0.0,
            zOrderOff: 0,
            hideGeom: false,
            lkTxEntry: false,
            adj: Vec::new(),
            adjSet: Vec::new(),
        }
    }
    pub fn setAdj(&mut self, idx: i64, v: f64) {
        while (self.adj.len() as i64) < idx {
            self.adj.push(0.0);
            self.adjSet.push(false);
        }
        self.adj[(idx - 1) as usize] = v;
        self.adjSet[(idx - 1) as usize] = true;
    }
}

#[derive(Clone)]
pub struct SaConstr {
    pub r#type: String,
    /// self ch des
    pub forRel: String,
    pub forName: String,
    pub ptType: String,
    pub refType: String,
    pub refFor: String,
    pub refForName: String,
    pub refPtType: String,
    /// none equ gte lte
    pub op: String,
    pub val: f64,
    pub hasVal: bool,
    pub fact: f64,
    pub hasFact: bool,
}

impl SaConstr {
    pub fn new() -> Self {
        Self {
            r#type: "none".to_string(),
            forRel: "self".to_string(),
            forName: String::new(),
            ptType: "all".to_string(),
            refType: "none".to_string(),
            refFor: "self".to_string(),
            refForName: String::new(),
            refPtType: "all".to_string(),
            op: "none".to_string(),
            val: 0.0,
            hasVal: false,
            fact: 1.0,
            hasFact: false,
        }
    }
}

#[derive(Clone)]
pub struct SaRule {
    pub r#type: String,
    pub forRel: String,
    pub forName: String,
    pub ptType: String,
    /// each of these may be left out (the file's "NaN")
    pub val: f64,
    pub hasVal: bool,
    pub fact: f64,
    pub hasFact: bool,
    pub max: f64,
    pub hasMax: bool,
}

impl SaRule {
    pub fn new() -> Self {
        Self {
            r#type: "none".to_string(),
            forRel: "self".to_string(),
            forName: String::new(),
            ptType: "all".to_string(),
            val: 0.0,
            hasVal: false,
            fact: 0.0,
            hasFact: false,
            max: 0.0,
            hasMax: false,
        }
    }
}

/// One element of the program: a layoutNode, a forEach, a choose, or one of a
/// choose's branches (if / else). They share a body: the children, in order.
#[derive(Clone)]
pub struct SaLoNode {
    /// layoutNode forEach choose if else
    pub kind: String,
    pub name: String,
    /// layoutNode
    pub styleLbl: String,
    /// b (bottom first) or t
    pub chOrder: String,
    pub moveWith: String,
    pub hasAlg: bool,
    pub alg: Rc<RefCell<SaAlgDef>>,
    pub hasShape: bool,
    pub shape: Rc<RefCell<SaShapeDef>>,
    pub hasPresOf: bool,
    pub presOf: Rc<RefCell<SaIter>>,
    pub constrs: Vec<Rc<RefCell<SaConstr>>>,
    pub rules: Vec<Rc<RefCell<SaRule>>>,
    pub vars: Vec<Rc<RefCell<SaParam>>>,
    /// forEach (and if): the walk
    pub iter: Rc<RefCell<SaIter>>,
    pub r#ref: String,
    pub func: String,
    pub arg: String,
    pub op: String,
    pub val: String,
    pub children: Vec<Rc<RefCell<SaLoNode>>>,
}

impl SaLoNode {
    pub fn new() -> Self {
        Self {
            kind: "layoutNode".to_string(),
            name: String::new(),
            styleLbl: String::new(),
            chOrder: "b".to_string(),
            moveWith: String::new(),
            hasAlg: false,
            alg: Rc::new(RefCell::new(SaAlgDef::new())),
            hasShape: false,
            shape: Rc::new(RefCell::new(SaShapeDef::new())),
            hasPresOf: false,
            presOf: Rc::new(RefCell::new(SaIter::new())),
            constrs: Vec::new(),
            rules: Vec::new(),
            vars: Vec::new(),
            iter: Rc::new(RefCell::new(SaIter::new())),
            r#ref: String::new(),
            func: String::new(),
            arg: String::new(),
            op: String::new(),
            val: String::new(),
            children: Vec::new(),
        }
    }
    pub fn varValue(&self, name: &str, fallback: &str) -> String {
        let mut i: i64 = 0;
        while i < (self.vars.len() as i64) {
            let mut p: Rc<RefCell<SaParam>> = self.vars[i as usize].clone();
            if p.borrow().name == name {
                return p.borrow().value.clone();
            }
            i += 1;
        }
        fallback.to_string()
    }
}

#[derive(Clone)]
pub struct SaLayoutDef {
    pub uniqueId: String,
    pub title: String,
    pub root: Rc<RefCell<SaLoNode>>,
}

impl SaLayoutDef {
    pub fn new() -> Self {
        Self {
            uniqueId: String::new(),
            title: String::new(),
            root: Rc::new(RefCell::new(SaLoNode::new())),
        }
    }
    pub fn findIn(n: &Rc<RefCell<SaLoNode>>, name: &str) -> Option<Rc<RefCell<SaLoNode>>> {
        if n.borrow().kind == "forEach" && n.borrow().name == name {
            let mut hit: Option<Rc<RefCell<SaLoNode>>> = Some(n.clone());
            return hit;
        }
        let mut i: i64 = 0;
        while i < (n.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<SaLoNode>> = n.borrow().children[i as usize].clone();
            let mut f: Option<Rc<RefCell<SaLoNode>>> = SaLayoutDef::findIn(&c, name);
            if f.is_some() {
                return f;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaLoNode>>> = None;
        none
    }
    pub fn shortOf(id: &str) -> String {
        let at: i64 = PptxText::last_index_of(&id, "/");
        if at < 0 {
            return id.to_string();
        }
        id[(at + 1) as usize..(id.as_bytes().len() as i64) as usize].to_string()
    }
    /// the forEach called `name` anywhere in the program, for `ref`
    pub fn forEachNamed(&self, name: &str) -> Option<Rc<RefCell<SaLoNode>>> {
        SaLayoutDef::findIn(&self.root.clone(), name)
    }
    /// the short name: the last segment of the uniqueId
    pub fn shortId(&self) -> String {
        SaLayoutDef::shortOf(&self.uniqueId.clone())
    }
}
