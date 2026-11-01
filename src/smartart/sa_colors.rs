// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaColors — colour sets (`dgm:colorsDef`), read, built in, and written
//! ==============================================================================
//!
//! A colour set says, for each style label, the colours of a shape's fill,
//! its line and its text: each a list (`fillClrLst`, `linClrLst`,
//! `txFillClrLst`) with a method saying how the shapes that carry the label
//! take colours from it:
//!
//!   repeat  the n-th shape the n-th colour, round again past the end
//!   cycle   there and back: 0 1 2 1 0 1 2 …
//!   span    from the first colour to the last across all of them; two
//!           tints or shades of one theme colour are blended, two different
//!           theme colours are split half and half
//!
//! The colours are theme colours (accent1, lt1, …) with DrawingML's
//! modifiers (tint, shade, alpha, lumMod, lumOff, satMod), resolved later
//! against the deck's theme or a host's palette.
//!
//! Built in, by the last segment of their ids (SaColors.builtin):
//!
//!   accent0_1 … accent0_3     dark 1 outline, dark 2 outline, dark 2 fill
//!   accentN_1                 coloured outline, N = 1 … 6
//!   accentN_2                 coloured fill (the default, accent1_2)
//!   accentN_3                 gradient range: a shade to a tint of accent N
//!   accentN_4                 gradient loop: the same, there and back
//!   accentN_5                 transparent gradient range
//!   colorful1                 the nodes in accent2 … accent6
//!   colorful2 … colorful5     accent2 and 3, 3 and 4, 4 and 5, 5 and 6
//!
//! They are written from the specification and from what PowerPoint draws;
//! the oracle decks check them. A file's own colour set (a whole diagram as
//! one file, SaFile) is read with `read` and used as it is.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::pptx_model::PptxColor;
use crate::rgr::XmlNode;
use crate::rgr::XmlParser;

#[derive(Clone)]
pub struct SaClrList {
    pub meth: String,
    pub colors: Vec<Rc<RefCell<PptxColor>>>,
}

