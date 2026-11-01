// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaQuickStyle — quick styles (`dgm:styleDef`), read, built in, and written
//! ==============================================================================
//!
//! A quick style says, for each style label, which of the theme's line, fill
//! and effect styles a shape takes (`lnRef`, `fillRef`, `effectRef`, by index:
//! 0 none, 1 subtle, 2 moderate, 3 intense) and which font (`fontRef`). The
//! colour set says the colours; this says how they are applied. 3-D (the
//! `scene3d`, `sp3d` a style may hold) is not drawn here.
//!
//! Drawn as:
//!
//!   lnRef     0 no line, 1 ½ pt, 2 1 pt, 3 2¼ pt
//!   fillRef   0 no fill, 1 … 3 filled
//!   effectRef 0 none; 1 … 3 an outer shadow, a little to a lot
//!
//! Built in, by the last segment of their ids:
//!
//!   simple1   Simple Fill: filled, a 1 pt line (the default)
//!   simple2   White Outline: filled, a 2¼ pt line
//!   simple3   Subtle Effect: simple1 with a small shadow
//!   simple4   Moderate Effect: with a larger one
//!   simple5   Intense Effect: with the largest
//!
//! A file's own quick style (a whole diagram as one file) is read with `read`.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::rgr::XmlNode;
use crate::rgr::XmlParser;
use crate::smartart::sa_colors::SaColors;

#[derive(Clone)]
pub struct SaQsLbl {
    pub name: String,
    pub ln: i64,
    pub fill: i64,
    pub effect: i64,
    pub font: String,
}

impl SaQsLbl {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            ln: 2,
            fill: 1,
            effect: 0,
            font: "minor".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct SaQuickStyleDef {
    pub uniqueId: String,
    pub labels: Vec<Rc<RefCell<SaQsLbl>>>,
}

impl SaQuickStyleDef {
    pub fn new() -> Self {
        Self {
            uniqueId: String::new(),
            labels: Vec::new(),
        }
    }
    /// the label's entry, or one of its family, or the defaults
    pub fn label(&self, name: &str) -> Rc<RefCell<SaQsLbl>> {
        let mut i: i64 = 0;
        while i < (self.labels.len() as i64) {
            let mut l: Rc<RefCell<SaQsLbl>> = self.labels[i as usize].clone();
            if l.borrow().name == name {
                return l;
            }
            i += 1;
        }
        let fam: String = SaColors::family(name);
        i = 0;
        while i < (self.labels.len() as i64) {
            let mut l2: Rc<RefCell<SaQsLbl>> = self.labels[i as usize].clone();
            if SaColors::family(&l2.borrow().name.clone()) == fam {
                return l2;
            }
            i += 1;
        }
        SaQuickStyle::familyDefault(name.to_string(), 0)
    }
}

#[derive(Clone)]
pub struct SaQuickStyleRead {
    pub ok: bool,
    pub error: String,
    pub def: Rc<RefCell<SaQuickStyleDef>>,
}

impl SaQuickStyleRead {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            def: Rc::new(RefCell::new(SaQuickStyleDef::new())),
        }
    }
}

#[derive(Clone)]
pub struct SaQuickStyle {
}

