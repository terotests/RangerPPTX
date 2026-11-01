// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaPackage — the parts a SmartArt diagram is saved as
//! ==============================================================================
//!
//! A diagram in a .pptx is a `graphicFrame` naming four parts by relationship
//! (data, layout, quick style, colours) and, through the data part's
//! `dsp:dataModelExt`, a fifth: the drawing, the shapes as they were last laid
//! out. PowerPoint lays the diagram out again from the first four; every other
//! reader draws the fifth. PptxWriter writes the frame, the relationships and
//! the drawing; this file writes the rest:
//!
//!   dataXml     the data model, every id a number (the schema's ST_ModelId
//!               is an int or a GUID, and the ids a hand-written file uses —
//!               "n1", "cxn-0-n1" — are neither), the document point naming
//!               the layout, style and colours it was drawn with, and the
//!               relationship id of its drawing
//!   styleDefXml a quick style: for each style label, the theme's line, fill,
//!               effect and font references (simple1 — flat, no 3-D)
//!   colorsDefXml
//!               a colour set: for each style label, the fill, line and text
//!               colours, as SaStyle draws them (accent1_2 … accent6_2,
//!               colorful1, colorful2)
//!
//! The layout part is the layout definition the engine ran (SaBuiltins, or
//! the one read from the file), as it is.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::smartart::sa_colors::SaColors;
use crate::smartart::sa_colors::SaColorsDef;
use crate::smartart::sa_data_reader::SaDataReader;
use crate::smartart::sa_data_reader::SaReadResult;
use crate::smartart::sa_data_writer::SaDataWriter;
use crate::smartart::sa_model::SaCxn;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_quick_style::SaQuickStyle;
use crate::smartart::sa_quick_style::SaQuickStyleDef;
use crate::smartart::sa_style::SaStyle;

/// A data model numbered for a package, and what each old id became.
#[derive(Clone)]
pub struct SaNumbering {
    pub ok: bool,
    pub model: Rc<RefCell<SaModel>>,
    pub from: Vec<String>,
    pub to: Vec<String>,
}

impl SaNumbering {
    pub fn new() -> Self {
        Self {
            ok: false,
            model: Rc::new(RefCell::new(SaModel::new())),
            from: Vec::new(),
            to: Vec::new(),
        }
    }
    pub fn mapped(&self, id: &str) -> String {
        let mut _tmp_775 = self.from.clone();
        SaPackage::mapped(&_tmp_775, &self.to.clone(), id)
    }
    pub fn dataXml(&self, drawing_rel_id: String) -> String {
        self.model.borrow_mut().drawingRelId = drawing_rel_id.clone();
        SaDataWriter::write(&self.model.clone())
    }
}

#[derive(Clone)]
pub struct SaPackage {
}

