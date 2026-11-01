// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaLayoutReader — `dgm:layoutDef` XML into an SaLayoutDef, or the reason not
//! ==============================================================================
//!
//! Two kinds of layout definition reach this: the ones this repository ships
//! (`builtin/`, written by hand) and the ones PowerPoint saves into every file
//! it writes. Both are read the same way, and both are held to the vocabulary
//! of the specification: an algorithm, constraint, axis or point type this
//! does not know is REFUSED by name rather than skipped. a layout that quietly
//! dropped a constraint would draw something, and that something would be
//! wrong in a way no message ever mentions.
//!
//! Elements that describe a layout rather than define it — its title, its
//! categories, the sample data PowerPoint's gallery draws, extensions — are
//! read past.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::rgr::XmlNode;
use crate::rgr::XmlParser;
use crate::smartart::sa_layout_def::SaConstr;
use crate::smartart::sa_layout_def::SaIter;
use crate::smartart::sa_layout_def::SaLayoutDef;
use crate::smartart::sa_layout_def::SaLoNode;
use crate::smartart::sa_layout_def::SaParam;
use crate::smartart::sa_layout_def::SaRule;

#[derive(Clone)]
pub struct SaLayoutReadResult {
    pub ok: bool,
    pub error: String,
    pub layout: Rc<RefCell<SaLayoutDef>>,
}

impl SaLayoutReadResult {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            layout: Rc::new(RefCell::new(SaLayoutDef::new())),
        }
    }
}

#[derive(Clone)]
pub struct SaLayoutReader {
}

