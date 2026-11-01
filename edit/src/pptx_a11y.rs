// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use pptx::pptx_model::PptxPresentation;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxSlide;
use pptx::pptx_model::PptxTableCell;
use crate::rgr::{EVGA11yNode, EVGA11yRole, EVGA11yTree};


/// ==============================================================================
/// PptxA11y — what the deck MEANS, beside what it looks like
/// ==============================================================================
///
/// The last of the three. The spreadsheet has published an `EVGA11yTree` for a
/// while and the document viewer now does too; a deck published nothing, and a
/// screen reader cannot read a canvas, so a presentation opened here was a blank
/// window to anyone using one.
///
/// What a deck is, to a reader, and what that decides:
///
///   A DECK IS A LIST OF SLIDES, and the position in it is the single most
///   important thing — "slide 7 of 30" is how anyone navigating a presentation
///   knows where they are. So every slide carries `posInSet` and `setSize`, and
///   the slide's name is its own title where it has one. A deck of untitled
///   slides is a deck nobody can navigate, and saying "Slide 12" for one is not
///   a workaround, it is the truth.
///
///   READING ORDER IS THE SHAPE ORDER, which is the z-order the file states and
///   NOT the order the shapes appear on screen. That is what PowerPoint's own
///   reading-order pane edits, and matching it means a deck fixed there is fixed
///   here. Sorting by position would look tidier and would disagree with every
///   other tool.
///
///   THE TITLE PLACEHOLDER IS A HEADING, because `p:ph type="title"` is the deck
///   equivalent of a heading style, and it is what makes a slide reachable by
///   the same navigate-by-heading command a document uses.
///
///   INHERITED CHROME IS NOT CONTENT. A shape that came from the master or the
///   layout is on every slide; met in the reading flow it would be read between
///   every two slides. It is left out, exactly as the document viewer leaves the
///   page header out of the body.
///
///   ALT TEXT IS THE AUTHOR'S OR IT IS ABSENT. `p:cNvPr/@descr` is all anyone can
///   say about a picture or a chart. Where a deck has none this says so rather
///   than inventing a description — a picture with no alt text is a documented
///   failure of the DECK, and covering it up makes it harder to find.
///
///   THE SPEAKER'S NOTES ARE PUBLISHED. They are the one part of a deck that is
///   already prose, and a reader that skips them skips the explanation.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxA11y {
}

