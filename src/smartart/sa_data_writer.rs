// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaDataWriter — an SaModel as `dgm:dataModel` XML
//! ==============================================================================
//!
//! The output is what PowerPoint itself writes into `ppt/diagrams/dataN.xml`:
//! every point with its type stated unless it is a `node`, every connection
//! with its type stated unless it is a `parOf`, and the attributes in the
//! schema's order. It is written from the model alone, so what the model does
//! not hold (bodyPr details, run properties beyond the ones `SaRun` carries) is
//! not written; a deck saved over its own file keeps the original part instead
//! (`PptxWriter`'s pass-through), so that loss only applies to a diagram this
//! code made.
//!
//! `read(write(m))` gives back `m`: that round trip is the test.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_model::SaCxn;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPara;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_model::SaRun;

#[derive(Clone)]
pub struct SaDataWriter {
}

impl SaDataWriter {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn esc(s: &str) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = s.as_bytes().len() as i64;
        let mut from: i64 = 0;
        while i < n {
            let c: i64 = (s.as_bytes()[i as usize] as i64);
            let mut rep: String = "";
            if c == 38 {
                rep = "&amp;";
            }
            if c == 60 {
                rep = "&lt;";
            }
            if c == 62 {
                rep = "&gt;";
            }
            if c == 34 {
                rep = "&quot;";
            }
            if !rep.is_empty() {
                out = format!("{}{}{}", out, s[(from) as usize..(i) as usize].to_string(), rep);
                from = i + 1;
            }
            i += 1;
        }
        format!("{}{}", out, s[(from) as usize..(n) as usize].to_string())
    }
    pub fn attr(name: &str, value: &str) -> String {
        if value.is_empty() {
            return String::new();
        }
        format!(" {}=\"{}\"", name, SaDataWriter::esc(value))
    }
    pub fn write(m: &Rc<RefCell<SaModel>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".to_string();
        out = format!("{}<dgm:dataModel xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">", out);
        out = format!("{}<dgm:ptLst>", out);
        let mut i: i64 = 0;
        while i < (m.borrow().points.len() as i64) {
            out = format!("{}{}", out, SaDataWriter::pointXml(&m.borrow().points[i as usize].clone()));
            i += 1;
        }
        out = format!("{}</dgm:ptLst>", out);
        if !m.borrow().cxns.is_empty() {
            out = format!("{}<dgm:cxnLst>", out);
            i = 0;
            while i < (m.borrow().cxns.len() as i64) {
                out = format!("{}{}", out, SaDataWriter::cxnXml(&m.borrow().cxns[i as usize].clone()));
                i += 1;
            }
            out = format!("{}</dgm:cxnLst>", out);
        }
        out = format!("{}<dgm:bg/><dgm:whole/>", out);
        if !m.borrow().drawingRelId.is_empty() {
            out = format!("{}<dgm:extLst><a:ext uri=\"http://schemas.microsoft.com/office/drawing/2008/diagram\"><dsp:dataModelExt xmlns:dsp=\"http://schemas.microsoft.com/office/drawing/2008/diagram\"{} minVer=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"/></a:ext></dgm:extLst>", out, SaDataWriter::attr("relId", &m.borrow().drawingRelId));
        }
        format!("{}</dgm:dataModel>", out)
    }
    pub fn pointXml(p: &Rc<RefCell<SaPoint>>) -> String {
        let mut out: String = format!("<dgm:pt{}", SaDataWriter::attr("modelId", &p.borrow().id.clone()));
        if p.borrow().r#type != "node" {
            out = format!("{}{}", out, SaDataWriter::attr("type", &p.borrow().r#type));
        }
        out = format!("{}{}>", out, SaDataWriter::attr("cxnId", &p.borrow().cxnId));
        let mut ps: String = String::new();
        ps = format!("{}{}", ps, SaDataWriter::attr("loTypeId", &p.borrow().layoutId));
        ps = format!("{}{}", ps, SaDataWriter::attr("qsTypeId", &p.borrow().styleId));
        ps = format!("{}{}", ps, SaDataWriter::attr("csTypeId", &p.borrow().colorsId));
        ps = format!("{}{}", ps, SaDataWriter::attr("presAssocID", &p.borrow().presAssocId));
        ps = format!("{}{}", ps, SaDataWriter::attr("presName", &p.borrow().presName));
        ps = format!("{}{}", ps, SaDataWriter::attr("presStyleLbl", &p.borrow().presStyleLbl));
        if p.borrow().presStyleIdx >= 0 {
            ps = format!("{}{}", ps, SaDataWriter::attr("presStyleIdx", &p.borrow().presStyleIdx.to_string()));
        }
        if p.borrow().presStyleCnt >= 0 {
            ps = format!("{}{}", ps, SaDataWriter::attr("presStyleCnt", &p.borrow().presStyleCnt.to_string()));
        }
        out = format!("{}<dgm:prSet{}/><dgm:spPr/>", out, ps);
        if p.borrow().hasText {
            out = format!("{}{}", out, SaDataWriter::textXml(p));
        }
        format!("{}</dgm:pt>", out)
    }
    pub fn textXml(p: &Rc<RefCell<SaPoint>>) -> String {
        let mut out: String = "<dgm:t><a:bodyPr/><a:lstStyle/>".to_string();
        if p.borrow().paras.is_empty() {
            out = format!("{}<a:p><a:endParaRPr lang=\"en-US\"/></a:p>", out);
        }
        let mut i: i64 = 0;
        while i < (p.borrow().paras.len() as i64) {
            let mut para: Rc<RefCell<SaPara>> = p.borrow().paras[i as usize].clone();
            out = format!("{}<a:p>", out);
            if para.borrow().level > 0 {
                out = format!("{}<a:pPr lvl=\"{}\"/>", out, para.borrow().level);
            }
            let mut k: i64 = 0;
            while k < (para.borrow().runs.len() as i64) {
                out = format!("{}{}", out, SaDataWriter::runXml(&para.borrow().runs[k as usize].clone()));
                k += 1;
            }
            out = format!("{}</a:p>", out);
            i += 1;
        }
        format!("{}</dgm:t>", out)
    }
    pub fn runXml(r: &Rc<RefCell<SaRun>>) -> String {
        if r.borrow().text == "\n" {
            return "<a:br><a:rPr lang=\"en-US\"/></a:br>".to_string();
        }
        let mut rp: String = " lang=\"en-US\"".to_string();
        if r.borrow().sizePt > 0.0 {
            let hundredths: i64 = (r.borrow().sizePt * 100.0 + 0.5).floor() as i64;
            rp = format!("{} sz=\"{}\"", rp, hundredths);
        }
        if r.borrow().bold {
            rp = format!("{} b=\"1\"", rp);
        }
        if r.borrow().italic {
            rp = format!("{} i=\"1\"", rp);
        }
        if r.borrow().underline {
            rp = format!("{} u=\"sng\"", rp);
        }
        let mut inner: String = String::new();
        if !r.borrow().color.is_empty() {
            inner = format!("<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>", SaDataWriter::esc(&r.borrow().color));
        }
        let mut rpr: String = format!("<a:rPr{}/>", rp);
        if !inner.is_empty() {
            rpr = format!("<a:rPr{}>{}</a:rPr>", rp, inner);
        }
        return format!("<a:r>{}<a:t>{}</a:t></a:r>", rpr, SaDataWriter::esc(&r.borrow().text.clone()));
    }
    pub fn cxnXml(c: &Rc<RefCell<SaCxn>>) -> String {
        let mut out: String = format!("<dgm:cxn{}", SaDataWriter::attr("modelId", &c.borrow().id.clone()));
        if c.borrow().r#type != "parOf" {
            out = format!("{}{}", out, SaDataWriter::attr("type", &c.borrow().r#type));
        }
        out = format!("{}{}{}", out, SaDataWriter::attr("srcId", &c.borrow().srcId), SaDataWriter::attr("destId", &c.borrow().destId));
        out = format!("{} srcOrd=\"{}\" destOrd=\"{}\"", out, c.borrow().srcOrd, c.borrow().destOrd);
        out = format!("{}{}{}{}", out, SaDataWriter::attr("parTransId", &c.borrow().parTransId), SaDataWriter::attr("sibTransId", &c.borrow().sibTransId), SaDataWriter::attr("presId", &c.borrow().presId));
        format!("{}/>", out)
    }
}
