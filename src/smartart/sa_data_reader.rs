// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaDataReader — `dgm:dataModel` XML into an SaModel, or the reason it is not
//! ==============================================================================
//!
//! Two kinds of file arrive here and both are read as they are. One PowerPoint
//! saved: every point and connection, model ids that are GUIDs, presentation
//! points. One an assistant wrote: a document point, the nodes and the parent-of
//! connections, nothing else. The schema's defaults are what make the second
//! one short — a point is a `node` and a connection a `parOf` unless they say
//! otherwise — and this reads them.
//!
//! What it does not do is guess. A file that cannot be a diagram is REFUSED,
//! and the refusal names the element and says why, because the reader of the
//! message is usually a model that will write the file again:
//!
//!   point 3: type "nod" is not one of doc, node, asst, parTrans, sibTrans, pres
//!   connection 7: destId "9" names no point
//!   point 4 has two parents (1 and 2)
//!
//! A refusal is a value (`SaReadResult.error`), never an exception: the editor
//! draws it in the diagram's place and the MCP server returns it to the caller.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::rgr::XmlNode;
use crate::rgr::XmlParser;
use crate::smartart::sa_model::SaCxn;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPara;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_model::SaRun;

#[derive(Clone)]
pub struct SaReadResult {
    pub ok: bool,
    pub error: String,
    pub model: Rc<RefCell<SaModel>>,
}

impl SaReadResult {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            model: Rc::new(RefCell::new(SaModel::new())),
        }
    }
}

#[derive(Clone)]
pub struct SaDataReader {
}

