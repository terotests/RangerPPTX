// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::pptx_model::PptxColor;
use crate::pptx_model::PptxFillStyle;
use crate::pptx_model::PptxLevelStyle;
use crate::pptx_model::PptxLineStyle;
use crate::pptx_model::PptxListStyle;
use crate::pptx_model::PptxParagraph;
use crate::pptx_model::PptxPresentation;
use crate::pptx_model::PptxShape;
use crate::pptx_model::PptxShapeKind;
use crate::pptx_model::PptxSheet;
use crate::pptx_model::PptxSlide;
use crate::pptx_model::PptxTableCell;
use crate::pptx_model::PptxTableLine;
use crate::pptx_model::PptxTextBody;
use crate::pptx_model::PptxTextRun;
use crate::pptx_model::PptxTheme;
use crate::rgr::SourceRef;


/// ==============================================================================
/// PptxResolver - theme / layout / master → resolved slide scene
/// ==============================================================================
///
/// Pipeline:
///   theme → slide master → slide layout → slide → ResolvedSlide → EVG
///
/// EVG never sees relationships, placeholders, or schemeClr — only concrete
/// geometry, sRGB colors, fonts, and image part paths after this pass.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxResolver {
}

impl PptxResolver {
    pub fn new() -> Self {
        Self {
        }
    }
    /// --- a box for a shape that was never given one ---------------------------
    ///
    /// Everything above is inheritance: the slide asks the layout, the layout
    /// asks the master. This is what happens when the answer is still nothing.
    ///
    /// A shape with words in it and no size is not a shape a reader can use.
    /// The painter clamps the wrap width to a few pixels, so the text comes
    /// down the left edge one word per line; the hit test asks whether the
    /// pointer is inside a box of zero width, and no pointer ever is, so the
    /// text cannot be selected, moved or corrected. Both of those were reported
    /// on the same deck, and they are the same missing number.
    ///
    /// So the reader states a box: where the file put the shape when it said,
    /// the slide's own margins when it did not, and a height the text actually
    /// needs. It is flagged `geomFallback` — the outline panel says so, and the
    /// writer never states it back into the file (`sheetXfrmSig`), so a save
    /// cannot turn this reader's guess into the document's own geometry.
    ///
    /// Only shapes that HAVE text get one. A zero-sized rectangle with nothing
    /// in it draws nothing in PowerPoint too, and inventing a box for it would
    /// put shapes on the slide that its author cannot see anywhere else.
    pub fn visibleText(shape: &Rc<RefCell<PptxShape>>) -> bool {
        if !shape.borrow().hasText {
            return false;
        }
        let mut body: Rc<RefCell<PptxTextBody>> = shape.borrow().text.clone().clone();
        let mut pi: i64 = 0;
        while pi < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[pi as usize].clone();
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                if !run.borrow().text.is_empty() {
                    return true;
                }
                ri += 1;
            }
            pi += 1;
        }
        false
    }
    /// How tall this text is at the sizes it resolved to, unwrapped: one line
    /// per paragraph, its tallest run, the spacing around it and the insets.
    /// It is a floor rather than a measurement — nothing here can measure a
    /// glyph — but a floor is what a box needs to stop being zero.
    pub fn textHeightPt(body: &Rc<RefCell<PptxTextBody>>) -> f64 {
        let mut total: f64 = 0.0;
        let mut pi: i64 = 0;
        while pi < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[pi as usize].clone();
            let mut size: f64 = 18.0;
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                if run.borrow().fontSizePt > size {
                    size = run.borrow().fontSizePt;
                }
                ri += 1;
            }
            let mut line: f64 = size * 1.2;
            line += para.borrow().spaceBeforePt + para.borrow().spaceAfterPt;
            total += line;
            pi += 1;
        }
        if total <= 0.0 {
            total = 21.6;
        }
        return total + (body.borrow().insetT + body.borrow().insetB);
    }
    /// Which of the master's three styles a placeholder of this kind uses.
    /// PowerPoint's own rule: titles take the title style, bodies and the things
    /// that behave like bodies take the body style, everything else — and every
    /// shape that is not a placeholder at all — takes the other style.
    pub fn styleForPh(master: &Rc<RefCell<PptxSheet>>, ph_type: &str) -> Rc<RefCell<PptxListStyle>> {
        if ph_type == "title" || ph_type == "ctrTitle" {
            return master.borrow().titleStyle.clone();
        }
        if (ph_type == "body" || ph_type == "subTitle") || ph_type == "obj" {
            return master.borrow().bodyStyle.clone();
        }
        return master.borrow().otherStyle.clone();
    }
    /// A size worth having: both sides positive. `cx="0"` is not a size and
    /// neither is a placeholder that was never given one.
    pub fn statedSize(s: &Rc<RefCell<PptxShape>>) -> bool {
        if s.borrow().width <= 0.0 {
            return false;
        }
        return s.borrow().height > 0.0;
    }
    /// --- colour adjustments ---------------------------------------------------
    ///
    /// DrawingML states a colour and then a chain of adjustments to it, and a
    /// theme's fill styles are made almost entirely of those adjustments — the
    /// colour in them is the placeholder `phClr`. Dropping them draws a themed
    /// shape flat, and dropping the STYLE draws it not at all.
    pub fn hexNibble(c: &str) -> i64 {
        let mut i: i64 = 0;
        let digits: String = "0123456789abcdef";
        let mut lower: String = c.to_string();
        if lower == "A" {
            lower = "a".to_string();
        }
        if lower == "B" {
            lower = "b".to_string();
        }
        if lower == "C" {
            lower = "c".to_string();
        }
        if lower == "D" {
            lower = "d".to_string();
        }
        if lower == "E" {
            lower = "e".to_string();
        }
        if lower == "F" {
            lower = "f".to_string();
        }
        while i < 16 {
            if digits[(i) as usize..(i + 1) as usize].to_string() == lower {
                return i;
            }
            i += 1;
        }
        0
    }
    /// Hex in, hex out, and the clamp between them — `OfficeColor`'s, shared
    /// with the spreadsheet. They were the same three functions written twice.
    pub fn byteAt(hex: &str, at: i64) -> i64 {
        OfficeColorMath::byteAt(hex, at)
    }
    pub fn hexOf(v: i64) -> String {
        OfficeColorMath::hexOf(v)
    }
    pub fn clamp01(v: f64) -> f64 {
        OfficeColorMath::clamp01(v)
    }
    /// A `PptxColor` as the shared carrier wants it. Only the modifiers matter
    /// here — this is handed straight to `applyDrawingMods` and thrown away.
    fn asOfficeColor(col: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<OfficeColor>> {
        let mut out: Rc<RefCell<OfficeColor>> = Rc::new(RefCell::new(OfficeColor::new()));
        out.borrow_mut().isSet = col.borrow().isSet;
        out.borrow_mut().isScheme = col.borrow().isScheme;
        out.borrow_mut().scheme = col.borrow().scheme.clone();
        out.borrow_mut().srgb = col.borrow().srgb.clone();
        out.borrow_mut().alpha = col.borrow().alpha;
        out.borrow_mut().lumMod = col.borrow().lumMod;
        out.borrow_mut().lumOff = col.borrow().lumOff;
        out.borrow_mut().tint = col.borrow().tint;
        out.borrow_mut().shade = col.borrow().shade;
        out.borrow_mut().satMod = col.borrow().satMod;
        out.borrow_mut().baked = col.borrow().baked;
        out
    }
    /// The adjustments, in the order DrawingML applies them. The maths is
    /// `OfficeColorMath.applyDrawingMods`, shared with the spreadsheet's
    /// drawings and charts — and deliberately NOT shared with Excel's cell
    /// `tint`, which scales HLS luminance rather than the RGB channels and is a
    /// different function with the same name.
    pub fn applyMods(hex: &str, col: &Rc<RefCell<PptxColor>>) -> String {
        OfficeColorMath::applyDrawingMods(hex, &PptxResolver::asOfficeColor(col))
    }
    /// Map placeholder / mapped scheme names used on slides (bg1, tx1, …) onto
    /// the theme's clrScheme keys. Masters can remap these; MVP uses defaults.
    pub fn mapSchemeName(&self, name: &str) -> String {
        OfficeTheme::mapSchemeName(name)
    }
    pub fn resolveColor(&self, c: &Rc<RefCell<PptxColor>>, theme: &Rc<RefCell<PptxTheme>>) -> Rc<RefCell<PptxColor>> {
        if !c.borrow().isSet {
            return c;
        }
        if !c.borrow().isScheme {
            return c;
        }
        if c.borrow().baked {
            return c;
        }
        let key: String = self.mapSchemeName(&c.borrow().scheme.clone());
        let hex: String = theme.borrow().colorOf(&key);
        let mut out: Rc<RefCell<PptxColor>> = PptxColor::srgbOf(hex.clone());
        out.borrow_mut().alpha = c.borrow().alpha;
        out.borrow_mut().copyModsFrom(c);
        // Where the colour CAME FROM is kept, not just what it resolved to.
        // Painting only ever reads `srgb`, so this costs nothing to draw — and
        // it is the whole of what re-theming a deck needs: every colour that
        // came from the palette can be asked to resolve again from a new one.
        out.borrow_mut().isScheme = true;
        out.borrow_mut().scheme = c.borrow().scheme.clone();
        out
    }
    pub fn resolveFont(&self, face: &str, theme: &Rc<RefCell<PptxTheme>>) -> String {
        if face.is_empty() {
            return theme.borrow().minorFont.clone();
        }
        if face == "+mj-lt" {
            return theme.borrow().majorFont.clone();
        }
        if face == "+mn-lt" {
            return theme.borrow().minorFont.clone();
        }
        if face == "+mj-ea" {
            return theme.borrow().majorFont.clone();
        }
        if face == "+mn-ea" {
            return theme.borrow().minorFont.clone();
        }
        face.to_string()
    }
    pub fn cloneColor(&self, c: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<PptxColor>> {
        let mut out: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        out.borrow_mut().isSet = c.borrow().isSet;
        out.borrow_mut().isScheme = c.borrow().isScheme;
        out.borrow_mut().scheme = c.borrow().scheme.clone();
        out.borrow_mut().srgb = c.borrow().srgb.clone();
        out.borrow_mut().alpha = c.borrow().alpha;
        out.borrow_mut().baked = c.borrow().baked;
        out.borrow_mut().copyModsFrom(c);
        out
    }
    pub fn cloneRun(&self, run: &Rc<RefCell<PptxTextRun>>) -> Rc<RefCell<PptxTextRun>> {
        let mut out: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        out.borrow_mut().text = run.borrow().text.clone();
        out.borrow_mut().fontFamily = run.borrow().fontFamily.clone();
        out.borrow_mut().fontSizePt = run.borrow().fontSizePt;
        out.borrow_mut().resolvedFontSig = run.borrow().resolvedFontSig.clone();
        out.borrow_mut().resolvedColorSig = run.borrow().resolvedColorSig.clone();
        out.borrow_mut().resolvedSizeSig = run.borrow().resolvedSizeSig.clone();
        out.borrow_mut().sizeSet = run.borrow().sizeSet;
        out.borrow_mut().bold = run.borrow().bold;
        out.borrow_mut().italic = run.borrow().italic;
        let _tmp_828 = self.cloneColor(&run.borrow().color.clone());
        out.borrow_mut().color = _tmp_828;
        out.borrow_mut().colorStated = run.borrow().colorStated;
        let _tmp_831 = self.cloneColor(&run.borrow().highlight.clone());
        out.borrow_mut().highlight = _tmp_831;
        out
    }
    pub fn cloneTextBody(&self, body: &Rc<RefCell<PptxTextBody>>) -> Rc<RefCell<PptxTextBody>> {
        let mut out: Rc<RefCell<PptxTextBody>> = Rc::new(RefCell::new(PptxTextBody::new()));
        out.borrow_mut().insetL = body.borrow().insetL;
        out.borrow_mut().insetT = body.borrow().insetT;
        out.borrow_mut().insetR = body.borrow().insetR;
        out.borrow_mut().insetB = body.borrow().insetB;
        out.borrow_mut().insetsSet = body.borrow().insetsSet;
        out.borrow_mut().anchor = body.borrow().anchor.clone();
        out.borrow_mut().anchorSet = body.borrow().anchorSet;
        out.borrow_mut().wrap = body.borrow().wrap;
        out.borrow_mut().hasDefRPr = body.borrow().hasDefRPr;
        out.borrow_mut().defFontSizePt = body.borrow().defFontSizePt;
        out.borrow_mut().defBold = body.borrow().defBold;
        out.borrow_mut().defItalic = body.borrow().defItalic;
        out.borrow_mut().defFontFamily = body.borrow().defFontFamily.clone();
        let _tmp_846 = self.cloneColor(&body.borrow().defColor.clone());
        out.borrow_mut().defColor = _tmp_846;
        out.borrow_mut().defColorFromStyle = body.borrow().defColorFromStyle;
        out.borrow_mut().defAlign = body.borrow().defAlign.clone();
        out.borrow_mut().fontScale = body.borrow().fontScale;
        out.borrow_mut().lineScale = body.borrow().lineScale;
        out.borrow_mut().listStyle = body.borrow().listStyle.clone();
        let mut i: i64 = 0;
        while i < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[i as usize].clone();
            let mut op: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            op.borrow_mut().align = para.borrow().align.clone();
            op.borrow_mut().alignSet = para.borrow().alignSet;
            op.borrow_mut().spaceBeforePt = para.borrow().spaceBeforePt;
            op.borrow_mut().spaceAfterPt = para.borrow().spaceAfterPt;
            op.borrow_mut().lineSpacingPct = para.borrow().lineSpacingPct;
            op.borrow_mut().level = para.borrow().level;
            op.borrow_mut().hasBullet = para.borrow().hasBullet;
            op.borrow_mut().bulletSet = para.borrow().bulletSet;
            op.borrow_mut().bulletChar = para.borrow().bulletChar.clone();
            op.borrow_mut().marginLeftPt = para.borrow().marginLeftPt;
            op.borrow_mut().marginSet = para.borrow().marginSet;
            op.borrow_mut().indentPt = para.borrow().indentPt;
            op.borrow_mut().indentSet = para.borrow().indentSet;
            let _tmp_866 = self.cloneColor(&para.borrow().bulletColor.clone());
            op.borrow_mut().bulletColor = _tmp_866;
            op.borrow_mut().bulletSizePct = para.borrow().bulletSizePct;
            op.borrow_mut().bulletSizePt = para.borrow().bulletSizePt;
            op.borrow_mut().bulletFont = para.borrow().bulletFont.clone();
            let mut j: i64 = 0;
            while j < (para.borrow().runs.len() as i64) {
                op.borrow_mut().runs.push(self.cloneRun(&para.borrow().runs[j as usize].clone()));
                j += 1;
            }
            out.borrow_mut().paragraphs.push(op.clone());
            i += 1;
        }
        out
    }
    pub fn cloneShape(&self, shape: &Rc<RefCell<PptxShape>>) -> Rc<RefCell<PptxShape>> {
        let mut out: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        out.borrow_mut().kind = shape.borrow().kind;
        out.borrow_mut().name = shape.borrow().name.clone();
        out.borrow_mut().x = shape.borrow().x;
        out.borrow_mut().y = shape.borrow().y;
        out.borrow_mut().width = shape.borrow().width;
        out.borrow_mut().height = shape.borrow().height;
        out.borrow_mut().hasXfrm = shape.borrow().hasXfrm;
        out.borrow_mut().hasOff = shape.borrow().hasOff;
        out.borrow_mut().hasExt = shape.borrow().hasExt;
        out.borrow_mut().geomFallback = shape.borrow().geomFallback;
        out.borrow_mut().rotation = shape.borrow().rotation;
        out.borrow_mut().preset = shape.borrow().preset.clone();
        out.borrow_mut().adj = shape.borrow().adj;
        out.borrow_mut().shapeId = shape.borrow().shapeId;
        out.borrow_mut().noFill = shape.borrow().noFill;
        out.borrow_mut().noLine = shape.borrow().noLine;
        out.borrow_mut().headEnd = shape.borrow().headEnd.clone();
        out.borrow_mut().tailEnd = shape.borrow().tailEnd.clone();
        out.borrow_mut().fillRef = shape.borrow().fillRef;
        let _tmp_890 = self.cloneColor(&shape.borrow().fillRefColor.clone());
        out.borrow_mut().fillRefColor = _tmp_890;
        out.borrow_mut().lineRef = shape.borrow().lineRef;
        let _tmp_893 = self.cloneColor(&shape.borrow().lineRefColor.clone());
        out.borrow_mut().lineRefColor = _tmp_893;
        out.borrow_mut().effectRef = shape.borrow().effectRef;
        let _tmp_896 = self.cloneColor(&shape.borrow().fontRefColor.clone());
        out.borrow_mut().fontRefColor = _tmp_896;
        out.borrow_mut().hasPath = shape.borrow().hasPath;
        out.borrow_mut().pathUnit = shape.borrow().pathUnit.clone();
        out.borrow_mut().pathEnds = shape.borrow().pathEnds.clone();
        // Dropping this drew every custom path closed, whatever the file said:
        // the parser reads `Z` and the renderer asks for it, and the resolved
        // copy in between was answering `false`... which the renderer's own
        // default then turned back into "closed". A hand-drawn open stroke came
        // out joined end to end.
        out.borrow_mut().pathClosed = shape.borrow().pathClosed;
        let _tmp_902 = self.cloneColor(&shape.borrow().fill.clone());
        out.borrow_mut().fill = _tmp_902;
        let _tmp_904 = self.cloneColor(&shape.borrow().line.clone());
        out.borrow_mut().line = _tmp_904;
        out.borrow_mut().lineWidthPt = shape.borrow().lineWidthPt;
        out.borrow_mut().hasText = shape.borrow().hasText;
        let _tmp_908 = self.cloneTextBody(&shape.borrow().text.clone());
        out.borrow_mut().text = _tmp_908;
        out.borrow_mut().imageRid = shape.borrow().imageRid.clone();
        out.borrow_mut().imagePart = shape.borrow().imagePart.clone();
        out.borrow_mut().imageBytes = shape.borrow().imageBytes.clone();
        out.borrow_mut().imageOk = shape.borrow().imageOk;
        out.borrow_mut().altText = shape.borrow().altText.clone();
        out.borrow_mut().hasStyle = shape.borrow().hasStyle;
        out.borrow_mut().fontRef = shape.borrow().fontRef.clone();
        out.borrow_mut().styleFillSig = shape.borrow().styleFillSig.clone();
        out.borrow_mut().styleLineSig = shape.borrow().styleLineSig.clone();
        out.borrow_mut().sheetXfrmSig = shape.borrow().sheetXfrmSig.clone();
        out.borrow_mut().isPlaceholder = shape.borrow().isPlaceholder;
        out.borrow_mut().phType = shape.borrow().phType.clone();
        out.borrow_mut().phIdx = shape.borrow().phIdx;
        out.borrow_mut().inherited = shape.borrow().inherited;
        out.borrow_mut().locked = shape.borrow().locked;
        out.borrow_mut().flipH = shape.borrow().flipH;
        out.borrow_mut().flipV = shape.borrow().flipV;
        out.borrow_mut().chX = shape.borrow().chX;
        out.borrow_mut().chY = shape.borrow().chY;
        out.borrow_mut().chW = shape.borrow().chW;
        out.borrow_mut().chH = shape.borrow().chH;
        out.borrow_mut().isDiagram = shape.borrow().isDiagram;
        let mut _tmp_931 = shape.borrow().diagramFrame.clone();
        SourceRef::copyFrom(&out.borrow().diagramFrame.clone(), &_tmp_931);
        out.borrow_mut().diagramSig = shape.borrow().diagramSig.clone();
        out.borrow_mut().diagramText = shape.borrow().diagramText.clone();
        out.borrow_mut().diagramModelXml = shape.borrow().diagramModelXml.clone();
        out.borrow_mut().diagramLayoutXml = shape.borrow().diagramLayoutXml.clone();
        out.borrow_mut().diagramLayoutId = shape.borrow().diagramLayoutId.clone();
        out.borrow_mut().diagramStyleId = shape.borrow().diagramStyleId.clone();
        out.borrow_mut().diagramColorsId = shape.borrow().diagramColorsId.clone();
        out.borrow_mut().diagramStyleXml = shape.borrow().diagramStyleXml.clone();
        out.borrow_mut().diagramColorsXml = shape.borrow().diagramColorsXml.clone();
        out.borrow_mut().diagramHadDrawing = shape.borrow().diagramHadDrawing;
        let mut dppi: i64 = 0;
        while dppi < (shape.borrow().diagramParaPts.len() as i64) {
            out.borrow_mut().diagramParaPts.push(shape.borrow().diagramParaPts[dppi as usize].clone());
            dppi += 1;
        }
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            out.borrow_mut().children.push(self.cloneShape(&shape.borrow().children[i as usize].clone()));
            i += 1;
        }
        out
    }
    pub fn phTypeCompatible(&self, a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
        // Title family
        if (a == "title" || a == "ctrTitle") && (b == "title" || b == "ctrTitle") {
            return true;
        }
        if ((a == "body" || a == "obj") || a == "subTitle") && ((b == "body" || b == "obj") || b == "subTitle") {
            return true;
        }
        false
    }
    pub fn findPlaceholder(&self, shapes: &Vec<Rc<RefCell<PptxShape>>>, ph_type: &str, ph_idx: i64) -> Option<Rc<RefCell<PptxShape>>> {
        // 1) exact type + idx
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut s: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if s.borrow().isPlaceholder {
                if (self.phTypeCompatible(&s.borrow().phType.clone(), ph_type) && s.borrow().phIdx == ph_idx) && ph_idx >= 0 {
                    let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(s.clone());
                    return hit;
                }
            }
            i += 1;
        }
        // 2) type match (ignore idx)
        let mut j: i64 = 0;
        while j < (shapes.len() as i64) {
            let mut s2: Rc<RefCell<PptxShape>> = shapes[j as usize].clone();
            if s2.borrow().isPlaceholder {
                if self.phTypeCompatible(&s2.borrow().phType.clone(), ph_type) {
                    let mut hit2: Option<Rc<RefCell<PptxShape>>> = Some(s2.clone());
                    return hit2;
                }
            }
            j += 1;
        }
        // 3) idx-only match
        if ph_idx >= 0 {
            let mut k: i64 = 0;
            while k < (shapes.len() as i64) {
                let mut s3: Rc<RefCell<PptxShape>> = shapes[k as usize].clone();
                if s3.borrow().isPlaceholder {
                    if s3.borrow().phIdx == ph_idx {
                        let mut hit3: Option<Rc<RefCell<PptxShape>>> = Some(s3.clone());
                        return hit3;
                    }
                }
                k += 1;
            }
        }
        let mut __missing: Option<Rc<RefCell<PptxShape>>> = None;
        __missing
    }
    /// `flowY` is the bottom of everything placed on this slide so far. A
    /// shape that never said WHERE it goes either is put under them rather
    /// than at the top of the slide, where it would be drawn across the
    /// title — the deck this came from has one of each, and a repair that
    /// buries the title under a repaired box has not repaired anything.
    pub fn fallbackBox(&self, shape: &Rc<RefCell<PptxShape>>, slide_w: f64, slide_h: f64, flow_y: f64) {
        if PptxResolver::statedSize(shape) {
            return;
        }
        if !PptxResolver::visibleText(shape) {
            return;
        }
        if slide_w <= 0.0 {
            return;
        }
        let margin: f64 = slide_w * 0.05;
        let mut x: f64 = margin;
        let mut y: f64 = slide_h * 0.05;
        if flow_y > y {
            y = flow_y + 6.0;
        }
        let mut w: f64 = slide_w - margin * 2.0;
        let mut h: f64 = PptxResolver::textHeightPt(&shape.borrow().text.clone());
        // A placeholder is not just any shape: the deck asked for the title or
        // the body, and PowerPoint's own default master says where those go.
        // Standing them in that band is closer to what the author saw than a
        // box the size of its own text would be.
        if shape.borrow().isPlaceholder {
            if shape.borrow().phType == "title" || shape.borrow().phType == "ctrTitle" {
                y = slide_h * 0.04;
                h = slide_h * 0.17;
            } else {
                if (shape.borrow().phType == "body" || shape.borrow().phType == "obj") || shape.borrow().phType == "subTitle" {
                    y = slide_h * 0.23;
                    h = slide_h * 0.66;
                }
            }
        }
        // Where the file DID say the shape goes, it goes — only the size is
        // this reader's. A box that states `a:off` and no `a:ext` is the
        // common half of the report, and moving it as well would put the text
        // somewhere the author never put it.
        if shape.borrow().hasOff {
            x = shape.borrow().x;
            y = shape.borrow().y;
            w = (slide_w - x) - margin;
        }
        if x < 0.0 {
            x = margin;
        }
        if x >= slide_w {
            x = margin;
        }
        if y < 0.0 {
            y = 0.0;
        }
        if w < slide_w * 0.2 {
            w = slide_w * 0.2;
        }
        if h < 14.4 {
            h = 14.4;
        }
        if y + h > slide_h {
            if h < slide_h {
                y = slide_h - h;
            } else {
                y = 0.0;
                h = slide_h;
            }
        }
        shape.borrow_mut().x = x;
        shape.borrow_mut().y = y;
        shape.borrow_mut().width = w;
        shape.borrow_mut().height = h;
        shape.borrow_mut().hasXfrm = true;
        shape.borrow_mut().hasOff = true;
        shape.borrow_mut().hasExt = true;
        shape.borrow_mut().geomFallback = true;
        // Invented, so never written back as the document's own — the same
        // rule inherited geometry goes out under.
        let hoist_6 = shape.borrow().xfrmSignature();
        shape.borrow_mut().sheetXfrmSig = hoist_6;
    }
    /// The typography a shape inherits when it is not a placeholder: the
    /// master's `otherStyle`, with whatever the shape itself said over it.
    ///
    /// `p:defaultTextStyle` in presentation.xml is the step below this one and
    /// is not read here. In every deck seen so far it repeats what `otherStyle`
    /// says — the one this was reported on states 14 points in both — so it
    /// would change nothing; a deck that has the second and not the first would
    /// still fall back to this reader's own default.
    pub fn applyOtherStyle(&self, shape: &Rc<RefCell<PptxShape>>, master: Option<Rc<RefCell<PptxSheet>>>) {
        if master.is_none() {
            return;
        }
        let mut mas: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
        if !mas.borrow().otherStyle.borrow().isSet {
            return;
        }
        let mut inherited: Rc<RefCell<PptxListStyle>> = Rc::new(RefCell::new(PptxListStyle::new()));
        inherited.borrow_mut().mergeFrom(&mas.borrow().otherStyle.clone());
        inherited.borrow_mut().mergeFrom(&shape.borrow().text.borrow().listStyle.clone());
        shape.borrow_mut().text.borrow_mut().listStyle = inherited.clone();
    }
    /// The same inheritance, down through a group.
    ///
    /// `resolveShape` already walks a group's children — for their fills, their
    /// lines and their own text bodies — but the typography was applied only to
    /// the shapes a slide lists directly, so anything inside a `p:grpSp` got
    /// none of it. On the deck this was reported from, four boxes sat in one
    /// group and were the only four still drawn in this reader's defaults: 18
    /// points where the deck said 14, and WHITE where the deck said black,
    /// because a missing inherited style loses the colour exactly as it loses
    /// the size. They were also the only four that looked wrong after the size
    /// was fixed everywhere else, which is what pointed at the group.
    pub fn inheritChildren(&self, shape: &Rc<RefCell<PptxShape>>, master: Option<Rc<RefCell<PptxSheet>>>, theme: &Rc<RefCell<PptxTheme>>) {
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            let mut kid: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
            if !kid.borrow().isPlaceholder {
                self.applyOtherStyle(&kid, master.clone());
            }
            self.applyListStyle(&kid.borrow().text.clone(), theme);
            self.inheritChildren(&kid, master.clone(), theme);
            i += 1;
        }
    }
    /// Put the effective list style onto the paragraphs: each one takes the
    /// level it sits at, and states nothing it stated itself. This is where nine
    /// levels of inheritance become one paragraph's size, colour and indent.
    pub fn applyListStyle(&self, body: &Rc<RefCell<PptxTextBody>>, theme: &Rc<RefCell<PptxTheme>>) {
        if !body.borrow().listStyle.borrow().isSet {
            return;
        }
        let mut i: i64 = 0;
        while i < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[i as usize].clone();
            let mut body_list_style: Rc<RefCell<PptxListStyle>> = body.borrow()
                .listStyle.clone()
                .clone();
            let mut lvl: Rc<RefCell<PptxLevelStyle>> = body_list_style.borrow().at(para.borrow().level);
            if !para.borrow().alignSet {
                if lvl.borrow().alignSet {
                    para.borrow_mut().align = lvl.borrow().align.clone();
                    para.borrow_mut().alignSet = true;
                }
            }
            // An indent taken from the level style is as much a stated indent
            // as one written on the paragraph — the deck said it, one link up
            // the chain — so the flag is set too. Without that, a deck saved
            // out of here (which writes the resolved indent on the paragraph)
            // laid its text out differently from the deck it came from.
            if lvl.borrow().marginSet {
                if !para.borrow().marginSet {
                    para.borrow_mut().marginLeftPt = lvl.borrow().marginLeftPt;
                    para.borrow_mut().marginSet = true;
                }
            }
            if lvl.borrow().indentSet {
                if !para.borrow().indentSet {
                    para.borrow_mut().indentPt = lvl.borrow().indentPt;
                    para.borrow_mut().indentSet = true;
                }
            }
            if lvl.borrow().spaceBeforeSet {
                if para.borrow().spaceBeforePt == 0.0 {
                    para.borrow_mut().spaceBeforePt = lvl.borrow().spaceBeforePt;
                }
            }
            if lvl.borrow().spaceAfterSet {
                if para.borrow().spaceAfterPt == 0.0 {
                    para.borrow_mut().spaceAfterPt = lvl.borrow().spaceAfterPt;
                }
            }
            if lvl.borrow().lineSpacingSet {
                if para.borrow().lineSpacingPct == 100.0 {
                    para.borrow_mut().lineSpacingPct = lvl.borrow().lineSpacingPct;
                }
            }
            // The bullet, when the paragraph did not decide for itself — and
            // `<a:buNone/>` on the paragraph IS deciding.
            if !para.borrow().bulletSet {
                if lvl.borrow().bulletKind == "char" {
                    para.borrow_mut().hasBullet = true;
                    para.borrow_mut().bulletChar = lvl.borrow().bulletChar.clone();
                }
                if lvl.borrow().bulletKind == "autonum" {
                    para.borrow_mut().hasBullet = true;
                    para.borrow_mut().bulletAutoNum = true;
                    para.borrow_mut().bulletScheme = lvl.borrow().bulletScheme.clone();
                    para.borrow_mut().bulletStartAt = lvl.borrow().bulletStartAt;
                }
                if lvl.borrow().bulletKind == "none" {
                    para.borrow_mut().hasBullet = false;
                }
            }
            if lvl.borrow().bulletColor.borrow().isSet {
                if !para.borrow().bulletColor.borrow().isSet {
                    let mut _tmp_960 = self.cloneColor(&lvl.borrow().bulletColor.clone());
                    para.borrow_mut().bulletColor = self.resolveColor(&_tmp_960, theme);
                }
            }
            if lvl.borrow().bulletSizePctSet {
                if para.borrow().bulletSizePct == 100.0 {
                    para.borrow_mut().bulletSizePct = lvl.borrow().bulletSizePct;
                }
            }
            if lvl.borrow().bulletSizePtSet {
                if para.borrow().bulletSizePt == 0.0 {
                    para.borrow_mut().bulletSizePt = lvl.borrow().bulletSizePt;
                }
            }
            if para.borrow().bulletFont.is_empty() {
                para.borrow_mut().bulletFont = lvl.borrow().bulletFont.clone();
            }
            // …and the run properties, for every run that states none.
            let mut k: i64 = 0;
            while k < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[k as usize].clone();
                if !run.borrow().sizeSet {
                    if lvl.borrow().sizeSet {
                        run.borrow_mut().fontSizePt = lvl.borrow().fontSizePt;
                        run.borrow_mut().sizeSet = true;
                    }
                }
                if run.borrow().fontFamily.is_empty() {
                    if !lvl.borrow().fontFamily.is_empty() {
                        run.borrow_mut().fontFamily = lvl.borrow().fontFamily.clone();
                    }
                }
                if !run.borrow().color.borrow().isSet {
                    if lvl.borrow().color.borrow().isSet {
                        let _tmp_968 = self.cloneColor(&lvl.borrow().color.clone());
                        run.borrow_mut().color = _tmp_968;
                    }
                }
                // The level style decides only when the RUN said nothing.
                // `run.bold == false` was the test here, which is also true of
                // a run that said `b="0"` — so a word deliberately taken out of
                // a bold list level came back bold.
                if !run.borrow().boldSet {
                    if lvl.borrow().boldSet {
                        run.borrow_mut().bold = lvl.borrow().bold;
                        run.borrow_mut().boldSet = true;
                    }
                }
                if !run.borrow().italicSet {
                    if lvl.borrow().italicSet {
                        run.borrow_mut().italic = lvl.borrow().italic;
                        run.borrow_mut().italicSet = true;
                    }
                }
                k += 1;
            }
            i += 1;
        }
        // What PowerPoint already shrank the text by to make it fit.
        if body.borrow().fontScale != 1.0 {
            let mut pi: i64 = 0;
            while pi < (body.borrow().paragraphs.len() as i64) {
                let mut p2: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[pi as usize].clone();
                let mut ri: i64 = 0;
                while ri < (p2.borrow().runs.len() as i64) {
                    let mut r2: Rc<RefCell<PptxTextRun>> = p2.borrow().runs[ri as usize].clone();
                    let hoist_7 = r2.borrow().fontSizePt * body.borrow().fontScale;
                    r2.borrow_mut().fontSizePt = hoist_7;
                    ri += 1;
                }
                let hoist_8 = p2.borrow().lineSpacingPct * body.borrow().lineScale;
                p2.borrow_mut().lineSpacingPct = hoist_8;
                pi += 1;
            }
        }
    }
    pub fn applyDefRPrToRun(&self, run: &Rc<RefCell<PptxTextRun>>, body: &Rc<RefCell<PptxTextBody>>) {
        if !body.borrow().hasDefRPr {
            return;
        }
        if !run.borrow().sizeSet {
            run.borrow_mut().fontSizePt = body.borrow().defFontSizePt;
            run.borrow_mut().sizeSet = true;
        }
        if run.borrow().fontFamily.is_empty() {
            if !body.borrow().defFontFamily.is_empty() {
                run.borrow_mut().fontFamily = body.borrow().defFontFamily.clone();
            }
        }
        if !run.borrow().color.borrow().isSet {
            if body.borrow().defColor.borrow().isSet {
                let _tmp_976 = self.cloneColor(&body.borrow().defColor.clone());
                run.borrow_mut().color = _tmp_976;
            }
        }
        if !run.borrow().boldSet {
            if body.borrow().defBoldSet {
                run.borrow_mut().bold = body.borrow().defBold;
                run.borrow_mut().boldSet = true;
            }
        }
        if !run.borrow().italicSet {
            if body.borrow().defItalicSet {
                run.borrow_mut().italic = body.borrow().defItalic;
                run.borrow_mut().italicSet = true;
            }
        }
    }
    pub fn mergePlaceholderFrom(&self, dst: &Rc<RefCell<PptxShape>>, src: &Rc<RefCell<PptxShape>>) {
        // Geometry, in two halves, because the file states it in two halves.
        //
        // The slide wins on each half it stated. What it did not state is the
        // layout's, and what the LAYOUT did not state is the master's — which
        // is the case this used to lose: the merge marked `hasXfrm` whether or
        // not the source had a size, so a layout placeholder that takes its own
        // geometry from the master (very common in a corporate template, where
        // the layout only says which placeholder goes where by naming it)
        // handed the slide a box of 0 × 0 and ended the search. The text then
        // wrapped at every word against the left edge and could not be clicked.
        // See `PptxShape.hasOff` / `hasExt`.
        let mut took_any: bool = false;
        let stated_off: bool = dst.borrow().hasOff;
        if !stated_off && (src.borrow().hasXfrm && src.borrow().hasOff) {
            dst.borrow_mut().x = src.borrow().x;
            dst.borrow_mut().y = src.borrow().y;
            dst.borrow_mut().hasOff = true;
            dst.borrow_mut().hasXfrm = true;
            took_any = true;
        }
        if !dst.borrow().hasExt && PptxResolver::statedSize(src) {
            dst.borrow_mut().width = src.borrow().width;
            dst.borrow_mut().height = src.borrow().height;
            dst.borrow_mut().hasExt = true;
            dst.borrow_mut().hasXfrm = true;
            // The rotation belongs to the box it turns, so it comes with the
            // size rather than on its own — and only when the slide did not
            // state a box of its own to turn.
            if !stated_off {
                dst.borrow_mut().rotation = src.borrow().rotation;
            }
            took_any = true;
        }
        if took_any {
            // Inherited, not stated — recorded as it stands now, so a later
            // move stops matching and IS written. See `sheetXfrmSig`.
            let hoist_9 = dst.borrow().xfrmSignature();
            dst.borrow_mut().sheetXfrmSig = hoist_9;
        }
        if dst.borrow().preset.is_empty() {
            dst.borrow_mut().preset = src.borrow().preset.clone();
        }
        if dst.borrow().preset == "rect" && src.borrow().preset != "rect" {
            if !dst.borrow().hasXfrm {
                dst.borrow_mut().preset = src.borrow().preset.clone();
            }
        }
        if !dst.borrow().fill.borrow().isSet {
            if src.borrow().fill.borrow().isSet {
                let _tmp_988 = self.cloneColor(&src.borrow().fill.clone());
                dst.borrow_mut().fill = _tmp_988;
            }
        }
        if !dst.borrow().line.borrow().isSet {
            if src.borrow().line.borrow().isSet {
                let _tmp_990 = self.cloneColor(&src.borrow().line.clone());
                dst.borrow_mut().line = _tmp_990;
                dst.borrow_mut().lineWidthPt = src.borrow().lineWidthPt;
            }
        }
        // Text style / body props from placeholder definition
        if src.borrow().hasText {
            let mut src_body: Rc<RefCell<PptxTextBody>> = src.borrow().text.clone().clone();
            if !dst.borrow().hasText {
                dst.borrow_mut().hasText = true;
                dst.borrow_mut().text = self.cloneTextBody(&src_body);
            } else {
                let mut dst_body: Rc<RefCell<PptxTextBody>> = dst.borrow().text.clone().clone();
                if !dst_body.borrow().insetsSet {
                    if src_body.borrow().insetsSet {
                        dst_body.borrow_mut().insetL = src_body.borrow().insetL;
                        dst_body.borrow_mut().insetT = src_body.borrow().insetT;
                        dst_body.borrow_mut().insetR = src_body.borrow().insetR;
                        dst_body.borrow_mut().insetB = src_body.borrow().insetB;
                        dst_body.borrow_mut().insetsSet = true;
                    }
                }
                if !dst_body.borrow().anchorSet {
                    if src_body.borrow().anchorSet {
                        dst_body.borrow_mut().anchor = src_body.borrow().anchor.clone();
                        dst_body.borrow_mut().anchorSet = true;
                    }
                }
                if !dst_body.borrow().hasDefRPr {
                    if src_body.borrow().hasDefRPr {
                        dst_body.borrow_mut().hasDefRPr = true;
                        dst_body.borrow_mut().defFontSizePt = src_body.borrow().defFontSizePt;
                        dst_body.borrow_mut().defBold = src_body.borrow().defBold;
                        dst_body.borrow_mut().defItalic = src_body.borrow().defItalic;
                        dst_body.borrow_mut().defFontFamily = src_body.borrow().defFontFamily.clone();
                        let _tmp_1003 = self.cloneColor(&src_body.borrow().defColor.clone());
                        dst_body.borrow_mut().defColor = _tmp_1003;
                        dst_body.borrow_mut().defColorFromStyle = src_body.borrow().defColorFromStyle;
                        dst_body.borrow_mut().defAlign = src_body.borrow().defAlign.clone();
                    }
                }
                let mut pi: i64 = 0;
                while pi < (dst_body.borrow().paragraphs.len() as i64) {
                    let mut para: Rc<RefCell<PptxParagraph>> = dst_body.borrow().paragraphs[pi as usize].clone();
                    if !para.borrow().alignSet {
                        if !dst_body.borrow().defAlign.is_empty() {
                            para.borrow_mut().align = dst_body.borrow().defAlign.clone();
                        } else {
                            if !src_body.borrow().paragraphs.is_empty() {
                                let mut sp: Rc<RefCell<PptxParagraph>> = src_body.borrow().paragraphs[0].clone();
                                if sp.borrow().alignSet {
                                    para.borrow_mut().align = sp.borrow().align.clone();
                                }
                            }
                        }
                    }
                    // The placeholder's default run properties are NOT applied
                    // here. They are level one of a list style, and applying
                    // them now marks every run as sized and coloured before the
                    // LEVEL each paragraph actually sits at has been consulted
                    // — which drew every sub-bullet in the top level's size.
                    // `resolveTextBody` applies them afterwards, to whatever the
                    // level style left unset, which is the right order.
                    pi += 1;
                }
            }
        }
    }
    pub fn resolveTextBody(&self, body: &Rc<RefCell<PptxTextBody>>, theme: &Rc<RefCell<PptxTheme>>) {
        let mut i: i64 = 0;
        while i < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[i as usize].clone();
            let mut j: i64 = 0;
            while j < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[j as usize].clone();
                // What the run itself said, before anything is filled in.
                let stated_font: bool = !run.borrow().fontFamily.is_empty();
                let stated_color: bool = run.borrow().color.borrow().isSet;
                let stated_size: bool = run.borrow().sizeSet;
                self.applyDefRPrToRun(&run, body);
                let _tmp_1009 = self.resolveFont(&run.borrow().fontFamily.clone(), theme);
                run.borrow_mut().fontFamily = _tmp_1009;
                if run.borrow().highlight.borrow().isSet {
                    let _tmp_1011 = self.resolveColor(&run.borrow().highlight.clone(), theme);
                    run.borrow_mut().highlight = _tmp_1011;
                }
                // Precedence, and the order is the whole of this:
                //
                //   1. what the RUN said
                //   2. the shape's `<p:style><a:fontRef>` colour
                //   3. the master's default text style
                //   4. black
                //
                // `applyDefRPrToRun` above has already written (3) into the
                // run, so by here a run with no colour of its own LOOKS like
                // one that stated black — and the white a themed button asks
                // for through `fontRef` never got a turn. `statedColor` is
                // what the run really said, taken before that call.
                if !run.borrow().colorStated {
                    if body.borrow().defColorFromStyle {
                        if body.borrow().defColor.borrow().isSet {
                            let _tmp_1013 = self.cloneColor(&body.borrow().defColor.clone());
                            run.borrow_mut().color = _tmp_1013;
                        }
                    }
                }
                if run.borrow().color.borrow().isSet {
                    let _tmp_1015 = self.resolveColor(&run.borrow().color.clone(), theme);
                    run.borrow_mut().color = _tmp_1015;
                } else {
                    if body.borrow().defColor.borrow().isSet {
                        let _tmp_1017 = self.resolveColor(&body.borrow().defColor.clone(), theme);
                        run.borrow_mut().color = _tmp_1017;
                    } else {
                        let _tmp_1018 = self.resolveColor(&PptxColor::schemeOf("dk1".to_string()),
                            theme
                        );
                        run.borrow_mut().color = _tmp_1018;
                    }
                }
                // Recorded as it stands now. Anything the user changes later
                // stops matching, and the writer states the change.
                if !stated_font {
                    let hoist_10 = run.borrow().fontFamily.clone();
                    run.borrow_mut().resolvedFontSig = hoist_10;
                }
                if !stated_color {
                    let hoist_11 = (run.borrow().color).borrow().signature();
                    run.borrow_mut().resolvedColorSig = hoist_11;
                }
                if !stated_size {
                    let hoist_12 = ((run.borrow().fontSizePt * 100.0 + 0.5).floor() as i64).to_string();
                    run.borrow_mut().resolvedSizeSig = hoist_12;
                }
                j += 1;
            }
            i += 1;
        }
    }
    /// A theme fill style, with the placeholder colour filled in from the
    /// shape's own `fillRef`. The style's colours carry the modifiers; the ref
    /// carries the colour they are applied to.
    pub fn styleColor(&self, style_col: &Rc<RefCell<PptxColor>>, ref_col: &Rc<RefCell<PptxColor>>, theme: &Rc<RefCell<PptxTheme>>) -> Rc<RefCell<PptxColor>> {
        let mut base: Rc<RefCell<PptxColor>> = ref_col.clone();
        if style_col.borrow().isSet {
            if !style_col.borrow().isScheme {
                base = style_col.clone();
            } else {
                if style_col.borrow().scheme != "phClr" {
                    base = style_col.clone();
                }
            }
        }
        let mut resolved: Rc<RefCell<PptxColor>> = self.resolveColor(&base, theme);
        let mut out: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        out.borrow_mut().isSet = true;
        out.borrow_mut().isScheme = resolved.borrow().isScheme;
        out.borrow_mut().scheme = resolved.borrow().scheme.clone();
        out.borrow_mut().alpha = resolved.borrow().alpha * style_col.borrow().alpha;
        // The reference's own modifiers apply first, then the style's.
        let once: String = PptxResolver::applyMods(&resolved.borrow().srgb.clone(), &base);
        out.borrow_mut().srgb = PptxResolver::applyMods(&once, style_col);
        out.borrow_mut().baked = true;
        out
    }
    /// A shape that states no fill of its own but carries a `<p:style>` gets the
    /// theme's fill — which is where a deck's gradients usually live.
    pub fn applyStyleRefs(&self, shape: &Rc<RefCell<PptxShape>>, theme: &Rc<RefCell<PptxTheme>>) {
        if shape.borrow().fillRef > 0 {
            if !shape.borrow().fill.borrow().isSet {
                if !shape.borrow().fillGrad.borrow().isSet {
                    if !shape.borrow().noFill {
                        let mut st: Option<Rc<RefCell<PptxFillStyle>>> = theme.borrow().fillStyleAt(shape.borrow().fillRef.clone());
                        if st.is_some() {
                            let mut style: Rc<RefCell<PptxFillStyle>> = st.clone().unwrap();
                            if style.borrow().kind == 1 {
                                shape.borrow_mut().fillGrad.borrow_mut().isSet = true;
                                shape.borrow_mut().fillGrad.borrow_mut().dir = style.borrow().dir;
                                let mut _tmp_1029 = style.borrow().c1.clone();
                                let _tmp_1031 = self.styleColor(&_tmp_1029, &shape.borrow().fillRefColor.clone(), theme);
                                shape.borrow_mut().fillGrad.borrow_mut().c1 = _tmp_1031;
                                let mut _tmp_1032 = style.borrow().c2.clone();
                                let _tmp_1034 = self.styleColor(&_tmp_1032, &shape.borrow().fillRefColor.clone(), theme);
                                shape.borrow_mut().fillGrad.borrow_mut().c2 = _tmp_1034;
                                // Worked out here, not read from the file. A
                                // save that writes it back as the shape's own
                                // fill would bake the theme into the deck.
                            } else {
                                if style.borrow().kind == 0 {
                                    let mut _tmp_1035 = style.borrow().solid.clone();
                                    let mut _tmp_1036 = shape.borrow().fillRefColor.clone();
                                    let _tmp_1037 = self.styleColor(&_tmp_1035,
                                        &_tmp_1036,
                                        theme
                                    );
                                    shape.borrow_mut().fill = _tmp_1037;
                                }
                            }
                            // Recorded as it stands NOW. If the user changes
                            // the fill later it stops matching, and the writer
                            // states the change rather than the inheritance.
                            let hoist_13 = shape.borrow().fillSignature();
                            shape.borrow_mut().styleFillSig = hoist_13;
                        }
                    }
                }
            }
        }
        if shape.borrow().lineRef > 0 {
            if !shape.borrow().line.borrow().isSet && !shape.borrow().noLine {
                let mut ls: Option<Rc<RefCell<PptxLineStyle>>> = theme.borrow().lineStyleAt(shape.borrow().lineRef.clone());
                if ls.is_some() {
                    let mut lstyle: Rc<RefCell<PptxLineStyle>> = ls.clone().unwrap();
                    let mut _tmp_1040 = lstyle.borrow().color.clone();
                    let _tmp_1042 = self.styleColor(&_tmp_1040, &shape.borrow().lineRefColor.clone(), theme);
                    shape.borrow_mut().line = _tmp_1042;
                    if shape.borrow().lineWidthPt <= 0.0 {
                        shape.borrow_mut().lineWidthPt = lstyle.borrow().widthPt;
                    }
                    let hoist_14 = shape.borrow().lineSignature();
                    shape.borrow_mut().styleLineSig = hoist_14;
                }
            }
        }
        if shape.borrow().effectRef > 0 {
            if !shape.borrow().shadow.borrow().isSet {
                if theme.borrow().effectShadowAt(shape.borrow().effectRef.clone()) {
                    shape.borrow_mut().shadow.borrow_mut().isSet = true;
                    shape.borrow_mut().shadow.borrow_mut().color = PptxColor::srgbOf("000000".to_string());
                }
            }
        }
    }
    /// The default text colour for a shape styled by the theme. `fontRef` sits
    /// below what a run says and below what the body's own default style says,
    /// and above the "black" everything else falls back to — which is why white
    /// text on a themed shape used to come out black.
    pub fn applyFontRef(&self, shape: &Rc<RefCell<PptxShape>>, theme: &Rc<RefCell<PptxTheme>>) {
        if !shape.borrow().fontRefColor.borrow().isSet {
            return;
        }
        if !shape.borrow().hasText {
            return;
        }
        if shape.borrow().text.borrow().defColor.borrow().isSet {
            return;
        }
        let mut _tmp_1047 = shape.borrow().fontRefColor.clone();
        let _tmp_1049 = self.styleColor(&_tmp_1047, &shape.borrow().fontRefColor.clone(), theme);
        shape.borrow_mut().text.borrow_mut().defColor = _tmp_1049;
        shape.borrow_mut().text.borrow_mut().defColorFromStyle = true;
    }
    pub fn resolveTableLine(&self, line: &Rc<RefCell<PptxTableLine>>, theme: &Rc<RefCell<PptxTheme>>) {
        if !line.borrow().isSet {
            return;
        }
        if line.borrow().color.borrow().isSet {
            let _tmp_1051 = self.resolveColor(&line.borrow().color.clone(), theme);
            line.borrow_mut().color = _tmp_1051;
        }
    }
    pub fn resolveShape(&self, shape: &Rc<RefCell<PptxShape>>, theme: &Rc<RefCell<PptxTheme>>) {
        self.applyStyleRefs(shape, theme);
        if shape.borrow().fill.borrow().isSet {
            let _tmp_1053 = self.resolveColor(&shape.borrow().fill.clone(), theme);
            shape.borrow_mut().fill = _tmp_1053;
        }
        if shape.borrow().fillGrad.borrow().isSet {
            let _tmp_1055 = self.resolveColor(&shape.borrow().fillGrad.borrow().c1.clone(), theme);
            shape.borrow_mut().fillGrad.borrow_mut().c1 = _tmp_1055;
            let _tmp_1057 = self.resolveColor(&shape.borrow().fillGrad.borrow().c2.clone(), theme);
            shape.borrow_mut().fillGrad.borrow_mut().c2 = _tmp_1057;
        }
        if shape.borrow().line.borrow().isSet {
            let _tmp_1059 = self.resolveColor(&shape.borrow().line.clone(), theme);
            shape.borrow_mut().line = _tmp_1059;
        }
        if shape.borrow().shadow.borrow().isSet {
            let _tmp_1061 = self.resolveColor(&shape.borrow().shadow.borrow().color.clone(), theme);
            shape.borrow_mut().shadow.borrow_mut().color = _tmp_1061;
        }
        if shape.borrow().hasText {
            self.applyFontRef(shape, theme);
            self.resolveTextBody(&shape.borrow().text.clone(), theme);
        }
        if shape.borrow().kind == PptxShapeKind::table() {
            let mut ti: i64 = 0;
            while ti < (shape.borrow().tableCells.len() as i64) {
                let mut cell: Rc<RefCell<PptxTableCell>> = shape.borrow().tableCells[ti as usize].clone();
                if cell.borrow().fill.borrow().isSet {
                    let _tmp_1064 = self.resolveColor(&cell.borrow().fill.clone(), theme);
                    cell.borrow_mut().fill = _tmp_1064;
                }
                self.resolveTextBody(&cell.borrow().body.clone(), theme);
                self.resolveTableLine(&cell.borrow().lnL.clone(), theme);
                self.resolveTableLine(&cell.borrow().lnR.clone(), theme);
                self.resolveTableLine(&cell.borrow().lnT.clone(), theme);
                self.resolveTableLine(&cell.borrow().lnB.clone(), theme);
                ti += 1;
            }
        }
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            self.resolveShape(&shape.borrow().children[i as usize].clone().clone(), theme);
            i += 1;
        }
    }
    pub fn sheetOf(&self, cache: &HashMap<String,Rc<RefCell<PptxSheet>>>, part: &str) -> Option<Rc<RefCell<PptxSheet>>> {
        if part.is_empty() {
            let mut __missing: Option<Rc<RefCell<PptxSheet>>> = None;
            return __missing;
        }
        let mut hit: Option<Rc<RefCell<PptxSheet>>> = cache.get(part).cloned();
        hit
    }
    pub fn resolveSlide(&self, slide: &Rc<RefCell<PptxSlide>>, theme: &Rc<RefCell<PptxTheme>>, layouts: &HashMap<String,Rc<RefCell<PptxSheet>>>, masters: &HashMap<String,Rc<RefCell<PptxSheet>>>) -> Rc<RefCell<PptxSlide>> {
        let mut _tmp_1070 = slide.borrow().layoutPart.clone();
        let mut layout: Option<Rc<RefCell<PptxSheet>>> = self.sheetOf(layouts,
            &_tmp_1070
        );
        let mut _tmp_1071 = slide.borrow().masterPart.clone();
        let mut master: Option<Rc<RefCell<PptxSheet>>> = self.sheetOf(masters,
            &_tmp_1071
        );
        // Background: slide → layout → master → lt1.
        //
        // Everything past the first line is INHERITED, and a save that wrote it
        // back as the slide's own would freeze it — see `bgFromSheet`.
        let hoist_15 = !slide.borrow().bg.borrow().isSet;
        slide.borrow_mut().bgFromSheet = hoist_15;
        if !slide.borrow().bg.borrow().isSet {
            if layout.is_some() {
                let mut lay: Rc<RefCell<PptxSheet>> = layout.clone().unwrap();
                if lay.borrow().bg.borrow().isSet {
                    let _tmp_1074 = self.cloneColor(&lay.borrow().bg.clone());
                    slide.borrow_mut().bg = _tmp_1074;
                }
            }
        }
        if !slide.borrow().bg.borrow().isSet {
            if master.is_some() {
                let mut mas: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
                if mas.borrow().bg.borrow().isSet {
                    let _tmp_1076 = self.cloneColor(&mas.borrow().bg.clone());
                    slide.borrow_mut().bg = _tmp_1076;
                }
            }
        }
        if slide.borrow().bg.borrow().isSet {
            let _tmp_1078 = self.resolveColor(&slide.borrow().bg.clone(), theme);
            slide.borrow_mut().bg = _tmp_1078;
        } else {
            let _tmp_1079 = self.resolveColor(&PptxColor::schemeOf("lt1".to_string()),
                theme
            );
            slide.borrow_mut().bg = _tmp_1079;
        }
        let mut resolved: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        // Master chrome (non-placeholders), then layout chrome
        if master.is_some() {
            let mut ms: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
            let mut mi: i64 = 0;
            while mi < (ms.borrow().shapes.len() as i64) {
                let mut mshape: Rc<RefCell<PptxShape>> = ms.borrow().shapes[mi as usize].clone();
                if !mshape.borrow().isPlaceholder {
                    let mut mc: Rc<RefCell<PptxShape>> = self.cloneShape(&mshape);
                    mc.borrow_mut().inherited = true;
                    self.resolveShape(&mc, theme);
                    resolved.push(mc.clone());
                }
                mi += 1;
            }
        }
        if layout.is_some() {
            let mut ls: Rc<RefCell<PptxSheet>> = layout.clone().unwrap();
            let mut li: i64 = 0;
            while li < (ls.borrow().shapes.len() as i64) {
                let mut lshape: Rc<RefCell<PptxShape>> = ls.borrow().shapes[li as usize].clone();
                if !lshape.borrow().isPlaceholder {
                    let mut lc: Rc<RefCell<PptxShape>> = self.cloneShape(&lshape);
                    lc.borrow_mut().inherited = true;
                    // Decoration from the layout is still a shape with text in
                    // it, and inherits the same way a slide's own does.
                    self.applyOtherStyle(&lc, master.clone());
                    self.applyListStyle(&lc.borrow().text.clone(), theme);
                    self.inheritChildren(&lc, master.clone(), theme);
                    self.resolveShape(&lc, theme);
                    resolved.push(lc.clone());
                }
                li += 1;
            }
        }
        // Slide shapes with placeholder inheritance
        let mut flow_y: f64 = 0.0;
        let mut i: i64 = 0;
        while i < (slide.borrow().shapes.len() as i64) {
            let mut shape: Rc<RefCell<PptxShape>> = slide.borrow().shapes[i as usize].clone();
            if shape.borrow().isPlaceholder {
                let mut matched: Option<Rc<RefCell<PptxShape>>> = None;
                if layout.is_some() {
                    let mut lay2: Rc<RefCell<PptxSheet>> = layout.clone().unwrap();
                    let mut _tmp_1081 = lay2.borrow().shapes.clone();
                    let mut _tmp_1082 = shape.borrow().phType.clone();
                    let mut _tmp_1083 = shape.borrow().phIdx.clone();
                    let _tmp_1084 = self.findPlaceholder(&_tmp_1081,
                        &_tmp_1082,
                        _tmp_1083
                    );
                    matched = _tmp_1084;
                }
                let mut from_master: Option<Rc<RefCell<PptxShape>>> = None;
                if master.is_some() {
                    let mut mas2: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
                    let mut _tmp_1085 = mas2.borrow().shapes.clone();
                    let mut _tmp_1086 = shape.borrow().phType.clone();
                    let mut _tmp_1087 = shape.borrow().phIdx.clone();
                    let _tmp_1088 = self.findPlaceholder(&_tmp_1085,
                        &_tmp_1086,
                        _tmp_1087
                    );
                    from_master = _tmp_1088;
                }
                if matched.is_some() {
                    self.mergePlaceholderFrom(&shape, &matched.clone().unwrap());
                }
                // …and then the master, for whatever the layout did not give.
                // The master used to be consulted ONLY when the layout had no
                // placeholder of this kind at all, which is not the chain the
                // format describes: a layout placeholder inherits from the
                // master one, so a slide that inherits from the layout
                // inherits through it. A template whose layouts name their
                // placeholders without repeating the geometry lost every box.
                if from_master.is_some() {
                    self.mergePlaceholderFrom(&shape, &from_master.clone().unwrap());
                }
                // The typography a placeholder inherits: the master's style for
                // this KIND of placeholder first, then the layout's own list
                // style over it, then whatever the shape itself says. Most
                // placeholders in most decks state nothing at all, so without
                // this a deck is drawn in this reader's defaults rather than in
                // its own.
                let mut inherited: Rc<RefCell<PptxListStyle>> = Rc::new(RefCell::new(PptxListStyle::new()));
                if master.is_some() {
                    let mut mas3: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
                    // `p:txStyles` is the LAST resort, under the master's own
                    // placeholder — which is where a deck built by anything but
                    // PowerPoint puts its real typography, leaving txStyles as
                    // a generic black nobody meant.
                    inherited.borrow_mut().mergeFrom(&PptxResolver::styleForPh(&mas3, &shape.borrow().phType.clone()));
                    let mut _tmp_1090 = mas3.borrow().shapes.clone();
                    let mut _tmp_1091 = shape.borrow().phType.clone();
                    let mut _tmp_1092 = shape.borrow().phIdx.clone();
                    let mut mph: Option<Rc<RefCell<PptxShape>>> = self.findPlaceholder(&_tmp_1090,
                        &_tmp_1091,
                        _tmp_1092
                    );
                    if mph.is_some() {
                        let mut mp: Rc<RefCell<PptxShape>> = mph.clone().unwrap();
                        inherited.borrow_mut().mergeFrom(&mp.borrow().text.borrow().listStyle.clone());
                    }
                }
                if matched.is_some() {
                    let mut mt: Rc<RefCell<PptxShape>> = matched.clone().unwrap();
                    inherited.borrow_mut().mergeFrom(&mt.borrow().text.borrow().listStyle.clone());
                }
                inherited.borrow_mut().mergeFrom(&shape.borrow().text.borrow().listStyle.clone());
                shape.borrow_mut().text.borrow_mut().listStyle = inherited.clone();
            } else {
                // …and a shape that is NOT a placeholder inherits too. The rule
                // is written a page up on `styleForPh` — "everything else, and
                // every shape that is not a placeholder at all, takes the other
                // style" — and this branch is where it was missing: the whole
                // block above sat inside the placeholder test, so a plain box
                // with words in it inherited nothing and fell back to THIS
                // READER's defaults.
                //
                // That is 18 points against the 14 the deck asked for, which is
                // a quarter too big: reported on a diagram of boxes where every
                // label overflowed its box and wrapped mid-word — "Prosess/i",
                // "Terveys-/tietokan/ta" — while the title, which states its own
                // size, was correct. A deck exported from Google Slides is
                // entirely such shapes; there is not one placeholder on the
                // slide.
                self.applyOtherStyle(&shape, master.clone());
            }
            self.applyListStyle(&shape.borrow().text.clone(), theme);
            self.inheritChildren(&shape, master.clone(), theme);
            self.resolveShape(&shape, theme);
            // Last, because the sizes the text resolved to are what a box with
            // no size of its own is built from.
            let mut _tmp_1097 = slide.borrow().width.clone();
            self.fallbackBox(&shape, _tmp_1097, slide.borrow().height.clone(), flow_y);
            // …and the next shape with no place of its own goes under this
            // one. See `fallbackBox`.
            let bottom: f64 = shape.borrow().y + shape.borrow().height;
            if bottom > flow_y {
                flow_y = bottom;
            }
            resolved.push(shape.clone());
            i += 1;
        }
        slide.borrow_mut().shapes = resolved.clone();
        slide
    }
    /// A master or layout can be opened for editing, and then it is drawn like a
    /// slide — so its own background has to be a colour, not a theme slot that
    /// nothing has looked up yet.
    pub fn resolveSheetBackgrounds(&self, cache: &HashMap<String,Rc<RefCell<PptxSheet>>>, theme: &Rc<RefCell<PptxTheme>>) {
        let mut parts: Vec<String> = cache.keys().cloned().collect::<Vec<String>>();
        let mut i: i64 = 0;
        while i < (parts.len() as i64) {
            let mut hit: Option<Rc<RefCell<PptxSheet>>> = cache.get(&parts[i as usize].clone()).cloned();
            if hit.is_some() {
                let mut sheet: Rc<RefCell<PptxSheet>> = hit.clone().unwrap();
                if sheet.borrow().bg.borrow().isSet {
                    let _tmp_1100 = self.resolveColor(&sheet.borrow().bg.clone(), theme);
                    sheet.borrow_mut().bg = _tmp_1100;
                } else {
                    let _tmp_1101 = self.resolveColor(&PptxColor::schemeOf("lt1".to_string()),
                        theme
                    );
                    sheet.borrow_mut().bg = _tmp_1101;
                }
            }
            i += 1;
        }
    }
    pub fn resolvePresentation(&self, pres: &Rc<RefCell<PptxPresentation>>) -> Rc<RefCell<PptxPresentation>> {
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            let mut _tmp_1102 = pres.borrow().theme.clone();
            let mut _tmp_1103 = pres.borrow().layouts.clone();
            let mut _tmp_1104 = pres.borrow().masters.clone();
            self.resolveSlide(&pres.borrow().slides[i as usize].clone().clone(), &_tmp_1102, &_tmp_1103, &_tmp_1104);
            i += 1;
        }
        let mut _tmp_1105 = pres.borrow().masters.clone();
        self.resolveSheetBackgrounds(&_tmp_1105, &pres.borrow().theme.clone());
        let mut _tmp_1107 = pres.borrow().layouts.clone();
        self.resolveSheetBackgrounds(&_tmp_1107, &pres.borrow().theme.clone());
        pres
    }
}
