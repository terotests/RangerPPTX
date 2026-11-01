// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::rgr::{OfficeCaretMotion};

use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxParagraph;
use pptx::pptx_model::PptxTextBody;
use pptx::pptx_model::PptxTextRun;


/// ==============================================================================
/// PptxTextEdit - a caret inside a shape's text, and the edits it makes
/// ==============================================================================
///
/// `PptxEditor` moves shapes; this moves a caret inside one. A position is a
/// paragraph and a column measured in UTF-16 units of that paragraph's plain
/// text — the same unit `strlen` and `substring` count in, so nothing has to be
/// converted before an edit, and `EVGCodepoint` is what steps over a surrogate
/// pair so a caret never lands inside one.
///
/// The runs are the point. A paragraph is a list of runs, each with its own
/// size, weight and colour, and an edit has to keep them: typing in the middle
/// of a bold word stays bold, and making a selection bold splits the runs it
/// covers rather than repainting the whole paragraph. That is why this is not
/// `EditorBuffer` from the text editor — that buffer's runs carry a weight and a
/// font name over a line of plain text, and OOXML's carry the text itself along
/// with a size and a colour. Two different shapes; one of them would have to be
/// converted on every keystroke.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxTextPos {
    pub para: i64,
    pub col: i64,
}

impl PptxTextPos {
    pub fn new() -> Self {
        Self {
            para: 0,
            col: 0,
        }
    }
    pub fn of(para: i64, col: i64) -> Rc<RefCell<PptxTextPos>> {
        let mut p: Rc<RefCell<PptxTextPos>> = Rc::new(RefCell::new(PptxTextPos::new()));
        p.borrow_mut().para = para;
        p.borrow_mut().col = col;
        p
    }
    pub fn copy(&self) -> Rc<RefCell<PptxTextPos>> {
        let mut _tmp_4774 = self.para.clone();
        PptxTextPos::of(_tmp_4774, self.col.clone())
    }
    pub fn isSame(&self, other: &Rc<RefCell<PptxTextPos>>) -> bool {
        if self.para != other.borrow().para {
            return false;
        }
        return self.col == other.borrow().col;
    }
}

#[derive(Clone)]
pub struct PptxTextEdit {
}