impl SaQuickStyle {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn fullId(s: &str) -> String {
        if PptxText::index_of(&s, ":") >= 0 {
            return s.to_string();
        }
        format!("urn:microsoft.com/office/officeart/2005/8/quickstyle/{}", s)
    }
    pub fn names() -> String {
        "simple1 … simple5".to_string()
    }
    /// line widths by lnRef, in points
    pub fn lineWidth(idx: i64) -> f64 {
        if idx <= 0 {
            return 0.0;
        }
        if idx == 1 {
            return 0.5;
        }
        if idx == 2 {
            return 1.0;
        }
        2.25
    }
    /// the defaults for a label of this family, with `effect` as the style's
    pub fn familyDefault(name: String, effect: i64) -> Rc<RefCell<SaQsLbl>> {
        let mut l: Rc<RefCell<SaQsLbl>> = Rc::new(RefCell::new(SaQsLbl::new()));
        l.borrow_mut().name = name.clone();
        let fam: String = SaColors::family(&name);
        l.borrow_mut().effect = effect;
        if fam == "node" {
            l.borrow_mut().ln = 2;
            l.borrow_mut().fill = 1;
        }
        if fam == "arrow" {
            l.borrow_mut().ln = 0;
            l.borrow_mut().fill = 1;
        }
        if fam == "line" {
            l.borrow_mut().ln = 2;
            l.borrow_mut().fill = 0;
            l.borrow_mut().effect = 0;
            l.borrow_mut().font = "minor".to_string();
        }
        if fam == "light" || (fam == "outlined" || fam == "dark") {
            l.borrow_mut().ln = 2;
            l.borrow_mut().fill = 1;
            l.borrow_mut().effect = 0;
        }
        if fam == "text" {
            l.borrow_mut().ln = 0;
            l.borrow_mut().fill = 0;
            l.borrow_mut().effect = 0;
        }
        l
    }
    pub fn known(id: &str) -> bool {
        let s: String = SaColors::short(id);
        if s.is_empty() {
            return true;
        }
        let mut d: Rc<RefCell<SaQuickStyleDef>> = SaQuickStyle::builtin(&s);
        return !d.borrow().labels.is_empty();
    }
    /// the quick style called `short`; one with no labels for a name that is
    /// not built in
    pub fn builtin(short: &str) -> Rc<RefCell<SaQuickStyleDef>> {
        let mut d: Rc<RefCell<SaQuickStyleDef>> = Rc::new(RefCell::new(SaQuickStyleDef::new()));
        let ok: bool = ((short == "simple1" || short == "simple2") || (short == "simple3" || short == "simple4")) || short == "simple5";
        if !ok {
            return d;
        }
        d.borrow_mut().uniqueId = SaQuickStyle::fullId(short);
        let mut effect: i64 = 0;
        if short == "simple3" {
            effect = 1;
        }
        if short == "simple4" {
            effect = 2;
        }
        if short == "simple5" {
            effect = 3;
        }
        let mut ls: Vec<String> = SaColors::labels();
        let mut i: i64 = 0;
        while i < (ls.len() as i64) {
            let mut l: Rc<RefCell<SaQsLbl>> = SaQuickStyle::familyDefault(ls[i as usize].clone(), effect);
            if short == "simple2" && l.borrow().ln == 2 {
                l.borrow_mut().ln = 3;
            }
            d.borrow_mut().labels.push(l.clone());
            i += 1;
        }
        d
    }
    /// --- reading a file's own style -------------------------------------------
    pub fn read(xml: &str) -> Rc<RefCell<SaQuickStyleRead>> {
        let mut res: Rc<RefCell<SaQuickStyleRead>> = Rc::new(RefCell::new(SaQuickStyleRead::new()));
        let mut parser: Rc<RefCell<XmlParser>> = Rc::new(RefCell::new(XmlParser::new()));
        let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&parser, xml.to_string());
        if root_opt.is_none() {
            res.borrow_mut().error = "not XML: no element could be read".to_string();
            return res;
        }
        let mut root: Rc<RefCell<XmlNode>> = root_opt.clone().unwrap();
        if root.borrow().name != "styleDef" {
            res.borrow_mut().error = format!("the root element is <{}>, not <dgm:styleDef>", root.borrow().name);
            return res;
        }
        res.borrow_mut().def.borrow_mut().uniqueId = XmlNode::attr(&root, "uniqueId");
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&root) {
            let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&root, i);
            if !c.borrow().isText && c.borrow().name == "styleLbl" {
                let mut l: Rc<RefCell<SaQsLbl>> = Rc::new(RefCell::new(SaQsLbl::new()));
                l.borrow_mut().name = XmlNode::attr(&c, "name");
                let mut st: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&c, "style");
                if st.is_some() {
                    let mut s: Rc<RefCell<XmlNode>> = st.clone().unwrap();
                    l.borrow_mut().ln = SaQuickStyle::idxOf(&s, "lnRef", 2);
                    l.borrow_mut().fill = SaQuickStyle::idxOf(&s, "fillRef", 1);
                    l.borrow_mut().effect = SaQuickStyle::idxOf(&s, "effectRef", 0);
                    let mut fr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&s, "fontRef");
                    if fr.is_some() {
                        l.borrow_mut().font = XmlNode::attr(&fr.clone().unwrap(), "idx");
                    }
                }
                res.borrow_mut().def.borrow_mut().labels.push(l.clone());
            }
            i += 1;
        }
        res.borrow_mut().ok = true;
        res
    }
    fn idxOf(s: &Rc<RefCell<XmlNode>>, name: &str, fallback: i64) -> i64 {
        let mut hit: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&s, name);
        if hit.is_none() {
            return fallback;
        }
        let v: Option<i64> = XmlNode::attr(&hit.clone().unwrap(), "idx").parse::<i64>().ok();
        if v.is_some() { v.unwrap() } else { fallback }
    }
    /// --- writing ---------------------------------------------------------------
    pub fn xml(d: &Rc<RefCell<SaQuickStyleDef>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".to_string();
        out = format!("{}<dgm:styleDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" uniqueId=\"{}\">", out, SaColors::esc(&d.borrow().uniqueId));
        out = format!("{}<dgm:title val=\"\"/><dgm:desc val=\"\"/><dgm:catLst><dgm:cat type=\"simple\" pri=\"10100\"/></dgm:catLst>", out);
        out = format!("{}{}", out, SaQuickStyle::scene());
        let mut i: i64 = 0;
        while i < (d.borrow().labels.len() as i64) {
            let mut l: Rc<RefCell<SaQsLbl>> = d.borrow().labels[i as usize].clone();
            let mut font: String = "lt1";
            let fam: String = SaColors::family(&l.borrow().name.clone());
            if ((fam == "line" || fam == "text") || fam == "light") || fam == "outlined" {
                font = "tx1";
            }
            out = format!("{}<dgm:styleLbl name=\"{}\">{}<dgm:sp3d/><dgm:txPr/>{}</dgm:styleLbl>", out, SaColors::esc(&l.borrow().name), SaQuickStyle::scene(), SaQuickStyle::refs(&l, font));
            i += 1;
        }
        format!("{}</dgm:styleDef>", out)
    }
    pub fn scene() -> String {
        "<dgm:scene3d><a:camera prst=\"orthographicFront\"/><a:lightRig rig=\"threePt\" dir=\"t\"/></dgm:scene3d>".to_string()
    }
    pub fn refs(l: &Rc<RefCell<SaQsLbl>>, font: &str) -> String {
        let zero: String = "<a:scrgbClr r=\"0\" g=\"0\" b=\"0\"/>";
        let mut font_idx: String = l.borrow().font.clone();
        if font_idx.is_empty() {
            font_idx = "minor".to_string();
        }
        return format!("<dgm:style><a:lnRef idx=\"{}\">{}</a:lnRef><a:fillRef idx=\"{}\">{}</a:fillRef><a:effectRef idx=\"{}\">{}</a:effectRef><a:fontRef idx=\"{}\"><a:schemeClr val=\"{}\"/></a:fontRef></dgm:style>", l.borrow().ln, zero, l.borrow().fill, zero, l.borrow().effect, zero, font_idx, font);
    }
}