impl SaClrList {
    pub fn new() -> Self {
        Self {
            meth: "repeat".to_string(),
            colors: Vec::new(),
        }
    }
    pub fn copy(c: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<PptxColor>> {
        let mut o: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        o.borrow_mut().isSet = c.borrow().isSet;
        o.borrow_mut().isScheme = c.borrow().isScheme;
        o.borrow_mut().scheme = c.borrow().scheme.clone();
        o.borrow_mut().srgb = c.borrow().srgb.clone();
        o.borrow_mut().alpha = c.borrow().alpha;
        o.borrow_mut().lumMod = c.borrow().lumMod;
        o.borrow_mut().lumOff = c.borrow().lumOff;
        o.borrow_mut().tint = c.borrow().tint;
        o.borrow_mut().shade = c.borrow().shade;
        o.borrow_mut().satMod = c.borrow().satMod;
        o
    }
    /// how far a colour is from its base: -1 black … 0 itself … +1 white
    pub fn lightness(c: &Rc<RefCell<PptxColor>>) -> f64 {
        if c.borrow().tint > 0.0 {
            return 1.0 - c.borrow().tint;
        }
        if c.borrow().shade > 0.0 {
            return 0.0 - (1.0 - c.borrow().shade);
        }
        0.0
    }
    /// between a and b, f of the way: one theme colour's tints and shades
    /// blended, its alpha too; two theme colours split at the middle
    pub fn blend(a: &Rc<RefCell<PptxColor>>, b: &Rc<RefCell<PptxColor>>, f: f64) -> Rc<RefCell<PptxColor>> {
        if a.borrow().scheme != b.borrow().scheme || a.borrow().srgb != b.borrow().srgb {
            if f < 0.5 {
                return SaClrList::copy(a);
            }
            return SaClrList::copy(b);
        }
        let mut o: Rc<RefCell<PptxColor>> = SaClrList::copy(a);
        let l: f64 = SaClrList::lightness(a) + (SaClrList::lightness(b) - SaClrList::lightness(a)) * f;
        o.borrow_mut().tint = 0.0;
        o.borrow_mut().shade = 0.0;
        if l > 0.0005 {
            o.borrow_mut().tint = 1.0 - l;
        }
        if l < -0.0005 {
            o.borrow_mut().shade = 1.0 + l;
        }
        o.borrow_mut().alpha = a.borrow().alpha + (b.borrow().alpha - a.borrow().alpha) * f;
        o
    }
    pub fn isEmpty(&self) -> bool {
        self.colors.is_empty()
    }
    /// the colour for the idx-th of cnt shapes
    pub fn pick(&self, idx: i64, cnt: i64) -> Rc<RefCell<PptxColor>> {
        let n: i64 = self.colors.len() as i64;
        if n == 0 {
            return Rc::new(RefCell::new(PptxColor::new()));
        }
        if n == 1 {
            return SaClrList::copy(&self.colors[0].clone());
        }
        if self.meth == "span" {
            let mut t: f64 = 0.0;
            if cnt > 1 {
                t = (idx as f64) / ((cnt - 1) as f64);
            }
            if t > 1.0 {
                t = 1.0;
            }
            // along the list: between which two, how far
            let pos: f64 = t * ((n - 1) as f64);
            let mut a: i64 = pos.floor() as i64;
            if a >= n - 1 {
                a = n - 2;
            }
            let f: f64 = pos - (a as f64);
            return SaClrList::blend(
                &self.colors[a as usize].clone(),
                &self.colors[(a + 1) as usize].clone(),
                f
            ).clone();
        }
        if self.meth == "cycle" {
            let period: i64 = 2 * n - 2;
            let mut k: i64 = idx % period;
            if k >= n {
                k = period - k;
            }
            return SaClrList::copy(&self.colors[k as usize].clone());
        }
        SaClrList::copy(&self.colors[(idx % n) as usize].clone())
    }
}

#[derive(Clone)]
pub struct SaColorsLbl {
    pub name: String,
    pub fill: Rc<RefCell<SaClrList>>,
    pub line: Rc<RefCell<SaClrList>>,
    pub text: Rc<RefCell<SaClrList>>,
}

impl SaColorsLbl {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            fill: Rc::new(RefCell::new(SaClrList::new())),
            line: Rc::new(RefCell::new(SaClrList::new())),
            text: Rc::new(RefCell::new(SaClrList::new())),
        }
    }
}

#[derive(Clone)]
pub struct SaColorsDef {
    pub uniqueId: String,
    pub category: String,
    pub labels: Vec<Rc<RefCell<SaColorsLbl>>>,
}

impl SaColorsDef {
    pub fn new() -> Self {
        Self {
            uniqueId: String::new(),
            category: String::new(),
            labels: Vec::new(),
        }
    }
    pub fn label(&self, name: &str) -> Option<Rc<RefCell<SaColorsLbl>>> {
        let mut i: i64 = 0;
        while i < (self.labels.len() as i64) {
            let mut l: Rc<RefCell<SaColorsLbl>> = self.labels[i as usize].clone();
            if l.borrow().name == name {
                let mut hit: Option<Rc<RefCell<SaColorsLbl>>> = Some(l.clone());
                return hit;
            }
            i += 1;
        }
        let mut none: Option<Rc<RefCell<SaColorsLbl>>> = None;
        none
    }
}

#[derive(Clone)]
pub struct SaColorsRead {
    pub ok: bool,
    pub error: String,
    pub def: Rc<RefCell<SaColorsDef>>,
}

impl SaColorsRead {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            def: Rc::new(RefCell::new(SaColorsDef::new())),
        }
    }
}

#[derive(Clone)]
pub struct SaColors {
}