impl PptxTextEdit {
    pub fn new() -> Self {
        Self {
        }
    }
    /// --- reading -------------------------------------------------------------
    pub fn paraCount(body: &Rc<RefCell<PptxTextBody>>) -> i64 {
        return body.borrow().paragraphs.len() as i64;
    }
    pub fn paraAt(body: &Rc<RefCell<PptxTextBody>>, i: i64) -> Option<Rc<RefCell<PptxParagraph>>> {
        if i < 0 {
            let mut __none: Option<Rc<RefCell<PptxParagraph>>> = None;
            return __none;
        }
        if i >= (body.borrow().paragraphs.len() as i64) {
            let mut __none2: Option<Rc<RefCell<PptxParagraph>>> = None;
            return __none2;
        }
        let mut hit: Option<Rc<RefCell<PptxParagraph>>> = Some(body.borrow().paragraphs[i as usize].clone());
        hit
    }
    pub fn paraText(body: &Rc<RefCell<PptxTextBody>>, i: i64) -> String {
        let mut p: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, i);
        if p.is_none() {
            return String::new();
        }
        let mut para: Rc<RefCell<PptxParagraph>> = p.clone().unwrap();
        let mut out: String = String::new();
        let mut r: i64 = 0;
        while r < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[r as usize].clone();
            out = format!("{}{}", out, run.borrow().text);
            r += 1;
        }
        out
    }
    pub fn paraLen(body: &Rc<RefCell<PptxTextBody>>, i: i64) -> i64 {
        (PptxTextEdit::paraText(body, i).as_bytes().len() as i64)
    }
    pub fn clamp(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) {
        let n: i64 = body.borrow().paragraphs.len() as i64;
        if n == 0 {
            pos.borrow_mut().para = 0;
            pos.borrow_mut().col = 0;
            return;
        }
        if pos.borrow().para < 0 {
            pos.borrow_mut().para = 0;
        }
        if pos.borrow().para >= n {
            pos.borrow_mut().para = n - 1;
        }
        let __len: i64 = PptxTextEdit::paraLen(body, pos.borrow().para.clone());
        if pos.borrow().col < 0 {
            pos.borrow_mut().col = 0;
        }
        if pos.borrow().col > __len {
            pos.borrow_mut().col = __len;
        }
    }
    /// -1, 0 or 1, the way a comparison reads.
    pub fn cmp(a: &Rc<RefCell<PptxTextPos>>, b: &Rc<RefCell<PptxTextPos>>) -> i64 {
        if a.borrow().para < b.borrow().para {
            return 0 - 1;
        }
        if a.borrow().para > b.borrow().para {
            return 1;
        }
        if a.borrow().col < b.borrow().col {
            return 0 - 1;
        }
        if a.borrow().col > b.borrow().col {
            return 1;
        }
        0
    }
    pub fn textInRange(body: &Rc<RefCell<PptxTextBody>>, a: &Rc<RefCell<PptxTextPos>>, b: &Rc<RefCell<PptxTextPos>>) -> String {
        let mut start: Rc<RefCell<PptxTextPos>> = a.clone();
        let mut end: Rc<RefCell<PptxTextPos>> = b.clone();
        if PptxTextEdit::cmp(a, b) > 0 {
            start = b.clone();
            end = a.clone();
        }
        if start.borrow().para == end.borrow().para {
            let line: String = PptxTextEdit::paraText(body, start.borrow().para.clone());
            return line[(start.borrow().col) as usize..(end.borrow().col) as usize].to_string();
        }
        let first: String = PptxTextEdit::paraText(body, start.borrow().para.clone());
        let mut out: String = first[(start.borrow().col) as usize..(first.as_bytes().len() as i64) as usize].to_string();
        let mut i: i64 = start.borrow().para + 1;
        while i < end.borrow().para {
            out = format!("{}\n{}", out, PptxTextEdit::paraText(body, i));
            i += 1;
        }
        let last: String = PptxTextEdit::paraText(body, end.borrow().para.clone());
        return format!("{}\n{}", out, last[(0) as usize..(end.borrow().col) as usize].to_string());
    }
    pub fn plainText(body: &Rc<RefCell<PptxTextBody>>) -> String {
        return body.borrow().plainText();
    }
    /// --- runs ----------------------------------------------------------------
    /// What a run in this body is worth when it says nothing about its weight.
    /// The body's own `defRPr` is what the editor has to hand; a list level's
    /// bold has already been folded into the run by `PptxResolver`, so a run
    /// that got its weight that way states it and is not affected by this.
    pub fn inheritedBold(body: &Rc<RefCell<PptxTextBody>>) -> bool {
        if body.borrow().defBoldSet {
            return body.borrow().defBold;
        }
        false
    }
    pub fn inheritedItalic(body: &Rc<RefCell<PptxTextBody>>) -> bool {
        if body.borrow().defItalicSet {
            return body.borrow().defItalic;
        }
        false
    }
    pub fn ensureRun(body: &Rc<RefCell<PptxTextBody>>, para: &Rc<RefCell<PptxParagraph>>) -> Rc<RefCell<PptxTextRun>> {
        if !para.borrow().runs.is_empty() {
            return para.borrow().runs[0].clone();
        }
        let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        run.borrow_mut().fontSizePt = body.borrow().defFontSizePt;
        run.borrow_mut().sizeSet = body.borrow().hasDefRPr;
        run.borrow_mut().bold = body.borrow().defBold;
        run.borrow_mut().boldSet = body.borrow().defBoldSet;
        run.borrow_mut().italic = body.borrow().defItalic;
        run.borrow_mut().italicSet = body.borrow().defItalicSet;
        run.borrow_mut().fontFamily = body.borrow().defFontFamily.clone();
        if body.borrow().defColor.borrow().isSet {
            run.borrow_mut().color = body.borrow().defColor.clone();
        }
        para.borrow_mut().runs.push(run.clone());
        run
    }
    /// Split the runs of a paragraph so that a boundary falls exactly at `col`,
    /// and answer how many runs are entirely before it. Splitting is what makes
    /// "bold this word" possible without touching the rest of the line.
    pub fn splitRunsAt(para: &Rc<RefCell<PptxParagraph>>, col: i64) -> i64 {
        let mut out: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
        let mut seen: i64 = 0;
        let mut boundary: i64 = 0;
        let mut found: bool = false;
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            let __len: i64 = run.borrow().text.as_bytes().len() as i64;
            let starts_at: i64 = seen;
            let ends_at: i64 = seen + __len;
            if !found {
                if col == starts_at {
                    boundary = out.len() as i64;
                    found = true;
                }
            }
            if !found {
                if col < ends_at {
                    if col > starts_at {
                        let mut left: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                        let mut right: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                        PptxTextEdit::copyStyle(&run, &left);
                        PptxTextEdit::copyStyle(&run, &right);
                        left.borrow_mut().text = (run.borrow().text)[(0) as usize..(col - starts_at) as usize].to_string();
                        right.borrow_mut().text = (run.borrow().text)[(col - starts_at) as usize..(__len) as usize].to_string();
                        out.push(left.clone());
                        boundary = out.len() as i64;
                        found = true;
                        out.push(right.clone());
                        seen = ends_at;
                        i += 1;
                        continue;
                    }
                }
            }
            out.push(run.clone());
            seen = ends_at;
            i += 1;
        }
        if !found {
            boundary = out.len() as i64;
        }
        para.borrow_mut().runs = out.clone();
        boundary
    }
    pub fn copyStyle(src: &Rc<RefCell<PptxTextRun>>, dst: &Rc<RefCell<PptxTextRun>>) {
        // Not the field: text typed beside a page number is text, not another
        // page number.
        //
        // Everything else goes through the model's own copy, including the
        // resolution signatures — see PptxTextRun.copyStyleTo. The list that
        // used to live here had gone stale and dropped them, and a run half
        // with no signature is one the resolver re-inherits from the layout,
        // which is how toggling bold on part of a line changed its size.
        src.borrow().copyStyleTo(dst);
        let _tmp_4791 = PptxTextEdit::copyColor(&src.borrow().color.clone());
        dst.borrow_mut().color = _tmp_4791;
        // The marker pen behind the run. Splitting a highlighted run used to
        // hand back two halves with no highlight at all, so putting a caret in
        // the middle of one erased it.
        let _tmp_4793 = PptxTextEdit::copyColor(&src.borrow().highlight.clone());
        dst.borrow_mut().highlight = _tmp_4793;
    }
    pub fn copyColor(src: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        c.borrow_mut().isSet = src.borrow().isSet;
        c.borrow_mut().isScheme = src.borrow().isScheme;
        c.borrow_mut().scheme = src.borrow().scheme.clone();
        c.borrow_mut().srgb = src.borrow().srgb.clone();
        c.borrow_mut().alpha = src.borrow().alpha;
        c.borrow_mut().baked = src.borrow().baked;
        c.borrow_mut().copyModsFrom(src);
        c
    }
    /// The run whose style newly typed text should take: the one to the LEFT of
    /// the caret, which is what every editor does — you type in the style of
    /// what you just typed.
    pub fn styleRunAt(para: &Rc<RefCell<PptxParagraph>>, col: i64) -> Option<Rc<RefCell<PptxTextRun>>> {
        let mut seen: i64 = 0;
        let mut best: Option<Rc<RefCell<PptxTextRun>>> = None;
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            let __len: i64 = run.borrow().text.as_bytes().len() as i64;
            if col > seen {
                best = Some(run.clone());
            }
            if col == 0 {
                if i == 0 {
                    best = Some(run.clone());
                }
            }
            seen += __len;
            i += 1;
        }
        best
    }
    /// --- editing -------------------------------------------------------------
    pub fn insertAt(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>, text: &str) -> Rc<RefCell<PptxTextPos>> {
        PptxTextEdit::clamp(body, pos);
        if body.borrow().paragraphs.is_empty() {
            let mut fresh: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            fresh.borrow_mut().align = body.borrow().defAlign.clone();
            body.borrow_mut().paragraphs.push(fresh.clone());
        }
        let mut caret: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        let mut start: i64 = 0;
        let n: i64 = text.as_bytes().len() as i64;
        let mut i: i64 = 0;
        while i <= n {
            let at_end: bool = i == n;
            let mut is_break: bool = false;
            if !at_end {
                if (text.as_bytes()[i as usize] as i64) == 10 {
                    is_break = true;
                }
            }
            if at_end || is_break {
                let piece: String = text[(start) as usize..(i) as usize].to_string();
                if !piece.is_empty() {
                    caret = PptxTextEdit::insertInline(body, &caret, piece.clone());
                }
                if is_break {
                    caret = PptxTextEdit::splitAt(body, &caret);
                }
                start = i + 1;
            }
            i += 1;
        }
        caret
    }
    pub fn insertInline(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>, text: String) -> Rc<RefCell<PptxTextPos>> {
        let mut p: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, pos.borrow().para.clone());
        if p.is_none() {
            return pos;
        }
        let mut para: Rc<RefCell<PptxParagraph>> = p.clone().unwrap();
        if para.borrow().runs.is_empty() {
            let mut only: Rc<RefCell<PptxTextRun>> = PptxTextEdit::ensureRun(body, &para);
            only.borrow_mut().text = text.clone();
            return PptxTextPos::of(pos.borrow().para.clone(), (text.as_bytes().len() as i64));
        }
        let mut proto: Option<Rc<RefCell<PptxTextRun>>> = PptxTextEdit::styleRunAt(&para, pos.borrow().col.clone());
        let at: i64 = PptxTextEdit::splitRunsAt(&para, pos.borrow().col.clone());
        let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        if proto.is_some() {
            PptxTextEdit::copyStyle(&proto.clone().unwrap(), &run);
        }
        run.borrow_mut().text = text.clone();
        // Insert the new run at the boundary and let the merge tidy up.
        let mut out: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            if i == at {
                out.push(run.clone());
            }
            out.push(para.borrow().runs[i as usize].clone());
            i += 1;
        }
        if at >= (para.borrow().runs.len() as i64) {
            out.push(run.clone());
        }
        para.borrow_mut().runs = out.clone();
        PptxTextEdit::mergeRuns(&para);
        return PptxTextPos::of(pos.borrow().para.clone(), pos.borrow().col + (text.as_bytes().len() as i64));
    }
    /// Runs that say the same thing about their style are one run. Without this
    /// a paragraph grows a run per keystroke.
    pub fn mergeRuns(para: &Rc<RefCell<PptxParagraph>>) {
        let mut out: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            if !run.borrow().text.is_empty() {
                let mut merged: bool = false;
                if !out.is_empty() {
                    let mut prev: Rc<RefCell<PptxTextRun>> = out[((out.len() as i64) - 1) as usize].clone();
                    if PptxTextEdit::sameStyle(&prev, &run) {
                        let hoist_9 = format!("{}{}", prev.borrow().text, run.borrow().text);
                        prev.borrow_mut().text = hoist_9;
                        merged = true;
                    }
                }
                if !merged {
                    out.push(run.clone());
                }
            }
            i += 1;
        }
        if out.is_empty() {
            let mut keep: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
            if !para.borrow().runs.is_empty() {
                PptxTextEdit::copyStyle(&para.borrow().runs[0].clone(), &keep);
            }
            keep.borrow_mut().text = String::new();
            out.push(keep.clone());
        }
        para.borrow_mut().runs = out.clone();
    }
    /// Two runs are the same run only if EVERY difference between them has been
    /// checked. What this misses, `mergeRuns` deletes: the second run's text is
    /// appended to the first and the second run — with whatever this failed to
    /// compare — is dropped. So an omission here is not a missed optimisation,
    /// it is formatting the user loses on the next keystroke.
    pub fn sameStyle(a: &Rc<RefCell<PptxTextRun>>, b: &Rc<RefCell<PptxTextRun>>) -> bool {
        if a.borrow().bold != b.borrow().bold {
            return false;
        }
        if a.borrow().italic != b.borrow().italic {
            return false;
        }
        // Whether a run STATED its weight is part of what it is. Two runs that
        // draw the same today but differ in that draw differently the moment
        // the placeholder behind them changes — and `mergeRuns` deletes the one
        // it merges away, so the distinction would be gone for good.
        if a.borrow().boldSet != b.borrow().boldSet {
            return false;
        }
        if a.borrow().italicSet != b.borrow().italicSet {
            return false;
        }
        if a.borrow().fontFamily != b.borrow().fontFamily {
            return false;
        }
        if ((a.borrow().fontSizePt - b.borrow().fontSizePt).abs()) > 0.01 {
            return false;
        }
        // A computed run — a slide number — is never the same as typed text,
        // whatever it happens to look like right now.
        if a.borrow().field != b.borrow().field {
            return false;
        }
        let mut _tmp_4809 = a.borrow().color.clone();
        if !PptxTextEdit::sameColor(&_tmp_4809, &b.borrow().color.clone()) {
            return false;
        }
        let mut _tmp_4811 = a.borrow().highlight.clone();
        if !PptxTextEdit::sameColor(&_tmp_4811, &b.borrow().highlight.clone()) {
            return false;
        }
        true
    }
    /// Everything that decides what a colour DRAWS as. `srgb` alone is not it:
    /// two theme colours that have not been resolved yet both carry an empty
    /// `srgb` and differ only in the scheme slot they name, and two resolved
    /// ones can share a base colour and differ entirely in their modifiers.
    pub fn sameColor(a: &Rc<RefCell<PptxColor>>, b: &Rc<RefCell<PptxColor>>) -> bool {
        if a.borrow().isSet != b.borrow().isSet {
            return false;
        }
        if !a.borrow().isSet && !b.borrow().isSet {
            return true;
        }
        if a.borrow().srgb != b.borrow().srgb {
            return false;
        }
        if a.borrow().isScheme != b.borrow().isScheme {
            return false;
        }
        if a.borrow().scheme != b.borrow().scheme {
            return false;
        }
        if ((a.borrow().alpha - b.borrow().alpha).abs()) > 0.001 {
            return false;
        }
        if ((a.borrow().lumMod - b.borrow().lumMod).abs()) > 0.001 {
            return false;
        }
        if ((a.borrow().lumOff - b.borrow().lumOff).abs()) > 0.001 {
            return false;
        }
        if ((a.borrow().tint - b.borrow().tint).abs()) > 0.001 {
            return false;
        }
        if ((a.borrow().shade - b.borrow().shade).abs()) > 0.001 {
            return false;
        }
        if ((a.borrow().satMod - b.borrow().satMod).abs()) > 0.001 {
            return false;
        }
        true
    }
    /// An empty run wearing another's style. `mergeRuns` keeps exactly one of
    /// these for a paragraph with nothing in it, and copies the style off the
    /// first run it finds — so a paragraph seeded with one keeps its formatting
    /// through the merge, and one seeded with nothing does not.
    pub fn emptyLike(proto: &Rc<RefCell<PptxTextRun>>) -> Rc<RefCell<PptxTextRun>> {
        let mut out: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        PptxTextEdit::copyStyle(proto, &out);
        out.borrow_mut().text = String::new();
        out
    }
    pub fn splitAt(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        PptxTextEdit::clamp(body, pos);
        let mut p: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, pos.borrow().para.clone());
        if p.is_none() {
            return pos;
        }
        let mut para: Rc<RefCell<PptxParagraph>> = p.clone().unwrap();
        // The run the caret is in, remembered BEFORE the split takes the
        // paragraph apart. Whichever side comes out with no runs is seeded
        // from it below — see there for why.
        let mut proto_opt: Option<Rc<RefCell<PptxTextRun>>> = PptxTextEdit::styleRunAt(&para, pos.borrow().col.clone());
        let at: i64 = PptxTextEdit::splitRunsAt(&para, pos.borrow().col.clone());
        let mut tail: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
        // ALL of the paragraph's formatting, not the seven fields somebody
        // remembered. See PptxParagraph.copyFormatTo: the hand-written list
        // this replaces had gone stale, and pressing Enter on a bulleted line
        // gave a paragraph that had lost its bullet colour — the dash went
        // from the deck's white to a default black — along with its bullet
        // font, its indent, its numbering scheme and its direction.
        para.borrow().copyFormatTo(&tail);
        let mut keep: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            if i < at {
                keep.push(run.clone());
            } else {
                tail.borrow_mut().runs.push(run.clone());
            }
            i += 1;
        }
        para.borrow_mut().runs = keep.clone();
        // Enter at the END of a line gives a paragraph with no runs at all,
        // and Enter at the START does the same to the one left behind. An
        // empty side keeps the paragraph's format — its bullet, its colour,
        // its indent — because `copyFormatTo` above copies that, and loses the
        // RUN's: the next character typed into it goes through `ensureRun`,
        // which builds a run out of the BODY's defaults and knows nothing
        // about the line the caret just left.
        //
        // So a new item added to a white list came out black. The bullet was
        // white and the words were not, which is what makes it read as a
        // colour bug rather than a formatting one.
        //
        // Seeding the empty side with a styleless copy of the run the caret
        // was in is what makes typing continue in the style you were typing
        // in, which is what every editor does and what the author means.
        if proto_opt.is_some() {
            let mut proto: Rc<RefCell<PptxTextRun>> = proto_opt.clone().unwrap();
            if tail.borrow().runs.is_empty() {
                tail.borrow_mut().runs.push(PptxTextEdit::emptyLike(&proto));
            }
            if para.borrow().runs.is_empty() {
                para.borrow_mut().runs.push(PptxTextEdit::emptyLike(&proto));
            }
        }
        PptxTextEdit::mergeRuns(&para);
        PptxTextEdit::mergeRuns(&tail);
        let mut paras: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
        let mut j: i64 = 0;
        while j < (body.borrow().paragraphs.len() as i64) {
            paras.push(body.borrow().paragraphs[j as usize].clone());
            if j == pos.borrow().para {
                paras.push(tail.clone());
            }
            j += 1;
        }
        body.borrow_mut().paragraphs = paras.clone();
        return PptxTextPos::of(pos.borrow().para + 1, 0);
    }
    pub fn deleteRange(body: &Rc<RefCell<PptxTextBody>>, a: &Rc<RefCell<PptxTextPos>>, b: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut start: Rc<RefCell<PptxTextPos>> = a.borrow().copy();
        let mut end: Rc<RefCell<PptxTextPos>> = b.borrow().copy();
        if PptxTextEdit::cmp(a, b) > 0 {
            start = b.borrow().copy();
            end = a.borrow().copy();
        }
        PptxTextEdit::clamp(body, &start);
        PptxTextEdit::clamp(body, &end);
        if start.borrow().isSame(&end) {
            return start;
        }
        let mut first_opt: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, start.borrow().para.clone());
        if first_opt.is_none() {
            return start;
        }
        let mut first: Rc<RefCell<PptxParagraph>> = first_opt.clone().unwrap();
        if start.borrow().para == end.borrow().para {
            let cut: i64 = PptxTextEdit::splitRunsAt(&first, start.borrow().col.clone());
            let cut2: i64 = PptxTextEdit::splitRunsAt(&first, end.borrow().col.clone());
            let mut keep: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
            let mut i: i64 = 0;
            while i < (first.borrow().runs.len() as i64) {
                if i < cut {
                    keep.push(first.borrow().runs[i as usize].clone());
                }
                if i >= cut2 {
                    keep.push(first.borrow().runs[i as usize].clone());
                }
                i += 1;
            }
            first.borrow_mut().runs = keep.clone();
            PptxTextEdit::mergeRuns(&first);
            return start;
        }
        let mut last_opt: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, end.borrow().para.clone());
        if last_opt.is_none() {
            return start;
        }
        let mut last: Rc<RefCell<PptxParagraph>> = last_opt.clone().unwrap();
        let head_cut: i64 = PptxTextEdit::splitRunsAt(&first, start.borrow().col.clone());
        let tail_cut: i64 = PptxTextEdit::splitRunsAt(&last, end.borrow().col.clone());
        let mut joined: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
        let mut h: i64 = 0;
        while h < head_cut {
            joined.push(first.borrow().runs[h as usize].clone());
            h += 1;
        }
        let mut t: i64 = tail_cut;
        while t < (last.borrow().runs.len() as i64) {
            joined.push(last.borrow().runs[t as usize].clone());
            t += 1;
        }
        first.borrow_mut().runs = joined.clone();
        PptxTextEdit::mergeRuns(&first);
        let mut paras: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
        let mut j: i64 = 0;
        while j < (body.borrow().paragraphs.len() as i64) {
            let mut keep_it: bool = true;
            if j > start.borrow().para {
                if j <= end.borrow().para {
                    keep_it = false;
                }
            }
            if keep_it {
                paras.push(body.borrow().paragraphs[j as usize].clone());
            }
            j += 1;
        }
        body.borrow_mut().paragraphs = paras.clone();
        start
    }
    /// --- moving --------------------------------------------------------------
    /// Shared with the .docx editor and the markdown one, in
    /// `gallery/office/text/OfficeCaretMotion`. The bodies were here; the .docx
    /// editor stepped by one UTF-16 unit and landed inside surrogate pairs,
    /// which is what moving them was for.
    pub fn stepBack(s: &str, col: i64) -> i64 {
        OfficeCaretMotion::stepBack(s, col)
    }
    pub fn stepForward(s: &str, col: i64) -> i64 {
        OfficeCaretMotion::stepForward(s, col)
    }
    pub fn moveLeft(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut out: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        PptxTextEdit::clamp(body, &out);
        if out.borrow().col > 0 {
            let line: String = PptxTextEdit::paraText(body, out.borrow().para.clone());
            let _tmp_4826 = PptxTextEdit::stepBack(&line, out.borrow().col.clone());
            out.borrow_mut().col = _tmp_4826;
            return out;
        }
        if out.borrow().para > 0 {
            let hoist_10 = out.borrow().para - 1;
            out.borrow_mut().para = hoist_10;
            let _tmp_4829 = PptxTextEdit::paraLen(body, out.borrow().para.clone());
            out.borrow_mut().col = _tmp_4829;
        }
        out
    }
    pub fn moveRight(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut out: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        PptxTextEdit::clamp(body, &out);
        let line: String = PptxTextEdit::paraText(body, out.borrow().para.clone());
        if out.borrow().col < (line.as_bytes().len() as i64) {
            let _tmp_4832 = PptxTextEdit::stepForward(&line, out.borrow().col.clone());
            out.borrow_mut().col = _tmp_4832;
            return out;
        }
        if out.borrow().para < (body.borrow().paragraphs.len() as i64) - 1 {
            let hoist_11 = out.borrow().para + 1;
            out.borrow_mut().para = hoist_11;
            out.borrow_mut().col = 0;
        }
        out
    }
    pub fn moveUp(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut out: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        if out.borrow().para > 0 {
            let hoist_12 = out.borrow().para - 1;
            out.borrow_mut().para = hoist_12;
        }
        PptxTextEdit::clamp(body, &out);
        out
    }
    pub fn moveDown(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut out: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        if out.borrow().para < (body.borrow().paragraphs.len() as i64) - 1 {
            let hoist_13 = out.borrow().para + 1;
            out.borrow_mut().para = hoist_13;
        }
        PptxTextEdit::clamp(body, &out);
        out
    }
    pub fn lineStart(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        return PptxTextPos::of(pos.borrow().para.clone(), 0);
    }
    pub fn lineEnd(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut _tmp_4837 = pos.borrow().para.clone();
        return PptxTextPos::of(_tmp_4837, PptxTextEdit::paraLen(body, pos.borrow().para.clone()));
    }
    pub fn docStart(body: &Rc<RefCell<PptxTextBody>>) -> Rc<RefCell<PptxTextPos>> {
        PptxTextPos::of(0, 0)
    }
    pub fn docEnd(body: &Rc<RefCell<PptxTextBody>>) -> Rc<RefCell<PptxTextPos>> {
        let n: i64 = body.borrow().paragraphs.len() as i64;
        if n == 0 {
            return PptxTextPos::of(0, 0);
        }
        PptxTextPos::of(n - 1, PptxTextEdit::paraLen(body, n - 1))
    }
    pub fn isWordChar(c: i64) -> bool {
        OfficeCaretMotion::isWordChar(c)
    }
    /// Back over any spaces, then back over the word before them. Forward is
    /// the mirror: over the word, then over the spaces after it.
    pub fn wordLeft(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut out: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        PptxTextEdit::clamp(body, &out);
        if out.borrow().col == 0 {
            return PptxTextEdit::moveLeft(body, &out);
        }
        let line: String = PptxTextEdit::paraText(body, out.borrow().para.clone());
        let _tmp_4841 = OfficeCaretMotion::wordLeft(&line, out.borrow().col.clone());
        out.borrow_mut().col = _tmp_4841;
        out
    }
    pub fn wordRight(body: &Rc<RefCell<PptxTextBody>>, pos: &Rc<RefCell<PptxTextPos>>) -> Rc<RefCell<PptxTextPos>> {
        let mut out: Rc<RefCell<PptxTextPos>> = pos.borrow().copy();
        PptxTextEdit::clamp(body, &out);
        let line: String = PptxTextEdit::paraText(body, out.borrow().para.clone());
        let n: i64 = line.as_bytes().len() as i64;
        if out.borrow().col >= n {
            return PptxTextEdit::moveRight(body, &out);
        }
        let _tmp_4844 = OfficeCaretMotion::wordRight(&line, out.borrow().col.clone());
        out.borrow_mut().col = _tmp_4844;
        out
    }
    /// --- styling a range -----------------------------------------------------
    /// `what` is "bold", "italic", "size", "color" or "family".
    pub fn applyStyle(body: &Rc<RefCell<PptxTextBody>>, a: &Rc<RefCell<PptxTextPos>>, b: &Rc<RefCell<PptxTextPos>>, what: &str, flag: bool, size: f64, text: String) {
        let mut start: Rc<RefCell<PptxTextPos>> = a.borrow().copy();
        let mut end: Rc<RefCell<PptxTextPos>> = b.borrow().copy();
        if PptxTextEdit::cmp(a, b) > 0 {
            start = b.borrow().copy();
            end = a.borrow().copy();
        }
        PptxTextEdit::clamp(body, &start);
        PptxTextEdit::clamp(body, &end);
        let mut pi: i64 = start.borrow().para;
        while pi <= end.borrow().para {
            let mut p: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, pi);
            if p.is_some() {
                let mut para: Rc<RefCell<PptxParagraph>> = p.clone().unwrap();
                let mut from: i64 = 0;
                let mut to: i64 = PptxTextEdit::paraLen(body, pi);
                if pi == start.borrow().para {
                    from = start.borrow().col;
                }
                if pi == end.borrow().para {
                    to = end.borrow().col;
                }
                if to > from {
                    let first_idx: i64 = PptxTextEdit::splitRunsAt(&para, from);
                    let last_idx: i64 = PptxTextEdit::splitRunsAt(&para, to);
                    let mut ri: i64 = first_idx;
                    while ri < last_idx {
                        if ri < (para.borrow().runs.len() as i64) {
                            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                            // State the weight only when it differs from what
                            // the run would inherit anyway. Un-bolding a word
                            // inside a bold placeholder MUST be recorded, or it
                            // comes back bold; un-bolding one inside plain text
                            // must NOT be, or the paragraph keeps a run that
                            // says what its neighbours already say and never
                            // merges back. This is what Word does with its own
                            // `w:b`, and for the same reason.
                            if what == "bold" {
                                run.borrow_mut().bold = flag;
                                run.borrow_mut().boldSet = flag != PptxTextEdit::inheritedBold(body);
                            }
                            if what == "italic" {
                                run.borrow_mut().italic = flag;
                                run.borrow_mut().italicSet = flag != PptxTextEdit::inheritedItalic(body);
                            }
                            if what == "size" {
                                run.borrow_mut().fontSizePt = size;
                                run.borrow_mut().sizeSet = true;
                            }
                            if what == "color" {
                                run.borrow_mut().color = PptxColor::srgbOf(text.clone());
                                run.borrow_mut().colorStated = true;
                            }
                            if what == "family" {
                                run.borrow_mut().fontFamily = text.clone();
                            }
                        }
                        ri += 1;
                    }
                    PptxTextEdit::mergeRuns(&para);
                }
            }
            pi += 1;
        }
    }
    /// What a toggle should do next: if everything covered is already bold, the
    /// answer is "un-bold", the way every editor's button behaves.
    pub fn rangeIsAll(body: &Rc<RefCell<PptxTextBody>>, a: &Rc<RefCell<PptxTextPos>>, b: &Rc<RefCell<PptxTextPos>>, what: &str) -> bool {
        let mut start: Rc<RefCell<PptxTextPos>> = a.borrow().copy();
        let mut end: Rc<RefCell<PptxTextPos>> = b.borrow().copy();
        if PptxTextEdit::cmp(a, b) > 0 {
            start = b.borrow().copy();
            end = a.borrow().copy();
        }
        PptxTextEdit::clamp(body, &start);
        PptxTextEdit::clamp(body, &end);
        let mut seen: i64 = 0;
        let mut all: bool = true;
        let mut pi: i64 = start.borrow().para;
        while pi <= end.borrow().para {
            let mut p: Option<Rc<RefCell<PptxParagraph>>> = PptxTextEdit::paraAt(body, pi);
            if p.is_some() {
                let mut para: Rc<RefCell<PptxParagraph>> = p.clone().unwrap();
                let mut from: i64 = 0;
                let mut to: i64 = PptxTextEdit::paraLen(body, pi);
                if pi == start.borrow().para {
                    from = start.borrow().col;
                }
                if pi == end.borrow().para {
                    to = end.borrow().col;
                }
                let mut at: i64 = 0;
                let mut ri: i64 = 0;
                while ri < (para.borrow().runs.len() as i64) {
                    let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                    let __len: i64 = run.borrow().text.as_bytes().len() as i64;
                    let s0: i64 = at;
                    let s1: i64 = at + __len;
                    let mut overlaps: bool = true;
                    if s1 <= from {
                        overlaps = false;
                    }
                    if s0 >= to {
                        overlaps = false;
                    }
                    if __len == 0 {
                        overlaps = false;
                    }
                    if overlaps {
                        seen += 1;
                        if what == "bold" {
                            if !run.borrow().bold {
                                all = false;
                            }
                        }
                        if what == "italic" {
                            if !run.borrow().italic {
                                all = false;
                            }
                        }
                    }
                    at = s1;
                    ri += 1;
                }
            }
            pi += 1;
        }
        if seen == 0 {
            return false;
        }
        all
    }
}
