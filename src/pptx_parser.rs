// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::text::PptxText;

use crate::pptx_geom::PptxGeom;
use crate::pptx_model::PptxBuildStep;
use crate::pptx_model::PptxChart;
use crate::pptx_model::PptxChartSeries;
use crate::pptx_model::PptxColor;
use crate::pptx_model::PptxFillStyle;
use crate::pptx_model::PptxGradient;
use crate::pptx_model::PptxLevelStyle;
use crate::pptx_model::PptxLineStyle;
use crate::pptx_model::PptxListStyle;
use crate::pptx_model::PptxOpaque;
use crate::pptx_model::PptxParagraph;
use crate::pptx_model::PptxPresentation;
use crate::pptx_model::PptxSection;
use crate::pptx_model::PptxShape;
use crate::pptx_model::PptxShapeKind;
use crate::pptx_model::PptxSheet;
use crate::pptx_model::PptxSlide;
use crate::pptx_model::PptxTableCell;
use crate::pptx_model::PptxTableLine;
use crate::pptx_model::PptxTextBody;
use crate::pptx_model::PptxTextRun;
use crate::pptx_model::PptxTheme;
use crate::pptx_model::PptxTransition;
use crate::pptx_model::PptxUnits;
use crate::rgr::PathRing;
use crate::rgr::SourceRef;
use crate::rgr::XmlNode;
use crate::rgr::XmlParser;
use crate::smartart::sa_builtins::SaBuiltins;
use crate::smartart::sa_data_reader::SaDataReader;
use crate::smartart::sa_data_reader::SaReadResult;
use crate::smartart::sa_engine::SaEngine;
use crate::smartart::sa_engine::SaOptions;
use crate::smartart::sa_engine::SaResult;
use crate::smartart::sa_layout_def::SaLayoutDef;
use crate::smartart::sa_layout_reader::SaLayoutReadResult;
use crate::smartart::sa_layout_reader::SaLayoutReader;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_style::SaStyle;


/// ==============================================================================
/// PptxParser - OPC package → PptxPresentation / raw slides
/// ==============================================================================
///
/// Reads presentation.xml for slide order + size, then each slide part into a
/// PptxSlide with shapes / pictures / groups / text. Theme and master inheritance
/// are left to PptxResolver — this layer only materialises what the XML says.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxParser {
    pkg: Rc<RefCell<OpcPackage>>,
    xmlParser: Rc<RefCell<XmlParser>>,
}

