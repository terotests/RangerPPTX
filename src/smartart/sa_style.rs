// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaStyle — the colours and line a shape is drawn with
//! ==============================================================================
//!
//! A layout node names a style label (`styleLbl="node1"`). The diagram's
//! colour set (SaColors) says that label's fill, line and text colours, as
//! theme colours, and how the shapes carrying it take them from its lists;
//! the quick style (SaQuickStyle) says whether the line and fill are drawn,
//! how wide the line is, and whether there is a shadow. The theme says what
//! the colours are, so a diagram is in the deck's colours without any colour
//! in the file, and changes with the theme.
//!
//! A colour set or quick style this engine does not have is drawn as
//! accent1_2 / simple1, and the engine says so (SaEngine).
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::pptx_model::PptxColor;
use crate::smartart::sa_colors::SaClrList;
use crate::smartart::sa_colors::SaColors;
use crate::smartart::sa_colors::SaColorsDef;
use crate::smartart::sa_colors::SaColorsLbl;
use crate::smartart::sa_quick_style::SaQsLbl;
use crate::smartart::sa_quick_style::SaQuickStyle;
use crate::smartart::sa_quick_style::SaQuickStyleDef;

#[derive(Clone)]
pub struct SaPaint {
    pub fill: Rc<RefCell<PptxColor>>,
    pub noFill: bool,
    pub line: Rc<RefCell<PptxColor>>,
    pub noLine: bool,
    pub lineWidthPt: f64,
    pub text: Rc<RefCell<PptxColor>>,
    /// an outer shadow, from the quick style's effect: how far it falls and
    /// how soft it is, in points
    pub shadow: bool,
    pub shadowDistPt: f64,
    pub shadowBlurPt: f64,
    /// …and how dark, 0..1
    pub shadowAlpha: f64,
}

impl SaPaint {
    pub fn new() -> Self {
        Self {
            fill: Rc::new(RefCell::new(PptxColor::new())),
            noFill: false,
            line: Rc::new(RefCell::new(PptxColor::new())),
            noLine: false,
            lineWidthPt: 1.0,
            text: Rc::new(RefCell::new(PptxColor::new())),
            shadow: false,
            shadowDistPt: 0.0,
            shadowBlurPt: 0.0,
            shadowAlpha: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct SaStyle {
}

impl SaStyle {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn knownColors(id: &str) -> bool {
        SaColors::known(id)
    }
    pub fn knownStyle(id: &str) -> bool {
        SaQuickStyle::known(id)
    }
    pub fn short(id: &str) -> String {
        SaColors::short(id)
    }
    pub fn family(lbl: &str) -> String {
        SaColors::family(lbl)
    }
    pub fn scheme(name: &str, tint: f64) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = PptxColor::schemeOf(name.to_string());
        c.borrow_mut().tint = tint;
        c
    }
    /// the colour set with this id, accent1_2 when it is none this engine has
    pub fn colorsDef(id: &str) -> Rc<RefCell<SaColorsDef>> {
        let mut d: Rc<RefCell<SaColorsDef>> = SaColors::builtin(&SaColors::short(id));
        if d.borrow().labels.is_empty() {
            return SaColors::builtin("accent1_2");
        }
        d
    }
    /// the quick style with this id, simple1 when it is none this engine has
    pub fn styleDef(id: &str) -> Rc<RefCell<SaQuickStyleDef>> {
        let mut d: Rc<RefCell<SaQuickStyleDef>> = SaQuickStyle::builtin(&SaColors::short(id));
        if d.borrow().labels.is_empty() {
            return SaQuickStyle::builtin("simple1");
        }
        d
    }
    /// The label's colours in a set: its own entry, else one of its family's,
    /// else accent1_2's for it.
    pub fn colorsLbl(cd: &Rc<RefCell<SaColorsDef>>, lbl: &str) -> Rc<RefCell<SaColorsLbl>> {
        let mut own: Option<Rc<RefCell<SaColorsLbl>>> = cd.borrow().label(lbl);
        if own.is_some() {
            return own.clone().unwrap();
        }
        let fam: String = SaColors::family(lbl);
        let mut i: i64 = 0;
        while i < (cd.borrow().labels.len() as i64) {
            let mut l: Rc<RefCell<SaColorsLbl>> = cd.borrow().labels[i as usize].clone();
            if SaColors::family(&l.borrow().name.clone()) == fam {
                return l;
            }
            i += 1;
        }
        SaColors::lblFor(lbl.to_string(), "accent_2", "accent1")
    }
    pub fn paint(lbl: &str, cd: &Rc<RefCell<SaColorsDef>>, qs: &Rc<RefCell<SaQuickStyleDef>>, idx: i64, cnt: i64) -> Rc<RefCell<SaPaint>> {
        let mut p: Rc<RefCell<SaPaint>> = Rc::new(RefCell::new(SaPaint::new()));
        let mut cl: Rc<RefCell<SaColorsLbl>> = SaStyle::colorsLbl(cd, lbl);
        let mut ql: Rc<RefCell<SaQsLbl>> = qs.borrow().label(lbl);
        if ql.borrow().fill > 0 && !(cl.borrow().fill).borrow().isEmpty() {
            p.borrow_mut().fill = (cl.borrow().fill).borrow().pick(idx, cnt);
        } else {
            p.borrow_mut().noFill = true;
        }
        if ql.borrow().ln > 0 && !(cl.borrow().line).borrow().isEmpty() {
            p.borrow_mut().line = (cl.borrow().line).borrow().pick(idx, cnt);
            let _tmp_672 = SaQuickStyle::lineWidth(ql.borrow().ln.clone());
            p.borrow_mut().lineWidthPt = _tmp_672;
        } else {
            p.borrow_mut().noLine = true;
        }
        if (cl.borrow().text).borrow().isEmpty() {
            p.borrow_mut().text = PptxColor::schemeOf("tx1".to_string());
        } else {
            p.borrow_mut().text = (cl.borrow().text).borrow().pick(idx, cnt);
        }
        if ql.borrow().effect > 0 && !p.borrow().noFill {
            p.borrow_mut().shadow = true;
            SaStyle::shadowOf(&p, ql.borrow().effect.clone());
        }
        p
    }
    /// The outer shadow of an effect level (1 Subtle, 2 Moderate, 3 Intense:
    /// simple3 … simple5). Each a step one sees from the one before on a
    /// slide: at 1.5 pt a level, the three were a hair apart and read as
    /// one style (Tero 2026-10-07).
    pub fn shadowOf(p: &Rc<RefCell<SaPaint>>, effect: i64) {
        let mut e: i64 = effect;
        if e > 3 {
            e = 3;
        }
        let mut dist: Vec<f64> = vec![0.0, 2.0, 4.0, 7.0];
        let mut blur: Vec<f64> = vec![0.0, 4.0, 9.0, 16.0];
        let mut alpha: Vec<f64> = vec![0.0, 0.3, 0.42, 0.55];
        p.borrow_mut().shadowDistPt = dist[e as usize];
        p.borrow_mut().shadowBlurPt = blur[e as usize];
        p.borrow_mut().shadowAlpha = alpha[e as usize];
    }
    /// the idx-th shape of a label, in a set named by id, simple1 (one of
    /// several, the count unknown: what a span needs is guessed)
    pub fn paintFor(lbl: &str, colors_id: &str, idx: i64) -> Rc<RefCell<SaPaint>> {
        let _tmp_676 = SaStyle::colorsDef(colors_id);
        SaStyle::paint(lbl, &_tmp_676, &SaStyle::styleDef(""), idx, idx + 1)
    }
}