impl SaDataReader {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn read(xml: &str) -> Rc<RefCell<SaReadResult>> {
        let mut res: Rc<RefCell<SaReadResult>> = Rc::new(RefCell::new(SaReadResult::new()));
        let mut parser: Rc<RefCell<XmlParser>> = Rc::new(RefCell::new(XmlParser::new()));
        let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&parser, xml.to_string());
        if root_opt.is_none() {
            res.borrow_mut().error = "not XML: no element could be read".to_string();
            return res;
        }
        let mut root: Rc<RefCell<XmlNode>> = root_opt.clone().unwrap();
        if parser.borrow().aborted {
            res.borrow_mut().error = "the file is too large or too deeply nested to be a diagram".to_string();
            return res;
        }
        if root.borrow().name != "dataModel" {
            res.borrow_mut().error = format!("the root element is <{}>, not <dgm:dataModel>", root.borrow().name);
            return res;
        }
        let mut model: Rc<RefCell<SaModel>> = res.borrow().model.clone().clone();
        let mut pt_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&root, "ptLst");
        if pt_lst.is_none() {
            res.borrow_mut().error = "<dgm:ptLst> is missing: a diagram needs at least its document point".to_string();
            return res;
        }
        let mut pts: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&pt_lst.clone().unwrap(), "pt");
        let mut i: i64 = 0;
        while i < (pts.len() as i64) {
            let mut pn: Rc<RefCell<XmlNode>> = pts[i as usize].clone();
            let err: String = SaDataReader::readPoint(&pn, &model, i + 1);
            if !err.is_empty() {
                res.borrow_mut().error = err.clone();
                return res;
            }
            i += 1;
        }
        let mut cxn_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&root, "cxnLst");
        if cxn_lst.is_some() {
            let mut cs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&cxn_lst.clone().unwrap(), "cxn");
            let mut j: i64 = 0;
            while j < (cs.len() as i64) {
                let mut cn: Rc<RefCell<XmlNode>> = cs[j as usize].clone();
                let cerr: String = SaDataReader::readCxn(&cn, &model, j + 1);
                if !cerr.is_empty() {
                    res.borrow_mut().error = cerr.clone();
                    return res;
                }
                j += 1;
            }
        }
        SaDataReader::readDrawingRel(&root, &model);
        let serr: String = SaDataReader::checkShape(&model);
        if !serr.is_empty() {
            res.borrow_mut().error = serr.clone();
            return res;
        }
        res.borrow_mut().ok = true;
        res
    }
    pub fn validType(t: &str) -> bool {
        if t == "doc" {
            return true;
        }
        if t == "node" {
            return true;
        }
        if t == "asst" {
            return true;
        }
        if t == "parTrans" {
            return true;
        }
        if t == "sibTrans" {
            return true;
        }
        if t == "pres" {
            return true;
        }
        false
    }
    pub fn validCxnType(t: &str) -> bool {
        if t == "parOf" {
            return true;
        }
        if t == "presOf" {
            return true;
        }
        if t == "presParOf" {
            return true;
        }
        false
    }
    /// An attribute that must be an integer when it is there. `fallback` when it
    /// is not; `ok` false when it is there and is not a number.
    fn intAttr(n: &Rc<RefCell<XmlNode>>, name: &str, fallback: i64) -> i64 {
        let s: String = XmlNode::attr(&n, name).trim().to_string();
        if s.is_empty() {
            return fallback;
        }
        let v: Option<i64> = s.parse::<i64>().ok();
        if v.is_some() { v.unwrap() } else { fallback }
    }
    fn badInt(n: &Rc<RefCell<XmlNode>>, name: &str) -> bool {
        let s: String = XmlNode::attr(&n, name).trim().to_string();
        if s.is_empty() {
            return false;
        }
        let v: Option<i64> = s.parse::<i64>().ok();
        v.is_none()
    }
    fn readPoint(pn: &Rc<RefCell<XmlNode>>, model: &Rc<RefCell<SaModel>>, ordinal: i64) -> String {
        let mut r#where: String = format!("point {}", ordinal);
        if !XmlNode::hasAttr(&pn, "modelId") {
            return format!("{}: modelId is missing", r#where);
        }
        let id: String = XmlNode::attr(&pn, "modelId").trim().to_string();
        if id.is_empty() {
            return format!("{}: modelId is empty", r#where);
        }
        r#where = format!("point {}", id);
        if model.borrow().has(&id) {
            return format!("{}: modelId is used twice", r#where);
        }
        let mut p: Rc<RefCell<SaPoint>> = Rc::new(RefCell::new(SaPoint::new()));
        p.borrow_mut().id = id.clone();
        let t: String = XmlNode::attrOr(&pn, "type", "node");
        if !SaDataReader::validType(&t) {
            return format!("{}: type \"{}\" is not one of doc, node, asst, parTrans, sibTrans, pres", r#where, t);
        }
        p.borrow_mut().r#type = t.clone();
        p.borrow_mut().cxnId = XmlNode::attr(&pn, "cxnId");
        if (t == "parTrans" || t == "sibTrans") && p.borrow().cxnId.is_empty() {
            return format!("{}: a {} point needs cxnId, the connection it belongs to", r#where, t);
        }
        let mut pr_set: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pn, "prSet");
        if pr_set.is_some() {
            let mut ps: Rc<RefCell<XmlNode>> = pr_set.clone().unwrap();
            p.borrow_mut().layoutId = XmlNode::attr(&ps, "loTypeId");
            p.borrow_mut().styleId = XmlNode::attr(&ps, "qsTypeId");
            p.borrow_mut().colorsId = XmlNode::attr(&ps, "csTypeId");
            p.borrow_mut().presName = XmlNode::attr(&ps, "presName");
            p.borrow_mut().presStyleLbl = XmlNode::attr(&ps, "presStyleLbl");
            p.borrow_mut().presAssocId = XmlNode::attr(&ps, "presAssocID");
            if SaDataReader::badInt(&ps, "presStyleIdx") {
                return format!("{}: presStyleIdx is not a whole number", r#where);
            }
            if SaDataReader::badInt(&ps, "presStyleCnt") {
                return format!("{}: presStyleCnt is not a whole number", r#where);
            }
            p.borrow_mut().presStyleIdx = SaDataReader::intAttr(&ps, "presStyleIdx", -1);
            p.borrow_mut().presStyleCnt = SaDataReader::intAttr(&ps, "presStyleCnt", -1);
        }
        let mut t_node: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pn, "t");
        if t_node.is_some() {
            p.borrow_mut().hasText = true;
            SaDataReader::readText(&t_node.clone().unwrap(), &p);
        }
        model.borrow_mut().points.push(p.clone());
        String::new()
    }
    /// `dgm:t` is a text body: `a:p` paragraphs of `a:r` runs. Line breaks
    /// (`a:br`) end a run; fields (`a:fld`) are read as their text.
    fn readText(t: &Rc<RefCell<XmlNode>>, p: &Rc<RefCell<SaPoint>>) {
        let mut paras: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&t, "p");
        let mut i: i64 = 0;
        while i < (paras.len() as i64) {
            let mut pn: Rc<RefCell<XmlNode>> = paras[i as usize].clone();
            let mut para: Rc<RefCell<SaPara>> = Rc::new(RefCell::new(SaPara::new()));
            let mut p_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pn, "pPr");
            if p_pr.is_some() {
                para.borrow_mut().level = SaDataReader::intAttr(&p_pr.clone().unwrap(), "lvl", 0);
            }
            let mut k: i64 = 0;
            while k < XmlNode::childCount(&pn) {
                let mut rn: Rc<RefCell<XmlNode>> = XmlNode::childAt(&pn, k);
                if rn.borrow().name == "r" || rn.borrow().name == "fld" {
                    para.borrow_mut().runs.push(SaDataReader::readRun(&rn));
                }
                if rn.borrow().name == "br" {
                    let mut br: Rc<RefCell<SaRun>> = Rc::new(RefCell::new(SaRun::new()));
                    br.borrow_mut().text = "\n".to_string();
                    para.borrow_mut().runs.push(br.clone());
                }
                k += 1;
            }
            p.borrow_mut().paras.push(para.clone());
            i += 1;
        }
    }
    fn readRun(rn: &Rc<RefCell<XmlNode>>) -> Rc<RefCell<SaRun>> {
        let mut run: Rc<RefCell<SaRun>> = Rc::new(RefCell::new(SaRun::new()));
        let mut tx: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rn, "t");
        if tx.is_some() {
            run.borrow_mut().text = XmlNode::textContent(&tx.clone().unwrap());
        }
        let mut r_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rn, "rPr");
        if r_pr.is_some() {
            let mut rp: Rc<RefCell<XmlNode>> = r_pr.clone().unwrap();
            run.borrow_mut().bold = XmlNode::attr(&rp, "b") == "1" || XmlNode::attr(&rp, "b") == "true";
            run.borrow_mut().italic = XmlNode::attr(&rp, "i") == "1" || XmlNode::attr(&rp, "i") == "true";
            let u: String = XmlNode::attr(&rp, "u");
            run.borrow_mut().underline = !u.is_empty() && u != "none";
            let sz: String = XmlNode::attr(&rp, "sz").trim().to_string();
            if !sz.is_empty() {
                let szv: Option<i64> = sz.parse::<i64>().ok();
                run.borrow_mut().sizePt = ((if szv.is_some() { szv.unwrap() } else { 0 }) as f64) / 100.0;
            }
            let mut fill: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rp, "solidFill");
            if fill.is_some() {
                let mut rgb: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&fill.clone().unwrap(), "srgbClr");
                if rgb.is_some() {
                    run.borrow_mut().color = XmlNode::attr(&rgb.clone().unwrap(), "val").to_uppercase();
                }
            }
        }
        run
    }
    fn readCxn(cn: &Rc<RefCell<XmlNode>>, model: &Rc<RefCell<SaModel>>, ordinal: i64) -> String {
        let mut c: Rc<RefCell<SaCxn>> = Rc::new(RefCell::new(SaCxn::new()));
        c.borrow_mut().id = XmlNode::attr(&cn, "modelId").trim().to_string();
        let mut r#where: String = format!("connection {}", ordinal);
        if !c.borrow().id.is_empty() {
            r#where = format!("connection {}", c.borrow().id);
            if model.borrow().cxnTaken(&c.borrow().id.clone()) {
                return format!("{}: modelId is used twice", r#where);
            }
            if model.borrow().has(&c.borrow().id) {
                return format!("{}: modelId is also a point's", r#where);
            }
        }
        let t: String = XmlNode::attrOr(&cn, "type", "parOf");
        if !SaDataReader::validCxnType(&t) {
            return format!("{}: type \"{}\" is not one of parOf, presOf, presParOf", r#where, t);
        }
        c.borrow_mut().r#type = t.clone();
        c.borrow_mut().srcId = XmlNode::attr(&cn, "srcId").trim().to_string();
        c.borrow_mut().destId = XmlNode::attr(&cn, "destId").trim().to_string();
        if c.borrow().srcId.is_empty() {
            return format!("{}: srcId is missing", r#where);
        }
        if c.borrow().destId.is_empty() {
            return format!("{}: destId is missing", r#where);
        }
        if !model.borrow().has(&c.borrow().srcId) {
            return format!("{}: srcId \"{}\" names no point", r#where, c.borrow().srcId);
        }
        if !model.borrow().has(&c.borrow().destId) {
            return format!("{}: destId \"{}\" names no point", r#where, c.borrow().destId);
        }
        if c.borrow().srcId == c.borrow().destId {
            return format!("{}: a point cannot be connected to itself", r#where);
        }
        if SaDataReader::badInt(cn, "srcOrd") {
            return format!("{}: srcOrd is not a whole number", r#where);
        }
        if SaDataReader::badInt(cn, "destOrd") {
            return format!("{}: destOrd is not a whole number", r#where);
        }
        // Left out, the order is the order of the file: the n-th connection
        // out of a point is its n-th child.
        let mut given: i64 = SaDataReader::intAttr(cn, "srcOrd", -1);
        if given < 0 {
            given = model.borrow().childCxns(&c.borrow().srcId).len() as i64;
        }
        c.borrow_mut().srcOrd = given;
        c.borrow_mut().destOrd = SaDataReader::intAttr(cn, "destOrd", 0);
        c.borrow_mut().parTransId = XmlNode::attr(&cn, "parTransId").trim().to_string();
        c.borrow_mut().sibTransId = XmlNode::attr(&cn, "sibTransId").trim().to_string();
        c.borrow_mut().presId = XmlNode::attr(&cn, "presId");
        model.borrow_mut().cxns.push(c.clone());
        String::new()
    }
    fn readDrawingRel(root: &Rc<RefCell<XmlNode>>, model: &Rc<RefCell<SaModel>>) {
        let mut ext: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&root, "dataModelExt");
        if ext.is_some() {
            let mut e: Rc<RefCell<XmlNode>> = ext.clone().unwrap();
            model.borrow_mut().drawingRelId = XmlNode::attr(&e, "relId");
        }
    }
    /// The graph as a whole: one document, every content point reachable from
    /// it by parent-of connections, none with two parents, and parent-of only
    /// between the points that can have it.
    pub fn checkShape(model: &Rc<RefCell<SaModel>>) -> String {
        let docs: i64 = model.borrow().countOf("doc");
        if docs == 0 {
            return "there is no document point (a <dgm:pt type=\"doc\">): the diagram has nothing to hang from".to_string();
        }
        if docs > 1 {
            return format!("there are {} document points; a diagram has one", docs);
        }
        let mut i: i64 = 0;
        while i < (model.borrow().cxns.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = model.borrow().cxns[i as usize].clone();
            if c.borrow().r#type == "parOf" {
                let mut s: Rc<RefCell<SaPoint>> = model.borrow().pointById(&c.borrow().srcId.clone()).unwrap();
                let mut d: Rc<RefCell<SaPoint>> = model.borrow().pointById(&c.borrow().destId.clone()).unwrap();
                if s.borrow().r#type != "doc" && !s.borrow().isContent() {
                    return format!("connection {}: a {} point cannot be a parent", c.borrow().id, s.borrow().r#type);
                }
                if !d.borrow().isContent() {
                    return format!("connection {}: a {} point cannot be a child", c.borrow().id, d.borrow().r#type);
                }
            }
            i += 1;
        }
        // two parents
        i = 0;
        while i < (model.borrow().points.len() as i64) {
            let mut p: Rc<RefCell<SaPoint>> = model.borrow().points[i as usize].clone();
            let mut parents: Vec<String> = Vec::new();
            let mut k: i64 = 0;
            while k < (model.borrow().cxns.len() as i64) {
                let mut c2: Rc<RefCell<SaCxn>> = model.borrow().cxns[k as usize].clone();
                if c2.borrow().r#type == "parOf" && c2.borrow().destId == p.borrow().id {
                    parents.push(c2.borrow().srcId.clone());
                }
                k += 1;
            }
            if (parents.len() as i64) > 1 {
                return format!("point {} has two parents ({} and {})", p.borrow().id, parents[0].clone(), parents[1].clone());
            }
            i += 1;
        }
        // reachable from the document, which also rules out a cycle: a point
        // on a cycle has its one parent on the cycle and never meets the root
        let mut d0: Rc<RefCell<SaPoint>> = model.borrow().doc();
        i = 0;
        while i < (model.borrow().points.len() as i64) {
            let mut q: Rc<RefCell<SaPoint>> = model.borrow().points[i as usize].clone();
            if q.borrow().isContent() {
                let mut _tmp_147 = q.borrow().id.clone();
                if !SaDataReader::reaches(model, &_tmp_147, &d0.borrow().id.clone()) {
                    return format!("point {} is not under the document: connect it with a parOf from the document or from another node", q.borrow().id);
                }
            }
            i += 1;
        }
        // transitions name a connection that exists
        i = 0;
        while i < (model.borrow().points.len() as i64) {
            let mut tp: Rc<RefCell<SaPoint>> = model.borrow().points[i as usize].clone();
            if tp.borrow().r#type == "parTrans" || tp.borrow().r#type == "sibTrans" {
                if !model.borrow().cxnTaken(&tp.borrow().cxnId.clone()) {
                    return format!("point {}: cxnId \"{}\" names no connection", tp.borrow().id, tp.borrow().cxnId);
                }
            }
            i += 1;
        }
        String::new()
    }
    pub fn reaches(model: &Rc<RefCell<SaModel>>, from: &str, root: &str) -> bool {
        let mut at: String = from.to_string();
        let mut steps: i64 = 0;
        while steps <= (model.borrow().points.len() as i64) {
            if at == root {
                return true;
            }
            let mut c: Option<Rc<RefCell<SaCxn>>> = model.borrow().parentCxn(&at);
            if c.is_none() {
                return false;
            }
            at = c.as_ref().unwrap().borrow().srcId.clone();
            steps += 1;
        }
        false
    }
}