impl PptxParser {
    pub fn new() -> Self {
        Self {
            pkg: Rc::new(RefCell::new(OpcPackage::new())),
            xmlParser: Rc::new(RefCell::new(XmlParser::new())),
        }
    }
    /// `a:srcRect` side, thousandths of a percent, as a fraction
    pub fn cropOf(v: &str) -> f64 {
        if v.is_empty() {
            return 0.0;
        }
        let n: Option<i64> = v.trim().to_string().parse::<i64>().ok();
        if n.is_none() {
            return 0.0;
        }
        ((n.unwrap()) as f64) / 100000.0
    }
    /// For a drawing shape (its `modelId` a presentation point), the data point
    /// behind each of its `n` paragraphs: the point it presents, then the
    /// points under that one in order (a bullet list), the last repeated.
    pub fn paraPtsOf(model: &Rc<RefCell<SaModel>>, pres_id: &str, n: i64) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if n == 0 || pres_id.is_empty() {
            return out;
        }
        let mut hit: Option<Rc<RefCell<SaPoint>>> = model.borrow().pointById(pres_id);
        if hit.is_none() {
            return out;
        }
        let mut p: Rc<RefCell<SaPoint>> = hit.clone().unwrap();
        let mut assoc: String = p.borrow().presAssocId.clone();
        if p.borrow().r#type != "pres" {
            assoc = p.borrow().id.clone();
        }
        if assoc.is_empty() {
            return out;
        }
        let mut order: Vec<String> = Vec::new();
        order.push(assoc.clone());
        PptxParser::nodesUnder(model, &assoc, &mut order);
        let mut i: i64 = 0;
        while i < n {
            if i < (order.len() as i64) {
                out.push(order[i as usize].clone());
            } else {
                out.push(order[((order.len() as i64) - 1) as usize].clone());
            }
            i += 1;
        }
        out
    }
    pub fn nodesUnder(model: &Rc<RefCell<SaModel>>, id: &str, mut out: &mut Vec<String>) {
        let mut kids: Vec<Rc<RefCell<SaPoint>>> = model.borrow().children(id);
        let mut i: i64 = 0;
        while i < (kids.len() as i64) {
            let mut k: Rc<RefCell<SaPoint>> = kids[i as usize].clone();
            out.push(k.borrow().id.clone());
            PptxParser::nodesUnder(model, &k.borrow().id.clone(), &mut out);
            i += 1;
        }
    }
    /// `a:lin ang` is in 1/60000 of a degree, clockwise from "left to right".
    /// The model only says vertical or horizontal, so the angle is rounded to
    /// whichever it is nearer.
    fn gradientDir(node: &Rc<RefCell<XmlNode>>) -> i64 {
        let mut lin: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "lin");
        if lin.is_none() {
            return 0;
        }
        let a: String = XmlNode::attr(&lin.clone().unwrap(), "ang");
        if a.is_empty() {
            return 0;
        }
        let av: Option<f64> = a.parse::<f64>().ok();
        let mut deg: f64 = (if av.is_some() { av.unwrap() } else { 0.0 }) / 60000.0;
        while deg < 0.0 {
            deg += 360.0;
        }
        while deg >= 360.0 {
            deg -= 360.0;
        }
        if deg < 45.0 {
            return 1;
        }
        if deg < 135.0 {
            return 0;
        }
        if deg < 225.0 {
            return 1;
        }
        if deg < 315.0 {
            return 0;
        }
        1
    }
    pub fn close(&self) {
        OpcPackage::close(&self.pkg.clone());
    }
    pub fn open(&self, full_path: &str) -> bool {
        return OpcPackage::open(&self.pkg, full_path);
    }
    pub fn openParts(&self, path: &str, name: &str) -> bool {
        return OpcPackage::openParts(&self.pkg, path.to_string(), name.to_string());
    }
    pub fn openBytes(&self, bytes: &[u8], name: &str) -> bool {
        return OpcPackage::openBytes(&self.pkg, bytes, name.to_string());
    }
    fn parseDoubleAttr(&self, node: &Rc<RefCell<XmlNode>>, name: &str) -> f64 {
        let s: String = XmlNode::attr(&node, name);
        if s.is_empty() {
            return 0.0;
        }
        let d: Option<f64> = s.parse::<f64>().ok();
        if d.is_some() { d.unwrap() } else { 0.0 }
    }
    /// A percentage attribute in DrawingML is in thousandths of a percent, so
    /// 100000 is 1.0. `fallback` is what "not stated" means for this modifier.
    fn parsePercent(&self, node: &Rc<RefCell<XmlNode>>, name: &str, fallback: f64) -> f64 {
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
    /// The adjustments a colour carries: they are children of the colour
    /// element, not of the fill, and they are what a theme's fill styles are
    /// made of — a style is `phClr` plus a chain of these.
    fn parseColorMods(&self, node: &Rc<RefCell<XmlNode>>, col: &Rc<RefCell<PptxColor>>) {
        col.borrow_mut().lumMod = self.parsePercent(node, "lumMod", 1.0);
        col.borrow_mut().lumOff = self.parsePercent(node, "lumOff", 0.0);
        col.borrow_mut().tint = self.parsePercent(node, "tint", 0.0);
        col.borrow_mut().shade = self.parsePercent(node, "shade", 0.0);
        col.borrow_mut().satMod = self.parsePercent(node, "satMod", 1.0);
        let mut a: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "alpha");
        if a.is_some() {
            let av: Option<f64> = XmlNode::attr(&a.clone().unwrap(), "val").parse::<f64>().ok();
            if av.is_some() {
                col.borrow_mut().alpha = av.unwrap() / 100000.0;
            }
        }
    }
    /// The colour a `srgbClr` / `schemeClr` / `sysClr` element states, with its
    /// adjustments. `parent` is the element that HOLDS the colour.
    fn colorIn(&self, parent: &Rc<RefCell<XmlNode>>) -> Option<Rc<RefCell<PptxColor>>> {
        let mut srgb: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "srgbClr");
        if srgb.is_some() {
            let mut sn: Rc<RefCell<XmlNode>> = srgb.clone().unwrap();
            let hex: String = XmlNode::attr(&sn, "val");
            if (hex.as_bytes().len() as i64) == 6 {
                let mut out: Rc<RefCell<PptxColor>> = PptxColor::srgbOf(hex.clone());
                self.parseColorMods(&sn, &out);
                let mut hit: Option<Rc<RefCell<PptxColor>>> = Some(out.clone());
                return hit;
            }
        }
        let mut sch: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "schemeClr");
        if sch.is_some() {
            let mut sc: Rc<RefCell<XmlNode>> = sch.clone().unwrap();
            let name: String = XmlNode::attr(&sc, "val");
            if !name.is_empty() {
                let mut out2: Rc<RefCell<PptxColor>> = PptxColor::schemeOf(name.clone());
                self.parseColorMods(&sc, &out2);
                let mut hit2: Option<Rc<RefCell<PptxColor>>> = Some(out2.clone());
                return hit2;
            }
        }
        // `sysClr` states the system's own colour and, helpfully, the last one
        // it resolved to — which is the only value a file can be read with.
        let mut sys: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "sysClr");
        if sys.is_some() {
            let mut sy: Rc<RefCell<XmlNode>> = sys.clone().unwrap();
            let last: String = XmlNode::attr(&sy, "lastClr");
            if (last.as_bytes().len() as i64) == 6 {
                let mut out3: Rc<RefCell<PptxColor>> = PptxColor::srgbOf(last.clone());
                self.parseColorMods(&sy, &out3);
                let mut hit3: Option<Rc<RefCell<PptxColor>>> = Some(out3.clone());
                return hit3;
            }
        }
        let mut __missing: Option<Rc<RefCell<PptxColor>>> = None;
        __missing
    }
    fn parseColor(&self, parent: &Rc<RefCell<XmlNode>>) -> Option<Rc<RefCell<PptxColor>>> {
        // Look for solidFill → srgbClr | schemeClr under parent
        let mut solid: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "solidFill");
        if solid.is_none() {
            // Also allow parent itself to be solidFill
            if parent.borrow().name == "solidFill" {
                solid = Some(parent.clone());
            }
        }
        if solid.is_none() {
            let mut __missing: Option<Rc<RefCell<PptxColor>>> = None;
            return __missing;
        }
        self.colorIn(&solid.clone().unwrap())
    }
    fn parseXfrm(&self, xfrm: &Rc<RefCell<XmlNode>>, shape: &Rc<RefCell<PptxShape>>) {
        shape.borrow_mut().hasXfrm = true;
        let rot_s: String = XmlNode::attr(&xfrm, "rot");
        if !rot_s.is_empty() {
            let rd: Option<f64> = rot_s.parse::<f64>().ok();
            let rot: f64 = if rd.is_some() { rd.unwrap() } else { 0.0 };
            shape.borrow_mut().rotation = PptxUnits::rotToDeg(rot);
        }
        if XmlNode::attr(&xfrm, "flipH") == "1" {
            shape.borrow_mut().flipH = true;
        }
        if XmlNode::attr(&xfrm, "flipV") == "1" {
            shape.borrow_mut().flipV = true;
        }
        let mut off: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&xfrm, "off");
        if off.is_some() {
            let mut o: Rc<RefCell<XmlNode>> = off.clone().unwrap();
            let _tmp_4514 = PptxUnits::emuToPt(self.parseDoubleAttr(&o, "x"));
            shape.borrow_mut().x = _tmp_4514;
            let _tmp_4516 = PptxUnits::emuToPt(self.parseDoubleAttr(&o, "y"));
            shape.borrow_mut().y = _tmp_4516;
            shape.borrow_mut().hasOff = true;
        }
        let mut ext: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&xfrm, "ext");
        if ext.is_some() {
            let mut e: Rc<RefCell<XmlNode>> = ext.clone().unwrap();
            let _tmp_4518 = PptxUnits::emuToPt(self.parseDoubleAttr(&e, "cx"));
            shape.borrow_mut().width = _tmp_4518;
            let _tmp_4520 = PptxUnits::emuToPt(self.parseDoubleAttr(&e, "cy"));
            shape.borrow_mut().height = _tmp_4520;
            // Stated, and stated as a SIZE. `cx="0"` is a degenerate box that
            // a writer leaves behind, not an instruction to draw nothing, and
            // treating it as stated is what stopped the layout's real size
            // from ever being asked for.
            if shape.borrow().width > 0.0 && shape.borrow().height > 0.0 {
                shape.borrow_mut().hasExt = true;
            }
        }
        let mut ch_off: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&xfrm, "chOff");
        if ch_off.is_some() {
            let mut co: Rc<RefCell<XmlNode>> = ch_off.clone().unwrap();
            let _tmp_4522 = PptxUnits::emuToPt(self.parseDoubleAttr(&co, "x"));
            shape.borrow_mut().chX = _tmp_4522;
            let _tmp_4524 = PptxUnits::emuToPt(self.parseDoubleAttr(&co, "y"));
            shape.borrow_mut().chY = _tmp_4524;
        }
        let mut ch_ext: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&xfrm, "chExt");
        if ch_ext.is_some() {
            let mut ce: Rc<RefCell<XmlNode>> = ch_ext.clone().unwrap();
            let _tmp_4526 = PptxUnits::emuToPt(self.parseDoubleAttr(&ce, "cx"));
            shape.borrow_mut().chW = _tmp_4526;
            let _tmp_4528 = PptxUnits::emuToPt(self.parseDoubleAttr(&ce, "cy"));
            shape.borrow_mut().chH = _tmp_4528;
        }
    }
    fn applyRPrToRun(&self, rp: &Rc<RefCell<XmlNode>>, run: &Rc<RefCell<PptxTextRun>>) {
        let sz: String = XmlNode::attr(&rp, "sz");
        if !sz.is_empty() {
            let sd: Option<f64> = sz.parse::<f64>().ok();
            run.borrow_mut().fontSizePt = PptxUnits::hundredthsPtToPt(if sd.is_some() { sd.unwrap() } else { 1800.0 });
            run.borrow_mut().sizeSet = true;
        }
        // What the run SAID, not just whether it said yes. `b="0"` takes a word
        // out of a bold placeholder; no `b` at all lets the placeholder decide.
        // The list-level parser below has always read it this way.
        let b_attr: String = XmlNode::attr(&rp, "b");
        if !b_attr.is_empty() {
            run.borrow_mut().bold = b_attr == "1";
            run.borrow_mut().boldSet = true;
        }
        let i_attr: String = XmlNode::attr(&rp, "i");
        if !i_attr.is_empty() {
            run.borrow_mut().italic = i_attr == "1";
            run.borrow_mut().italicSet = true;
        }
        let mut latin: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rp, "latin");
        if latin.is_some() {
            let mut ln: Rc<RefCell<XmlNode>> = latin.clone().unwrap();
            let tf: String = XmlNode::attr(&ln, "typeface");
            if !tf.is_empty() {
                run.borrow_mut().fontFamily = tf.clone();
            }
        }
        let mut col: Option<Rc<RefCell<PptxColor>>> = self.parseColor(rp);
        if col.is_some() {
            run.borrow_mut().color = col.clone().unwrap();
            run.borrow_mut().colorStated = true;
        }
        let mut hl: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rp, "highlight");
        if hl.is_some() {
            let mut hc: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&hl.clone().unwrap());
            if hc.is_some() {
                run.borrow_mut().highlight = hc.clone().unwrap();
            }
        }
    }
    /// --- list styles ----------------------------------------------------------
    ///
    /// `a:lvl1pPr` … `a:lvl9pPr`, or `a:defPPr` standing for all of them. Every
    /// value carries a flag rather than a default, because these are merged down
    /// a chain and a level that says nothing must not overwrite one that does.
    fn parseLevelStyle(&self, node: &Rc<RefCell<XmlNode>>, lvl: &Rc<RefCell<PptxLevelStyle>>) {
        let algn: String = XmlNode::attr(&node, "algn");
        if !algn.is_empty() {
            lvl.borrow_mut().align = algn.clone();
            lvl.borrow_mut().alignSet = true;
        }
        let mar_l: String = XmlNode::attr(&node, "marL");
        if !mar_l.is_empty() {
            let m: Option<f64> = mar_l.parse::<f64>().ok();
            lvl.borrow_mut().marginLeftPt = PptxUnits::emuToPt(if m.is_some() { m.unwrap() } else { 0.0 });
            lvl.borrow_mut().marginSet = true;
        }
        let indent: String = XmlNode::attr(&node, "indent");
        if !indent.is_empty() {
            let d: Option<f64> = indent.parse::<f64>().ok();
            lvl.borrow_mut().indentPt = PptxUnits::emuToPt(if d.is_some() { d.unwrap() } else { 0.0 });
            lvl.borrow_mut().indentSet = true;
        }
        let mut spc_bef: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "spcBef");
        if spc_bef.is_some() {
            let v: f64 = self.spacingPoints(&spc_bef.clone().unwrap());
            if v >= 0.0 {
                lvl.borrow_mut().spaceBeforePt = v;
                lvl.borrow_mut().spaceBeforeSet = true;
            }
        }
        let mut spc_aft: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "spcAft");
        if spc_aft.is_some() {
            let v2: f64 = self.spacingPoints(&spc_aft.clone().unwrap());
            if v2 >= 0.0 {
                lvl.borrow_mut().spaceAfterPt = v2;
                lvl.borrow_mut().spaceAfterSet = true;
            }
        }
        let mut ln_spc: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "lnSpc");
        if ln_spc.is_some() {
            let mut pct: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&ln_spc.clone().unwrap(), "spcPct");
            if pct.is_some() {
                let pv: Option<f64> = XmlNode::attr(&pct.clone().unwrap(), "val").parse::<f64>().ok();
                lvl.borrow_mut().lineSpacingPct = (if pv.is_some() { pv.unwrap() } else { 100000.0 }) / 1000.0;
                lvl.borrow_mut().lineSpacingSet = true;
            }
        }
        // The bullet.
        let mut none: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buNone");
        if none.is_some() {
            lvl.borrow_mut().bulletKind = "none".to_string();
        }
        let mut ch: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buChar");
        if ch.is_some() {
            let c: String = XmlNode::attr(&ch.clone().unwrap(), "char");
            lvl.borrow_mut().bulletKind = "char".to_string();
            if !c.is_empty() {
                lvl.borrow_mut().bulletChar = c.clone();
            }
        }
        let mut num: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buAutoNum");
        if num.is_some() {
            let mut nn: Rc<RefCell<XmlNode>> = num.clone().unwrap();
            lvl.borrow_mut().bulletKind = "autonum".to_string();
            let sch: String = XmlNode::attr(&nn, "type");
            if !sch.is_empty() {
                lvl.borrow_mut().bulletScheme = sch.clone();
            }
            let st: String = XmlNode::attr(&nn, "startAt");
            if !st.is_empty() {
                let sv: Option<i64> = st.parse::<i64>().ok();
                lvl.borrow_mut().bulletStartAt = if sv.is_some() { sv.unwrap() } else { 1 };
            }
        }
        let mut bf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buFont");
        if bf.is_some() {
            let tf: String = XmlNode::attr(&bf.clone().unwrap(), "typeface");
            if !tf.is_empty() {
                lvl.borrow_mut().bulletFont = tf.clone();
            }
        }
        let mut bc: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buClr");
        if bc.is_some() {
            let mut col: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&bc.clone().unwrap());
            if col.is_some() {
                lvl.borrow_mut().bulletColor = col.clone().unwrap();
            }
        }
        let mut bsp: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buSzPct");
        if bsp.is_some() {
            let pv2: Option<f64> = XmlNode::attr(&bsp.clone().unwrap(), "val").parse::<f64>().ok();
            lvl.borrow_mut().bulletSizePct = (if pv2.is_some() { pv2.unwrap() } else { 100000.0 }) / 1000.0;
            lvl.borrow_mut().bulletSizePctSet = true;
        }
        let mut bst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "buSzPts");
        if bst.is_some() {
            let tv: Option<f64> = XmlNode::attr(&bst.clone().unwrap(), "val").parse::<f64>().ok();
            lvl.borrow_mut().bulletSizePt = PptxUnits::hundredthsPtToPt(if tv.is_some() { tv.unwrap() } else { 1800.0 });
            lvl.borrow_mut().bulletSizePtSet = true;
        }
        // The run properties this level gives its text.
        let mut rpr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "defRPr");
        if rpr.is_none() {
            return;
        }
        let mut rp: Rc<RefCell<XmlNode>> = rpr.clone().unwrap();
        let sz: String = XmlNode::attr(&rp, "sz");
        if !sz.is_empty() {
            let sd: Option<f64> = sz.parse::<f64>().ok();
            lvl.borrow_mut().fontSizePt = PptxUnits::hundredthsPtToPt(if sd.is_some() { sd.unwrap() } else { 1800.0 });
            lvl.borrow_mut().sizeSet = true;
        }
        let b: String = XmlNode::attr(&rp, "b");
        if !b.is_empty() {
            lvl.borrow_mut().bold = b == "1";
            lvl.borrow_mut().boldSet = true;
        }
        let it: String = XmlNode::attr(&rp, "i");
        if !it.is_empty() {
            lvl.borrow_mut().italic = it == "1";
            lvl.borrow_mut().italicSet = true;
        }
        let mut latin: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rp, "latin");
        if latin.is_some() {
            let tf2: String = XmlNode::attr(&latin.clone().unwrap(), "typeface");
            if !tf2.is_empty() {
                lvl.borrow_mut().fontFamily = tf2.clone();
            }
        }
        let mut col2: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&rp);
        if col2.is_some() {
            lvl.borrow_mut().color = col2.clone().unwrap();
        }
    }
    /// A `spcBef` / `spcAft` in points, or -1 when it is stated as a percentage
    /// this reader does not turn into points without knowing the font size.
    fn spacingPoints(&self, node: &Rc<RefCell<XmlNode>>) -> f64 {
        let mut pts: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "spcPts");
        if pts.is_some() {
            let v: Option<f64> = XmlNode::attr(&pts.clone().unwrap(), "val").parse::<f64>().ok();
            return PptxUnits::hundredthsPtToPt(if v.is_some() { v.unwrap() } else { 0.0 });
        }
        0.0 - 1.0
    }
    fn parseListStyle(&self, node: &Rc<RefCell<XmlNode>>, style: &Rc<RefCell<PptxListStyle>>) {
        let mut any: bool = false;
        let mut def0: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "defPPr");
        if def0.is_some() {
            // `defPPr` stands for every level that does not state itself.
            let mut i: i64 = 0;
            while i < 9 {
                self.parseLevelStyle(&def0.clone().unwrap(),
                    &style.borrow().at(i)
                );
                i += 1;
            }
            any = true;
        }
        let mut names: Vec<String> = vec!["lvl1pPr".to_string(), "lvl2pPr".to_string(), "lvl3pPr".to_string(), "lvl4pPr".to_string(), "lvl5pPr".to_string(), "lvl6pPr".to_string(), "lvl7pPr".to_string(), "lvl8pPr".to_string(), "lvl9pPr".to_string()];
        let mut k: i64 = 0;
        while k < 9 {
            let mut hit: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, &names[k as usize].clone());
            if hit.is_some() {
                self.parseLevelStyle(&hit.clone().unwrap(),
                    &style.borrow().at(k)
                );
                any = true;
            }
            k += 1;
        }
        if any {
            style.borrow_mut().isSet = true;
        }
    }
    fn parseLstStyleDefaults(&self, tx_body: &Rc<RefCell<XmlNode>>, body: &Rc<RefCell<PptxTextBody>>) {
        let mut lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&tx_body, "lstStyle");
        if lst.is_none() {
            return;
        }
        let mut ls: Rc<RefCell<XmlNode>> = lst.clone().unwrap();
        self.parseListStyle(&ls, &body.borrow().listStyle.clone());
        // Prefer lvl1pPr, then defPPr
        let mut lvl: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&ls, "lvl1pPr");
        if lvl.is_none() {
            lvl = XmlNode::firstChild(&ls, "defPPr");
        }
        if lvl.is_none() {
            return;
        }
        let mut ln: Rc<RefCell<XmlNode>> = lvl.clone().unwrap();
        let algn: String = XmlNode::attr(&ln, "algn");
        if !algn.is_empty() {
            body.borrow_mut().defAlign = algn.clone();
        }
        let mut def_r_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&ln, "defRPr");
        if def_r_pr.is_none() {
            return;
        }
        let mut rp: Rc<RefCell<XmlNode>> = def_r_pr.clone().unwrap();
        body.borrow_mut().hasDefRPr = true;
        let sz: String = XmlNode::attr(&rp, "sz");
        if !sz.is_empty() {
            let sd: Option<f64> = sz.parse::<f64>().ok();
            body.borrow_mut().defFontSizePt = PptxUnits::hundredthsPtToPt(if sd.is_some() { sd.unwrap() } else { 1800.0 });
        }
        let def_b: String = XmlNode::attr(&rp, "b");
        if !def_b.is_empty() {
            body.borrow_mut().defBold = def_b == "1";
            body.borrow_mut().defBoldSet = true;
        }
        let def_i: String = XmlNode::attr(&rp, "i");
        if !def_i.is_empty() {
            body.borrow_mut().defItalic = def_i == "1";
            body.borrow_mut().defItalicSet = true;
        }
        let mut latin: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&rp, "latin");
        if latin.is_some() {
            let tf: String = XmlNode::attr(&latin.clone().unwrap(), "typeface");
            if !tf.is_empty() {
                body.borrow_mut().defFontFamily = tf.clone();
            }
        }
        let mut col: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&rp);
        if col.is_some() {
            body.borrow_mut().defColor = col.clone().unwrap();
        }
    }
    fn parseTextBody(&self, tx_body: &Rc<RefCell<XmlNode>>) -> Rc<RefCell<PptxTextBody>> {
        let mut body: Rc<RefCell<PptxTextBody>> = Rc::new(RefCell::new(PptxTextBody::new()));
        let mut body_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&tx_body, "bodyPr");
        if body_pr.is_some() {
            let mut bp: Rc<RefCell<XmlNode>> = body_pr.clone().unwrap();
            let l_ins: String = XmlNode::attr(&bp, "lIns");
            if !l_ins.is_empty() {
                let ld: Option<f64> = l_ins.parse::<f64>().ok();
                body.borrow_mut().insetL = PptxUnits::emuToPt(if ld.is_some() { ld.unwrap() } else { 0.0 });
                body.borrow_mut().insetsSet = true;
            }
            let t_ins: String = XmlNode::attr(&bp, "tIns");
            if !t_ins.is_empty() {
                let td: Option<f64> = t_ins.parse::<f64>().ok();
                body.borrow_mut().insetT = PptxUnits::emuToPt(if td.is_some() { td.unwrap() } else { 0.0 });
                body.borrow_mut().insetsSet = true;
            }
            let r_ins: String = XmlNode::attr(&bp, "rIns");
            if !r_ins.is_empty() {
                let rd: Option<f64> = r_ins.parse::<f64>().ok();
                body.borrow_mut().insetR = PptxUnits::emuToPt(if rd.is_some() { rd.unwrap() } else { 0.0 });
                body.borrow_mut().insetsSet = true;
            }
            let b_ins: String = XmlNode::attr(&bp, "bIns");
            if !b_ins.is_empty() {
                let bd: Option<f64> = b_ins.parse::<f64>().ok();
                body.borrow_mut().insetB = PptxUnits::emuToPt(if bd.is_some() { bd.unwrap() } else { 0.0 });
                body.borrow_mut().insetsSet = true;
            }
            let anc: String = XmlNode::attr(&bp, "anchor");
            if !anc.is_empty() {
                body.borrow_mut().anchor = anc.clone();
                body.borrow_mut().anchorSet = true;
            }
            let wrap: String = XmlNode::attr(&bp, "wrap");
            if wrap == "none" {
                body.borrow_mut().wrap = false;
            }
            let num_col: String = XmlNode::attr(&bp, "numCol");
            if !num_col.is_empty() {
                let nc: Option<i64> = num_col.parse::<i64>().ok();
                let n: i64 = if nc.is_some() { nc.unwrap() } else { 1 };
                if n > 1 {
                    body.borrow_mut().columns = n;
                }
            }
            let spc_col: String = XmlNode::attr(&bp, "spcCol");
            if !spc_col.is_empty() {
                let sc: Option<f64> = spc_col.parse::<f64>().ok();
                body.borrow_mut().columnGapPt = PptxUnits::emuToPt(if sc.is_some() { sc.unwrap() } else { 0.0 });
            }
            // `a:normAutofit` — PowerPoint already worked out how much the text
            // had to shrink to fit this box and wrote the answer down. Drawing
            // it at full size in a box sized for the shrunken version is the
            // overflow a reader that ignores this always has.
            let mut fit: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bp, "normAutofit");
            if fit.is_some() {
                let mut r#fn: Rc<RefCell<XmlNode>> = fit.clone().unwrap();
                let fs: String = XmlNode::attr(&r#fn, "fontScale");
                if !fs.is_empty() {
                    let fv: Option<f64> = fs.parse::<f64>().ok();
                    body.borrow_mut().fontScale = (if fv.is_some() { fv.unwrap() } else { 100000.0 }) / 100000.0;
                }
                let ls2: String = XmlNode::attr(&r#fn, "lnSpcReduction");
                if !ls2.is_empty() {
                    let lv2: Option<f64> = ls2.parse::<f64>().ok();
                    body.borrow_mut().lineScale = 1.0 - (if lv2.is_some() { lv2.unwrap() } else { 0.0 }) / 100000.0;
                }
            }
        }
        self.parseLstStyleDefaults(tx_body, &body);
        let mut paras: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&tx_body, "p");
        let mut i: i64 = 0;
        while i < (paras.len() as i64) {
            let mut pnode: Rc<RefCell<XmlNode>> = paras[i as usize].clone();
            let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            let mut p_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pnode, "pPr");
            if p_pr.is_some() {
                let mut pp: Rc<RefCell<XmlNode>> = p_pr.clone().unwrap();
                let algn: String = XmlNode::attr(&pp, "algn");
                if !algn.is_empty() {
                    para.borrow_mut().align = algn.clone();
                    para.borrow_mut().alignSet = true;
                }
                // The direction, read before the alignment default below
                // because it decides what "unaligned" means.
                let rtl_s: String = XmlNode::attr(&pp, "rtl");
                if !rtl_s.is_empty() {
                    para.borrow_mut().rtl = 0;
                    if rtl_s == "1" || rtl_s == "true" {
                        para.borrow_mut().rtl = 1;
                    }
                }
                // A right-to-left paragraph with no `algn` on it is set
                // against the RIGHT edge of its box. PowerPoint does not write
                // an alignment for that — the direction IS the alignment — so
                // a reader that only looks at `algn` puts every Arabic
                // paragraph in the deck against the left edge.
                if para.borrow().rtl == 1 {
                    if !para.borrow().alignSet {
                        para.borrow_mut().align = "r".to_string();
                    }
                }
                let lvl_s: String = XmlNode::attr(&pp, "lvl");
                if !lvl_s.is_empty() {
                    let lv: Option<i64> = lvl_s.parse::<i64>().ok();
                    para.borrow_mut().level = if lv.is_some() { lv.unwrap() } else { 0 };
                }
                // A stated margin or indent — including a stated zero, which is
                // how a deck cancels the level's indent for one paragraph.
                let mar_l2: String = XmlNode::attr(&pp, "marL");
                if !mar_l2.is_empty() {
                    let mv: Option<f64> = mar_l2.parse::<f64>().ok();
                    para.borrow_mut().marginLeftPt = PptxUnits::emuToPt(if mv.is_some() { mv.unwrap() } else { 0.0 });
                    para.borrow_mut().marginSet = true;
                }
                let ind2: String = XmlNode::attr(&pp, "indent");
                if !ind2.is_empty() {
                    let iv: Option<f64> = ind2.parse::<f64>().ok();
                    para.borrow_mut().indentPt = PptxUnits::emuToPt(if iv.is_some() { iv.unwrap() } else { 0.0 });
                    para.borrow_mut().indentSet = true;
                }
                // A paragraph can state its bullet's colour, size and face as
                // well as its character — and a deck saved out of this reader
                // does exactly that, because there is no master left to inherit
                // them from.
                let mut bu_clr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buClr");
                if bu_clr.is_some() {
                    let mut bcol: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&bu_clr.clone().unwrap());
                    if bcol.is_some() {
                        para.borrow_mut().bulletColor = bcol.clone().unwrap();
                    }
                }
                let mut bu_sz_pct: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buSzPct");
                if bu_sz_pct.is_some() {
                    let bp_1: Option<f64> = XmlNode::attr(&bu_sz_pct.clone().unwrap(), "val").parse::<f64>().ok();
                    para.borrow_mut().bulletSizePct = (if bp_1.is_some() { bp_1.unwrap() } else { 100000.0 }) / 1000.0;
                }
                let mut bu_sz_pts: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buSzPts");
                if bu_sz_pts.is_some() {
                    let bt: Option<f64> = XmlNode::attr(&bu_sz_pts.clone().unwrap(), "val").parse::<f64>().ok();
                    para.borrow_mut().bulletSizePt = PptxUnits::hundredthsPtToPt(if bt.is_some() { bt.unwrap() } else { 1800.0 });
                }
                let mut bu_font: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buFont");
                if bu_font.is_some() {
                    let btf: String = XmlNode::attr(&bu_font.clone().unwrap(), "typeface");
                    if !btf.is_empty() {
                        para.borrow_mut().bulletFont = btf.clone();
                    }
                }
                let mut bu_char: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buChar");
                let mut bu_none: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buNone");
                let mut bu_auto: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "buAutoNum");
                if bu_none.is_some() {
                    para.borrow_mut().hasBullet = false;
                    para.borrow_mut().bulletSet = true;
                } else {
                    if bu_char.is_some() {
                        para.borrow_mut().hasBullet = true;
                        para.borrow_mut().bulletSet = true;
                        let ch: String = XmlNode::attr(&bu_char.clone().unwrap(), "char");
                        if !ch.is_empty() {
                            para.borrow_mut().bulletChar = ch.clone();
                        }
                    } else {
                        if bu_auto.is_some() {
                            let mut ba: Rc<RefCell<XmlNode>> = bu_auto.clone().unwrap();
                            para.borrow_mut().hasBullet = true;
                            para.borrow_mut().bulletAutoNum = true;
                            para.borrow_mut().bulletSet = true;
                            let bsch: String = XmlNode::attr(&ba, "type");
                            if !bsch.is_empty() {
                                para.borrow_mut().bulletScheme = bsch.clone();
                            }
                            let bst: String = XmlNode::attr(&ba, "startAt");
                            if !bst.is_empty() {
                                let bsv: Option<i64> = bst.parse::<i64>().ok();
                                para.borrow_mut().bulletStartAt = if bsv.is_some() { bsv.unwrap() } else { 1 };
                            }
                        } else {
                            if bu_font.is_some() {
                                para.borrow_mut().hasBullet = true;
                                para.borrow_mut().bulletSet = true;
                            }
                        }
                    }
                }
                let mut spc_bef: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "spcBef");
                if spc_bef.is_some() {
                    let mut spc: Rc<RefCell<XmlNode>> = spc_bef.clone().unwrap();
                    let mut pts: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&spc, "spcPts");
                    if pts.is_some() {
                        let pv: Option<f64> = XmlNode::attr(&pts.clone().unwrap(), "val").parse::<f64>().ok();
                        para.borrow_mut().spaceBeforePt = (if pv.is_some() { pv.unwrap() } else { 0.0 }) / 100.0;
                    }
                }
                let mut spc_aft: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "spcAft");
                if spc_aft.is_some() {
                    let mut spa: Rc<RefCell<XmlNode>> = spc_aft.clone().unwrap();
                    let mut pts2: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&spa, "spcPts");
                    if pts2.is_some() {
                        let pv2: Option<f64> = XmlNode::attr(&pts2.clone().unwrap(), "val").parse::<f64>().ok();
                        para.borrow_mut().spaceAfterPt = (if pv2.is_some() { pv2.unwrap() } else { 0.0 }) / 100.0;
                    }
                }
                let mut ln_spc: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pp, "lnSpc");
                if ln_spc.is_some() {
                    let mut lsp: Rc<RefCell<XmlNode>> = ln_spc.clone().unwrap();
                    let mut pct: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&lsp, "spcPct");
                    if pct.is_some() {
                        let pcv: Option<f64> = XmlNode::attr(&pct.clone().unwrap(), "val").parse::<f64>().ok();
                        para.borrow_mut().lineSpacingPct = (if pcv.is_some() { pcv.unwrap() } else { 100000.0 }) / 1000.0;
                    }
                }
            }
            // Runs and FIELDS, in the order the paragraph has them: a page
            // number sits between words often enough that reading the runs
            // first and the fields after would reorder the line.
            let mut j: i64 = 0;
            while j < XmlNode::childCount(&pnode) {
                let mut child: Rc<RefCell<XmlNode>> = XmlNode::childAt(&pnode, j);
                let mut is_run: bool = false;
                let mut is_field: bool = false;
                let mut is_break: bool = false;
                if !child.borrow().isText {
                    if child.borrow().name == "r" {
                        is_run = true;
                    }
                    if child.borrow().name == "fld" {
                        is_field = true;
                    }
                    // `<a:br/>` — shift+Enter. A line inside a paragraph, not
                    // a paragraph of its own: it keeps the paragraph's bullet,
                    // its number and its spacing. Carried as a newline in the
                    // run text, which is what the layout breaks on and what
                    // the writer turns back into a `br`.
                    if child.borrow().name == "br" {
                        is_break = true;
                    }
                }
                if is_break {
                    let mut brk: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                    let mut br_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&child, "rPr");
                    if br_pr.is_some() {
                        self.applyRPrToRun(&br_pr.clone().unwrap(), &brk);
                    }
                    brk.borrow_mut().text = "\n".to_string();
                    para.borrow_mut().runs.push(brk.clone());
                }
                if is_run || is_field {
                    let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                    let mut r_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&child, "rPr");
                    if r_pr.is_some() {
                        self.applyRPrToRun(&r_pr.clone().unwrap(), &run);
                    }
                    let mut t: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&child, "t");
                    if t.is_some() {
                        let mut tn: Rc<RefCell<XmlNode>> = t.clone().unwrap();
                        run.borrow_mut().text = XmlNode::textContent(&tn);
                    }
                    if is_field {
                        run.borrow_mut().field = XmlNode::attr(&child, "type");
                        if run.borrow().field.is_empty() {
                            run.borrow_mut().field = "slidenum".to_string();
                        }
                    }
                    para.borrow_mut().runs.push(run.clone());
                }
                j += 1;
            }
            body.borrow_mut().paragraphs.push(para.clone());
            i += 1;
        }
        body
    }
    fn parsePlaceholderAttrs(&self, sp: &Rc<RefCell<XmlNode>>, shape: &Rc<RefCell<PptxShape>>) {
        let mut nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp, "nvSpPr");
        if nv.is_none() {
            // Pictures use nvPicPr
            nv = XmlNode::firstChild(&sp, "nvPicPr");
        }
        if nv.is_none() {
            return;
        }
        let mut nv_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nv.clone().unwrap(), "nvPr");
        if nv_pr.is_none() {
            return;
        }
        let mut ph: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nv_pr.clone().unwrap(), "ph");
        if ph.is_none() {
            return;
        }
        let mut phn: Rc<RefCell<XmlNode>> = ph.clone().unwrap();
        shape.borrow_mut().isPlaceholder = true;
        let typ: String = XmlNode::attr(&phn, "type");
        if !typ.is_empty() {
            shape.borrow_mut().phType = typ.clone();
        } else {
            // OOXML: omitted type means body
            shape.borrow_mut().phType = "body".to_string();
        }
        let idx_s: String = XmlNode::attr(&phn, "idx");
        if !idx_s.is_empty() {
            let id: Option<i64> = idx_s.parse::<i64>().ok();
            shape.borrow_mut().phIdx = if id.is_some() { id.unwrap() } else { -1 };
        }
    }
    fn parseDirectColor(&self, parent: &Rc<RefCell<XmlNode>>) -> Option<Rc<RefCell<PptxColor>>> {
        let mut srgb: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "srgbClr");
        if srgb.is_some() {
            let hex: String = XmlNode::attr(&srgb.clone().unwrap(), "val");
            if (hex.as_bytes().len() as i64) == 6 {
                let mut hit: Option<Rc<RefCell<PptxColor>>> = Some(PptxColor::srgbOf(hex.clone()));
                return hit;
            }
        }
        let mut sch: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "schemeClr");
        if sch.is_some() {
            let name: String = XmlNode::attr(&sch.clone().unwrap(), "val");
            if !name.is_empty() {
                let mut hit2: Option<Rc<RefCell<PptxColor>>> = Some(PptxColor::schemeOf(name.clone()));
                return hit2;
            }
        }
        let mut via_solid: Option<Rc<RefCell<PptxColor>>> = self.parseColor(parent);
        via_solid
    }
    fn parseGradient(&self, parent: &Rc<RefCell<XmlNode>>) -> Option<Rc<RefCell<PptxGradient>>> {
        let mut gf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&parent, "gradFill");
        if gf.is_none() {
            let mut __missing: Option<Rc<RefCell<PptxGradient>>> = None;
            return __missing;
        }
        let mut gnode: Rc<RefCell<XmlNode>> = gf.clone().unwrap();
        let mut g: Rc<RefCell<PptxGradient>> = Rc::new(RefCell::new(PptxGradient::new()));
        g.borrow_mut().isSet = true;
        let mut lin: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&gnode, "lin");
        if lin.is_some() {
            let ang_s: String = XmlNode::attr(&lin.clone().unwrap(), "ang");
            let ang: Option<f64> = ang_s.parse::<f64>().ok();
            let a: f64 = if ang.is_some() { ang.unwrap() } else { 0.0 };
            if a > 2700000.0 {
                g.borrow_mut().dir = 0;
            } else {
                g.borrow_mut().dir = 1;
            }
        }
        let mut gs_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&gnode, "gsLst");
        if gs_lst.is_some() {
            let mut lst: Rc<RefCell<XmlNode>> = gs_lst.clone().unwrap();
            let mut stops: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&lst, "gs");
            if !stops.is_empty() {
                let mut c0: Option<Rc<RefCell<PptxColor>>> = self.parseDirectColor(&stops[0].clone());
                if c0.is_some() {
                    g.borrow_mut().c1 = c0.clone().unwrap();
                }
            }
            if (stops.len() as i64) > 1 {
                let last: i64 = (stops.len() as i64) - 1;
                let mut c1: Option<Rc<RefCell<PptxColor>>> = self.parseDirectColor(&stops[last as usize].clone());
                if c1.is_some() {
                    g.borrow_mut().c2 = c1.clone().unwrap();
                }
            }
        }
        if !g.borrow().c1.borrow().isSet {
            g.borrow_mut().c1 = PptxColor::srgbOf("4472C4".to_string());
        }
        if !g.borrow().c2.borrow().isSet {
            g.borrow_mut().c2 = PptxColor::srgbOf("FFFFFF".to_string());
        }
        let mut hit: Option<Rc<RefCell<PptxGradient>>> = Some(g.clone());
        hit
    }
    fn parseShadow(&self, sp_pr: &Rc<RefCell<XmlNode>>, shape: &Rc<RefCell<PptxShape>>) {
        let mut eff: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "effectLst");
        if eff.is_none() {
            return;
        }
        let mut outer: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&eff.clone().unwrap(), "outerShdw");
        if outer.is_none() {
            return;
        }
        let mut o: Rc<RefCell<XmlNode>> = outer.clone().unwrap();
        shape.borrow_mut().shadow.borrow_mut().isSet = true;
        let blur_s: String = XmlNode::attr(&o, "blurRad");
        if !blur_s.is_empty() {
            let bd: Option<f64> = blur_s.parse::<f64>().ok();
            shape.borrow_mut().shadow.borrow_mut().blurPt = PptxUnits::emuToPt(if bd.is_some() { bd.unwrap() } else { 0.0 });
        }
        let dist_s: String = XmlNode::attr(&o, "dist");
        let dir_s: String = XmlNode::attr(&o, "dir");
        let mut dist: f64 = 0.0;
        if !dist_s.is_empty() {
            let dd: Option<f64> = dist_s.parse::<f64>().ok();
            dist = PptxUnits::emuToPt(if dd.is_some() { dd.unwrap() } else { 0.0 });
        }
        let mut dir_deg: f64 = 45.0;
        if !dir_s.is_empty() {
            let dr: Option<f64> = dir_s.parse::<f64>().ok();
            dir_deg = PptxUnits::rotToDeg(if dr.is_some() { dr.unwrap() } else { 2700000.0 });
        }
        // Approximate offset from polar dist/dir (dir 0 = right in DrawingML)
        // Use simple quadrant: default down-right.
        shape.borrow_mut().shadow.borrow_mut().offsetXPt = dist * 0.7;
        shape.borrow_mut().shadow.borrow_mut().offsetYPt = dist * 0.7;
        if dir_deg > 90.0 {
            if dir_deg < 180.0 {
                shape.borrow_mut().shadow.borrow_mut().offsetXPt = 0.0 - dist * 0.7;
            }
        }
        let mut col: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&o);
        if col.is_some() {
            shape.borrow_mut().shadow.borrow_mut().color = col.clone().unwrap();
        } else {
            // Often srgbClr is direct child of outerShdw
            let mut srgb: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&o, "srgbClr");
            if srgb.is_some() {
                let hex: String = XmlNode::attr(&srgb.clone().unwrap(), "val");
                if (hex.as_bytes().len() as i64) == 6 {
                    shape.borrow_mut().shadow.borrow_mut().color = PptxColor::srgbOf(hex.clone());
                }
                let mut alpha: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&srgb.clone().unwrap(), "alpha");
                if alpha.is_some() {
                    let av: Option<f64> = XmlNode::attr(&alpha.clone().unwrap(), "val").parse::<f64>().ok();
                    // alpha is 1000ths of a percent (100000 = opaque)
                    shape.borrow_mut()
                        .shadow.borrow_mut()
                        .alpha = (if av.is_some() { av.unwrap() } else { 35000.0 }) / 100000.0;
                }
            }
            let mut sch: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&o, "schemeClr");
            if sch.is_some() {
                shape.borrow_mut().shadow.borrow_mut().color = PptxColor::schemeOf(XmlNode::attr(&sch.clone().unwrap(), "val"));
            }
        }
        if !shape.borrow().shadow.borrow().color.borrow().isSet {
            shape.borrow_mut().shadow.borrow_mut().color = PptxColor::srgbOf("000000".to_string());
        }
    }
    fn parseCustGeomPath(&self, sp_pr: &Rc<RefCell<XmlNode>>, shape: &Rc<RefCell<PptxShape>>) {
        let mut cust: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "custGeom");
        if cust.is_none() {
            return;
        }
        let mut path_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&cust.clone().unwrap(), "pathLst");
        if path_lst.is_none() {
            return;
        }
        // EVERY path, not the first one.
        //
        // This read `firstChild("path")` and stopped, which is correct for the
        // one-ring shapes a viewer draws and wrong for anything with a hole in
        // it: a `<a:pathLst>` with four `<a:path>` under it came back as its
        // outer contour and nothing else. It also silently discarded the other
        // half of every shape this writer now produces.
        let mut paths: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&path_lst.clone().unwrap(), "path");
        if paths.is_empty() {
            return;
        }
        let mut data: String = String::new();
        let mut pi: i64 = 0;
        while pi < (paths.len() as i64) {
            data = format!("{}{}", data, self.custPathData(&paths[pi as usize].clone()));
            pi += 1;
        }
        if data.is_empty() {
            return;
        }
        // One contour per `<a:path>`, kept apart. `pathEnds` is left empty for
        // the ordinary one-ring case so nothing downstream has to look at it.
        let mut rings: Vec<Rc<RefCell<PathRing>>> = PptxGeom::flattenDataRings(&data);
        let mut pts: Vec<f64> = Vec::new();
        let mut ends: Vec<i64> = Vec::new();
        let mut ri: i64 = 0;
        while ri < (rings.len() as i64) {
            let mut ring: Rc<RefCell<PathRing>> = rings[ri as usize].clone();
            let mut k: i64 = 0;
            while k < (ring.borrow().pts.len() as i64) {
                pts.push(ring.borrow().pts[k as usize]);
                k += 1;
            }
            ends.push(pts.len() as i64);
            ri += 1;
        }
        if (pts.len() as i64) >= 4 {
            shape.borrow_mut().hasPath = true;
            shape.borrow_mut().pathUnit = pts.clone();
            if (rings.len() as i64) > 1 {
                shape.borrow_mut().pathEnds = ends.clone();
            }
            // Kept, because the flattening above is about to lose it and the
            // painter cannot tell a filled shape from a drawn line without it.
            shape.borrow_mut().pathClosed = PptxText::index_of(&data, "Z") >= 0;
        }
    }
    /// @internal One `<a:path>` as SVG path data.
    ///
    /// DrawingML's path grammar is not SVG's, but it says the same things, so it
    /// is TRANSLATED here and flattened by EVG's own path stack. That is what
    /// makes `cubicBezTo`, `quadBezTo` and `arcTo` draw at all: they used to be
    /// skipped, so a curved custom shape came out as a polygon of whichever of
    /// its points happened to be straight lines.
    fn custPathData(&self, pnode: &Rc<RefCell<XmlNode>>) -> String {
        let w_s: String = XmlNode::attr(&pnode, "w");
        let h_s: String = XmlNode::attr(&pnode, "h");
        let mut pw: f64 = 1.0;
        let mut ph: f64 = 1.0;
        if !w_s.is_empty() {
            let wd: Option<f64> = w_s.parse::<f64>().ok();
            pw = if wd.is_some() { wd.unwrap() } else { 1.0 };
        }
        if !h_s.is_empty() {
            let hd: Option<f64> = h_s.parse::<f64>().ok();
            ph = if hd.is_some() { hd.unwrap() } else { 1.0 };
        }
        if pw < 1.0 {
            pw = 1.0;
        }
        if ph < 1.0 {
            ph = 1.0;
        }
        let mut data: String = String::new();
        let mut cur_x: f64 = 0.0;
        let mut cur_y: f64 = 0.0;
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&pnode) {
            let mut ch: Rc<RefCell<XmlNode>> = XmlNode::childAt(&pnode, i);
            if !ch.borrow().isText {
                let mut kids: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&ch, "pt");
                if ch.borrow().name == "moveTo" {
                    if (kids.len() as i64) >= 1 {
                        let mut m0: Rc<RefCell<XmlNode>> = kids[0].clone();
                        cur_x = self.parseDoubleAttr(&m0, "x") / pw;
                        cur_y = self.parseDoubleAttr(&m0, "y") / ph;
                        data = format!("{} M {},{}", data, PptxGeom::pathNum(cur_x), PptxGeom::pathNum(cur_y));
                    }
                }
                if ch.borrow().name == "lnTo" {
                    if (kids.len() as i64) >= 1 {
                        let mut l0: Rc<RefCell<XmlNode>> = kids[0].clone();
                        cur_x = self.parseDoubleAttr(&l0, "x") / pw;
                        cur_y = self.parseDoubleAttr(&l0, "y") / ph;
                        data = format!("{} L {},{}", data, PptxGeom::pathNum(cur_x), PptxGeom::pathNum(cur_y));
                    }
                }
                if ch.borrow().name == "cubicBezTo" {
                    if (kids.len() as i64) >= 3 {
                        let mut c1: Rc<RefCell<XmlNode>> = kids[0].clone();
                        let mut c2: Rc<RefCell<XmlNode>> = kids[1].clone();
                        let mut c3: Rc<RefCell<XmlNode>> = kids[2].clone();
                        let x1: f64 = self.parseDoubleAttr(&c1, "x") / pw;
                        let y1: f64 = self.parseDoubleAttr(&c1, "y") / ph;
                        let x2: f64 = self.parseDoubleAttr(&c2, "x") / pw;
                        let y2: f64 = self.parseDoubleAttr(&c2, "y") / ph;
                        cur_x = self.parseDoubleAttr(&c3, "x") / pw;
                        cur_y = self.parseDoubleAttr(&c3, "y") / ph;
                        data = format!("{} C {},{} {},{} {},{}", data, PptxGeom::pathNum(x1), PptxGeom::pathNum(y1), PptxGeom::pathNum(x2), PptxGeom::pathNum(y2), PptxGeom::pathNum(cur_x), PptxGeom::pathNum(cur_y));
                    }
                }
                if ch.borrow().name == "quadBezTo" {
                    if (kids.len() as i64) >= 2 {
                        let mut q1: Rc<RefCell<XmlNode>> = kids[0].clone();
                        let mut q2: Rc<RefCell<XmlNode>> = kids[1].clone();
                        let qx: f64 = self.parseDoubleAttr(&q1, "x") / pw;
                        let qy: f64 = self.parseDoubleAttr(&q1, "y") / ph;
                        cur_x = self.parseDoubleAttr(&q2, "x") / pw;
                        cur_y = self.parseDoubleAttr(&q2, "y") / ph;
                        data = format!("{} Q {},{} {},{}", data, PptxGeom::pathNum(qx), PptxGeom::pathNum(qy), PptxGeom::pathNum(cur_x), PptxGeom::pathNum(cur_y));
                    }
                }
                if ch.borrow().name == "arcTo" {
                    // An arc is stated from where the pen is, as two radii and
                    // two angles; SVG states it as two radii and where the pen
                    // ENDS, plus which of the four arcs between them is meant.
                    let w_r: f64 = self.parseDoubleAttr(&ch, "wR") / pw;
                    let h_r: f64 = self.parseDoubleAttr(&ch, "hR") / ph;
                    let st_deg: f64 = self.parseDoubleAttr(&ch, "stAng") / 60000.0;
                    let sw_deg: f64 = self.parseDoubleAttr(&ch, "swAng") / 60000.0;
                    let rad: f64 = 0.017453292519943295;
                    let st: f64 = st_deg * rad;
                    let en: f64 = (st_deg + sw_deg) * rad;
                    let ccx: f64 = cur_x - w_r * st.cos();
                    let ccy: f64 = cur_y - h_r * st.sin();
                    let ex: f64 = ccx + w_r * en.cos();
                    let ey: f64 = ccy + h_r * en.sin();
                    let mut large: String = "0";
                    let span_abs: f64 = sw_deg.abs();
                    if span_abs > 180.0 {
                        large = "1";
                    }
                    let mut sweep: String = "0";
                    if sw_deg > 0.0 {
                        sweep = "1";
                    }
                    let rx: f64 = w_r.abs();
                    let ry: f64 = h_r.abs();
                    data = format!("{} A {},{} 0 {} {} {},{}", data, PptxGeom::pathNum(rx), PptxGeom::pathNum(ry), large, sweep, PptxGeom::pathNum(ex), PptxGeom::pathNum(ey));
                    cur_x = ex;
                    cur_y = ey;
                }
                if ch.borrow().name == "close" {
                    data = format!("{} Z", data);
                }
            }
            i += 1;
        }
        data
    }
    fn parseSpPr(&self, sp_pr: &Rc<RefCell<XmlNode>>, shape: &Rc<RefCell<PptxShape>>) {
        let mut xfrm: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "xfrm");
        if xfrm.is_some() {
            self.parseXfrm(&xfrm.clone().unwrap(), shape);
        }
        let mut geom: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "prstGeom");
        if geom.is_some() {
            let mut g: Rc<RefCell<XmlNode>> = geom.clone().unwrap();
            let prst: String = XmlNode::attr(&g, "prst");
            if !prst.is_empty() {
                shape.borrow_mut().preset = prst.clone();
            }
        }
        self.parseCustGeomPath(sp_pr, shape);
        let mut fill: Option<Rc<RefCell<PptxColor>>> = self.parseColor(sp_pr);
        if fill.is_some() {
            shape.borrow_mut().fill = fill.clone().unwrap();
        }
        let mut prst_1: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "prstGeom");
        if prst_1.is_some() {
            let mut av: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&prst_1.clone().unwrap(), "avLst");
            if av.is_some() {
                let mut gds: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&av.clone().unwrap(), "gd");
                let mut gi: i64 = 0;
                while gi < (gds.len() as i64) {
                    let mut gd: Rc<RefCell<XmlNode>> = gds[gi as usize].clone();
                    let gname: String = XmlNode::attr(&gd, "name");
                    let fmla: String = XmlNode::attr(&gd, "fmla");
                    let sp: i64 = PptxText::index_of(&fmla, " ");
                    if sp >= 0 {
                        let num: String = fmla[(sp + 1) as usize..(fmla.as_bytes().len() as i64) as usize].to_string();
                        let nv: Option<f64> = num.parse::<f64>().ok();
                        if nv.is_some() {
                            let raw: f64 = nv.unwrap();
                            // EVERY adjustment, under its own name, in the
                            // unit the formulas use — hundred-thousandths.
                            //
                            // Only the one called `adj` was read before, and
                            // only as a fraction for the rounded rectangle.
                            // A callout states four and an arrow two, so a
                            // deck's arrows came out with the author's head
                            // length and the default shaft width.
                            shape.borrow_mut().adjNames.push(gname.clone());
                            shape.borrow_mut().adjValues.push(raw);
                            if gname == "adj" {
                                shape.borrow_mut().adj = raw / 100000.0;
                            }
                        }
                    }
                    gi += 1;
                }
            }
        }
        let mut grad: Option<Rc<RefCell<PptxGradient>>> = self.parseGradient(sp_pr);
        if grad.is_some() {
            shape.borrow_mut().fillGrad = grad.clone().unwrap();
        }
        // noFill explicitly clears
        let mut nf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "noFill");
        if nf.is_some() {
            shape.borrow_mut().fill = Rc::new(RefCell::new(PptxColor::new()));
            shape.borrow_mut().fillGrad = Rc::new(RefCell::new(PptxGradient::new()));
            shape.borrow_mut().noFill = true;
        }
        let mut ln: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp_pr, "ln");
        if ln.is_some() {
            let mut line_node: Rc<RefCell<XmlNode>> = ln.clone().unwrap();
            let w: String = XmlNode::attr(&line_node, "w");
            if !w.is_empty() {
                let wd: Option<f64> = w.parse::<f64>().ok();
                shape.borrow_mut().lineWidthPt = PptxUnits::emuToPt(if wd.is_some() { wd.unwrap() } else { 0.0 });
            }
            let mut lc: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&line_node);
            if lc.is_some() {
                shape.borrow_mut().line = lc.clone().unwrap();
            }
            let mut lnf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&line_node, "noFill");
            if lnf.is_some() {
                shape.borrow_mut().noLine = true;
                shape.borrow_mut().line = Rc::new(RefCell::new(PptxColor::new()));
            }
            // The arrowheads a connector carries. A line with an arrow on it is
            // a different picture from a line without one, and it is these two
            // elements that say so.
            let mut head: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&line_node, "headEnd");
            if head.is_some() {
                shape.borrow_mut().headEnd = XmlNode::attr(&head.clone().unwrap(), "type");
            }
            let mut tail: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&line_node, "tailEnd");
            if tail.is_some() {
                shape.borrow_mut().tailEnd = XmlNode::attr(&tail.clone().unwrap(), "type");
            }
        }
        self.parseShadow(sp_pr, shape);
    }
    fn parseShape(&self, sp: &Rc<RefCell<XmlNode>>) -> Rc<RefCell<PptxShape>> {
        let mut shape: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        shape.borrow_mut().kind = PptxShapeKind::shape();
        let mut nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp, "nvSpPr");
        if nv.is_some() {
            let mut nvn: Rc<RefCell<XmlNode>> = nv.clone().unwrap();
            let mut c_nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nvn, "cNvPr");
            if c_nv.is_some() {
                let mut cn: Rc<RefCell<XmlNode>> = c_nv.clone().unwrap();
                shape.borrow_mut().name = XmlNode::attr(&cn, "name");
                shape.borrow_mut().altText = XmlNode::attr(&cn, "descr");
                let sid: Option<i64> = XmlNode::attr(&cn, "id").parse::<i64>().ok();
                shape.borrow_mut().shapeId = if sid.is_some() { sid.unwrap() } else { 0 };
            }
            // `a:spLocks` is how a deck says a shape is not to be dragged.
            // Any of the three we care about counts as locked, because an
            // editor that let you move a shape it refuses to resize would be
            // the worst of both.
            let mut sp2: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nvn, "cNvSpPr");
            if sp2.is_some() {
                let mut locks: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp2.clone().unwrap(), "spLocks");
                if locks.is_some() {
                    let mut ln: Rc<RefCell<XmlNode>> = locks.clone().unwrap();
                    if XmlNode::attr(&ln, "noMove") == "1" {
                        shape.borrow_mut().locked = true;
                    }
                    if XmlNode::attr(&ln, "noResize") == "1" {
                        shape.borrow_mut().locked = true;
                    }
                    if XmlNode::attr(&ln, "noSelect") == "1" {
                        shape.borrow_mut().locked = true;
                    }
                }
            }
        }
        self.parsePlaceholderAttrs(sp, &shape);
        let mut sp_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp, "spPr");
        if sp_pr.is_some() {
            self.parseSpPr(&sp_pr.clone().unwrap(), &shape);
        }
        self.parseStyle(sp, &shape);
        let mut tx: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sp, "txBody");
        if tx.is_some() {
            shape.borrow_mut().hasText = true;
            shape.borrow_mut().text = self.parseTextBody(&tx.clone().unwrap());
        }
        shape
    }
    fn parsePicture(&self, pic: &Rc<RefCell<XmlNode>>, slide_part: &str) -> Rc<RefCell<PptxShape>> {
        let mut shape: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        shape.borrow_mut().kind = PptxShapeKind::picture();
        let mut nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pic, "nvPicPr");
        if nv.is_some() {
            let mut nvn: Rc<RefCell<XmlNode>> = nv.clone().unwrap();
            let mut c_nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nvn, "cNvPr");
            if c_nv.is_some() {
                let mut cn: Rc<RefCell<XmlNode>> = c_nv.clone().unwrap();
                shape.borrow_mut().name = XmlNode::attr(&cn, "name");
                shape.borrow_mut().altText = XmlNode::attr(&cn, "descr");
                let sid: Option<i64> = XmlNode::attr(&cn, "id").parse::<i64>().ok();
                shape.borrow_mut().shapeId = if sid.is_some() { sid.unwrap() } else { 0 };
            }
        }
        let mut blip_fill: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pic, "blipFill");
        if blip_fill.is_some() {
            let mut bf: Rc<RefCell<XmlNode>> = blip_fill.clone().unwrap();
            let mut blip: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bf, "blip");
            if blip.is_some() {
                let mut b: Rc<RefCell<XmlNode>> = blip.clone().unwrap();
                let mut rid: String = XmlNode::attr(&b, "r:embed");
                if rid.is_empty() {
                    rid = XmlNode::attr(&b, "embed");
                }
                shape.borrow_mut().imageRid = rid.clone();
                if !rid.is_empty() {
                    let _tmp_4599 = OpcPackage::resolveRelationship(
                        &self.pkg,
                        slide_part,
                        &rid
                    );
                    shape.borrow_mut().imagePart = _tmp_4599;
                    if !shape.borrow().imagePart.is_empty() {
                        let _tmp_4602 = OpcPackage::readBinary(&self.pkg, &shape.borrow().imagePart.clone());
                        shape.borrow_mut().imageBytes = _tmp_4602;
                    }
                }
            }
            // the part shown: `a:srcRect`, thousandths of a percent a side
            let mut sr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bf, "srcRect");
            if sr.is_some() {
                let mut r: Rc<RefCell<XmlNode>> = sr.clone().unwrap();
                shape.borrow_mut().cropL = PptxParser::cropOf(&XmlNode::attr(&r, "l"));
                shape.borrow_mut().cropT = PptxParser::cropOf(&XmlNode::attr(&r, "t"));
                shape.borrow_mut().cropR = PptxParser::cropOf(&XmlNode::attr(&r, "r"));
                shape.borrow_mut().cropB = PptxParser::cropOf(&XmlNode::attr(&r, "b"));
            }
        }
        let mut sp_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pic, "spPr");
        if sp_pr.is_some() {
            self.parseSpPr(&sp_pr.clone().unwrap(), &shape);
        }
        shape
    }
    fn parseGroup(&self, grp: &Rc<RefCell<XmlNode>>, slide_part: &str) -> Rc<RefCell<PptxShape>> {
        let mut shape: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        shape.borrow_mut().kind = PptxShapeKind::group();
        let mut nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&grp, "nvGrpSpPr");
        if nv.is_some() {
            let mut nvn: Rc<RefCell<XmlNode>> = nv.clone().unwrap();
            let mut c_nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nvn, "cNvPr");
            if c_nv.is_some() {
                let mut cn: Rc<RefCell<XmlNode>> = c_nv.clone().unwrap();
                shape.borrow_mut().name = XmlNode::attr(&cn, "name");
                shape.borrow_mut().altText = XmlNode::attr(&cn, "descr");
                let sid: Option<i64> = XmlNode::attr(&cn, "id").parse::<i64>().ok();
                shape.borrow_mut().shapeId = if sid.is_some() { sid.unwrap() } else { 0 };
                // …and what the group was GENERATED FROM, when something
                // generated it. See `PptxShape.embedSource`: without this a
                // drawing can be moved and resized and never edited again,
                // because a class diagram and a sequence diagram draw the same
                // rectangles.
                let mut ext_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&cn, "extLst");
                if ext_lst.is_some() {
                    let mut el: Rc<RefCell<XmlNode>> = ext_lst.clone().unwrap();
                    let mut ext: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&el, "ext");
                    if ext.is_some() {
                        let mut ex: Rc<RefCell<XmlNode>> = ext.clone().unwrap();
                        let mut emb: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&ex, "embed");
                        if emb.is_some() {
                            let mut em: Rc<RefCell<XmlNode>> = emb.clone().unwrap();
                            shape.borrow_mut().embedNotation = XmlNode::attr(&em, "notation");
                            // The source is element CONTENT, and `XmlCore`
                            // puts content in TEXT CHILDREN rather than on the
                            // element — so `em.text` is empty here and always
                            // would be.
                            let mut tx: String = String::new();
                            let mut ti: i64 = 0;
                            while ti < (em.borrow().children.len() as i64) {
                                let mut tc: Rc<RefCell<XmlNode>> = em.borrow().children[ti as usize].clone();
                                if tc.borrow().isText {
                                    tx = format!("{}{}", tx, tc.borrow().text);
                                }
                                ti += 1;
                            }
                            shape.borrow_mut().embedSource = tx.clone();
                        }
                    }
                }
            }
        }
        let mut grp_sp_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&grp, "grpSpPr");
        if grp_sp_pr.is_some() {
            let mut gsp: Rc<RefCell<XmlNode>> = grp_sp_pr.clone().unwrap();
            let mut xfrm: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&gsp, "xfrm");
            if xfrm.is_some() {
                self.parseXfrm(&xfrm.clone().unwrap(), &shape);
            }
        }
        // Children: sp / pic / grpSp / cxnSp
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&grp) {
            let mut child: Rc<RefCell<XmlNode>> = XmlNode::childAt(&grp, i);
            if !child.borrow().isText {
                let mut parsed: Option<Rc<RefCell<PptxShape>>> = self.parseTreeChild(&child,
                    slide_part
                );
                if parsed.is_some() {
                    shape.borrow_mut().children.push(parsed.clone().unwrap());
                }
            }
            i += 1;
        }
        shape
    }
    fn parseTableLine(&self, tc_pr: &Rc<RefCell<XmlNode>>, name: &str, out: &Rc<RefCell<PptxTableLine>>) {
        let mut hit: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&tc_pr, name);
        if hit.is_none() {
            return;
        }
        let mut node: Rc<RefCell<XmlNode>> = hit.clone().unwrap();
        out.borrow_mut().isSet = true;
        let w: String = XmlNode::attr(&node, "w");
        if !w.is_empty() {
            let wv: Option<f64> = w.parse::<f64>().ok();
            out.borrow_mut().widthPt = PptxUnits::emuToPt(if wv.is_some() { wv.unwrap() } else { 9525.0 });
        }
        let mut nf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "noFill");
        if nf.is_some() {
            out.borrow_mut().isSet = false;
            return;
        }
        let mut solid: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "solidFill");
        if solid.is_some() {
            let mut col: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&solid.clone().unwrap());
            if col.is_some() {
                out.borrow_mut().color = col.clone().unwrap();
            }
        }
    }
    fn tableMargin(&self, node: &Rc<RefCell<XmlNode>>, name: &str, fallback: f64) -> f64 {
        let v: String = XmlNode::attr(&node, name);
        if v.is_empty() {
            return fallback;
        }
        let d: Option<f64> = v.parse::<f64>().ok();
        if d.is_none() {
            return fallback;
        }
        PptxUnits::emuToPt(d.unwrap())
    }
    fn parseTableCell(&self, tc: &Rc<RefCell<XmlNode>>) -> Rc<RefCell<PptxTableCell>> {
        let mut cell: Rc<RefCell<PptxTableCell>> = Rc::new(RefCell::new(PptxTableCell::new()));
        // A covered cell of a merge draws nothing; the cell that spans over it
        // is the one with the words.
        if XmlNode::attr(&tc, "hMerge") == "1" {
            cell.borrow_mut().merged = true;
        }
        if XmlNode::attr(&tc, "vMerge") == "1" {
            cell.borrow_mut().merged = true;
        }
        let span: Option<i64> = XmlNode::attr(&tc, "gridSpan").parse::<i64>().ok();
        if span.is_some() {
            let sv: i64 = span.unwrap();
            if sv > 1 {
                cell.borrow_mut().gridSpan = sv;
            }
        }
        let rspan: Option<i64> = XmlNode::attr(&tc, "rowSpan").parse::<i64>().ok();
        if rspan.is_some() {
            let rv: i64 = rspan.unwrap();
            if rv > 1 {
                cell.borrow_mut().rowSpan = rv;
            }
        }
        let mut tx_body: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&tc, "txBody");
        if tx_body.is_some() {
            let mut body: Rc<RefCell<PptxTextBody>> = self.parseTextBody(&tx_body.clone().unwrap());
            cell.borrow_mut().body = body.clone();
            cell.borrow_mut().text = body.borrow().plainText();
            if !body.borrow().paragraphs.is_empty() {
                let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[0].clone();
                cell.borrow_mut().align = para.borrow().align.clone();
                if !para.borrow().runs.is_empty() {
                    let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[0].clone();
                    cell.borrow_mut().fontSizePt = run.borrow().fontSizePt;
                    cell.borrow_mut().bold = run.borrow().bold;
                }
            }
        }
        let mut tc_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&tc, "tcPr");
        if tc_pr.is_some() {
            let mut pr: Rc<RefCell<XmlNode>> = tc_pr.clone().unwrap();
            let mut fill: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&pr);
            if fill.is_some() {
                cell.borrow_mut().fill = fill.clone().unwrap();
            }
            let anchor: String = XmlNode::attr(&pr, "anchor");
            if !anchor.is_empty() {
                cell.borrow_mut().anchor = anchor.clone();
            }
            let _tmp_4617 = self.tableMargin(&pr, "marL", cell.borrow().marLPt.clone());
            cell.borrow_mut().marLPt = _tmp_4617;
            let _tmp_4619 = self.tableMargin(&pr, "marR", cell.borrow().marRPt.clone());
            cell.borrow_mut().marRPt = _tmp_4619;
            let _tmp_4621 = self.tableMargin(&pr, "marT", cell.borrow().marTPt.clone());
            cell.borrow_mut().marTPt = _tmp_4621;
            let _tmp_4623 = self.tableMargin(&pr, "marB", cell.borrow().marBPt.clone());
            cell.borrow_mut().marBPt = _tmp_4623;
            self.parseTableLine(&pr, "lnL", &cell.borrow().lnL.clone());
            self.parseTableLine(&pr, "lnR", &cell.borrow().lnR.clone());
            self.parseTableLine(&pr, "lnT", &cell.borrow().lnT.clone());
            self.parseTableLine(&pr, "lnB", &cell.borrow().lnB.clone());
        }
        cell
    }
    fn parseTable(&self, tbl: &Rc<RefCell<XmlNode>>, x: f64, y: f64, w: f64, h: f64) -> Rc<RefCell<PptxShape>> {
        let mut shape: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        shape.borrow_mut().kind = PptxShapeKind::table();
        shape.borrow_mut().name = "Table".to_string();
        shape.borrow_mut().x = x;
        shape.borrow_mut().y = y;
        shape.borrow_mut().width = w;
        shape.borrow_mut().height = h;
        shape.borrow_mut().hasXfrm = true;
        shape.borrow_mut().hasOff = true;
        shape.borrow_mut().hasExt = w > 0.0 && h > 0.0;
        let mut grid: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&tbl, "tblGrid");
        let mut cols: i64 = 0;
        if grid.is_some() {
            let mut gcs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&grid.clone().unwrap(), "gridCol");
            cols = gcs.len() as i64;
            let mut gi: i64 = 0;
            while gi < (gcs.len() as i64) {
                let mut gc: Rc<RefCell<XmlNode>> = gcs[gi as usize].clone();
                shape.borrow_mut().tableColW.push(PptxUnits::emuToPt(self.parseDoubleAttr(&gc,
                    "w"
                )));
                gi += 1;
            }
        }
        let mut rows: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&tbl, "tr");
        shape.borrow_mut().tableRows = rows.len() as i64;
        shape.borrow_mut().tableCols = cols;
        // A table's real size is the grid it states, not the frame around it:
        // PowerPoint treats `p:xfrm/a:ext` as advisory and some writers put a
        // round placeholder there (3000000 × 3000000), which drew every table
        // in those decks as a small square with the text piled up inside it.
        let mut grid_w: f64 = 0.0;
        let mut gw: i64 = 0;
        while gw < (shape.borrow().tableColW.len() as i64) {
            grid_w += shape.borrow().tableColW[gw as usize];
            gw += 1;
        }
        if grid_w > 0.0 {
            shape.borrow_mut().width = grid_w;
        }
        let mut ri: i64 = 0;
        while ri < (rows.len() as i64) {
            let mut tr: Rc<RefCell<XmlNode>> = rows[ri as usize].clone();
            shape.borrow_mut().tableRowH.push(PptxUnits::emuToPt(self.parseDoubleAttr(&tr,
                "h"
            )));
            let mut tcs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&tr, "tc");
            if (tcs.len() as i64) > shape.borrow().tableCols {
                shape.borrow_mut().tableCols = tcs.len() as i64;
            }
            let mut ci: i64 = 0;
            while ci < (tcs.len() as i64) {
                shape.borrow_mut().tableCells.push(self.parseTableCell(&tcs[ci as usize].clone()));
                ci += 1;
            }
            // Pad row if short
            while ci < shape.borrow().tableCols {
                shape.borrow_mut().tableCells.push(Rc::new(RefCell::new(PptxTableCell::new())));
                ci += 1;
            }
            ri += 1;
        }
        let mut grid_h: f64 = 0.0;
        let mut gh: i64 = 0;
        while gh < (shape.borrow().tableRowH.len() as i64) {
            grid_h += shape.borrow().tableRowH[gh as usize];
            gh += 1;
        }
        // A row's stated height is a MINIMUM — it grows to fit its words — so
        // the sum of them wins over the frame only when it is the larger.
        if grid_h > shape.borrow().height {
            shape.borrow_mut().height = grid_h;
        }
        shape
    }
    pub fn parseChartPart(&self, chart_part: &str) -> Rc<RefCell<PptxChart>> {
        let mut chart: Rc<RefCell<PptxChart>> = Rc::new(RefCell::new(PptxChart::new()));
        let xml: String = OpcPackage::readXml(&self.pkg, chart_part);
        if xml.is_empty() {
            return chart;
        }
        let mut root: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, xml.clone());
        if root.is_none() {
            return chart;
        }
        let mut doc: Rc<RefCell<XmlNode>> = root.clone().unwrap();
        let mut title: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "title");
        if title.is_some() {
            let mut t: Rc<RefCell<XmlNode>> = title.clone().unwrap();
            let mut tv: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&t, "t");
            if tv.is_some() {
                chart.borrow_mut().title = XmlNode::textContent(&tv.clone().unwrap());
            }
        }
        let mut bar: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "barChart");
        if bar.is_some() {
            chart.borrow_mut().chartType = "bar".to_string();
        }
        let mut line: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "lineChart");
        if line.is_some() {
            chart.borrow_mut().chartType = "line".to_string();
        }
        let mut pie: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "pieChart");
        if pie.is_some() {
            chart.borrow_mut().chartType = "pie".to_string();
        }
        // Categories from first ser/cat/strRef/strCache or numCache
        let mut ser: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "ser");
        if ser.is_none() {
            return chart;
        }
        let mut snode: Rc<RefCell<XmlNode>> = ser.clone().unwrap();
        let mut series: Rc<RefCell<PptxChartSeries>> = Rc::new(RefCell::new(PptxChartSeries::new()));
        let mut tx: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&snode, "tx");
        if tx.is_some() {
            let mut vt: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&tx.clone().unwrap(), "v");
            if vt.is_some() {
                series.borrow_mut().name = XmlNode::textContent(&vt.clone().unwrap());
            }
        }
        let mut cat: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&snode, "cat");
        if cat.is_some() {
            let mut cache: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&cat.clone().unwrap(), "strCache");
            if cache.is_none() {
                cache = XmlNode::find(&cat.clone().unwrap(), "numCache");
            }
            if cache.is_some() {
                let mut pts2: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&cache.clone().unwrap(), "pt");
                let mut pi: i64 = 0;
                while pi < (pts2.len() as i64) {
                    let mut pt: Rc<RefCell<XmlNode>> = pts2[pi as usize].clone();
                    let mut v: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&pt, "v");
                    if v.is_some() {
                        chart.borrow_mut().categories.push(XmlNode::textContent(&v.clone().unwrap()));
                    }
                    pi += 1;
                }
            }
        }
        let mut val: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&snode, "val");
        if val.is_some() {
            let mut ncache: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&val.clone().unwrap(), "numCache");
            if ncache.is_some() {
                let mut vpts: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&ncache.clone().unwrap(), "pt");
                let mut vi: i64 = 0;
                while vi < (vpts.len() as i64) {
                    let mut vp: Rc<RefCell<XmlNode>> = vpts[vi as usize].clone();
                    let mut vv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&vp, "v");
                    if vv.is_some() {
                        let vd: Option<f64> = XmlNode::textContent(&vv.clone().unwrap()).parse::<f64>().ok();
                        series.borrow_mut().values.push(if vd.is_some() { vd.unwrap() } else { 0.0 });
                    }
                    vi += 1;
                }
            }
        }
        chart.borrow_mut().series.push(series.clone());
        chart
    }
    fn parseGraphicFrame(&self, gf: &Rc<RefCell<XmlNode>>, slide_part: &str) -> Option<Rc<RefCell<PptxShape>>> {
        let mut x: f64 = 0.0;
        let mut y: f64 = 0.0;
        let mut w: f64 = 0.0;
        let mut h: f64 = 0.0;
        // The frame's own name and id: a table or a chart is a shape like any
        // other to everything above here, and it needs to be findable by name.
        let mut frame_name: String = String::new();
        let mut frame_alt: String = String::new();
        let mut frame_id: i64 = 0;
        let mut nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&gf, "nvGraphicFramePr");
        if nv.is_some() {
            let mut c_nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nv.clone().unwrap(), "cNvPr");
            if c_nv.is_some() {
                let mut cn: Rc<RefCell<XmlNode>> = c_nv.clone().unwrap();
                frame_name = XmlNode::attr(&cn, "name");
                frame_alt = XmlNode::attr(&cn, "descr");
                let fid: Option<i64> = XmlNode::attr(&cn, "id").parse::<i64>().ok();
                frame_id = if fid.is_some() { fid.unwrap() } else { 0 };
            }
        }
        let mut xfrm: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&gf, "xfrm");
        if xfrm.is_some() {
            let mut tmp: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            self.parseXfrm(&xfrm.clone().unwrap(), &tmp);
            x = tmp.borrow().x;
            y = tmp.borrow().y;
            w = tmp.borrow().width;
            h = tmp.borrow().height;
        }
        // Table?
        let mut tbl: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&gf, "tbl");
        if tbl.is_some() {
            let mut table: Rc<RefCell<PptxShape>> = self.parseTable(&tbl.clone().unwrap(),
                x,
                y,
                w,
                h
            );
            if !frame_name.is_empty() {
                table.borrow_mut().name = frame_name.clone();
            }
            table.borrow_mut().altText = frame_alt.clone();
            table.borrow_mut().shapeId = frame_id;
            let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(table.clone());
            return hit;
        }
        // Chart via graphicData / chart r:id
        let mut chart_el: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&gf, "chart");
        if chart_el.is_some() {
            let mut rid: String = XmlNode::attr(&chart_el.clone().unwrap(), "r:id");
            if rid.is_empty() {
                rid = XmlNode::attr(&chart_el.clone().unwrap(), "id");
            }
            let mut shape: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            shape.borrow_mut().kind = PptxShapeKind::chart();
            shape.borrow_mut().name = "Chart".to_string();
            if !frame_name.is_empty() {
                shape.borrow_mut().name = frame_name.clone();
            }
            shape.borrow_mut().altText = frame_alt.clone();
            shape.borrow_mut().shapeId = frame_id;
            shape.borrow_mut().x = x;
            shape.borrow_mut().y = y;
            shape.borrow_mut().width = w;
            shape.borrow_mut().height = h;
            shape.borrow_mut().hasXfrm = true;
            shape.borrow_mut().hasOff = true;
            shape.borrow_mut().hasExt = w > 0.0 && h > 0.0;
            if !rid.is_empty() {
                let part: String = OpcPackage::resolveRelationship(
                    &self.pkg,
                    slide_part,
                    &rid
                );
                if !part.is_empty() {
                    shape.borrow_mut().chart = self.parseChartPart(&part);
                }
            }
            let mut hit2: Option<Rc<RefCell<PptxShape>>> = Some(shape.clone());
            return hit2;
        }
        // SmartArt: `dgm:relIds` names the data, layout, style and colours
        let mut rel_ids: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&gf, "relIds");
        if rel_ids.is_some() {
            let mut frame: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            frame.borrow_mut().name = frame_name.clone();
            frame.borrow_mut().altText = frame_alt.clone();
            frame.borrow_mut().shapeId = frame_id;
            frame.borrow_mut().x = x;
            frame.borrow_mut().y = y;
            frame.borrow_mut().width = w;
            frame.borrow_mut().height = h;
            return self.parseDiagram(gf,
                &rel_ids.clone().unwrap(),
                slide_part,
                &frame
            ).clone();
        }
        let mut __missing: Option<Rc<RefCell<PptxShape>>> = None;
        __missing
    }
    /// A SmartArt diagram, drawn from the drawing part PowerPoint saved with it.
    ///
    /// The data part's `dsp:dataModelExt/@relId` names, from the SLIDE, the
    /// drawing part: PowerPoint's own last layout of the diagram, as ordinary
    /// DrawingML shapes in the frame's coordinate space. Those shapes are read
    /// by the same code as a slide's (`dsp:sp` has the same children as `p:sp`)
    /// and become the children of a group whose child space is the frame.
    ///
    /// A diagram with no drawing part — what anything but PowerPoint writes —
    /// comes back as nothing here and stays an opaque element, kept and not
    /// drawn, until the layout engine can draw it (PLAN_SMARTART.md, phase 1).
    fn parseDiagram(&self, gf: &Rc<RefCell<XmlNode>>, rel: &Rc<RefCell<XmlNode>>, slide_part: &str, frame: &Rc<RefCell<PptxShape>>) -> Option<Rc<RefCell<PptxShape>>> {
        let mut none: Option<Rc<RefCell<PptxShape>>> = None;
        let dm: String = XmlNode::attr(&rel, "r:dm");
        if dm.is_empty() {
            return none;
        }
        let data_part: String = OpcPackage::resolveRelationship(
            &self.pkg,
            slide_part,
            &dm
        );
        if data_part.is_empty() {
            return none;
        }
        let mut read: Rc<RefCell<SaReadResult>> = SaDataReader::read(&OpcPackage::readXml(&self.pkg, &data_part));
        if !read.borrow().ok {
            return none;
        }
        let mut model: Rc<RefCell<SaModel>> = read.borrow().model.clone().clone();
        let mut draw_part: String = String::new();
        if !model.borrow().drawingRelId.is_empty() {
            draw_part = OpcPackage::resolveRelationship(
                &self.pkg,
                slide_part,
                &model.borrow().drawingRelId
            );
        }
        let mut tree_opt: Option<Rc<RefCell<XmlNode>>> = None;
        if !draw_part.is_empty() {
            let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, OpcPackage::readXml(&self.pkg, &draw_part));
            if root_opt.is_some() {
                tree_opt = XmlNode::firstChild(&root_opt.clone().unwrap(), "spTree");
            }
        }
        if tree_opt.is_none() {
            // no drawing: the layout engine draws it from the data
            return self.layoutDiagram(gf,
                rel,
                slide_part,
                frame,
                model.clone()
            ).clone();
        }
        let mut tree: Rc<RefCell<XmlNode>> = tree_opt.clone().unwrap();
        let mut shape: Rc<RefCell<PptxShape>> = frame.clone();
        shape.borrow_mut().kind = PptxShapeKind::group();
        shape.borrow_mut().isDiagram = true;
        if shape.borrow().name.is_empty() {
            shape.borrow_mut().name = "Diagram".to_string();
        }
        shape.borrow_mut().hasXfrm = true;
        shape.borrow_mut().hasOff = true;
        let hoist_1 = shape.borrow().width > 0.0 && shape.borrow().height > 0.0;
        shape.borrow_mut().hasExt = hoist_1;
        shape.borrow_mut().chX = 0.0;
        shape.borrow_mut().chY = 0.0;
        let hoist_2 = shape.borrow().width;
        shape.borrow_mut().chW = hoist_2;
        let hoist_3 = shape.borrow().height;
        shape.borrow_mut().chH = hoist_3;
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&tree) {
            let mut child: Rc<RefCell<XmlNode>> = XmlNode::childAt(&tree, i);
            if !child.borrow().isText {
                if child.borrow().name != "nvGrpSpPr" && child.borrow().name != "grpSpPr" {
                    let mut parsed: Option<Rc<RefCell<PptxShape>>> = self.parseTreeChild(&child,
                        &draw_part
                    );
                    if parsed.is_some() {
                        let mut ps: Rc<RefCell<PptxShape>> = parsed.clone().unwrap();
                        let _tmp_4657 = PptxParser::paraPtsOf(
                            &model,
                            &XmlNode::attr(&child, "modelId"),
                            ps.borrow().text.borrow().paragraphs.len() as i64
                        );
                        ps.borrow_mut().diagramParaPts = _tmp_4657;
                        shape.borrow_mut().children.push(ps.clone());
                    }
                }
            }
            i += 1;
        }
        if shape.borrow().children.is_empty() {
            return none;
        }
        shape.borrow().diagramFrame.borrow_mut().set(slide_part.to_string(), gf.borrow().srcStart, gf.borrow().srcEnd);
        shape.borrow_mut().diagramText = model.borrow().outline();
        let hoist_4 = shape.borrow().diagramSignature();
        shape.borrow_mut().diagramSig = hoist_4;
        shape.borrow_mut().diagramHadDrawing = true;
        // what it was made from, so an edit to its text can lay it out again
        // (the engine then draws it; until then, PowerPoint's drawing stands)
        shape.borrow_mut().diagramModelXml = OpcPackage::readXml(&self.pkg, &data_part);
        shape.borrow_mut().diagramLayoutXml = self.diagramLayoutFor(rel, slide_part, &model);
        let mut d0: Rc<RefCell<SaPoint>> = model.borrow().doc();
        shape.borrow_mut().diagramLayoutId = d0.borrow().layoutId.clone();
        shape.borrow_mut().diagramStyleId = d0.borrow().styleId.clone();
        if shape.borrow().diagramStyleId.is_empty() {
            shape.borrow_mut().diagramStyleId = "urn:microsoft.com/office/officeart/2005/8/quickstyle/simple1".to_string();
        }
        shape.borrow_mut().diagramColorsId = d0.borrow().colorsId.clone();
        if !SaStyle::knownColors(&shape.borrow().diagramColorsId.clone()) {
            shape.borrow_mut().diagramColorsId = String::new();
        }
        if shape.borrow().diagramColorsId.is_empty() {
            shape.borrow_mut().diagramColorsId = "urn:microsoft.com/office/officeart/2005/8/colors/accent1_2".to_string();
        }
        let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(shape.clone());
        hit
    }
    /// The layout definition a diagram can be laid out again with: the
    /// engine's own for the id its data names, else the one saved in the file
    /// when the engine reads it, else none.
    fn diagramLayoutFor(&self, rel: &Rc<RefCell<XmlNode>>, slide_part: &str, model: &Rc<RefCell<SaModel>>) -> String {
        let builtin: String = SaBuiltins::layoutXml(&SaLayoutDef::shortOf(&model.borrow().layoutId()));
        if !builtin.is_empty() {
            return builtin;
        }
        let lo: String = XmlNode::attr(&rel, "r:lo");
        if lo.is_empty() {
            return String::new();
        }
        let lo_part: String = OpcPackage::resolveRelationship(
            &self.pkg,
            slide_part,
            &lo
        );
        if lo_part.is_empty() {
            return String::new();
        }
        let lo_xml: String = OpcPackage::readXml(&self.pkg, &lo_part);
        let mut lr: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&lo_xml);
        if lr.borrow().ok {
            return lo_xml;
        }
        String::new()
    }
    /// A diagram with no drawing part — what anything but PowerPoint writes —
    /// laid out by the engine (smartart/, PLAN_SMARTART.md): the layout the data
    /// names, from the engine's own definitions when it has one for that id,
    /// else from the definition saved in the file, else vList2. The result is
    /// the same group a drawing part gives, so it is drawn, edited and saved
    /// the same way.
    fn layoutDiagram(&self, gf: &Rc<RefCell<XmlNode>>, rel: &Rc<RefCell<XmlNode>>, slide_part: &str, frame: &Rc<RefCell<PptxShape>>, model: Rc<RefCell<SaModel>>) -> Option<Rc<RefCell<PptxShape>>> {
        let mut none: Option<Rc<RefCell<PptxShape>>> = None;
        if frame.borrow().width <= 0.0 || frame.borrow().height <= 0.0 {
            return none;
        }
        let mut opts: Rc<RefCell<SaOptions>> = Rc::new(RefCell::new(SaOptions::new()));
        let builtin: String = SaBuiltins::layoutXml(&SaLayoutDef::shortOf(&model.borrow().layoutId()));
        if builtin.is_empty() {
            let lo: String = XmlNode::attr(&rel, "r:lo");
            if !lo.is_empty() {
                let lo_part: String = OpcPackage::resolveRelationship(
                    &self.pkg,
                    slide_part,
                    &lo
                );
                if !lo_part.is_empty() {
                    let lo_xml: String = OpcPackage::readXml(&self.pkg, &lo_part);
                    let mut lr: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&lo_xml);
                    if lr.borrow().ok {
                        opts.borrow_mut().layoutXml = lo_xml.clone();
                    }
                }
            }
        }
        let mut _tmp_4672 = frame.borrow().width.clone();
        let mut _tmp_4673 = frame.borrow().height.clone();
        let mut res: Rc<RefCell<SaResult>> = SaEngine::layoutModel(
            model.clone(),
            &opts,
            _tmp_4672,
            _tmp_4673
        );
        if !res.borrow().ok {
            return none;
        }
        let mut shape: Rc<RefCell<PptxShape>> = res.borrow().shape.clone().clone();
        shape.borrow_mut().name = frame.borrow().name.clone();
        if shape.borrow().name.is_empty() {
            shape.borrow_mut().name = "Diagram".to_string();
        }
        shape.borrow_mut().altText = frame.borrow().altText.clone();
        shape.borrow_mut().shapeId = frame.borrow().shapeId;
        shape.borrow_mut().x = frame.borrow().x;
        shape.borrow_mut().y = frame.borrow().y;
        shape.borrow_mut().width = frame.borrow().width;
        shape.borrow_mut().height = frame.borrow().height;
        shape.borrow().diagramFrame.borrow_mut().set(slide_part.to_string(), gf.borrow().srcStart, gf.borrow().srcEnd);
        let hoist_5 = shape.borrow().diagramSignature();
        shape.borrow_mut().diagramSig = hoist_5;
        let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(shape.clone());
        hit
    }
    fn parseTreeChild(&self, node: &Rc<RefCell<XmlNode>>, slide_part: &str) -> Option<Rc<RefCell<PptxShape>>> {
        if node.borrow().name == "sp" {
            let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(self.parseShape(node));
            return hit;
        }
        if node.borrow().name == "pic" {
            let mut hit2: Option<Rc<RefCell<PptxShape>>> = Some(self.parsePicture(node,
                slide_part
            ));
            return hit2;
        }
        if node.borrow().name == "grpSp" {
            let mut hit3: Option<Rc<RefCell<PptxShape>>> = Some(self.parseGroup(node,
                slide_part
            ));
            return hit3;
        }
        if node.borrow().name == "cxnSp" {
            let mut shape: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            shape.borrow_mut().kind = PptxShapeKind::connector();
            let mut nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "nvCxnSpPr");
            if nv.is_some() {
                let mut c_nv: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&nv.clone().unwrap(), "cNvPr");
                if c_nv.is_some() {
                    let mut cn: Rc<RefCell<XmlNode>> = c_nv.clone().unwrap();
                    shape.borrow_mut().name = XmlNode::attr(&cn, "name");
                    shape.borrow_mut().altText = XmlNode::attr(&cn, "descr");
                    let sid: Option<i64> = XmlNode::attr(&cn, "id").parse::<i64>().ok();
                    shape.borrow_mut().shapeId = if sid.is_some() { sid.unwrap() } else { 0 };
                }
            }
            let mut sp_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "spPr");
            if sp_pr.is_some() {
                self.parseSpPr(&sp_pr.clone().unwrap(), &shape);
            }
            self.parseStyle(node, &shape);
            let mut hit4: Option<Rc<RefCell<PptxShape>>> = Some(shape.clone());
            return hit4;
        }
        if node.borrow().name == "graphicFrame" {
            return self.parseGraphicFrame(node, slide_part);
        }
        let mut __missing: Option<Rc<RefCell<PptxShape>>> = None;
        __missing
    }
    /// The shape tree, in order — and everything in it that is not a shape.
    ///
    /// A child this reader does not know is not noise. It is SmartArt, an
    /// embedded object, ink, or an `mc:AlternateContent` block holding a
    /// rendering of something newer than this reader. Walking past it is right;
    /// FORGETTING it is what makes a save delete it, so each one is remembered
    /// as the span it occupies in the part, and where in z-order it sat.
    ///
    /// `nvGrpSpPr` and `grpSpPr` are the tree's own properties rather than
    /// children of it, and the writer emits its own; they are the two things
    /// here that are genuinely skipped.
    fn parseSpTreeInto(&self, tree: &Rc<RefCell<XmlNode>>, part_path: &str, mut shapes: &mut Vec<Rc<RefCell<PptxShape>>>, mut opaque: &mut Vec<Rc<RefCell<PptxOpaque>>>) {
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&tree) {
            let mut child: Rc<RefCell<XmlNode>> = XmlNode::childAt(&tree, i);
            if !child.borrow().isText {
                if child.borrow().name != "nvGrpSpPr" && child.borrow().name != "grpSpPr" {
                    let mut parsed: Option<Rc<RefCell<PptxShape>>> = self.parseTreeChild(&child,
                        part_path
                    );
                    if parsed.is_some() {
                        shapes.push(parsed.clone().unwrap());
                    } else {
                        let mut keep: Rc<RefCell<PptxOpaque>> = Rc::new(RefCell::new(PptxOpaque::new()));
                        keep.borrow_mut().name = child.borrow().name.clone();
                        keep.borrow_mut().afterShapes = shapes.len() as i64;
                        keep.borrow().source.borrow_mut().set(part_path.to_string(), child.borrow().srcStart, child.borrow().srcEnd);
                        // A span that came back empty is a span that cannot be
                        // put back, and an empty one written out would be a
                        // silent hole rather than a preserved element.
                        if keep.borrow().source.borrow().known {
                            opaque.push(keep.clone());
                        }
                    }
                }
            }
            i += 1;
        }
    }
    pub fn parseSheet(&self, part_path: String) -> Rc<RefCell<PptxSheet>> {
        let mut sheet: Rc<RefCell<PptxSheet>> = Rc::new(RefCell::new(PptxSheet::new()));
        sheet.borrow_mut().partPath = part_path.clone();
        let xml: String = OpcPackage::readXml(&self.pkg, &part_path);
        if xml.is_empty() {
            return sheet;
        }
        let mut root: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, xml.clone());
        if root.is_none() {
            return sheet;
        }
        let mut doc: Rc<RefCell<XmlNode>> = root.clone().unwrap();
        let mut c_sld: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "cSld");
        if c_sld.is_some() {
            let mut cs: Rc<RefCell<XmlNode>> = c_sld.clone().unwrap();
            let mut bg: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&cs, "bg");
            if bg.is_some() {
                let mut bg_node: Rc<RefCell<XmlNode>> = bg.clone().unwrap();
                let mut bg_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bg_node, "bgPr");
                if bg_pr.is_some() {
                    let mut bp: Rc<RefCell<XmlNode>> = bg_pr.clone().unwrap();
                    let mut col: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&bp);
                    if col.is_some() {
                        sheet.borrow_mut().bg = col.clone().unwrap();
                    }
                    // …and a PICTURE behind the slide, which is what a company
                    // template is mostly made of and what this used to read
                    // past on its way to the colour.
                    let mut bf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bp, "blipFill");
                    if bf.is_some() {
                        let mut bfn: Rc<RefCell<XmlNode>> = bf.clone().unwrap();
                        let mut blip: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bfn, "blip");
                        if blip.is_some() {
                            let mut bl: Rc<RefCell<XmlNode>> = blip.clone().unwrap();
                            let rid: String = XmlNode::attr(&bl, "r:embed");
                            if !rid.is_empty() {
                                let part: String = OpcPackage::resolveRelationship(
                                    &self.pkg,
                                    &part_path,
                                    &rid
                                );
                                if !part.is_empty() {
                                    sheet.borrow_mut().bgImageName = part.clone();
                                    sheet.borrow_mut().bgImage = OpcPackage::readBinary(&self.pkg, &part);
                                }
                            }
                        }
                    }
                }
            }
            let mut sp_tree: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&cs, "spTree");
            if sp_tree.is_some() {
                let mut __arg_0 = sheet.borrow().shapes.clone();
                let mut __arg_1 = sheet.borrow().opaque.clone();
                self.parseSpTreeInto(&sp_tree.clone().unwrap(),
                    &part_path,
                    &mut __arg_0,
                    &mut __arg_1
                );
                sheet.borrow_mut().shapes = __arg_0;
                sheet.borrow_mut().opaque = __arg_1;
            }
        }
        // `p:txStyles` — a master's own typography, in three list styles. Most
        // placeholders in most decks state nothing at all and take everything
        // from here, so a reader that skips it draws a deck in its own defaults
        // rather than in the deck's.
        let mut tx: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&doc, "txStyles");
        if tx.is_some() {
            let mut t: Rc<RefCell<XmlNode>> = tx.clone().unwrap();
            let mut ttl: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&t, "titleStyle");
            if ttl.is_some() {
                self.parseListStyle(&ttl.clone().unwrap(), &sheet.borrow().titleStyle.clone());
            }
            let mut bdy: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&t, "bodyStyle");
            if bdy.is_some() {
                self.parseListStyle(&bdy.clone().unwrap(), &sheet.borrow().bodyStyle.clone());
            }
            let mut oth: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&t, "otherStyle");
            if oth.is_some() {
                self.parseListStyle(&oth.clone().unwrap(), &sheet.borrow().otherStyle.clone());
            }
        }
        sheet
    }
    pub fn ensureSheet(&self, mut cache: &mut HashMap<String,Rc<RefCell<PptxSheet>>>, part_path: &str) -> Option<Rc<RefCell<PptxSheet>>> {
        if part_path.is_empty() {
            let mut __missing: Option<Rc<RefCell<PptxSheet>>> = None;
            return __missing;
        }
        let mut existing: Option<Rc<RefCell<PptxSheet>>> = cache.get(part_path).cloned();
        if existing.is_some() {
            return existing;
        }
        let mut sheet: Rc<RefCell<PptxSheet>> = self.parseSheet(part_path.to_string());
        cache.insert(part_path.to_string(), sheet.clone());
        let mut hit: Option<Rc<RefCell<PptxSheet>>> = Some(sheet.clone());
        hit
    }
    /// The speaker's notes, which live in a part of their own that the slide
    /// points at. Only the body placeholder is notes — a notes slide also holds
    /// a thumbnail of the slide and its number, and neither is anything anyone
    /// typed.
    pub fn parseNotes(&self, slide: &Rc<RefCell<PptxSlide>>) {
        let mut _tmp_4698 = slide.borrow().partPath.clone();
        let part: String = OpcPackage::findRelationshipByType(
            &self.pkg,
            &_tmp_4698,
            "/notesSlide"
        );
        if part.is_empty() {
            return;
        }
        slide.borrow_mut().notesPart = part.clone();
        let xml: String = OpcPackage::readXml(&self.pkg, &part);
        if xml.is_empty() {
            return;
        }
        let mut root: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, xml.clone());
        if root.is_none() {
            return;
        }
        let mut doc: Rc<RefCell<XmlNode>> = root.clone().unwrap();
        let mut tree: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "spTree");
        if tree.is_none() {
            return;
        }
        let mut shapes: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&tree.clone().unwrap(), "sp");
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sp: Rc<RefCell<XmlNode>> = shapes[i as usize].clone();
            let mut shape: Rc<RefCell<PptxShape>> = self.parseShape(&sp);
            let mut is_body: bool = false;
            if shape.borrow().isPlaceholder {
                if shape.borrow().phType == "body" {
                    is_body = true;
                }
            }
            if is_body {
                slide.borrow_mut().notes = shape.clone();
                slide.borrow_mut().notes.borrow_mut().inherited = false;
                return;
            }
            i += 1;
        }
    }
    /// Children of `<p:sld>` this model has no room for, split at the point
    /// where the transition and the timing go. The schema orders them
    /// `cSld, clrMapOvr, transition, timing, extLst`, so an element kept from
    /// before that point has to go back before it and one from after has to go
    /// back after — putting them all at one end writes a part PowerPoint
    /// rejects.
    fn parseSlideLevel(&self, slide: &Rc<RefCell<PptxSlide>>, part_path: &str, doc: &Rc<RefCell<XmlNode>>) {
        // Where the modelled middle is. Anything at or after it is "tail".
        let mut pivot: i64 = XmlNode::childCount(&doc);
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&doc) {
            let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&doc, i);
            if !c.borrow().isText {
                if c.borrow().name == "transition" || c.borrow().name == "timing" {
                    if i < pivot {
                        pivot = i;
                    }
                }
            }
            i += 1;
        }
        let mut j: i64 = 0;
        while j < XmlNode::childCount(&doc) {
            let mut ch: Rc<RefCell<XmlNode>> = XmlNode::childAt(&doc, j);
            if !ch.borrow().isText {
                let mut modelled: bool = false;
                if ch.borrow().name == "cSld" {
                    modelled = true;
                }
                if ch.borrow().name == "transition" {
                    modelled = true;
                }
                if ch.borrow().name == "timing" {
                    modelled = true;
                }
                if !modelled {
                    let mut keep: Rc<RefCell<PptxOpaque>> = Rc::new(RefCell::new(PptxOpaque::new()));
                    keep.borrow_mut().name = ch.borrow().name.clone();
                    keep.borrow_mut().afterShapes = j;
                    keep.borrow().source.borrow_mut().set(part_path.to_string(), ch.borrow().srcStart, ch.borrow().srcEnd);
                    if keep.borrow().source.borrow().known {
                        if j < pivot {
                            slide.borrow_mut().sldHead.push(keep.clone());
                        } else {
                            slide.borrow_mut().sldTail.push(keep.clone());
                        }
                    }
                }
            }
            j += 1;
        }
    }
    pub fn parseSlide(&self, part_path: String, width: f64, height: f64) -> Rc<RefCell<PptxSlide>> {
        let mut slide: Rc<RefCell<PptxSlide>> = Rc::new(RefCell::new(PptxSlide::new()));
        slide.borrow_mut().partPath = part_path.clone();
        slide.borrow_mut().width = width;
        slide.borrow_mut().height = height;
        let xml: String = OpcPackage::readXml(&self.pkg, &part_path);
        if xml.is_empty() {
            return slide;
        }
        let mut root: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, xml.clone());
        if root.is_none() {
            return slide;
        }
        let mut doc: Rc<RefCell<XmlNode>> = root.clone().unwrap();
        // Background
        let mut c_sld: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "cSld");
        if c_sld.is_some() {
            let mut cs: Rc<RefCell<XmlNode>> = c_sld.clone().unwrap();
            let mut bg: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&cs, "bg");
            if bg.is_some() {
                let mut bg_node: Rc<RefCell<XmlNode>> = bg.clone().unwrap();
                let mut bg_pr: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bg_node, "bgPr");
                if bg_pr.is_some() {
                    let mut bp: Rc<RefCell<XmlNode>> = bg_pr.clone().unwrap();
                    let mut col: Option<Rc<RefCell<PptxColor>>> = self.parseColor(&bp);
                    if col.is_some() {
                        slide.borrow_mut().bg = col.clone().unwrap();
                    }
                    // …and the picture, which is what this read past on its
                    // way to the colour.
                    let mut bf: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bp, "blipFill");
                    if bf.is_some() {
                        let mut bfn: Rc<RefCell<XmlNode>> = bf.clone().unwrap();
                        let mut blip: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&bfn, "blip");
                        if blip.is_some() {
                            let mut bl: Rc<RefCell<XmlNode>> = blip.clone().unwrap();
                            let rid: String = XmlNode::attr(&bl, "r:embed");
                            if !rid.is_empty() {
                                let part: String = OpcPackage::resolveRelationship(
                                    &self.pkg,
                                    &part_path,
                                    &rid
                                );
                                if !part.is_empty() {
                                    slide.borrow_mut().bgImageName = part.clone();
                                    slide.borrow_mut().bgImage = OpcPackage::readBinary(&self.pkg, &part);
                                }
                            }
                        }
                    }
                }
            }
            let mut sp_tree: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&cs, "spTree");
            if sp_tree.is_some() {
                let mut __arg_0 = slide.borrow().shapes.clone();
                let mut __arg_1 = slide.borrow().opaque.clone();
                self.parseSpTreeInto(&sp_tree.clone().unwrap(),
                    &part_path,
                    &mut __arg_0,
                    &mut __arg_1
                );
                slide.borrow_mut().shapes = __arg_0;
                slide.borrow_mut().opaque = __arg_1;
            }
        }
        // Everything else `<p:sld>` holds. `cSld`, the transition and the
        // timing are modelled; a colour map override, an extension list, a
        // custom data list or an `mc:AlternateContent` block are not, and a
        // rewrite used to drop all of them — including replacing a real colour
        // map override with a plain one.
        self.parseSlideLevel(&slide, &part_path, &doc);
        // How the slide arrives, and what is built on it once it has.
        self.parseTransition(&slide, &doc);
        let mut timing: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "timing");
        if timing.is_some() {
            self.parseTiming(&slide, &timing.clone().unwrap());
        }
        // Layout relationship
        let _tmp_4710 = OpcPackage::findRelationshipByType(
            &self.pkg,
            &part_path,
            "/slideLayout"
        );
        slide.borrow_mut().layoutPart = _tmp_4710;
        self.parseNotes(&slide);
        slide
    }
    /// `p:transition` names its effect by the element inside it — `<p:fade/>`,
    /// `<p:push dir="u"/>` — with the speed as a word and the real length, when
    /// a writer states one, in milliseconds out in the extension list.
    fn parseTransition(&self, slide: &Rc<RefCell<PptxSlide>>, doc: &Rc<RefCell<XmlNode>>) {
        let mut hit: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "transition");
        if hit.is_none() {
            return;
        }
        let mut node: Rc<RefCell<XmlNode>> = hit.clone().unwrap();
        let mut tr: Rc<RefCell<PptxTransition>> = slide.borrow().transition.clone().clone();
        tr.borrow_mut().isSet = true;
        let mut i: i64 = 0;
        while i < (node.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<XmlNode>> = node.borrow().children[i as usize].clone();
            if !c.borrow().isText {
                if tr.borrow().kind.is_empty() {
                    if c.borrow().name != "extLst" {
                        tr.borrow_mut().kind = c.borrow().name.clone();
                        tr.borrow_mut().dir = XmlNode::attr(&c, "dir");
                    }
                }
            }
            i += 1;
        }
        if tr.borrow().kind.is_empty() {
            tr.borrow_mut().kind = "fade".to_string();
        }
        let spd: String = XmlNode::attr(&node, "spd");
        if spd == "slow" {
            tr.borrow_mut().seconds = 1.5;
        }
        if spd == "med" {
            tr.borrow_mut().seconds = 0.75;
        }
        if spd == "fast" {
            tr.borrow_mut().seconds = 0.4;
        }
        // A stated duration beats the speed word.
        let mut dur: String = XmlNode::attr(&node, "p14:dur");
        if dur.is_empty() {
            let mut ext: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&node, "dur");
            if ext.is_some() {
                dur = XmlNode::attr(&ext.clone().unwrap(), "val");
            }
        }
        if !dur.is_empty() {
            let ms: Option<f64> = dur.parse::<f64>().ok();
            if ms.is_some() {
                let v: f64 = ms.unwrap();
                if v > 0.0 {
                    tr.borrow_mut().seconds = v / 1000.0;
                }
            }
        }
        let click: String = XmlNode::attr(&node, "advClick");
        if click == "0" {
            tr.borrow_mut().onClick = false;
        }
        let adv: String = XmlNode::attr(&node, "advTm");
        if !adv.is_empty() {
            let at: Option<f64> = adv.parse::<f64>().ok();
            if at.is_some() {
                tr.borrow_mut().afterSeconds = at.unwrap() / 1000.0;
            }
        }
    }
    /// A slide's build, read out of `p:timing`. The tree is deep — a sequence of
    /// click groups, each a tree of parallel and sequential nodes — and what is
    /// wanted from it is flat: which shape, which paragraph, what effect, and
    /// whether it waits for a click. So the walk looks for the INNERMOST `p:par`
    /// that names a shape, which is one effect, and reads the answer off it.
    fn parseTiming(&self, slide: &Rc<RefCell<PptxSlide>>, node: &Rc<RefCell<XmlNode>>) {
        self.collectBuild(slide, node);
    }
    fn collectBuild(&self, slide: &Rc<RefCell<PptxSlide>>, node: &Rc<RefCell<XmlNode>>) -> bool {
        let mut deeper: bool = false;
        let mut i: i64 = 0;
        while i < (node.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<XmlNode>> = node.borrow().children[i as usize].clone();
            if !c.borrow().isText {
                if self.collectBuild(slide, &c) {
                    deeper = true;
                }
            }
            i += 1;
        }
        if deeper {
            return true;
        }
        if node.borrow().name != "par" {
            return false;
        }
        let mut tgt: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&node, "spTgt");
        if tgt.is_none() {
            return false;
        }
        let mut sp: Rc<RefCell<XmlNode>> = tgt.clone().unwrap();
        let spid: Option<i64> = XmlNode::attr(&sp, "spid").parse::<i64>().ok();
        if spid.is_none() {
            return false;
        }
        let mut step: Rc<RefCell<PptxBuildStep>> = Rc::new(RefCell::new(PptxBuildStep::new()));
        step.borrow_mut().shapeId = spid.unwrap();
        let mut eff: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&node, "animEffect");
        if eff.is_some() {
            let filter: String = XmlNode::attr(&eff.clone().unwrap(), "filter");
            if !filter.is_empty() {
                step.borrow_mut().kind = filter.clone();
            }
        }
        let mut rng: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&node, "pRg");
        if rng.is_some() {
            let st: Option<i64> = XmlNode::attr(&rng.clone().unwrap(), "st").parse::<i64>().ok();
            if st.is_some() {
                step.borrow_mut().paragraph = st.unwrap();
            }
        }
        let mut ctn: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "cTn");
        if ctn.is_some() {
            let mut cn: Rc<RefCell<XmlNode>> = ctn.clone().unwrap();
            let kind: String = XmlNode::attr(&cn, "nodeType");
            if kind == "withEffect" {
                step.borrow_mut().start = "with".to_string();
            }
            if kind == "afterEffect" {
                step.borrow_mut().start = "after".to_string();
            }
            let dur: String = XmlNode::attr(&cn, "dur");
            if !dur.is_empty() {
                let ms: Option<f64> = dur.parse::<f64>().ok();
                if ms.is_some() {
                    let v: f64 = ms.unwrap();
                    if v > 0.0 {
                        step.borrow_mut().seconds = v / 1000.0;
                    }
                }
            }
        }
        slide.borrow_mut().build.push(step.clone());
        true
    }
    /// `a:fmtScheme`: the fills, outlines and effects that a shape's `<p:style>`
    /// indexes into. Every colour in here is the placeholder `phClr` carrying
    /// the modifiers that make the style what it is, so what is stored is the
    /// MODIFIERS — the colour itself arrives from the shape.
    fn parseFormatScheme(&self, theme: &Rc<RefCell<PptxTheme>>, doc: &Rc<RefCell<XmlNode>>) {
        let mut fmt: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "fmtScheme");
        if fmt.is_none() {
            return;
        }
        let mut node: Rc<RefCell<XmlNode>> = fmt.clone().unwrap();
        let mut fills: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "fillStyleLst");
        if fills.is_some() {
            let mut fl: Rc<RefCell<XmlNode>> = fills.clone().unwrap();
            let mut i: i64 = 0;
            while i < XmlNode::childCount(&fl) {
                let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&fl, i);
                if !c.borrow().isText {
                    theme.borrow_mut().fillStyles.push(self.parseFillStyle(&c));
                }
                i += 1;
            }
        }
        let mut lines: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "lnStyleLst");
        if lines.is_some() {
            let mut ll: Rc<RefCell<XmlNode>> = lines.clone().unwrap();
            let mut k: i64 = 0;
            while k < XmlNode::childCount(&ll) {
                let mut c2: Rc<RefCell<XmlNode>> = XmlNode::childAt(&ll, k);
                if !c2.borrow().isText {
                    let mut st: Rc<RefCell<PptxLineStyle>> = Rc::new(RefCell::new(PptxLineStyle::new()));
                    st.borrow_mut().isSet = true;
                    let w: String = XmlNode::attr(&c2, "w");
                    if !w.is_empty() {
                        let wv: Option<f64> = w.parse::<f64>().ok();
                        st.borrow_mut().widthPt = PptxUnits::emuToPt(if wv.is_some() { wv.unwrap() } else { 9525.0 });
                    }
                    let mut solid: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&c2, "solidFill");
                    if solid.is_some() {
                        let mut col: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&solid.clone().unwrap());
                        if col.is_some() {
                            st.borrow_mut().color = col.clone().unwrap();
                        }
                    }
                    theme.borrow_mut().lineStyles.push(st.clone());
                }
                k += 1;
            }
        }
        let mut effects: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "effectStyleLst");
        if effects.is_some() {
            let mut el: Rc<RefCell<XmlNode>> = effects.clone().unwrap();
            let mut e: i64 = 0;
            while e < XmlNode::childCount(&el) {
                let mut c3: Rc<RefCell<XmlNode>> = XmlNode::childAt(&el, e);
                if !c3.borrow().isText {
                    let mut shadow: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&c3, "outerShdw");
                    theme.borrow_mut().effectShadows.push(shadow.is_some());
                }
                e += 1;
            }
        }
    }
    fn parseFillStyle(&self, node: &Rc<RefCell<XmlNode>>) -> Rc<RefCell<PptxFillStyle>> {
        let mut st: Rc<RefCell<PptxFillStyle>> = Rc::new(RefCell::new(PptxFillStyle::new()));
        st.borrow_mut().isSet = true;
        if node.borrow().name == "noFill" {
            st.borrow_mut().kind = 2;
            return st;
        }
        if node.borrow().name == "gradFill" {
            st.borrow_mut().kind = 1;
            let mut stops: Vec<Rc<RefCell<PptxColor>>> = self.gradientStops(node);
            if !stops.is_empty() {
                st.borrow_mut().c1 = stops[0].clone();
                st.borrow_mut().c2 = stops[((stops.len() as i64) - 1) as usize].clone();
            }
            st.borrow_mut().dir = PptxParser::gradientDir(node);
            return st;
        }
        // Anything else is treated as a solid: a pattern or a picture fill in a
        // theme is rare, and one flat colour is a better answer than nothing.
        st.borrow_mut().kind = 0;
        let mut col: Option<Rc<RefCell<PptxColor>>> = self.colorIn(node);
        if col.is_some() {
            st.borrow_mut().solid = col.clone().unwrap();
        } else {
            st.borrow_mut().solid = PptxColor::schemeOf("phClr".to_string());
        }
        st
    }
    fn gradientStops(&self, node: &Rc<RefCell<XmlNode>>) -> Vec<Rc<RefCell<PptxColor>>> {
        let mut out: Vec<Rc<RefCell<PptxColor>>> = Vec::new();
        let mut lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "gsLst");
        if lst.is_none() {
            return out;
        }
        let mut gl: Rc<RefCell<XmlNode>> = lst.clone().unwrap();
        let mut gs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&gl, "gs");
        let mut i: i64 = 0;
        while i < (gs.len() as i64) {
            let mut col: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&gs[i as usize].clone());
            if col.is_some() {
                out.push(col.clone().unwrap());
            }
            i += 1;
        }
        out
    }
    /// `<p:style>` — the shape asking for a theme fill, outline, effect and text
    /// colour rather than stating its own. Extremely common: every shape a user
    /// inserts and does not restyle by hand is written this way, and a reader
    /// that skips it draws them all as nothing.
    fn parseStyle(&self, node: &Rc<RefCell<XmlNode>>, shape: &Rc<RefCell<PptxShape>>) {
        let mut st: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&node, "style");
        if st.is_none() {
            return;
        }
        let mut sn: Rc<RefCell<XmlNode>> = st.clone().unwrap();
        // That the block EXISTS is the thing the writer needs: a shape with a
        // style reference follows the theme, and one without it does not.
        shape.borrow_mut().hasStyle = true;
        let mut fill: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sn, "fillRef");
        if fill.is_some() {
            let mut fr: Rc<RefCell<XmlNode>> = fill.clone().unwrap();
            let idx: Option<i64> = XmlNode::attr(&fr, "idx").parse::<i64>().ok();
            shape.borrow_mut().fillRef = if idx.is_some() { idx.unwrap() } else { 0 };
            let mut col: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&fr);
            if col.is_some() {
                shape.borrow_mut().fillRefColor = col.clone().unwrap();
            }
        }
        let mut line: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sn, "lnRef");
        if line.is_some() {
            let mut lr: Rc<RefCell<XmlNode>> = line.clone().unwrap();
            let idx2: Option<i64> = XmlNode::attr(&lr, "idx").parse::<i64>().ok();
            shape.borrow_mut().lineRef = if idx2.is_some() { idx2.unwrap() } else { 0 };
            let mut col2: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&lr);
            if col2.is_some() {
                shape.borrow_mut().lineRefColor = col2.clone().unwrap();
            }
        }
        let mut eff: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sn, "effectRef");
        if eff.is_some() {
            let idx3: Option<i64> = XmlNode::attr(&eff.clone().unwrap(), "idx").parse::<i64>().ok();
            shape.borrow_mut().effectRef = if idx3.is_some() { idx3.unwrap() } else { 0 };
        }
        let mut font: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&sn, "fontRef");
        if font.is_some() {
            let mut r#fn: Rc<RefCell<XmlNode>> = font.clone().unwrap();
            shape.borrow_mut().fontRef = XmlNode::attr(&r#fn, "idx");
            let mut col3: Option<Rc<RefCell<PptxColor>>> = self.colorIn(&r#fn);
            if col3.is_some() {
                shape.borrow_mut().fontRefColor = col3.clone().unwrap();
            }
        }
    }
    pub fn parseTheme(&self, part_path: &str) -> Rc<RefCell<PptxTheme>> {
        let mut theme: Rc<RefCell<PptxTheme>> = Rc::new(RefCell::new(PptxTheme::new()));
        // Sensible Office defaults even if theme is missing
        theme.borrow_mut().setColor("dk1", "000000");
        theme.borrow_mut().setColor("lt1", "FFFFFF");
        theme.borrow_mut().setColor("dk2", "44546A");
        theme.borrow_mut().setColor("lt2", "E7E6E6");
        theme.borrow_mut().setColor("accent1", "4472C4");
        theme.borrow_mut().setColor("accent2", "ED7D31");
        theme.borrow_mut().setColor("accent3", "A5A5A5");
        theme.borrow_mut().setColor("accent4", "FFC000");
        theme.borrow_mut().setColor("accent5", "5B9BD5");
        theme.borrow_mut().setColor("accent6", "70AD47");
        theme.borrow_mut().setColor("hlink", "0563C1");
        theme.borrow_mut().setColor("folHlink", "954F72");
        if part_path.is_empty() {
            return theme;
        }
        let xml: String = OpcPackage::readXml(&self.pkg, part_path);
        if xml.is_empty() {
            return theme;
        }
        let mut root: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, xml.clone());
        if root.is_none() {
            return theme;
        }
        let mut doc: Rc<RefCell<XmlNode>> = root.clone().unwrap();
        let mut scheme: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "clrScheme");
        if scheme.is_some() {
            let mut sch: Rc<RefCell<XmlNode>> = scheme.clone().unwrap();
            let mut i: i64 = 0;
            while i < XmlNode::childCount(&sch) {
                let mut child: Rc<RefCell<XmlNode>> = XmlNode::childAt(&sch, i);
                if !child.borrow().isText {
                    let name: String = child.borrow().name.clone();
                    let mut srgb: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&child, "srgbClr");
                    if srgb.is_some() {
                        theme.borrow_mut().setColor(&name, &XmlNode::attr(&srgb.clone().unwrap(), "val"));
                    }
                    let mut sys: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&child, "sysClr");
                    if sys.is_some() {
                        let last: String = XmlNode::attr(&sys.clone().unwrap(), "lastClr");
                        if (last.as_bytes().len() as i64) == 6 {
                            theme.borrow_mut().setColor(&name, &last);
                        }
                    }
                }
                i += 1;
            }
        }
        self.parseFormatScheme(&theme, &doc);
        let mut major: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "majorFont");
        if major.is_some() {
            let mut latin: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&major.clone().unwrap(), "latin");
            if latin.is_some() {
                let tf: String = XmlNode::attr(&latin.clone().unwrap(), "typeface");
                if !tf.is_empty() {
                    theme.borrow_mut().majorFont = tf.clone();
                }
            }
        }
        let mut minor: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "minorFont");
        if minor.is_some() {
            let mut latin2: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&minor.clone().unwrap(), "latin");
            if latin2.is_some() {
                let tf2: String = XmlNode::attr(&latin2.clone().unwrap(), "typeface");
                if !tf2.is_empty() {
                    theme.borrow_mut().minorFont = tf2.clone();
                }
            }
        }
        theme
    }
    /// `p14:sectionLst`, which lives in the presentation's extLst. A deck with
    /// no sections is the common case and parses to an empty list, not an error.
    fn parseSections(&self, pres: &Rc<RefCell<PptxPresentation>>, doc: &Rc<RefCell<XmlNode>>) {
        let mut lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "sectionLst");
        if lst.is_none() {
            return;
        }
        let mut node: Rc<RefCell<XmlNode>> = lst.clone().unwrap();
        let mut secs: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&node, "section");
        let mut i: i64 = 0;
        while i < (secs.len() as i64) {
            let mut sn: Rc<RefCell<XmlNode>> = secs[i as usize].clone();
            let mut sec: Rc<RefCell<PptxSection>> = Rc::new(RefCell::new(PptxSection::new()));
            sec.borrow_mut().name = XmlNode::attr(&sn, "name");
            sec.borrow_mut().id = XmlNode::attr(&sn, "id");
            let mut ids: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&sn, "sldIdLst");
            if ids.is_some() {
                let mut id_node: Rc<RefCell<XmlNode>> = ids.clone().unwrap();
                let mut kids: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&id_node, "sldId");
                let mut k: i64 = 0;
                while k < (kids.len() as i64) {
                    let mut kid: Rc<RefCell<XmlNode>> = kids[k as usize].clone();
                    let v: Option<i64> = XmlNode::attr(&kid, "id").parse::<i64>().ok();
                    sec.borrow_mut().slideIds.push(if v.is_some() { v.unwrap() } else { 0 });
                    k += 1;
                }
            }
            pres.borrow_mut().sections.push(sec.clone());
            i += 1;
        }
    }
    pub fn parsePresentation(&self) -> Rc<RefCell<PptxPresentation>> {
        let mut pres: Rc<RefCell<PptxPresentation>> = Rc::new(RefCell::new(PptxPresentation::new()));
        let xml: String = OpcPackage::readXml(&self.pkg, "ppt/presentation.xml");
        if xml.is_empty() {
            return pres;
        }
        let mut root: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&self.xmlParser, xml.clone());
        if root.is_none() {
            return pres;
        }
        let mut doc: Rc<RefCell<XmlNode>> = root.clone().unwrap();
        let mut sld_sz: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "sldSz");
        if sld_sz.is_some() {
            let mut sz: Rc<RefCell<XmlNode>> = sld_sz.clone().unwrap();
            let _tmp_4734 = PptxUnits::emuToPt(self.parseDoubleAttr(&sz, "cx"));
            pres.borrow_mut().slideWidth = _tmp_4734;
            let _tmp_4736 = PptxUnits::emuToPt(self.parseDoubleAttr(&sz, "cy"));
            pres.borrow_mut().slideHeight = _tmp_4736;
        }
        // Slide order from sldIdLst → relationships. The `id` on each entry is
        // the name sections use for a slide, so it is kept alongside the part.
        let mut slide_ids: Vec<i64> = Vec::new();
        let mut id_lst: Option<Rc<RefCell<XmlNode>>> = XmlNode::find(&doc, "sldIdLst");
        if id_lst.is_some() {
            let mut lst: Rc<RefCell<XmlNode>> = id_lst.clone().unwrap();
            let mut ids: Vec<Rc<RefCell<XmlNode>>> = XmlNode::childrenNamed(&lst, "sldId");
            let mut i: i64 = 0;
            while i < (ids.len() as i64) {
                let mut id_node: Rc<RefCell<XmlNode>> = ids[i as usize].clone();
                let mut rid: String = XmlNode::attr(&id_node, "r:id");
                let mut own: String = XmlNode::attr(&id_node, "id");
                if rid.is_empty() {
                    // Rare writers omit the prefix
                    rid = own.clone();
                    own = String::new();
                }
                let part: String = OpcPackage::resolveRelationship(
                    &self.pkg,
                    "ppt/presentation.xml",
                    &rid
                );
                if !part.is_empty() {
                    pres.borrow_mut().slideParts.push(part.clone());
                    let sid: Option<i64> = own.parse::<i64>().ok();
                    slide_ids.push(if sid.is_some() { sid.unwrap() } else { 0 });
                }
                i += 1;
            }
        }
        self.parseSections(&pres, &doc);
        // Theme via master
        let master_part: String = OpcPackage::findRelationshipByType(
            &self.pkg,
            "ppt/presentation.xml",
            "/slideMaster"
        );
        if !master_part.is_empty() {
            let _tmp_4740 = OpcPackage::findRelationshipByType(
                &self.pkg,
                &master_part,
                "/theme"
            );
            pres.borrow_mut().themePart = _tmp_4740;
        }
        if pres.borrow().themePart.is_empty() {
            // Common default path
            if OpcPackage::hasPart(&self.pkg, "ppt/theme/theme1.xml") {
                pres.borrow_mut().themePart = "ppt/theme/theme1.xml".to_string();
            }
        }
        let _tmp_4743 = self.parseTheme(&pres.borrow().themePart.clone());
        pres.borrow_mut().theme = _tmp_4743;
        // Parse each slide + ensure layout/master sheets are cached
        let mut si: i64 = 0;
        while si < (pres.borrow().slideParts.len() as i64) {
            let part_1: String = pres.borrow().slideParts[si as usize].clone();
            let mut _tmp_4744 = pres.borrow().slideWidth.clone();
            let mut _tmp_4745 = pres.borrow().slideHeight.clone();
            let mut slide: Rc<RefCell<PptxSlide>> = self.parseSlide(part_1.clone(),
                _tmp_4744,
                _tmp_4745
            );
            if si < (slide_ids.len() as i64) {
                slide.borrow_mut().slideId = slide_ids[si as usize];
            }
            if !slide.borrow().layoutPart.is_empty() {
                let mut _tmp_4747 = slide.borrow().layoutPart.clone();
                let _tmp_4748 = OpcPackage::findRelationshipByType(
                    &self.pkg,
                    &_tmp_4747,
                    "/slideMaster"
                );
                slide.borrow_mut().masterPart = _tmp_4748;
                let mut _tmp_4749 = slide.borrow().layoutPart.clone();
                let mut __arg_0 = pres.borrow().layouts.clone();
                self.ensureSheet(&mut __arg_0, &_tmp_4749);
                pres.borrow_mut().layouts = __arg_0;
            }
            if !slide.borrow().masterPart.is_empty() {
                let mut _tmp_4750 = slide.borrow().masterPart.clone();
                let mut __arg_0 = pres.borrow().masters.clone();
                self.ensureSheet(&mut __arg_0, &_tmp_4750);
                pres.borrow_mut().masters = __arg_0;
            }
            pres.borrow_mut().slides.push(slide.clone());
            si += 1;
        }
        pres
    }
}