impl SaLayoutReader {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn read(xml: &str) -> Rc<RefCell<SaLayoutReadResult>> {
        let mut res: Rc<RefCell<SaLayoutReadResult>> = Rc::new(RefCell::new(SaLayoutReadResult::new()));
        let mut parser: Rc<RefCell<XmlParser>> = Rc::new(RefCell::new(XmlParser::new()));
        let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&parser, xml.to_string());
        if root_opt.is_none() {
            res.borrow_mut().error = "not XML: no element could be read".to_string();
            return res;
        }
        let mut root: Rc<RefCell<XmlNode>> = root_opt.clone().unwrap();
        if root.borrow().name != "layoutDef" {
            res.borrow_mut().error = format!("the root element is <{}>, not <dgm:layoutDef>", root.borrow().name);
            return res;
        }
        res.borrow_mut().layout.borrow_mut().uniqueId = XmlNode::attr(&root, "uniqueId");
        let mut title: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&root, "title");
        if title.is_some() {
            res.borrow_mut()
                .layout.borrow_mut()
                .title = XmlNode::attr(&title.clone().unwrap(), "val");
        }
        let mut ln_opt: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&root, "layoutNode");
        if ln_opt.is_none() {
            res.borrow_mut().error = "<dgm:layoutNode> is missing: a layout needs its root node".to_string();
            return res;
        }
        let mut ln: Rc<RefCell<XmlNode>> = ln_opt.clone().unwrap();
        let mut root_node: Rc<RefCell<SaLoNode>> = Rc::new(RefCell::new(SaLoNode::new()));
        let err: String = SaLayoutReader::readBody(&ln, &root_node, "layoutNode".to_string());
        if !err.is_empty() {
            res.borrow_mut().error = err.clone();
            return res;
        }
        res.borrow_mut().layout.borrow_mut().root = root_node.clone();
        res.borrow_mut().ok = true;
        res
    }
    pub fn isOneOf(v: &str, list: &str) -> bool {
        let padded: String = format!(" {} ", list);
        PptxText::index_of(&padded, &format!(" {} ", v)) >= 0
    }
    pub fn algTypes() -> String {
        "composite conn cycle hierChild hierRoot pyra lin sp tx snake".to_string()
    }
    pub fn constrTypes() -> String {
        "none alignOff begMarg bendDist begPad b bMarg bOff ctrX ctrXOff ctrY ctrYOff connDist diam endMarg endPad h hArH hOff l lMarg lOff r rMarg rOff primFontSz pyraAcctRatio secFontSz sibSp secSibSp sp stemThick t tMarg tOff userA userB userC userD userE userF userG userH userI userJ userK userL userM userN userO userP userQ userR userS userT userU userV userW userX userY userZ w wArH wOff".to_string()
    }
    pub fn axes() -> String {
        "self ch des desOrSelf par ancst ancstOrSelf followSib precedSib follow preced root none".to_string()
    }
    pub fn ptTypes() -> String {
        "all doc node norm nonNorm asst nonAsst parTrans pres sibTrans".to_string()
    }
    pub fn varNames() -> String {
        "orgChart chMax chPref bulletEnabled dir hierBranch animOne animLvl resizeHandles".to_string()
    }
    /// "1 2 3" → words
    pub fn words(s: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cur: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = s.as_bytes().len() as i64;
        while i < n {
            let c: i64 = (s.as_bytes()[i as usize] as i64);
            if (c == 32 || c == 9) || (c == 10 || c == 13) {
                if !cur.is_empty() {
                    out.push(cur.clone());
                    cur = String::new();
                }
            } else {
                cur = format!("{}{}", cur, s[(i) as usize..(i + 1) as usize].to_string());
            }
            i += 1;
        }
        if !cur.is_empty() {
            out.push(cur.clone());
        }
        out
    }
    /// a number as the files write them. "NaN" is "not given"; "INF" is very
    /// large. `ok` false when it is neither and not a number either.
    pub fn number(s: &str) -> f64 {
        let t: String = s.trim().to_string();
        if t == "INF" || t == "+INF" {
            return 1000000000.0;
        }
        if t == "-INF" {
            return -1000000000.0;
        }
        let v: Option<f64> = t.parse::<f64>().ok();
        if v.is_some() { v.unwrap() } else { 0.0 }
    }
    pub fn isNumber(s: &str) -> bool {
        let t: String = s.trim().to_string();
        if (t == "INF" || t == "+INF") || (t == "-INF" || t == "NaN") {
            return true;
        }
        let v: Option<f64> = t.parse::<f64>().ok();
        v.is_some()
    }
    pub fn isNaN(s: &str) -> bool {
        s.trim().to_string() == "NaN"
    }
    /// The iteration attributes, as lists, each value checked.
    fn readIter(n: &Rc<RefCell<XmlNode>>, it: &Rc<RefCell<SaIter>>, r#where: &str) -> String {
        let mut axes: Vec<String> = SaLayoutReader::words(&XmlNode::attr(&n, "axis"));
        let mut i: i64 = 0;
        while i < (axes.len() as i64) {
            let a: String = axes[i as usize].clone();
            if !SaLayoutReader::isOneOf(&a, &SaLayoutReader::axes()) {
                return format!("{}: axis \"{}\" is not one of {}", r#where, a, SaLayoutReader::axes());
            }
            it.borrow_mut().axis.push(a.clone());
            i += 1;
        }
        let mut pts: Vec<String> = SaLayoutReader::words(&XmlNode::attr(&n, "ptType"));
        i = 0;
        while i < (pts.len() as i64) {
            let p: String = pts[i as usize].clone();
            if !SaLayoutReader::isOneOf(&p, &SaLayoutReader::ptTypes()) {
                return format!("{}: ptType \"{}\" is not one of {}", r#where, p, SaLayoutReader::ptTypes());
            }
            it.borrow_mut().ptType.push(p.clone());
            i += 1;
        }
        let e1: String = SaLayoutReader::readInts(n, "st", &mut it.borrow_mut().st, r#where);
        if !e1.is_empty() {
            return e1;
        }
        let e2: String = SaLayoutReader::readInts(n, "cnt", &mut it.borrow_mut().cnt, r#where);
        if !e2.is_empty() {
            return e2;
        }
        let e3: String = SaLayoutReader::readInts(n, "step", &mut it.borrow_mut().step, r#where);
        if !e3.is_empty() {
            return e3;
        }
        let mut hs: Vec<String> = SaLayoutReader::words(&XmlNode::attr(&n, "hideLastTrans"));
        i = 0;
        while i < (hs.len() as i64) {
            let h: String = hs[i as usize].clone();
            it.borrow_mut().hideLastTrans.push(h == "1" || h == "true");
            i += 1;
        }
        String::new()
    }
    fn readInts(n: &Rc<RefCell<XmlNode>>, name: &str, out: &mut Vec<i64>, r#where: &str) -> String {
        let mut ws: Vec<String> = SaLayoutReader::words(&XmlNode::attr(&n, name));
        let mut i: i64 = 0;
        while i < (ws.len() as i64) {
            let w: String = ws[i as usize].clone();
            let v: Option<i64> = w.parse::<i64>().ok();
            if v.is_none() {
                return format!("{}: {} \"{}\" is not a whole number", r#where, name, w);
            }
            out.push(v.unwrap());
            i += 1;
        }
        String::new()
    }
    /// The children of a layoutNode, forEach, if or else, into `node`.
    fn readBody(x: &Rc<RefCell<XmlNode>>, node: &Rc<RefCell<SaLoNode>>, kind: String) -> String {
        node.borrow_mut().kind = kind.clone();
        node.borrow_mut().name = XmlNode::attr(&x, "name");
        let mut r#where: String = format!("<dgm:{}>", kind);
        if !node.borrow().name.is_empty() {
            r#where = format!("<dgm:{} name=\"{}\">", kind, node.borrow().name);
        }
        if kind == "layoutNode" {
            node.borrow_mut().styleLbl = XmlNode::attr(&x, "styleLbl");
            node.borrow_mut().chOrder = XmlNode::attrOr(&x, "chOrder", "b");
            node.borrow_mut().moveWith = XmlNode::attr(&x, "moveWith");
        }
        if kind == "forEach" || kind == "if" {
            let ie: String = SaLayoutReader::readIter(x, &node.borrow().iter.clone(), &r#where);
            if !ie.is_empty() {
                return ie;
            }
        }
        if kind == "forEach" {
            node.borrow_mut().r#ref = XmlNode::attr(&x, "ref");
        }
        if kind == "if" {
            node.borrow_mut().func = XmlNode::attr(&x, "func");
            node.borrow_mut().arg = XmlNode::attrOr(&x, "arg", "none");
            node.borrow_mut().op = XmlNode::attr(&x, "op");
            node.borrow_mut().val = XmlNode::attr(&x, "val");
            if !SaLayoutReader::isOneOf(&node.borrow().func.clone(), "cnt pos revPos posEven posOdd var depth maxDepth") {
                return format!("{}: func \"{}\" is not one of cnt, pos, revPos, posEven, posOdd, var, depth, maxDepth", r#where, node.borrow().func);
            }
            if !SaLayoutReader::isOneOf(&node.borrow().op.clone(), "equ neq gt lt gte lte") {
                return format!("{}: op \"{}\" is not one of equ, neq, gt, lt, gte, lte", r#where, node.borrow().op);
            }
        }
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&x) {
            let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&x, i);
            if !c.borrow().isText {
                let err: String = SaLayoutReader::readChild(&c, node, &r#where);
                if !err.is_empty() {
                    return err;
                }
            }
            i += 1;
        }
        String::new()
    }
    fn readChild(c: &Rc<RefCell<XmlNode>>, node: &Rc<RefCell<SaLoNode>>, r#where: &str) -> String {
        let nm: String = c.borrow().name.clone();
        if nm == "extLst" {
            return String::new();
        }
        if nm == "layoutNode" {
            let mut k: Rc<RefCell<SaLoNode>> = Rc::new(RefCell::new(SaLoNode::new()));
            node.borrow_mut().children.push(k.clone());
            return SaLayoutReader::readBody(c, &k, "layoutNode".to_string());
        }
        if nm == "forEach" {
            let mut f: Rc<RefCell<SaLoNode>> = Rc::new(RefCell::new(SaLoNode::new()));
            node.borrow_mut().children.push(f.clone());
            return SaLayoutReader::readBody(c, &f, "forEach".to_string());
        }
        if nm == "choose" {
            let mut ch: Rc<RefCell<SaLoNode>> = Rc::new(RefCell::new(SaLoNode::new()));
            ch.borrow_mut().kind = "choose".to_string();
            ch.borrow_mut().name = XmlNode::attr(&c, "name");
            node.borrow_mut().children.push(ch.clone());
            let mut j: i64 = 0;
            while j < XmlNode::childCount(&c) {
                let mut b: Rc<RefCell<XmlNode>> = XmlNode::childAt(&c, j);
                if !b.borrow().isText {
                    if b.borrow().name == "if" || b.borrow().name == "else" {
                        let mut br: Rc<RefCell<SaLoNode>> = Rc::new(RefCell::new(SaLoNode::new()));
                        ch.borrow_mut().children.push(br.clone());
                        let be: String = SaLayoutReader::readBody(&b, &br, b.borrow().name.clone());
                        if !be.is_empty() {
                            return be;
                        }
                    } else {
                        return format!("<dgm:choose>: <dgm:{}> is not if or else", b.borrow().name);
                    }
                }
                j += 1;
            }
            return String::new();
        }
        if nm == "alg" {
            node.borrow_mut().hasAlg = true;
            node.borrow_mut().alg.borrow_mut().r#type = XmlNode::attr(&c, "type");
            let mut _tmp_165 = node.borrow().alg.borrow().r#type.clone();
            if !SaLayoutReader::isOneOf(&_tmp_165, &SaLayoutReader::algTypes()) {
                return format!("{}: algorithm \"{}\" is not one of {}", r#where, node.borrow().alg.borrow().r#type, SaLayoutReader::algTypes());
            }
            let mut ps: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&c, "param");
            let mut k2: i64 = 0;
            while k2 < (ps.len() as i64) {
                let mut pn: Rc<RefCell<XmlNode>> = ps[k2 as usize].clone();
                let mut prm: Rc<RefCell<SaParam>> = Rc::new(RefCell::new(SaParam::new()));
                prm.borrow_mut().name = XmlNode::attr(&pn, "type");
                prm.borrow_mut().value = XmlNode::attr(&pn, "val");
                node.borrow_mut().alg.borrow_mut().params.push(prm.clone());
                k2 += 1;
            }
            return String::new();
        }
        if nm == "shape" {
            node.borrow_mut().hasShape = true;
            node.borrow_mut().shape.borrow_mut().r#type = XmlNode::attrOr(&c, "type", "none");
            node.borrow_mut().shape.borrow_mut().rot = SaLayoutReader::number(&XmlNode::attrOr(&c, "rot", "0"));
            let zo: Option<i64> = XmlNode::attrOr(&c, "zOrderOff", "0")
                .trim()
                .to_string()
                .parse::<i64>()
                .ok();
            node.borrow_mut().shape.borrow_mut().zOrderOff = if zo.is_some() { zo.unwrap() } else { 0 };
            node.borrow_mut().shape.borrow_mut().hideGeom = XmlNode::attr(&c, "hideGeom") == "1";
            node.borrow_mut().shape.borrow_mut().lkTxEntry = XmlNode::attr(&c, "lkTxEntry") == "1";
            let mut adj_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&c, "adjLst");
            if adj_lst.is_some() {
                let mut adjs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&adj_lst.clone().unwrap(), "adj");
                let mut k3: i64 = 0;
                while k3 < (adjs.len() as i64) {
                    let mut an: Rc<RefCell<XmlNode>> = adjs[k3 as usize].clone();
                    let idx: Option<i64> = XmlNode::attr(&an, "idx").trim().to_string().parse::<i64>().ok();
                    if idx.is_none() || idx.unwrap() < 1 {
                        return format!("{}: an adj's idx \"{}\" is not a number from 1", r#where, XmlNode::attr(&an, "idx"));
                    }
                    node.borrow().shape.clone().borrow_mut().setAdj(idx.unwrap(), SaLayoutReader::number(&XmlNode::attr(&an, "val")));
                    k3 += 1;
                }
            }
            return String::new();
        }
        if nm == "presOf" {
            node.borrow_mut().hasPresOf = true;
            return SaLayoutReader::readIter(c, &node.borrow().presOf.clone(), &format!("{} <dgm:presOf>", r#where));
        }
        if nm == "constrLst" {
            let mut cs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&c, "constr");
            let mut k4: i64 = 0;
            while k4 < (cs.len() as i64) {
                let mut cn: Rc<RefCell<XmlNode>> = cs[k4 as usize].clone();
                let ce: String = SaLayoutReader::readConstr(&cn, node, r#where);
                if !ce.is_empty() {
                    return ce;
                }
                k4 += 1;
            }
            return String::new();
        }
        if nm == "ruleLst" {
            let mut rs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&c, "rule");
            let mut k5: i64 = 0;
            while k5 < (rs.len() as i64) {
                let mut rn: Rc<RefCell<XmlNode>> = rs[k5 as usize].clone();
                let re: String = SaLayoutReader::readRule(&rn, node, r#where);
                if !re.is_empty() {
                    return re;
                }
                k5 += 1;
            }
            return String::new();
        }
        if nm == "varLst" {
            let mut k6: i64 = 0;
            while k6 < XmlNode::childCount(&c) {
                let mut vn: Rc<RefCell<XmlNode>> = XmlNode::childAt(&c, k6);
                if !vn.borrow().isText {
                    let mut _tmp_176 = vn.borrow().name.clone();
                    if !SaLayoutReader::isOneOf(&_tmp_176, &SaLayoutReader::varNames()) {
                        return format!("{}: variable <dgm:{}> is not one of {}", r#where, vn.borrow().name, SaLayoutReader::varNames());
                    }
                    let mut v: Rc<RefCell<SaParam>> = Rc::new(RefCell::new(SaParam::new()));
                    v.borrow_mut().name = vn.borrow().name.clone();
                    let _tmp_179 = SaLayoutReader::varDefault(&vn.borrow().name);
                    v.borrow_mut().value = XmlNode::attrOr(&vn, "val", &_tmp_179);
                    node.borrow_mut().vars.push(v.clone());
                }
                k6 += 1;
            }
            return String::new();
        }
        format!("{}: <dgm:{}> is not part of a layout definition", r#where, nm)
    }
    /// what a variable is when it is named with no val
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
        if name == "animOne" {
            return "one".to_string();
        }
        if name == "animLvl" {
            return "none".to_string();
        }
        if name == "resizeHandles" {
            return "rel".to_string();
        }
        String::new()
    }
    fn readConstr(cn: &Rc<RefCell<XmlNode>>, node: &Rc<RefCell<SaLoNode>>, r#where: &str) -> String {
        let mut k: Rc<RefCell<SaConstr>> = Rc::new(RefCell::new(SaConstr::new()));
        k.borrow_mut().r#type = XmlNode::attrOr(&cn, "type", "none");
        let w: String = format!("{} <dgm:constr type=\"{}\">", r#where, k.borrow().r#type);
        let mut _tmp_183 = k.borrow().r#type.clone();
        if !SaLayoutReader::isOneOf(&_tmp_183, &SaLayoutReader::constrTypes()) {
            return format!("{}: constraint type \"{}\" is not one the specification defines", r#where, k.borrow().r#type);
        }
        k.borrow_mut().forRel = XmlNode::attrOr(&cn, "for", "self");
        if !SaLayoutReader::isOneOf(&k.borrow().forRel.clone(), "self ch des") {
            return format!("{}: for \"{}\" is not self, ch or des", w, k.borrow().forRel);
        }
        k.borrow_mut().forName = XmlNode::attr(&cn, "forName");
        k.borrow_mut().ptType = XmlNode::attrOr(&cn, "ptType", "all");
        k.borrow_mut().refType = XmlNode::attrOr(&cn, "refType", "none");
        let mut _tmp_190 = k.borrow().refType.clone();
        if !SaLayoutReader::isOneOf(&_tmp_190, &SaLayoutReader::constrTypes()) {
            return format!("{}: refType \"{}\" is not a constraint type", w, k.borrow().refType);
        }
        k.borrow_mut().refFor = XmlNode::attrOr(&cn, "refFor", "self");
        if !SaLayoutReader::isOneOf(&k.borrow().refFor.clone(), "self ch des") {
            return format!("{}: refFor \"{}\" is not self, ch or des", w, k.borrow().refFor);
        }
        k.borrow_mut().refForName = XmlNode::attr(&cn, "refForName");
        k.borrow_mut().refPtType = XmlNode::attrOr(&cn, "refPtType", "all");
        k.borrow_mut().op = XmlNode::attrOr(&cn, "op", "none");
        if !SaLayoutReader::isOneOf(&k.borrow().op.clone(), "none equ gte lte") {
            return format!("{}: op \"{}\" is not none, equ, gte or lte", w, k.borrow().op);
        }
        if XmlNode::hasAttr(&cn, "val") {
            if !SaLayoutReader::isNumber(&XmlNode::attr(&cn, "val")) {
                return format!("{}: val \"{}\" is not a number", w, XmlNode::attr(&cn, "val"));
            }
            if !SaLayoutReader::isNaN(&XmlNode::attr(&cn, "val")) {
                k.borrow_mut().val = SaLayoutReader::number(&XmlNode::attr(&cn, "val"));
                k.borrow_mut().hasVal = true;
            }
        }
        if XmlNode::hasAttr(&cn, "fact") {
            if !SaLayoutReader::isNumber(&XmlNode::attr(&cn, "fact")) {
                return format!("{}: fact \"{}\" is not a number", w, XmlNode::attr(&cn, "fact"));
            }
            if !SaLayoutReader::isNaN(&XmlNode::attr(&cn, "fact")) {
                k.borrow_mut().fact = SaLayoutReader::number(&XmlNode::attr(&cn, "fact"));
                k.borrow_mut().hasFact = true;
            }
        }
        node.borrow_mut().constrs.push(k.clone());
        String::new()
    }
    fn readRule(rn: &Rc<RefCell<XmlNode>>, node: &Rc<RefCell<SaLoNode>>, r#where: &str) -> String {
        let mut r: Rc<RefCell<SaRule>> = Rc::new(RefCell::new(SaRule::new()));
        r.borrow_mut().r#type = XmlNode::attrOr(&rn, "type", "none");
        let w: String = format!("{} <dgm:rule type=\"{}\">", r#where, r.borrow().r#type);
        let mut _tmp_201 = r.borrow().r#type.clone();
        if !SaLayoutReader::isOneOf(&_tmp_201, &SaLayoutReader::constrTypes()) {
            return format!("{}: rule type \"{}\" is not a constraint type", r#where, r.borrow().r#type);
        }
        r.borrow_mut().forRel = XmlNode::attrOr(&rn, "for", "self");
        if !SaLayoutReader::isOneOf(&r.borrow().forRel.clone(), "self ch des") {
            return format!("{}: for \"{}\" is not self, ch or des", w, r.borrow().forRel);
        }
        r.borrow_mut().forName = XmlNode::attr(&rn, "forName");
        r.borrow_mut().ptType = XmlNode::attrOr(&rn, "ptType", "all");
        let mut names: Vec<String> = vec!["val".to_string(), "fact".to_string(), "max".to_string()];
        let mut i: i64 = 0;
        while i < 3 {
            let a: String = names[i as usize].clone();
            if XmlNode::hasAttr(&rn, &a) {
                let s: String = XmlNode::attr(&rn, &a);
                if !SaLayoutReader::isNumber(&s) {
                    return format!("{}: {} \"{}\" is not a number", w, a, s);
                }
                if !SaLayoutReader::isNaN(&s) {
                    let v: f64 = SaLayoutReader::number(&s);
                    if a == "val" {
                        r.borrow_mut().val = v;
                        r.borrow_mut().hasVal = true;
                    }
                    if a == "fact" {
                        r.borrow_mut().fact = v;
                        r.borrow_mut().hasFact = true;
                    }
                    if a == "max" {
                        r.borrow_mut().max = v;
                        r.borrow_mut().hasMax = true;
                    }
                }
            }
            i += 1;
        }
        node.borrow_mut().rules.push(r.clone());
        String::new()
    }
}