impl SaPackage {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn dgmNs() -> String {
        "http://schemas.openxmlformats.org/drawingml/2006/diagram".to_string()
    }
    pub fn aNs() -> String {
        "http://schemas.openxmlformats.org/drawingml/2006/main".to_string()
    }
    /// the style labels the parts describe: every label the built-in layouts
    /// use, and the ones PowerPoint's own layouts commonly name
    pub fn labels() -> Vec<String> {
        SaColors::labels()
    }
    /// --- the data part -------------------------------------------------------
    /// `modelXml` (SaDataWriter's output, or any data model) with every id a
    /// number and its drawing named by `drawingRelId`; empty when it does not
    /// read
    pub fn dataXml(model_xml: &str, drawing_rel_id: &str) -> String {
        let mut nb: Rc<RefCell<SaNumbering>> = SaPackage::number(model_xml);
        if !nb.borrow().ok {
            return String::new();
        }
        return nb.borrow().dataXml(drawing_rel_id.to_string());
    }
    /// the model with every id a number, and which number each old id got
    pub fn number(model_xml: &str) -> Rc<RefCell<SaNumbering>> {
        let mut nb: Rc<RefCell<SaNumbering>> = Rc::new(RefCell::new(SaNumbering::new()));
        let mut r: Rc<RefCell<SaReadResult>> = SaDataReader::read(model_xml);
        if !r.borrow().ok {
            return nb;
        }
        let mut m: Rc<RefCell<SaModel>> = r.borrow().model.clone().clone();
        m.borrow_mut().complete();
        let mut n: i64 = 0;
        let mut i: i64 = 0;
        while i < (m.borrow().points.len() as i64) {
            let mut p: Rc<RefCell<SaPoint>> = m.borrow().points[i as usize].clone();
            nb.borrow_mut().from.push(p.borrow().id.clone());
            nb.borrow_mut().to.push(n.to_string());
            n += 1;
            i += 1;
        }
        i = 0;
        while i < (m.borrow().cxns.len() as i64) {
            let mut c: Rc<RefCell<SaCxn>> = m.borrow().cxns[i as usize].clone();
            nb.borrow_mut().from.push(c.borrow().id.clone());
            nb.borrow_mut().to.push(n.to_string());
            n += 1;
            i += 1;
        }
        i = 0;
        while i < (m.borrow().points.len() as i64) {
            let mut p2: Rc<RefCell<SaPoint>> = m.borrow().points[i as usize].clone();
            let _tmp_779 = nb.borrow().mapped(&p2.borrow().id.clone());
            p2.borrow_mut().id = _tmp_779;
            let _tmp_781 = nb.borrow().mapped(&p2.borrow().cxnId.clone());
            p2.borrow_mut().cxnId = _tmp_781;
            let _tmp_783 = nb.borrow().mapped(&p2.borrow().presAssocId.clone());
            p2.borrow_mut().presAssocId = _tmp_783;
            i += 1;
        }
        i = 0;
        while i < (m.borrow().cxns.len() as i64) {
            let mut c2: Rc<RefCell<SaCxn>> = m.borrow().cxns[i as usize].clone();
            let _tmp_785 = nb.borrow().mapped(&c2.borrow().id.clone());
            c2.borrow_mut().id = _tmp_785;
            let _tmp_787 = nb.borrow().mapped(&c2.borrow().srcId.clone());
            c2.borrow_mut().srcId = _tmp_787;
            let _tmp_789 = nb.borrow().mapped(&c2.borrow().destId.clone());
            c2.borrow_mut().destId = _tmp_789;
            let _tmp_791 = nb.borrow().mapped(&c2.borrow().parTransId.clone());
            c2.borrow_mut().parTransId = _tmp_791;
            let _tmp_793 = nb.borrow().mapped(&c2.borrow().sibTransId.clone());
            c2.borrow_mut().sibTransId = _tmp_793;
            let _tmp_795 = nb.borrow().mapped(&c2.borrow().presId.clone());
            c2.borrow_mut().presId = _tmp_795;
            i += 1;
        }
        nb.borrow_mut().model = m.clone();
        nb.borrow_mut().ok = true;
        nb
    }
    pub fn mapped(from: &[String], to: &[String], id: &str) -> String {
        if id.is_empty() {
            return String::new();
        }
        let mut i: i64 = 0;
        while i < (from.len() as i64) {
            if from[i as usize].clone() == id {
                return to[i as usize].clone();
            }
            i += 1;
        }
        id.to_string()
    }
    /// A drawing shape's model id: a GUID made from its index, so the same
    /// diagram writes the same ids every time.
    pub fn shapeModelId(i: i64) -> String {
        let hex: String = "0123456789ABCDEF";
        let mut tail: String = String::new();
        let mut v: i64 = i;
        let mut k: i64 = 0;
        while k < 12 {
            let d: i64 = v % 16;
            tail = format!("{}{}", hex[(d) as usize..(d + 1) as usize].to_string(), tail);
            v = ((v as f64) / 16.0).floor() as i64;
            k += 1;
        }
        format!("{}{}{}", "{5A0D0000-0000-4000-8000-".to_string(), tail, "}".to_string())
    }
    /// --- the quick style and the colour set --------------------------------
    pub fn styleDefXml(style_id: &str) -> String {
        let mut d: Rc<RefCell<SaQuickStyleDef>> = SaStyle::styleDef(style_id);
        let x: String = SaQuickStyle::xml(&d);
        x
    }
    pub fn colorsDefXml(colors_id: &str) -> String {
        let mut d: Rc<RefCell<SaColorsDef>> = SaStyle::colorsDef(colors_id);
        SaColors::xml(&d)
    }
}
