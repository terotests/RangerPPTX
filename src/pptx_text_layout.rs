// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::pptx_model::PptxParagraph;
use crate::pptx_model::PptxTextBody;
use crate::pptx_model::PptxTextRun;
use crate::rgr::EVGCodepoint;
use crate::rgr::{EVGCodepoint};


/// ==============================================================================
/// PptxTextLayout - a shape's text broken into lines, once, for everyone
/// ==============================================================================
///
/// The painter used to hand the renderer a `maxWidth` and let it decide where a
/// line breaks, and the caret assumed a paragraph was one line. Both were true
/// separately and could not be true together: nothing in the program knew where
/// the second line of a wrapped paragraph started.
///
/// This is that knowledge, computed once from the model and the measurer, and
/// used by three things — the painter, the caret, and the click that puts the
/// caret somewhere. A line records which paragraph and which columns it covers,
/// so a position in the text and a position on the screen convert both ways.
///
/// Coordinates are painter pixels, with the pt→px factor handed in.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxTextLine {
    pub para: i64,
    pub startCol: i64,
    pub endCol: i64,
    /// where the text of this line starts
    pub x: f64,
    /// top of the line box
    pub y: f64,
    /// measured width, trailing spaces trimmed
    pub w: f64,
    /// line height
    pub h: f64,
    /// px of the tallest run on it
    pub size: f64,
    pub align: String,
    /// first line of its paragraph
    pub first: bool,
    /// drawn before the first line, when there is one
    pub bullet: String,
    pub bulletX: f64,
    pub bulletSize: f64,
    /// The scale this line was laid out at, glyphs and all.
    ///
    /// It is not always the host's scale. A body whose face was substituted is
    /// shrunk to fit by `substituteFit`, and the painter then lays out AND
    /// draws at that reduced scale. Anything that measures into a line after
    /// the fact — the caret, the selection band, the column a click landed on —
    /// has to use the same number or it measures glyphs that were never drawn
    /// at that size. It used to use the host's scale, and the caret drifted to
    /// the right of the text by however much the body had been shrunk: a pixel
    /// on a short label, and further with every column on a long one, which is
    /// exactly the "slightly wrong place" a person notices while typing.
    pub scale: f64,
}

impl PptxTextLine {
    pub fn new() -> Self {
        Self {
            para: 0,
            startCol: 0,
            endCol: 0,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            size: 18.0,
            align: "left".to_string(),
            first: false,
            bullet: String::new(),
            bulletX: 0.0,
            bulletSize: 14.0,
            scale: 1.0,
        }
    }
}

#[derive(Clone)]
pub struct PptxTextLayout {
}