impl PptxA11y {
    pub fn new() -> Self {
        Self {
        }
    }
    /// --- naming ---------------------------------------------------------------
    /// The slide's own title, from its title placeholder. Empty when the slide
    /// has none — which is common, and is why the caller falls back to the
    /// number rather than to the first text it can find: the first text box on a
    /// slide is very often a label, not a title, and announcing it as one sends
    /// a reader looking for a section that is not there.
    pub fn slideTitle(slide: &Rc<RefCell<PptxSlide>>) -> Rc<RefCell<PptxShape>> {
        let mut i: i64 = 0;
        while i < (slide.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = slide.borrow().shapes[i as usize].clone();
            if PptxA11y::isTitle(&sh) {
                if sh.borrow().hasText {
                    let txt: String = (sh.borrow().text).borrow().plainText();
                    if !txt.is_empty() {
                        return sh;
                    }
                }
            }
            i += 1;
        }
        Rc::new(RefCell::new(PptxShape::new()))
    }
    pub fn isTitle(sh: &Rc<RefCell<PptxShape>>) -> bool {
        if !sh.borrow().isPlaceholder {
            return false;
        }
        if sh.borrow().phType == "title" {
            return true;
        }
        if sh.borrow().phType == "ctrTitle" {
            return true;
        }
        false
    }
    /// What a reader says for a shape that has no text of its own.
    pub fn shapeName(sh: &Rc<RefCell<PptxShape>>) -> String {
        if !sh.borrow().altText.is_empty() {
            return sh.borrow().altText.clone();
        }
        if sh.borrow().hasText {
            let txt: String = (sh.borrow().text).borrow().plainText();
            if !txt.is_empty() {
                return txt;
            }
        }
        if !sh.borrow().name.is_empty() {
            return sh.borrow().name.clone();
        }
        PptxA11y::roleWord(sh)
    }
    pub fn roleWord(sh: &Rc<RefCell<PptxShape>>) -> String {
        if sh.borrow().kind == PptxShapeKind::picture() {
            return "Picture".to_string();
        }
        if sh.borrow().kind == PptxShapeKind::chart() {
            return "Chart".to_string();
        }
        if sh.borrow().kind == PptxShapeKind::table() {
            return "Table".to_string();
        }
        if sh.borrow().kind == PptxShapeKind::group() {
            return "Group".to_string();
        }
        if sh.borrow().kind == PptxShapeKind::connector() {
            return "Connector".to_string();
        }
        "Shape".to_string()
    }
    /// A shape a reader has nothing to say about: no text, no alt text, no name
    /// beyond the generic one. An empty decorative rectangle is exactly this,
    /// and publishing thousands of them would bury the slide's content.
    pub fn isDecorative(sh: &Rc<RefCell<PptxShape>>) -> bool {
        if !sh.borrow().altText.is_empty() {
            return false;
        }
        if sh.borrow().kind != PptxShapeKind::shape() {
            return false;
        }
        if sh.borrow().hasText {
            let txt: String = (sh.borrow().text).borrow().plainText();
            if !txt.is_empty() {
                return false;
            }
        }
        true
    }
    /// --- the tree -------------------------------------------------------------
    /// `currentSlide` is the slide on screen, -1 when none. `selectedEditId` is
    /// the shape the editor has selected, 0 when nothing is.
    fn build(pres: &Rc<RefCell<PptxPresentation>>, current_slide: i64, selected_edit_id: i64) -> Rc<RefCell<EVGA11yTree>> {
        let mut t: Rc<RefCell<EVGA11yTree>> = Rc::new(RefCell::new(EVGA11yTree::new()));
        let total: i64 = pres.borrow().slides.len() as i64;
        let mut root: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::group(
            &t,
            "deck",
            "",
            format!("Presentation, {} slides", total)
        );
        root.borrow_mut().setSize = total;
        let mut si: i64 = 0;
        while si < total {
            let mut slide: Rc<RefCell<PptxSlide>> = pres.borrow().slides[si as usize].clone();
            let sid: String = format!("deck/slide:{}", si + 1);
            let mut g: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::group(
                &t,
                &sid,
                "deck",
                PptxA11y::slideLabel(&slide, si)
            );
            // "Slide 7 of 30" comes from these and nothing else. It is the only
            // sense of place a deck gives.
            g.borrow_mut().posInSet = si + 1;
            g.borrow_mut().setSize = total;
            g.borrow_mut().selected = si == current_slide;
            let x: i64 = slide.borrow().width.floor() as i64;
            let y: i64 = slide.borrow().height.floor() as i64;
            EVGA11yTree::place(&t, &g, 0, 0, x, y);
            let mut i: i64 = 0;
            while i < (slide.borrow().shapes.len() as i64) {
                let mut sh: Rc<RefCell<PptxShape>> = slide.borrow().shapes[i as usize].clone();
                PptxA11y::addShape(&t, &sh, &sid, i, selected_edit_id);
                i += 1;
            }
            // The notes, after the slide's own content and inside the slide, so
            // a reader meets them where the speaker would.
            let notes: String = slide.borrow().notesText();
            if !notes.is_empty() {
                let mut n: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::node(
                    &t,
                    format!("{}/notes", sid),
                    sid.clone(),
                    EVGA11yRole::text()
                );
                n.borrow_mut().name = "Speaker notes".to_string();
                n.borrow_mut().value = notes.clone();
            }
            si += 1;
        }
        if selected_edit_id > 0 {
            PptxA11y::focusSelected(&t);
        }
        t
    }
    pub fn slideLabel(slide: &Rc<RefCell<PptxSlide>>, index: i64) -> String {
        let mut title: Rc<RefCell<PptxShape>> = PptxA11y::slideTitle(slide);
        let num: String = format!("Slide {}", index + 1);
        if title.borrow().hasText {
            let txt: String = (title.borrow().text).borrow().plainText();
            if !txt.is_empty() {
                return format!("{}: {}", num, txt);
            }
        }
        num
    }
    /// One shape, and its children if it is a group.
    ///
    /// `ordinal` is the shape's position in the tree, which is the z-order the
    /// file states. It is in the id because a shape has no id of its own that
    /// survives a reload — `editId` is minted fresh on every attach — so the
    /// honest stable name is "the nth shape of this slide", which does survive
    /// one as long as the slide is not edited. Durable identity is the open
    /// problem noted in office/COLLABORATION.md; this does not pretend to have
    /// solved it.
    fn addShape(t: &Rc<RefCell<EVGA11yTree>>, sh: &Rc<RefCell<PptxShape>>, parent_id: &str, ordinal: i64, selected_edit_id: i64) {
        // Master and layout chrome is on every slide. Met in the reading flow
        // it would be read between every two slides.
        if sh.borrow().inherited {
            return;
        }
        if PptxA11y::isDecorative(sh) {
            return;
        }
        let id: String = format!("{}/shape:{}", parent_id, ordinal);
        let mut role: i64 = EVGA11yRole::text();
        if sh.borrow().kind == PptxShapeKind::picture() {
            role = EVGA11yRole::image();
        }
        if sh.borrow().kind == PptxShapeKind::chart() {
            role = EVGA11yRole::image();
        }
        if sh.borrow().kind == PptxShapeKind::group() {
            role = EVGA11yRole::group();
        }
        if sh.borrow().kind == PptxShapeKind::table() {
            PptxA11y::addTable(t, sh, &id, parent_id, selected_edit_id);
            return;
        }
        if PptxA11y::isTitle(sh) {
            // `p:ph type="title"` is the deck's heading style, and it is what
            // makes a slide reachable by a navigate-by-heading command.
            role = EVGA11yRole::heading();
        }
        let mut n: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::node(
            &t,
            id.clone(),
            parent_id.to_string(),
            role
        );
        n.borrow_mut().name = PptxA11y::shapeName(sh);
        if PptxA11y::isTitle(sh) {
            n.borrow_mut().description = "Slide title".to_string();
        }
        if sh.borrow().kind == PptxShapeKind::picture() {
            if sh.borrow().altText.is_empty() {
                n.borrow_mut().description = "No alt text".to_string();
            }
        }
        if sh.borrow().kind == PptxShapeKind::chart() {
            if sh.borrow().altText.is_empty() {
                n.borrow_mut().description = "No alt text".to_string();
            }
        }
        if sh.borrow().editId == selected_edit_id {
            n.borrow_mut().selected = true;
            n.borrow_mut().focusable = true;
        }
        EVGA11yTree::place(
            &t,
            &n,
            sh.borrow().x.floor() as i64,
            sh.borrow().y.floor() as i64,
            sh.borrow().width.floor() as i64,
            sh.borrow().height.floor() as i64
        );
        if sh.borrow().kind == PptxShapeKind::group() {
            let mut i: i64 = 0;
            while i < (sh.borrow().children.len() as i64) {
                let mut child: Rc<RefCell<PptxShape>> = sh.borrow().children[i as usize].clone();
                PptxA11y::addShape(t, &child, &id, i, selected_edit_id);
                i += 1;
            }
        }
    }
    /// A table is a grid. Its first row is announced as data rather than as
    /// headers: PowerPoint records a header row in the table's style, which this
    /// reader does not carry, and claiming every table's first row is headers
    /// would be wrong more often than it is right.
    fn addTable(t: &Rc<RefCell<EVGA11yTree>>, sh: &Rc<RefCell<PptxShape>>, id: &str, parent_id: &str, selected_edit_id: i64) {
        let mut g: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::node(
            &t,
            id.to_string(),
            parent_id.to_string(),
            EVGA11yRole::grid()
        );
        let rows: i64 = sh.borrow().tableRows;
        let cols: i64 = sh.borrow().tableCols;
        if !sh.borrow().altText.is_empty() {
            g.borrow_mut().name = sh.borrow().altText.clone();
        } else {
            g.borrow_mut().name = format!("Table, {} rows by {} columns", rows, cols.to_string());
        }
        g.borrow_mut().rowCount = rows;
        g.borrow_mut().colCount = cols;
        if sh.borrow().editId == selected_edit_id {
            g.borrow_mut().selected = true;
            g.borrow_mut().focusable = true;
        }
        EVGA11yTree::place(
            &t,
            &g,
            sh.borrow().x.floor() as i64,
            sh.borrow().y.floor() as i64,
            sh.borrow().width.floor() as i64,
            sh.borrow().height.floor() as i64
        );
        let mut r: i64 = 0;
        while r < rows {
            let rid: String = format!("{}/row:{}", id, r);
            let mut rn: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::node(
                &t,
                rid.clone(),
                id.to_string(),
                EVGA11yRole::row()
            );
            rn.borrow_mut().name = format!("Row {}", r + 1);
            rn.borrow_mut().rowIndex = r;
            rn.borrow_mut().colCount = cols;
            let mut c: i64 = 0;
            while c < cols {
                let idx: i64 = r * cols + c;
                if idx < (sh.borrow().tableCells.len() as i64) {
                    let mut cell: Rc<RefCell<PptxTableCell>> = sh.borrow().tableCells[idx as usize].clone();
                    let mut cn: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::node(
                        &t,
                        format!("{}/cell:{}", rid, c),
                        rid.clone(),
                        EVGA11yRole::cell()
                    );
                    let mut txt: String = cell.borrow().text.clone();
                    if txt.is_empty() {
                        txt = "Empty".to_string();
                    }
                    cn.borrow_mut().name = txt.clone();
                    cn.borrow_mut().rowIndex = r;
                    cn.borrow_mut().colIndex = c;
                    if cell.borrow().gridSpan > 1 {
                        cn.borrow_mut().colCount = cell.borrow().gridSpan;
                    }
                    if cell.borrow().rowSpan > 1 {
                        cn.borrow_mut().rowCount = cell.borrow().rowSpan;
                    }
                }
                c += 1;
            }
            r += 1;
        }
    }
    /// Focus follows the selection. `setFocus` is the tree's own, so exactly one
    /// node ends up focused however many are selected — two focused nodes is a
    /// reader with none.
    fn focusSelected(t: &Rc<RefCell<EVGA11yTree>>) {
        let mut i: i64 = 0;
        while i < EVGA11yTree::count(&t) {
            let mut n: Rc<RefCell<EVGA11yNode>> = EVGA11yTree::at(&t, i);
            if n.borrow().selected {
                if n.borrow().focusable {
                    EVGA11yTree::setFocus(&t, n.borrow().id.clone());
                    return;
                }
            }
            i += 1;
        }
    }
}