impl SaColors {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn short(id: &str) -> String {
        let at: i64 = PptxText::last_index_of(&id, "/");
        if at < 0 {
            return id.to_string();
        }
        id[(at + 1) as usize..(id.as_bytes().len() as i64) as usize].to_string()
    }
    pub fn fullId(s: &str) -> String {
        if PptxText::index_of(&s, ":") >= 0 {
            return s.to_string();
        }
        format!("urn:microsoft.com/office/officeart/2005/8/colors/{}", s)
    }
    /// --- the families of style labels ----------------------------------------
    /// The family a style label belongs to; what a built-in set says about it.
    pub fn family(lbl: &str) -> String {
        if lbl == "revTx" || lbl.is_empty() {
            return "text".to_string();
        }
        if PptxText::index_of(&lbl, "sibTrans2D") == 0 {
            return "arrow".to_string();
        }
        if PptxText::index_of(&lbl, "parChTrans2D") == 0 {
            return "arrow".to_string();
        }
        if PptxText::index_of(&lbl, "sibTrans1D") == 0 || PptxText::index_of(&lbl, "parChTrans1D") == 0 {
            return "line".to_string();
        }
        if PptxText::index_of(&lbl, "alignAcc") == 0 || PptxText::index_of(&lbl, "bgAcc") == 0 {
            return "light".to_string();
        }
        if PptxText::index_of(&lbl, "fgAcc") == 0 || PptxText::index_of(&lbl, "trAlignAcc") == 0 {
            return "outlined".to_string();
        }
        if PptxText::index_of(&lbl, "bgShp") == 0 || PptxText::index_of(&lbl, "trBgShp") == 0 {
            return "light".to_string();
        }
        if PptxText::index_of(&lbl, "dkBgShp") == 0 {
            return "dark".to_string();
        }
        "node".to_string()
    }
    /// the labels a built-in set describes: every label the built-in layouts
    /// use, and the ones PowerPoint's own layouts commonly name
    pub fn labels() -> Vec<String> {
        vec!["node0".to_string(), "node1".to_string(), "node2".to_string(), "node3".to_string(), "node4".to_string(), "lnNode1".to_string(), "vennNode1".to_string(), "alignNode1".to_string(), "trAlignAcc1".to_string(), "alignAcc1".to_string(), "bgAcc1".to_string(), "fgAcc1".to_string(), "solidFgAcc1".to_string(), "bgShp".to_string(), "dkBgShp".to_string(), "trBgShp".to_string(), "fgShp".to_string(), "revTx".to_string(), "sibTrans2D1".to_string(), "sibTrans1D1".to_string(), "callout".to_string(), "asst0".to_string(), "asst1".to_string(), "parChTrans1D1".to_string(), "parChTrans1D2".to_string(), "parChTrans2D1".to_string(), "fgAccFollowNode1".to_string(), "bgAccFollowNode1".to_string()].clone()
    }
    /// --- reading a file's own set ---------------------------------------------
    pub fn read(xml: &str) -> Rc<RefCell<SaColorsRead>> {
        let mut res: Rc<RefCell<SaColorsRead>> = Rc::new(RefCell::new(SaColorsRead::new()));
        let mut parser: Rc<RefCell<XmlParser>> = Rc::new(RefCell::new(XmlParser::new()));
        let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&parser, xml.to_string());
        if root_opt.is_none() {
            res.borrow_mut().error = "not XML: no element could be read".to_string();
            return res;
        }
        let mut root: Rc<RefCell<XmlNode>> = root_opt.clone().unwrap();
        if root.borrow().name != "colorsDef" {
            res.borrow_mut().error = format!("the root element is <{}>, not <dgm:colorsDef>", root.borrow().name);
            return res;
        }
        res.borrow_mut().def.borrow_mut().uniqueId = XmlNode::attr(&root, "uniqueId");
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&root) {
            let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&root, i);
            if !c.borrow().isText && c.borrow().name == "styleLbl" {
                let mut l: Rc<RefCell<SaColorsLbl>> = Rc::new(RefCell::new(SaColorsLbl::new()));
                l.borrow_mut().name = XmlNode::attr(&c, "name");
                l.borrow_mut().fill = SaColors::listOf(&c, "fillClrLst");
                l.borrow_mut().line = SaColors::listOf(&c, "linClrLst");
                l.borrow_mut().text = SaColors::listOf(&c, "txFillClrLst");
                res.borrow_mut().def.borrow_mut().labels.push(l.clone());
            }
            if !c.borrow().isText && c.borrow().name == "catLst" {
                let mut cat: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&c, "cat");
                if cat.is_some() {
                    res.borrow_mut()
                        .def.borrow_mut()
                        .category = XmlNode::attr(&cat.clone().unwrap(), "type");
                }
            }
            i += 1;
        }
        res.borrow_mut().ok = true;
        res
    }
    fn listOf(lbl: &Rc<RefCell<XmlNode>>, name: &str) -> Rc<RefCell<SaClrList>> {
        let mut out: Rc<RefCell<SaClrList>> = Rc::new(RefCell::new(SaClrList::new()));
        let mut hit: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&lbl, name);
        if hit.is_none() {
            return out;
        }
        let mut node: Rc<RefCell<XmlNode>> = hit.clone().unwrap();
        let m: String = XmlNode::attr(&node, "meth");
        if !m.is_empty() {
            out.borrow_mut().meth = m.clone();
        }
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&node) {
            let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&node, i);
            if !c.borrow().isText {
                let mut col: Option<Rc<RefCell<PptxColor>>> = SaColors::colorOf(&c);
                if col.is_some() {
                    out.borrow_mut().colors.push(col.clone().unwrap());
                }
            }
            i += 1;
        }
        out
    }
    fn colorOf(c: &Rc<RefCell<XmlNode>>) -> Option<Rc<RefCell<PptxColor>>> {
        let mut none: Option<Rc<RefCell<PptxColor>>> = None;
        let mut col: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        if c.borrow().name == "schemeClr" {
            col = PptxColor::schemeOf(XmlNode::attr(&c, "val"));
        } else {
            if c.borrow().name == "srgbClr" {
                col = PptxColor::srgbOf(XmlNode::attr(&c, "val"));
            } else {
                if c.borrow().name == "prstClr" {
                    let v: String = XmlNode::attr(&c, "val");
                    let mut hex: String = "000000";
                    if v == "white" {
                        hex = "FFFFFF";
                    }
                    col = PptxColor::srgbOf(hex.to_string());
                } else {
                    return none;
                }
            }
        }
        col.borrow_mut().tint = SaColors::pct(c, "tint", 0.0);
        col.borrow_mut().shade = SaColors::pct(c, "shade", 0.0);
        col.borrow_mut().lumMod = SaColors::pct(c, "lumMod", 1.0);
        col.borrow_mut().lumOff = SaColors::pct(c, "lumOff", 0.0);
        col.borrow_mut().satMod = SaColors::pct(c, "satMod", 1.0);
        col.borrow_mut().alpha = SaColors::pct(c, "alpha", 1.0);
        let mut hit: Option<Rc<RefCell<PptxColor>>> = Some(col.clone());
        hit
    }
    fn pct(node: &Rc<RefCell<XmlNode>>, name: &str, fallback: f64) -> f64 {
        let mut hit: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, name);
        if hit.is_none() {
            return fallback;
        }
        let v: Option<f64> = XmlNode::attr(&hit.clone().unwrap(), "val").parse::<f64>().ok();
        if v.is_none() {
            return fallback;
        }
        v.unwrap() / 100000.0
    }
    /// --- the built-in sets ----------------------------------------------------
    pub fn known(id: &str) -> bool {
        let s: String = SaColors::short(id);
        if s.is_empty() {
            return true;
        }
        let mut d: Rc<RefCell<SaColorsDef>> = SaColors::builtin(&s);
        return !d.borrow().labels.is_empty();
    }
    pub fn names() -> String {
        "accent0_1 … accent0_3, accent1_1 … accent6_5, colorful1 … colorful5".to_string()
    }
    pub fn sch(name: &str) -> Rc<RefCell<PptxColor>> {
        PptxColor::schemeOf(name.to_string())
    }
    pub fn tinted(name: &str, t: f64) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = PptxColor::schemeOf(name.to_string());
        c.borrow_mut().tint = t;
        c
    }
    pub fn shaded(name: &str, s: f64) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = PptxColor::schemeOf(name.to_string());
        c.borrow_mut().shade = s;
        c
    }
    pub fn alphaOf(name: &str, a: f64) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = PptxColor::schemeOf(name.to_string());
        c.borrow_mut().alpha = a;
        c
    }
    pub fn one(c: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<SaClrList>> {
        let mut l: Rc<RefCell<SaClrList>> = Rc::new(RefCell::new(SaClrList::new()));
        l.borrow_mut().colors.push(c.clone());
        l
    }
    pub fn two(meth: String, a: &Rc<RefCell<PptxColor>>, b: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<SaClrList>> {
        let mut l: Rc<RefCell<SaClrList>> = Rc::new(RefCell::new(SaClrList::new()));
        l.borrow_mut().meth = meth.clone();
        l.borrow_mut().colors.push(a.clone());
        l.borrow_mut().colors.push(b.clone());
        l
    }
    /// The set called `short` (accent1_2, colorful1 …); one with no labels for
    /// a name that is not built in.
    pub fn builtin(short: &str) -> Rc<RefCell<SaColorsDef>> {
        let mut d: Rc<RefCell<SaColorsDef>> = Rc::new(RefCell::new(SaColorsDef::new()));
        let mut kind: String = String::new();
        let mut acc: String = "accent1".to_string();
        if PptxText::index_of(&short, "colorful") == 0 {
            kind = short.to_string();
        }
        if (PptxText::index_of(&short, "accent") == 0 && (short.as_bytes().len() as i64) == 9) && short[(7) as usize..(8) as usize].to_string() == "_" {
            let digit: String = short[(6) as usize..(7) as usize].to_string();
            let variant: String = short[(8) as usize..(9) as usize].to_string();
            let mut ok: bool = PptxText::index_of(&"0123456".to_string(), &digit) >= 0;
            if digit == "0" {
                ok = PptxText::index_of(&"123".to_string(), &variant) >= 0;
            } else {
                ok = ok && PptxText::index_of(&"12345".to_string(), &variant) >= 0;
            }
            if ok {
                kind = format!("accent_{}", variant);
                acc = format!("accent{}", digit);
                if digit == "0" {
                    kind = format!("dark_{}", variant);
                }
            }
        }
        let known_kind: String = " colorful1 colorful2 colorful3 colorful4 colorful5 accent_1 accent_2 accent_3 accent_4 accent_5 dark_1 dark_2 dark_3 ";
        if PptxText::index_of(&known_kind, &format!(" {} ", kind)) < 0 {
            return d;
        }
        d.borrow_mut().uniqueId = SaColors::fullId(short);
        d.borrow_mut().category = short[(0) as usize..(7) as usize].to_string();
        if PptxText::index_of(&short, "colorful") == 0 {
            d.borrow_mut().category = "colorful".to_string();
        }
        let mut ls: Vec<String> = SaColors::labels();
        let mut i: i64 = 0;
        while i < (ls.len() as i64) {
            let name: String = ls[i as usize].clone();
            let mut l: Rc<RefCell<SaColorsLbl>> = SaColors::lblFor(name.clone(), &kind, &acc);
            d.borrow_mut().labels.push(l.clone());
            i += 1;
        }
        d
    }
    /// the main colour of a node in each kind of set
    pub fn nodeFill(kind: &str, acc: &str) -> Rc<RefCell<SaClrList>> {
        if kind == "colorful1" {
            let mut l: Rc<RefCell<SaClrList>> = Rc::new(RefCell::new(SaClrList::new()));
            l.borrow_mut().colors.push(SaColors::sch("accent2"));
            l.borrow_mut().colors.push(SaColors::sch("accent3"));
            l.borrow_mut().colors.push(SaColors::sch("accent4"));
            l.borrow_mut().colors.push(SaColors::sch("accent5"));
            l.borrow_mut().colors.push(SaColors::sch("accent6"));
            return l;
        }
        if kind == "colorful2" {
            let _tmp_601 = SaColors::sch("accent2");
            return SaColors::two("repeat".to_string(), &_tmp_601, &SaColors::sch("accent3"));
        }
        if kind == "colorful3" {
            let _tmp_603 = SaColors::sch("accent3");
            return SaColors::two("repeat".to_string(), &_tmp_603, &SaColors::sch("accent4"));
        }
        if kind == "colorful4" {
            let _tmp_605 = SaColors::sch("accent4");
            return SaColors::two("repeat".to_string(), &_tmp_605, &SaColors::sch("accent5"));
        }
        if kind == "colorful5" {
            let _tmp_607 = SaColors::sch("accent5");
            return SaColors::two("repeat".to_string(), &_tmp_607, &SaColors::sch("accent6"));
        }
        if kind == "accent_1" {
            return SaColors::one(&SaColors::sch("lt1"));
        }
        if kind == "accent_3" {
            let _tmp_610 = SaColors::shaded(acc, 0.8);
            return SaColors::two("span".to_string(), &_tmp_610, &SaColors::tinted(acc, 0.7));
        }
        if kind == "accent_4" {
            let _tmp_612 = SaColors::shaded(acc, 0.5);
            return SaColors::two("cycle".to_string(), &_tmp_612, &SaColors::tinted(acc, 0.55));
        }
        if kind == "accent_5" {
            let _tmp_614 = SaColors::alphaOf(acc, 0.9);
            return SaColors::two("span".to_string(), &_tmp_614, &SaColors::alphaOf(acc, 0.5));
        }
        if kind == "dark_1" || kind == "dark_2" {
            return SaColors::one(&SaColors::sch("lt1"));
        }
        if kind == "dark_3" {
            return SaColors::one(&SaColors::sch("dk2"));
        }
        SaColors::one(&SaColors::sch(acc))
    }
    /// the colour that outlines a node, and that a node's text is, in each kind
    pub fn nodeLine(kind: &str, acc: &str) -> Rc<RefCell<PptxColor>> {
        if kind == "accent_1" {
            return SaColors::sch(acc);
        }
        if kind == "dark_1" {
            return SaColors::sch("dk1");
        }
        if kind == "dark_2" {
            return SaColors::sch("dk2");
        }
        SaColors::sch("lt1")
    }
    pub fn nodeText(kind: &str) -> Rc<RefCell<PptxColor>> {
        if kind == "accent_1" || (kind == "accent_5" || kind == "dark_1") {
            return SaColors::sch("dk1");
        }
        if kind == "dark_2" {
            return SaColors::sch("dk2");
        }
        SaColors::sch("lt1")
    }
    /// the accent the lighter things (arrows, lines, backgrounds) are in
    pub fn quiet(kind: &str, acc: &str) -> String {
        if PptxText::index_of(&kind, "colorful") == 0 {
            return "accent2".to_string();
        }
        if PptxText::index_of(&kind, "dark") == 0 {
            return "dk2".to_string();
        }
        acc.to_string()
    }
    pub fn lblFor(name: String, kind: &str, acc: &str) -> Rc<RefCell<SaColorsLbl>> {
        let mut l: Rc<RefCell<SaColorsLbl>> = Rc::new(RefCell::new(SaColorsLbl::new()));
        l.borrow_mut().name = name.clone();
        let fam: String = SaColors::family(&name);
        let q: String = SaColors::quiet(kind, acc);
        if fam == "node" {
            l.borrow_mut().fill = SaColors::nodeFill(kind, acc);
            let _tmp_621 = SaColors::one(&SaColors::nodeLine(kind, acc));
            l.borrow_mut().line = _tmp_621;
            let _tmp_623 = SaColors::one(&SaColors::nodeText(kind));
            l.borrow_mut().text = _tmp_623;
            if PptxText::index_of(&name, "vennNode") == 0 {
                // see-through, so where circles meet shows; the text dark
                let mut vf: Rc<RefCell<SaClrList>> = SaColors::nodeFill(kind, acc);
                let mut vi: i64 = 0;
                while vi < (vf.borrow().colors.len() as i64) {
                    let mut vc: Rc<RefCell<PptxColor>> = vf.borrow().colors[vi as usize].clone();
                    vc.borrow_mut().alpha = 0.5;
                    vi += 1;
                }
                l.borrow_mut().fill = vf.clone();
                let _tmp_625 = SaColors::one(&SaColors::sch("tx1"));
                l.borrow_mut().text = _tmp_625;
            }
            if name == "node0" {
                // the one in the middle: the first colour, whatever the set
                let mut f0: Rc<RefCell<SaClrList>> = SaColors::nodeFill(kind, acc);
                l.borrow_mut().fill = SaColors::one(&f0.borrow().colors[0].clone());
            }
        }
        if fam == "arrow" {
            let _tmp_628 = SaColors::one(&SaColors::tinted(&q, 0.6));
            l.borrow_mut().fill = _tmp_628;
            let _tmp_630 = SaColors::one(&SaColors::sch("lt1"));
            l.borrow_mut().text = _tmp_630;
        }
        if fam == "line" {
            let _tmp_632 = SaColors::one(&SaColors::tinted(&q, 0.6));
            l.borrow_mut().line = _tmp_632;
            if name == "parChTrans1D2" {
                let _tmp_634 = SaColors::one(&SaColors::shaded(&q, 0.6));
                l.borrow_mut().line = _tmp_634;
            }
            let _tmp_636 = SaColors::one(&SaColors::sch("tx1"));
            l.borrow_mut().text = _tmp_636;
        }
        if fam == "light" {
            let _tmp_638 = SaColors::one(&SaColors::tinted(&q, 0.4));
            l.borrow_mut().fill = _tmp_638;
            let _tmp_640 = SaColors::one(&SaColors::tinted(&q, 0.4));
            l.borrow_mut().line = _tmp_640;
            let _tmp_642 = SaColors::one(&SaColors::sch("tx1"));
            l.borrow_mut().text = _tmp_642;
        }
        if fam == "dark" {
            let _tmp_644 = SaColors::one(&SaColors::shaded(&q, 0.8));
            l.borrow_mut().fill = _tmp_644;
            let _tmp_646 = SaColors::one(&SaColors::shaded(&q, 0.8));
            l.borrow_mut().line = _tmp_646;
            let _tmp_648 = SaColors::one(&SaColors::sch("lt1"));
            l.borrow_mut().text = _tmp_648;
        }
        if fam == "outlined" {
            let _tmp_650 = SaColors::one(&SaColors::sch("lt1"));
            l.borrow_mut().fill = _tmp_650;
            let _tmp_652 = SaColors::one(&SaColors::sch(&q));
            l.borrow_mut().line = _tmp_652;
            let _tmp_654 = SaColors::one(&SaColors::sch("tx1"));
            l.borrow_mut().text = _tmp_654;
        }
        if fam == "text" {
            let _tmp_656 = SaColors::one(&SaColors::sch("tx1"));
            l.borrow_mut().text = _tmp_656;
        }
        l
    }
    /// --- writing ---------------------------------------------------------------
    pub fn xml(d: &Rc<RefCell<SaColorsDef>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".to_string();
        out = format!("{}<dgm:colorsDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" uniqueId=\"{}\">", out, SaColors::esc(&d.borrow().uniqueId));
        let mut cat: String = d.borrow().category.clone();
        if cat.is_empty() {
            cat = "accent1".to_string();
        }
        out = format!("{}<dgm:title val=\"\"/><dgm:desc val=\"\"/><dgm:catLst><dgm:cat type=\"{}\" pri=\"11200\"/></dgm:catLst>", out, SaColors::esc(&cat));
        let mut i: i64 = 0;
        while i < (d.borrow().labels.len() as i64) {
            let mut l: Rc<RefCell<SaColorsLbl>> = d.borrow().labels[i as usize].clone();
            out = format!("{}<dgm:styleLbl name=\"{}\">{}{}<dgm:effectClrLst/><dgm:txLinClrLst/>{}<dgm:txEffectClrLst/></dgm:styleLbl>", out, SaColors::esc(&l.borrow().name), SaColors::listXml("fillClrLst", &l.borrow().fill), SaColors::listXml("linClrLst", &l.borrow().line), SaColors::listXml("txFillClrLst", &l.borrow().text));
            i += 1;
        }
        format!("{}</dgm:colorsDef>", out)
    }
    pub fn listXml(tag: &str, l: &Rc<RefCell<SaClrList>>) -> String {
        if l.borrow().isEmpty() {
            return format!("<dgm:{}/>", tag);
        }
        let mut out: String = format!("<dgm:{} meth=\"{}\">", tag, l.borrow().meth);
        let mut i: i64 = 0;
        while i < (l.borrow().colors.len() as i64) {
            out = format!("{}{}", out, SaColors::clrXml(&l.borrow().colors[i as usize].clone()));
            i += 1;
        }
        format!("{}</dgm:{}>", out, tag)
    }
    pub fn clrXml(c: &Rc<RefCell<PptxColor>>) -> String {
        let mut mods: String = String::new();
        if c.borrow().tint > 0.0 {
            mods = format!("{}<a:tint val=\"{}\"/>", mods, SaColors::k(c.borrow().tint));
        }
        if c.borrow().shade > 0.0 {
            mods = format!("{}<a:shade val=\"{}\"/>", mods, SaColors::k(c.borrow().shade));
        }
        if c.borrow().alpha < 1.0 {
            mods = format!("{}<a:alpha val=\"{}\"/>", mods, SaColors::k(c.borrow().alpha));
        }
        if c.borrow().lumMod != 1.0 {
            mods = format!("{}<a:lumMod val=\"{}\"/>", mods, SaColors::k(c.borrow().lumMod));
        }
        if c.borrow().lumOff != 0.0 {
            mods = format!("{}<a:lumOff val=\"{}\"/>", mods, SaColors::k(c.borrow().lumOff));
        }
        if c.borrow().satMod != 1.0 {
            mods = format!("{}<a:satMod val=\"{}\"/>", mods, SaColors::k(c.borrow().satMod));
        }
        let mut open: String = String::new();
        let mut tag: String = "";
        if c.borrow().isScheme {
            tag = "a:schemeClr";
            open = format!("<a:schemeClr val=\"{}\"", c.borrow().scheme);
        } else {
            tag = "a:srgbClr";
            open = format!("<a:srgbClr val=\"{}\"", c.borrow().srgb);
        }
        if mods.is_empty() {
            return format!("{}/>", open);
        }
        format!("{}>{}</{}>", open, mods, tag)
    }
    pub fn k(v: f64) -> String {
        ((v * 100000.0 + 0.5).floor() as i64).to_string()
    }
    pub fn esc(s: &str) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (s.as_bytes().len() as i64) {
            let mut ch: String = s[(i) as usize..(i + 1) as usize].to_string();
            if ch == "&" {
                ch = "&amp;".to_string();
            }
            if ch == "<" {
                ch = "&lt;".to_string();
            }
            if ch == "\"" {
                ch = "&quot;".to_string();
            }
            out = format!("{}{}", out, ch);
            i += 1;
        }
        out
    }
}