impl PptxTextLayout {
    pub fn new() -> Self {
        Self {
        }
    }
    /// --- measuring a slice of a paragraph ------------------------------------
    pub fn paraText(para: &Rc<RefCell<PptxParagraph>>) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            out = format!("{}{}", out, run.borrow().text);
            i += 1;
        }
        out
    }
    /// The width of columns [from, to) of a paragraph, run by run, because a
    /// slice can cross a run boundary and the halves are different sizes.
    fn measureSlice(para: &Rc<RefCell<PptxParagraph>>, from: i64, to: i64, m: &Rc<RefCell<OfficeTextMeasure>>, scale: f64) -> f64 {
        if to <= from {
            return 0.0;
        }
        let mut total: f64 = 0.0;
        let mut at: i64 = 0;
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            let __len: i64 = run.borrow().text.as_bytes().len() as i64;
            let s0: i64 = at;
            let s1: i64 = at + __len;
            let mut a: i64 = from;
            let mut b: i64 = to;
            if a < s0 {
                a = s0;
            }
            if b > s1 {
                b = s1;
            }
            if b > a {
                let piece: String = (run.borrow().text)[(a - s0) as usize..(b - s0) as usize].to_string();
                let size: f64 = run.borrow().fontSizePt * scale;
                total += OfficeTextMeasure::widthOfIn(
                    &m,
                    &piece,
                    size,
                    run.borrow().bold,
                    run.borrow().italic,
                    &run.borrow().fontFamily
                );
            }
            at = s1;
            i += 1;
        }
        total
    }
    pub fn maxRunSize(para: &Rc<RefCell<PptxParagraph>>, from: i64, to: i64, scale: f64) -> f64 {
        let mut best: f64 = 0.0;
        let mut at: i64 = 0;
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
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
                if i == 0 {
                    overlaps = true;
                }
            }
            if overlaps {
                let size: f64 = run.borrow().fontSizePt * scale;
                if size > best {
                    best = size;
                }
            }
            at = s1;
            i += 1;
        }
        if best <= 0.0 {
            best = 18.0 * scale;
        }
        best
    }
    /// --- breaking ------------------------------------------------------------
    /// One rule, in `EVGCodepoint`, so this wrapper and the one the .docx
    /// reader goes through cannot disagree about where a line may break.
    pub fn isSpace(c: i64) -> bool {
        EVGCodepoint::isSpace(c)
    }
    /// The column after the next run of spaces, i.e. where the next line could
    /// start if this were the break. A chunk is "word + the spaces after it",
    /// which is what makes trailing spaces belong to the line above.
    /// What a numbered item is numbered.
    ///
    /// The file does not store the number. It stores that the paragraph is
    /// numbered, in which scheme, and where a run of them starts counting; the
    /// number is the paragraph's position among the numbered ones directly
    /// above it at the same level. A paragraph that is not numbered, or is at a
    /// shallower level, ends the run — which is what makes a sub-list restart
    /// at one and the list it interrupted carry on afterwards.
    pub fn autoNumber(body: &Rc<RefCell<PptxTextBody>>, at: i64) -> String {
        let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[at as usize].clone();
        let mut n: i64 = para.borrow().bulletStartAt;
        let mut i: i64 = at - 1;
        let mut counting: bool = true;
        while counting {
            if i < 0 {
                counting = false;
            } else {
                let mut prev: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[i as usize].clone();
                if prev.borrow().level < para.borrow().level {
                    // The item this one is nested under: the sub-list starts
                    // over rather than continuing its parent's count.
                    counting = false;
                } else {
                    if prev.borrow().level == para.borrow().level {
                        if !prev.borrow().bulletAutoNum {
                            counting = false;
                        } else {
                            n += 1;
                        }
                    }
                    // A DEEPER level between two items is the sub-list under
                    // the earlier one, and does not interrupt this count.
                }
                i -= 1;
            }
        }
        return PptxTextLayout::numberIn(&para.borrow().bulletScheme.clone(), n);
    }
    /// The ordinal written the way the scheme asks for it. The schemes here are
    /// the ones a deck actually uses; anything else counts in arabic, which is
    /// wrong in style but never wrong in order.
    pub fn numberIn(scheme: &str, n: i64) -> String {
        let mut body: String = n.to_string();
        if PptxText::index_of(&scheme, "alphaLc") == 0 {
            body = PptxTextLayout::alpha(n, false);
        }
        if PptxText::index_of(&scheme, "alphaUc") == 0 {
            body = PptxTextLayout::alpha(n, true);
        }
        if PptxText::index_of(&scheme, "romanLc") == 0 {
            body = PptxTextLayout::roman(n, false);
        }
        if PptxText::index_of(&scheme, "romanUc") == 0 {
            body = PptxTextLayout::roman(n, true);
        }
        if PptxTextLayout::tailIs(scheme, "ParenBoth") {
            return format!("({})", body);
        }
        if PptxTextLayout::tailIs(scheme, "ParenR") {
            return format!("{})", body);
        }
        format!("{}.", body)
    }
    pub fn tailIs(s: &str, tail: &str) -> bool {
        let n: i64 = s.as_bytes().len() as i64;
        let t: i64 = tail.as_bytes().len() as i64;
        if t > n {
            return false;
        }
        s[(n - t) as usize..(n) as usize].to_string() == tail
    }
    /// a, b, … z, aa, ab — the spreadsheet column rule, which is what these
    /// schemes use once they run past 26.
    pub fn alpha(n: i64, upper: bool) -> String {
        let mut letters: String = "abcdefghijklmnopqrstuvwxyz";
        if upper {
            letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        }
        if n < 1 {
            return letters[(0) as usize..(1) as usize].to_string();
        }
        let mut out: String = String::new();
        let mut v: i64 = n;
        while v > 0 {
            let r: i64 = (v - 1) % 26;
            out = format!("{}{}", letters[(r) as usize..(r + 1) as usize].to_string(), out);
            v = (v - 1) / 26;
        }
        out
    }
    pub fn roman(n: i64, upper: bool) -> String {
        if n < 1 {
            return "i".to_string();
        }
        if n > 3999 {
            return n.to_string();
        }
        let mut vals: Vec<i64> = vec![1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
        let mut syms: Vec<String> = vec!["m".to_string(), "cm".to_string(), "d".to_string(), "cd".to_string(), "c".to_string(), "xc".to_string(), "l".to_string(), "xl".to_string(), "x".to_string(), "ix".to_string(), "v".to_string(), "iv".to_string(), "i".to_string()];
        if upper {
            syms = vec!["M".to_string(), "CM".to_string(), "D".to_string(), "CD".to_string(), "C".to_string(), "XC".to_string(), "L".to_string(), "XL".to_string(), "X".to_string(), "IX".to_string(), "V".to_string(), "IV".to_string(), "I".to_string()];
        }
        let mut out: String = String::new();
        let mut v: i64 = n;
        let mut i: i64 = 0;
        while i < (vals.len() as i64) {
            let val: i64 = vals[i as usize];
            while v >= val {
                out = format!("{}{}", out, syms[i as usize].clone());
                v -= val;
            }
            i += 1;
        }
        out
    }
    /// Where this line has to stop regardless of width: the next line break in
    /// the paragraph, or the end of it.
    pub fn nextBreak(text: &str, from: i64, n: i64) -> i64 {
        let mut i: i64 = from;
        while i < n {
            if (text.as_bytes()[i as usize] as i64) == 10 {
                return i;
            }
            i += 1;
        }
        n
    }
    pub fn nextChunk(text: &str, from: i64) -> i64 {
        let n: i64 = text.as_bytes().len() as i64;
        let mut i: i64 = from;
        let mut in_word: bool = true;
        while in_word {
            if i >= n {
                in_word = false;
            } else {
                let c: i64 = (text.as_bytes()[i as usize] as i64);
                if PptxTextLayout::isSpace(c) {
                    in_word = false;
                } else {
                    i += 1;
                    // A hyphen ends the chunk AFTER itself, so it stays on the
                    // line above — see `EVGCodepoint.breaksAfter`. Not when it
                    // is the chunk's first character: "-5" is not a place to
                    // break, and a chunk of nothing but a hyphen would loop.
                    if EVGCodepoint::breaksAfter(c) {
                        if i > from + 1 {
                            in_word = false;
                        }
                    }
                }
            }
        }
        let mut in_space: bool = true;
        while in_space {
            if i >= n {
                in_space = false;
            } else {
                if PptxTextLayout::isSpace((text.as_bytes()[i as usize] as i64)) {
                    i += 1;
                } else {
                    in_space = false;
                }
            }
        }
        if i <= from {
            return from + 1;
        }
        i
    }
    pub fn trimEndCol(text: &str, from: i64, to: i64) -> i64 {
        let mut end: i64 = to;
        while end > from {
            if PptxTextLayout::isSpace((text.as_bytes()[(end - 1) as usize] as i64)) {
                end -= 1;
            } else {
                return end;
            }
        }
        from
    }
    /// --- the layout ----------------------------------------------------------
    ///
    /// `x` / `y` / `w` / `h` are the shape's box in painter pixels; the insets
    /// are taken out here, the same way the painter used to take them out.
    /// Which edge a paragraph is aligned to.
    ///
    /// A paragraph that STATES an alignment gets it. One that does not is
    /// aligned to the start of its own reading direction: left for a
    /// left-to-right paragraph, RIGHT for a right-to-left one. That is what
    /// PowerPoint does, and it is the only thing it can do — DrawingML has no
    /// "start" or "end" the way Word's `w:jc` does, so an unstated alignment
    /// has to be resolved from the direction rather than defaulted to a side.
    ///
    /// The deck this was reported on is the ordinary case: an Arabic body with
    /// no `<a:pPr>` at all, so no `algn` and no `rtl` either. Every line of it
    /// came out ragged against the LEFT edge of the box while PowerPoint had it
    /// flush against the right.
    ///
    /// Where the direction comes from, in order: what the paragraph says, and
    /// failing that what its text says — which is the same first-strong-
    /// character rule the bidirectional algorithm resolves a paragraph level
    /// with, and is exactly the question `OfficeText.isRtl` answers.
    pub fn alignOf(para: &Rc<RefCell<PptxParagraph>>, text: &str) -> String {
        if para.borrow().alignSet {
            return para.borrow().align.clone();
        }
        if para.borrow().rtl == 1 {
            return "r".to_string();
        }
        if para.borrow().rtl == 0 {
            return para.borrow().align.clone();
        }
        if OfficeText::isRtl(text) {
            return "r".to_string();
        }
        return para.borrow().align.clone();
    }
    /// ------------------------------------------------------------------------
    /// Text columns
    /// ------------------------------------------------------------------------
    ///
    /// `a:bodyPr@numCol` — a text body that fills the first column to the
    /// bottom of the box and carries on in the next. PowerPoint does it; a
    /// reader that ignores it draws every column's worth of text down the first
    /// one and out through the bottom of the box, over whatever is under it.
    ///
    /// The lines are laid out at ONE column's width and then dealt into the
    /// columns, which is the same shape as the markdown layout's own column
    /// flow and for the same reason: laying each column out separately
    /// re-measures and re-wraps, and the second pass would not agree with the
    /// first about where a line ends.
    ///
    /// A line keeps its y RELATIVE to the top of its column rather than being
    /// re-stacked by height, so the space a paragraph asked for before and
    /// after it survives the deal.
    fn layout(body: &Rc<RefCell<PptxTextBody>>, m: &Rc<RefCell<OfficeTextMeasure>>, scale: f64, x: f64, y: f64, w: f64, h: f64) -> Vec<Rc<RefCell<PptxTextLine>>> {
        if body.borrow().columns <= 1 {
            return PptxTextLayout::layoutFlow(body, m, scale, x, y, w, h);
        }
        let n: i64 = body.borrow().columns;
        let gap: f64 = body.borrow().columnGapPt * scale;
        let col_w: f64 = (w - gap * ((n - 1) as f64)) / (n as f64);
        if col_w < 16.0 {
            return PptxTextLayout::layoutFlow(body, m, scale, x, y, w, h);
        }
        let mut lines: Vec<Rc<RefCell<PptxTextLine>>> = PptxTextLayout::layoutFlow(
            body,
            m,
            scale,
            x,
            y,
            col_w,
            h
        );
        let count: i64 = lines.len() as i64;
        if count == 0 {
            return lines;
        }
        let content_h: f64 = (h - body.borrow().insetT * scale) - body.borrow().insetB * scale;
        if content_h < 8.0 {
            return lines;
        }
        // Dealt by PARAGRAPH, not by line. A paragraph's bullet is on its
        // first line and its wrapped remainder on the rest, so a deal that
        // cuts between them puts the bullet in one column and the words in
        // the next — which is what the first attempt at this did.
        let mut first_line: Rc<RefCell<PptxTextLine>> = lines[0].clone();
        let top: f64 = first_line.borrow().y;
        let mut col_start: f64 = first_line.borrow().y;
        let mut col: i64 = 0;
        let mut i: i64 = 0;
        while i < count {
            let mut ln: Rc<RefCell<PptxTextLine>> = lines[i as usize].clone();
            if ln.borrow().first {
                // How tall this whole paragraph is, before deciding where it
                // goes: the distance from its first line to the end of its
                // last.
                let para: i64 = ln.borrow().para;
                let mut bottom: f64 = ln.borrow().y + ln.borrow().h;
                let mut k: i64 = i;
                while k < count {
                    let mut peek: Rc<RefCell<PptxTextLine>> = lines[k as usize].clone();
                    if peek.borrow().para != para {
                        break;
                    }
                    let pb: f64 = peek.borrow().y + peek.borrow().h;
                    if pb > bottom {
                        bottom = pb;
                    }
                    k += 1;
                }
                let need: f64 = bottom - ln.borrow().y;
                let used: f64 = ln.borrow().y - col_start;
                // The first paragraph is in the first column whatever its
                // height: a box shorter than one line would otherwise push
                // everything one column along and leave the first empty.
                if i > 0 {
                    if used + need > content_h {
                        if col < n - 1 {
                            col += 1;
                            col_start = ln.borrow().y;
                        }
                    }
                }
            }
            let rel: f64 = ln.borrow().y - col_start;
            let dx: f64 = (col as f64) * (col_w + gap);
            let hoist_16 = ln.borrow().x + dx;
            ln.borrow_mut().x = hoist_16;
            let hoist_17 = ln.borrow().bulletX + dx;
            ln.borrow_mut().bulletX = hoist_17;
            ln.borrow_mut().y = top + rel;
            i += 1;
        }
        lines
    }
    fn layoutFlow(body: &Rc<RefCell<PptxTextBody>>, m: &Rc<RefCell<OfficeTextMeasure>>, scale: f64, x: f64, y: f64, w: f64, h: f64) -> Vec<Rc<RefCell<PptxTextLine>>> {
        let mut out: Vec<Rc<RefCell<PptxTextLine>>> = Vec::new();
        let pad_l: f64 = body.borrow().insetL * scale;
        let pad_t: f64 = body.borrow().insetT * scale;
        let pad_r: f64 = body.borrow().insetR * scale;
        let pad_b: f64 = body.borrow().insetB * scale;
        let mut content_w: f64 = (w - pad_l) - pad_r;
        if content_w < 8.0 {
            content_w = 8.0;
        }
        let content_h: f64 = (h - pad_t) - pad_b;
        let content_x: f64 = x + pad_l;
        let mut total: f64 = 0.0;
        let mut pi: i64 = 0;
        while pi < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[pi as usize].clone();
            let text: String = PptxTextLayout::paraText(&para);
            let n: i64 = text.as_bytes().len() as i64;
            let align: String = PptxTextLayout::alignOf(&para, &text);
            // Where the text starts, and where the bullet hangs. The deck says
            // both — `marL` is the text's left edge and `indent` is the bullet's
            // offset from it, negative for the hanging bullet every list has —
            // and only when it says neither is a level's worth invented.
            let mut indent: f64 = 0.0;
            if para.borrow().marginSet {
                indent = para.borrow().marginLeftPt * scale;
            } else {
                indent = ((para.borrow().level as f64) * 18.0) * scale;
            }
            let mut bullet: String = String::new();
            let mut bullet_w: f64 = 0.0;
            let text_size: f64 = PptxTextLayout::maxRunSize(&para, 0, n, scale);
            let mut bullet_size: f64 = text_size;
            if para.borrow().bulletSizePt > 0.0 {
                bullet_size = para.borrow().bulletSizePt * scale;
            } else {
                if para.borrow().bulletSizePct != 100.0 {
                    bullet_size = text_size * (para.borrow().bulletSizePct / 100.0);
                }
            }
            if para.borrow().hasBullet {
                bullet = format!("{} ", para.borrow().bulletChar);
                if para.borrow().bulletAutoNum {
                    bullet = format!("{} ", PptxTextLayout::autoNumber(body, pi));
                }
                bullet_w = OfficeTextMeasure::widthOf(&m, &bullet, bullet_size, false, false);
                if !para.borrow().indentSet {
                    indent += 12.0 * scale;
                }
            }
            // Where the bullet hangs. With an `indent` the deck says it: that
            // far from the text's left edge, to the left for the hanging
            // bullet every list has, and the text keeps its own edge. Without
            // one the bullet sits where the level's indent began and the text
            // follows it.
            let mut bullet_at: f64 = (content_x + indent) - 12.0 * scale;
            let mut text_shift: f64 = bullet_w;
            if para.borrow().indentSet {
                bullet_at = (content_x + indent) + para.borrow().indentPt * scale;
                // a bullet that does not hang (indent ≥ 0) pushes the first
                // line's text past itself, never under it
                text_shift = 0.0;
                if bullet_w > 0.0 && bullet_at + bullet_w > content_x + indent {
                    text_shift = (bullet_at + bullet_w) - (content_x + indent);
                }
            }
            let mut avail: f64 = (content_w - indent) - text_shift;
            if avail < 8.0 {
                avail = 8.0;
            }
            let mut spacing: f64 = para.borrow().lineSpacingPct / 100.0;
            if spacing < 0.8 {
                spacing = 1.0;
            }
            total += para.borrow().spaceBeforePt * scale;
            let mut start: i64 = 0;
            let mut more: bool = true;
            let mut first_line: bool = true;
            while more {
                // A line break INSIDE the paragraph: `<a:br/>`, which a deck
                // writes wherever someone pressed shift+Enter, and a newline
                // left in a run's text. Both end the line here whatever room
                // is left on it, so this line's text stops at `limit` and the
                // wrap below only ever looks inside that.
                let limit: i64 = PptxTextLayout::nextBreak(&text, start, n);
                let mut end: i64 = limit;
                if body.borrow().wrap {
                    end = start;
                    let mut acc: f64 = 0.0;
                    let mut cursor: i64 = start;
                    let mut stop: bool = false;
                    while !stop {
                        if cursor >= limit {
                            stop = true;
                        } else {
                            let mut next_at: i64 = PptxTextLayout::nextChunk(&text, cursor);
                            if next_at > limit {
                                next_at = limit;
                            }
                            let chunk_w: f64 = PptxTextLayout::measureSlice(&para, cursor, next_at, m, scale);
                            let mut fits: bool = true;
                            if acc + chunk_w > avail {
                                fits = false;
                            }
                            if fits {
                                acc += chunk_w;
                                cursor = next_at;
                                end = next_at;
                            } else {
                                if end > start {
                                    stop = true;
                                } else {
                                    // Nothing fits, and the chunk is a single
                                    // word wider than the box. It goes on the
                                    // line ANYWAY and overflows.
                                    //
                                    // This used to break between letters, which
                                    // is a thing no office program does: neither
                                    // PowerPoint nor Word nor Google Slides
                                    // hyphenates on its own, and a label chopped
                                    // mid-word reads as a bug in the document
                                    // rather than as a box that is too small.
                                    // The author can see a word sticking out;
                                    // they cannot see why "Käyttöliittymät"
                                    // became "Käyttöliittymä" and "t".
                                    end = next_at;
                                    stop = true;
                                }
                            }
                        }
                    }
                    if end <= start {
                        end = limit;
                        if limit > start {
                            end = start + 1;
                        }
                    }
                }
                let mut line: Rc<RefCell<PptxTextLine>> = Rc::new(RefCell::new(PptxTextLine::new()));
                line.borrow_mut().para = pi;
                line.borrow_mut().startCol = start;
                line.borrow_mut().endCol = end;
                line.borrow_mut().first = first_line;
                line.borrow_mut().align = align.clone();
                line.borrow_mut().size = PptxTextLayout::maxRunSize(&para, start, end, scale);
                let mut line_h: f64 = (line.borrow().size * spacing) * 1.2;
                if line_h < line.borrow().size * 1.15 {
                    line_h = line.borrow().size * 1.15;
                }
                line.borrow_mut().h = line_h;
                let visible_end: i64 = PptxTextLayout::trimEndCol(&text, start, end);
                line.borrow_mut().w = PptxTextLayout::measureSlice(&para, start, visible_end, m, scale);
                let mut line_x: f64 = (content_x + indent) + text_shift;
                if align == "ctr" {
                    line_x += (avail - line.borrow().w) * 0.5;
                }
                if align == "r" {
                    line_x += avail - line.borrow().w;
                }
                line.borrow_mut().x = line_x;
                line.borrow_mut().y = total;
                line.borrow_mut().scale = scale;
                if first_line {
                    line.borrow_mut().bullet = bullet.clone();
                    line.borrow_mut().bulletX = bullet_at;
                    line.borrow_mut().bulletSize = bullet_size;
                }
                out.push(line.clone());
                total += line_h;
                first_line = false;
                start = end;
                // The break itself belongs to neither line.
                if start < n {
                    if (text.as_bytes()[start as usize] as i64) == 10 {
                        start += 1;
                        // A break as the last thing in a paragraph still opens
                        // a line, empty, the way it does everywhere else.
                        if start >= n {
                            let mut tail: Rc<RefCell<PptxTextLine>> = Rc::new(RefCell::new(PptxTextLine::new()));
                            tail.borrow_mut().para = pi;
                            tail.borrow_mut().startCol = start;
                            tail.borrow_mut().endCol = start;
                            tail.borrow_mut().first = false;
                            tail.borrow_mut().align = align.clone();
                            tail.borrow_mut().size = PptxTextLayout::maxRunSize(&para, 0, n, scale);
                            let hoist_18 = (tail.borrow().size * spacing) * 1.2;
                            tail.borrow_mut().h = hoist_18;
                            tail.borrow_mut().x = content_x + indent;
                            tail.borrow_mut().y = total;
                            tail.borrow_mut().scale = scale;
                            out.push(tail.clone());
                            total += tail.borrow().h;
                        }
                    }
                }
                if start >= n {
                    more = false;
                }
            }
            total += para.borrow().spaceAfterPt * scale;
            pi += 1;
        }
        // Anchoring: now that every line is known, so is the block's height.
        let mut origin_y: f64 = y + pad_t;
        if body.borrow().anchor == "ctr" {
            origin_y = (y + pad_t) + (content_h - total) * 0.5;
        }
        if body.borrow().anchor == "b" {
            origin_y = (y + pad_t) + (content_h - total);
        }
        // No clamp to the top of the box.
        //
        // Text taller than the box it is in OVERFLOWS, and which way it
        // overflows is exactly what the anchor decides: a bottom-anchored
        // block keeps its last line on the box's bottom edge and grows
        // upward, a centred one grows both ways. Clamping the origin to the
        // top of the box turned every one of those into a top-anchored block
        // that spilled DOWNWARD instead — a title bottom-anchored in its
        // placeholder, which is how a deck puts a title above a subtitle, came
        // out written over the subtitle.
        //
        // PowerPoint and Google Slides both let it overflow. Nothing is lost
        // by allowing it: the shape has no clip, so the only thing the clamp
        // ever achieved was moving the overflow to the wrong end.
        let mut li: i64 = 0;
        while li < (out.len() as i64) {
            let mut ln: Rc<RefCell<PptxTextLine>> = out[li as usize].clone();
            let hoist_19 = ln.borrow().y + origin_y;
            ln.borrow_mut().y = hoist_19;
            li += 1;
        }
        out
    }
    /// --- looking a position up -----------------------------------------------
    pub fn lineIndexFor(lines: &Vec<Rc<RefCell<PptxTextLine>>>, para: i64, col: i64) -> i64 {
        let mut best: i64 = 0 - 1;
        let mut i: i64 = 0;
        while i < (lines.len() as i64) {
            let mut ln: Rc<RefCell<PptxTextLine>> = lines[i as usize].clone();
            if ln.borrow().para == para {
                if col >= ln.borrow().startCol {
                    best = i;
                }
                if best < 0 {
                    best = i;
                }
            }
            i += 1;
        }
        best
    }
    /// The nearest line to a y, for a click that landed between or outside them.
    pub fn lineAtY(lines: &Vec<Rc<RefCell<PptxTextLine>>>, py: f64) -> i64 {
        if lines.is_empty() {
            return 0 - 1;
        }
        let mut best: i64 = 0;
        let mut best_dist: f64 = 1000000.0;
        let mut i: i64 = 0;
        while i < (lines.len() as i64) {
            let mut ln: Rc<RefCell<PptxTextLine>> = lines[i as usize].clone();
            let mid: f64 = ln.borrow().y + ln.borrow().h * 0.5;
            let d: f64 = (py - mid).abs();
            if d < best_dist {
                best_dist = d;
                best = i;
            }
            i += 1;
        }
        best
    }
}
