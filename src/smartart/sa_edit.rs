// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaEdit — a diagram laid out again after its text was edited
//! ==============================================================================
//!
//! A shape inside a diagram the engine laid out knows, paragraph by
//! paragraph, which data point its text came from (`diagramParaPts`). An edit
//! to that text is written back to those points and the diagram is laid out
//! again from its data, with the layout, colours and style it was drawn with,
//! at its own size. So typing a longer label shrinks every step's type the
//! way PowerPoint does, and the file saved afterwards is still SmartArt.
//!
//! Each point gets the paragraphs that now map to it. A point that keeps its
//! number of paragraphs keeps their levels and run properties, only the text
//! changes; otherwise its paragraphs are made again, plain. Paragraphs beyond
//! the ones the shape had go to the last point it showed; a point whose
//! paragraphs were all deleted is left with no text, as PowerPoint leaves an
//! emptied node. Adding or removing nodes is not an edit of text and is not
//! done here.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::pptx_model::PptxParagraph;
use crate::pptx_model::PptxShape;
use crate::pptx_model::PptxTextRun;
use crate::smartart::sa_data_reader::SaDataReader;
use crate::smartart::sa_data_reader::SaReadResult;
use crate::smartart::sa_engine::SaEngine;
use crate::smartart::sa_engine::SaOptions;
use crate::smartart::sa_engine::SaResult;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPara;
use crate::smartart::sa_model::SaPoint;
use crate::smartart::sa_model::SaRun;

#[derive(Clone)]
pub struct SaEdit {
}

impl SaEdit {
    pub fn new() -> Self {
        Self {
        }
    }
    /// Whether `d` (a diagram group) can be laid out again from what it holds.
    pub fn canRelayout(d: &Rc<RefCell<PptxShape>>) -> bool {
        if !d.borrow().isDiagram {
            return false;
        }
        return !d.borrow().diagramModelXml.is_empty() && !d.borrow().diagramLayoutXml.is_empty();
    }
    /// `d` laid out again with the text `edited` (one of its shapes) now has.
    /// `ok` false, and the diagram left as it is, when it cannot be.
    pub fn relayout(d: &Rc<RefCell<PptxShape>>, edited: &Rc<RefCell<PptxShape>>) -> Rc<RefCell<SaResult>> {
        let mut fail: Rc<RefCell<SaResult>> = Rc::new(RefCell::new(SaResult::new()));
        if !SaEdit::canRelayout(d) {
            fail.borrow_mut().error = "the diagram has no data to lay it out from".to_string();
            return fail;
        }
        let mut pts: Vec<String> = edited.borrow().diagramParaPts.clone();
        if pts.is_empty() {
            fail.borrow_mut().error = "the shape shows no point of the diagram's data".to_string();
            return fail;
        }
        let mut read: Rc<RefCell<SaReadResult>> = SaDataReader::read(&d.borrow().diagramModelXml.clone());
        if !read.borrow().ok {
            fail.borrow_mut().error = read.borrow().error.clone();
            return fail;
        }
        let mut model: Rc<RefCell<SaModel>> = read.borrow().model.clone().clone();
        model.borrow_mut().complete();
        // the points, in the order the shape showed them, and the paragraphs
        // that now belong to each
        let mut order: Vec<String> = Vec::new();
        let mut i: i64 = 0;
        while i < (pts.len() as i64) {
            let id: String = pts[i as usize].clone();
            if SaEdit::indexIn(&order, &id) < 0 {
                order.push(id.clone());
            }
            i += 1;
        }
        let last: String = pts[((pts.len() as i64) - 1) as usize].clone();
        let mut k: i64 = 0;
        while k < (order.len() as i64) {
            let pid: String = order[k as usize].clone();
            let mut hit: Option<Rc<RefCell<SaPoint>>> = model.borrow().pointById(&pid);
            if hit.is_some() {
                let mut p: Rc<RefCell<SaPoint>> = hit.clone().unwrap();
                let mut mine: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
                let mut j: i64 = 0;
                while j < (edited.borrow().text.borrow().paragraphs.len() as i64) {
                    let mut owner: String = last.clone();
                    if j < (pts.len() as i64) {
                        owner = pts[j as usize].clone();
                    }
                    if owner == pid {
                        mine.push(edited.borrow().text.borrow().paragraphs[j as usize].clone());
                    }
                    j += 1;
                }
                SaEdit::setText(&p, &mine);
            }
            k += 1;
        }
        let mut opts: Rc<RefCell<SaOptions>> = Rc::new(RefCell::new(SaOptions::new()));
        opts.borrow_mut().layoutXml = d.borrow().diagramLayoutXml.clone();
        opts.borrow_mut().colors = d.borrow().diagramColorsId.clone();
        opts.borrow_mut().style = d.borrow().diagramStyleId.clone();
        opts.borrow_mut().colorsXml = d.borrow().diagramColorsXml.clone();
        opts.borrow_mut().styleXml = d.borrow().diagramStyleXml.clone();
        let mut _tmp_766 = d.borrow().width.clone();
        return SaEngine::layoutModel(model.clone(), &opts, _tmp_766, d.borrow().height.clone());
    }
    pub fn indexIn(list: &[String], v: &str) -> i64 {
        let mut i: i64 = 0;
        while i < (list.len() as i64) {
            if list[i as usize].clone() == v {
                return i;
            }
            i += 1;
        }
        -1
    }
    /// A point's text made the paragraphs' text. The same number of
    /// paragraphs: each keeps its level and its first run's properties.
    pub fn setText(p: &Rc<RefCell<SaPoint>>, paras: &Vec<Rc<RefCell<PptxParagraph>>>) {
        let same: bool = (paras.len() as i64) == (p.borrow().paras.len() as i64);
        let mut out: Vec<Rc<RefCell<SaPara>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (paras.len() as i64) {
            let mut src: Rc<RefCell<PptxParagraph>> = paras[i as usize].clone();
            let text: String = SaEdit::plain(&src);
            let mut np: Rc<RefCell<SaPara>> = Rc::new(RefCell::new(SaPara::new()));
            let mut run: Rc<RefCell<SaRun>> = Rc::new(RefCell::new(SaRun::new()));
            if same {
                let mut old: Rc<RefCell<SaPara>> = p.borrow().paras[i as usize].clone();
                np.borrow_mut().level = old.borrow().level;
                if !old.borrow().runs.is_empty() {
                    let mut r0: Rc<RefCell<SaRun>> = old.borrow().runs[0].clone();
                    run.borrow_mut().bold = r0.borrow().bold;
                    run.borrow_mut().italic = r0.borrow().italic;
                    run.borrow_mut().underline = r0.borrow().underline;
                    run.borrow_mut().sizePt = r0.borrow().sizePt;
                    run.borrow_mut().color = r0.borrow().color.clone();
                }
            }
            run.borrow_mut().text = text.clone();
            np.borrow_mut().runs.push(run.clone());
            out.push(np.clone());
            i += 1;
        }
        p.borrow_mut().paras = out.clone();
        p.borrow_mut().hasText = true;
    }
    pub fn plain(para: &Rc<RefCell<PptxParagraph>>) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut r: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            out = format!("{}{}", out, r.borrow().text);
            i += 1;
        }
        out
    }
}
