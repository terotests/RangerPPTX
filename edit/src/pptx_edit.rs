// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::pptx_text_edit::PptxTextEdit;
use crate::pptx_text_edit::PptxTextPos;
use crate::rgr::OfficeCaretMotion;
use crate::rgr::OfficeHistory;

/// ==============================================================================
/// PptxEdit - editing core for the PPTX scene (selection, transforms, history)
/// ==============================================================================
///
/// The viewer's model was always mutable — nothing here parses or paints. This
/// is the layer between a host's pointer and `PptxModel`: what is selected, what
/// a drag does to it, and how to take it back.
///
/// Written against the OOXML model directly, not against any editor's document
/// format. Coordinates are the model's: points (1/72"), origin at the slide's
/// top-left, rotation in degrees clockwise about the shape's centre.
///
/// History is whole-deck snapshots rather than an operation log. A shape is a
/// tree (a group holds shapes, a shape holds paragraphs, a paragraph holds runs)
/// and every operation below can touch several of them at once, so the cheap
/// correct thing is to copy the deck and keep the copies. An op log is the later
/// optimization, and it needs the ops to be settled first.
/// ==============================================================================
/// A rectangle in slide points. Ranger returns one value; a bounding box is four.
#[derive(Clone)]
pub struct PptxEditRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl PptxEditRect {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }
    pub fn of(x: f64, y: f64, w: f64, h: f64) -> Rc<RefCell<PptxEditRect>> {
        let mut r: Rc<RefCell<PptxEditRect>> = Rc::new(RefCell::new(PptxEditRect::new()));
        r.borrow_mut().x = x;
        r.borrow_mut().y = y;
        r.borrow_mut().w = w;
        r.borrow_mut().h = h;
        r
    }
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
    pub fn midX(&self) -> f64 {
        self.x + self.w * 0.5
    }
    pub fn midY(&self) -> f64 {
        self.y + self.h * 0.5
    }
}

/// ------------------------------------------------------------------------------
/// Deep copy. Every field, because a snapshot that shares one object with the
/// live deck is a snapshot that changes under you.
/// ------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PptxClone {
}

impl PptxClone {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn color(src: &Rc<RefCell<PptxColor>>) -> Rc<RefCell<PptxColor>> {
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
    pub fn gradient(src: &Rc<RefCell<PptxGradient>>) -> Rc<RefCell<PptxGradient>> {
        let mut g: Rc<RefCell<PptxGradient>> = Rc::new(RefCell::new(PptxGradient::new()));
        g.borrow_mut().isSet = src.borrow().isSet;
        g.borrow_mut().dir = src.borrow().dir;
        let _tmp_6048 = PptxClone::color(&src.borrow().c1.clone());
        g.borrow_mut().c1 = _tmp_6048;
        let _tmp_6050 = PptxClone::color(&src.borrow().c2.clone());
        g.borrow_mut().c2 = _tmp_6050;
        g
    }
    pub fn shadow(src: &Rc<RefCell<PptxShadow>>) -> Rc<RefCell<PptxShadow>> {
        let mut s: Rc<RefCell<PptxShadow>> = Rc::new(RefCell::new(PptxShadow::new()));
        s.borrow_mut().isSet = src.borrow().isSet;
        s.borrow_mut().offsetXPt = src.borrow().offsetXPt;
        s.borrow_mut().offsetYPt = src.borrow().offsetYPt;
        s.borrow_mut().blurPt = src.borrow().blurPt;
        let _tmp_6056 = PptxClone::color(&src.borrow().color.clone());
        s.borrow_mut().color = _tmp_6056;
        s.borrow_mut().alpha = src.borrow().alpha;
        s
    }
    pub fn run(src: &Rc<RefCell<PptxTextRun>>) -> Rc<RefCell<PptxTextRun>> {
        let mut r: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        r.borrow_mut().text = src.borrow().text.clone();
        r.borrow_mut().field = src.borrow().field.clone();
        r.borrow_mut().fontFamily = src.borrow().fontFamily.clone();
        r.borrow_mut().resolvedFontSig = src.borrow().resolvedFontSig.clone();
        r.borrow_mut().resolvedColorSig = src.borrow().resolvedColorSig.clone();
        r.borrow_mut().resolvedSizeSig = src.borrow().resolvedSizeSig.clone();
        r.borrow_mut().fontSizePt = src.borrow().fontSizePt;
        r.borrow_mut().sizeSet = src.borrow().sizeSet;
        r.borrow_mut().boldSet = src.borrow().boldSet;
        r.borrow_mut().bold = src.borrow().bold;
        r.borrow_mut().italicSet = src.borrow().italicSet;
        r.borrow_mut().italic = src.borrow().italic;
        let _tmp_6071 = PptxClone::color(&src.borrow().color.clone());
        r.borrow_mut().color = _tmp_6071;
        r.borrow_mut().colorStated = src.borrow().colorStated;
        let _tmp_6074 = PptxClone::color(&src.borrow().highlight.clone());
        r.borrow_mut().highlight = _tmp_6074;
        r
    }
    pub fn paragraph(src: &Rc<RefCell<PptxParagraph>>) -> Rc<RefCell<PptxParagraph>> {
        let mut p: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
        p.borrow_mut().align = src.borrow().align.clone();
        p.borrow_mut().alignSet = src.borrow().alignSet;
        p.borrow_mut().spaceBeforePt = src.borrow().spaceBeforePt;
        p.borrow_mut().spaceAfterPt = src.borrow().spaceAfterPt;
        p.borrow_mut().lineSpacingPct = src.borrow().lineSpacingPct;
        p.borrow_mut().level = src.borrow().level;
        p.borrow_mut().hasBullet = src.borrow().hasBullet;
        p.borrow_mut().bulletSet = src.borrow().bulletSet;
        p.borrow_mut().bulletChar = src.borrow().bulletChar.clone();
        p.borrow_mut().marginLeftPt = src.borrow().marginLeftPt;
        p.borrow_mut().marginSet = src.borrow().marginSet;
        p.borrow_mut().indentPt = src.borrow().indentPt;
        p.borrow_mut().indentSet = src.borrow().indentSet;
        let _tmp_6089 = PptxClone::color(&src.borrow().bulletColor.clone());
        p.borrow_mut().bulletColor = _tmp_6089;
        p.borrow_mut().bulletSizePct = src.borrow().bulletSizePct;
        p.borrow_mut().bulletSizePt = src.borrow().bulletSizePt;
        p.borrow_mut().bulletFont = src.borrow().bulletFont.clone();
        let mut i: i64 = 0;
        while i < (src.borrow().runs.len() as i64) {
            let mut r: Rc<RefCell<PptxTextRun>> = src.borrow().runs[i as usize].clone();
            p.borrow_mut().runs.push(PptxClone::run(&r));
            i += 1;
        }
        p
    }
    pub fn textBody(src: &Rc<RefCell<PptxTextBody>>) -> Rc<RefCell<PptxTextBody>> {
        let mut t: Rc<RefCell<PptxTextBody>> = Rc::new(RefCell::new(PptxTextBody::new()));
        t.borrow_mut().insetL = src.borrow().insetL;
        t.borrow_mut().insetT = src.borrow().insetT;
        t.borrow_mut().insetR = src.borrow().insetR;
        t.borrow_mut().insetB = src.borrow().insetB;
        t.borrow_mut().insetsSet = src.borrow().insetsSet;
        t.borrow_mut().anchor = src.borrow().anchor.clone();
        t.borrow_mut().anchorSet = src.borrow().anchorSet;
        t.borrow_mut().wrap = src.borrow().wrap;
        t.borrow_mut().hasDefRPr = src.borrow().hasDefRPr;
        t.borrow_mut().defFontSizePt = src.borrow().defFontSizePt;
        t.borrow_mut().defBold = src.borrow().defBold;
        t.borrow_mut().defItalic = src.borrow().defItalic;
        t.borrow_mut().defFontFamily = src.borrow().defFontFamily.clone();
        let _tmp_6107 = PptxClone::color(&src.borrow().defColor.clone());
        t.borrow_mut().defColor = _tmp_6107;
        t.borrow_mut().defAlign = src.borrow().defAlign.clone();
        let mut i: i64 = 0;
        while i < (src.borrow().paragraphs.len() as i64) {
            let mut p: Rc<RefCell<PptxParagraph>> = src.borrow().paragraphs[i as usize].clone();
            t.borrow_mut().paragraphs.push(PptxClone::paragraph(&p));
            i += 1;
        }
        t
    }
    pub fn tableCell(src: &Rc<RefCell<PptxTableCell>>) -> Rc<RefCell<PptxTableCell>> {
        let mut c: Rc<RefCell<PptxTableCell>> = Rc::new(RefCell::new(PptxTableCell::new()));
        c.borrow_mut().text = src.borrow().text.clone();
        let _tmp_6111 = PptxClone::color(&src.borrow().fill.clone());
        c.borrow_mut().fill = _tmp_6111;
        c.borrow_mut().bold = src.borrow().bold;
        c.borrow_mut().fontSizePt = src.borrow().fontSizePt;
        c.borrow_mut().align = src.borrow().align.clone();
        c
    }
    pub fn chart(src: &Rc<RefCell<PptxChart>>) -> Rc<RefCell<PptxChart>> {
        let mut c: Rc<RefCell<PptxChart>> = Rc::new(RefCell::new(PptxChart::new()));
        c.borrow_mut().chartType = src.borrow().chartType.clone();
        c.borrow_mut().title = src.borrow().title.clone();
        let mut i: i64 = 0;
        while i < (src.borrow().categories.len() as i64) {
            c.borrow_mut().categories.push(src.borrow().categories[i as usize].clone());
            i += 1;
        }
        let mut j: i64 = 0;
        while j < (src.borrow().series.len() as i64) {
            let mut s: Rc<RefCell<PptxChartSeries>> = src.borrow().series[j as usize].clone();
            let mut out: Rc<RefCell<PptxChartSeries>> = Rc::new(RefCell::new(PptxChartSeries::new()));
            out.borrow_mut().name = s.borrow().name.clone();
            let mut k: i64 = 0;
            while k < (s.borrow().values.len() as i64) {
                out.borrow_mut().values.push(s.borrow().values[k as usize]);
                k += 1;
            }
            c.borrow_mut().series.push(out.clone());
            j += 1;
        }
        c
    }
    /// Image bytes are shared on purpose: the parser owns them, nothing edits
    /// them, and copying a JPEG per undo step is the one copy worth avoiding.
    pub fn shape(src: &Rc<RefCell<PptxShape>>) -> Rc<RefCell<PptxShape>> {
        let mut s: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        s.borrow_mut().kind = src.borrow().kind;
        s.borrow_mut().editId = src.borrow().editId;
        s.borrow_mut().shapeId = src.borrow().shapeId;
        s.borrow_mut().name = src.borrow().name.clone();
        s.borrow_mut().x = src.borrow().x;
        s.borrow_mut().y = src.borrow().y;
        s.borrow_mut().width = src.borrow().width;
        s.borrow_mut().height = src.borrow().height;
        s.borrow_mut().hasXfrm = src.borrow().hasXfrm;
        s.borrow_mut().hasOff = src.borrow().hasOff;
        s.borrow_mut().hasExt = src.borrow().hasExt;
        s.borrow_mut().geomFallback = src.borrow().geomFallback;
        s.borrow_mut().rotation = src.borrow().rotation;
        s.borrow_mut().flipH = src.borrow().flipH;
        s.borrow_mut().flipV = src.borrow().flipV;
        s.borrow_mut().preset = src.borrow().preset.clone();
        s.borrow_mut().adj = src.borrow().adj;
        s.borrow_mut().noFill = src.borrow().noFill;
        s.borrow_mut().noLine = src.borrow().noLine;
        s.borrow_mut().headEnd = src.borrow().headEnd.clone();
        s.borrow_mut().tailEnd = src.borrow().tailEnd.clone();
        s.borrow_mut().fillRef = src.borrow().fillRef;
        let _tmp_6141 = PptxClone::color(&src.borrow().fillRefColor.clone());
        s.borrow_mut().fillRefColor = _tmp_6141;
        s.borrow_mut().lineRef = src.borrow().lineRef;
        let _tmp_6144 = PptxClone::color(&src.borrow().lineRefColor.clone());
        s.borrow_mut().lineRefColor = _tmp_6144;
        s.borrow_mut().effectRef = src.borrow().effectRef;
        let _tmp_6147 = PptxClone::color(&src.borrow().fontRefColor.clone());
        s.borrow_mut().fontRefColor = _tmp_6147;
        let _tmp_6149 = PptxClone::color(&src.borrow().fill.clone());
        s.borrow_mut().fill = _tmp_6149;
        let _tmp_6151 = PptxClone::gradient(&src.borrow().fillGrad.clone());
        s.borrow_mut().fillGrad = _tmp_6151;
        let _tmp_6153 = PptxClone::color(&src.borrow().line.clone());
        s.borrow_mut().line = _tmp_6153;
        s.borrow_mut().lineWidthPt = src.borrow().lineWidthPt;
        let _tmp_6156 = PptxClone::shadow(&src.borrow().shadow.clone());
        s.borrow_mut().shadow = _tmp_6156;
        s.borrow_mut().hasText = src.borrow().hasText;
        let _tmp_6159 = PptxClone::textBody(&src.borrow().text.clone());
        s.borrow_mut().text = _tmp_6159;
        s.borrow_mut().imageRid = src.borrow().imageRid.clone();
        s.borrow_mut().imagePart = src.borrow().imagePart.clone();
        s.borrow_mut().imageBytes = src.borrow().imageBytes.clone();
        s.borrow_mut().imageOk = src.borrow().imageOk;
        s.borrow_mut().svgBytes = src.borrow().svgBytes.clone();
        s.borrow_mut().svgPart = src.borrow().svgPart.clone();
        s.borrow_mut().locked = src.borrow().locked;
        s.borrow_mut().altText = src.borrow().altText.clone();
        // Provenance travels with the shape. A copy that dropped it would turn
        // an editable diagram into ink the first time a reader duplicated a
        // slide.
        s.borrow_mut().embedNotation = src.borrow().embedNotation.clone();
        s.borrow_mut().embedSource = src.borrow().embedSource.clone();
        s.borrow_mut().mdSrcStart = src.borrow().mdSrcStart;
        s.borrow_mut().mdSrcEnd = src.borrow().mdSrcEnd;
        s.borrow_mut().mdLastText = src.borrow().mdLastText.clone();
        s.borrow_mut().hasStyle = src.borrow().hasStyle;
        s.borrow_mut().fontRef = src.borrow().fontRef.clone();
        s.borrow_mut().styleFillSig = src.borrow().styleFillSig.clone();
        s.borrow_mut().styleLineSig = src.borrow().styleLineSig.clone();
        s.borrow_mut().sheetXfrmSig = src.borrow().sheetXfrmSig.clone();
        s.borrow_mut().isPlaceholder = src.borrow().isPlaceholder;
        s.borrow_mut().phType = src.borrow().phType.clone();
        s.borrow_mut().phIdx = src.borrow().phIdx;
        s.borrow_mut().inherited = src.borrow().inherited;
        s.borrow_mut().hasPath = src.borrow().hasPath;
        s.borrow_mut().pathClosed = src.borrow().pathClosed;
        let mut pi: i64 = 0;
        while pi < (src.borrow().pathUnit.len() as i64) {
            s.borrow_mut().pathUnit.push(src.borrow().pathUnit[pi as usize]);
            pi += 1;
        }
        // The contour boundaries travel with the points. A duplicate that
        // kept the points and dropped these is a shape whose holes fill in.
        let mut pe: i64 = 0;
        while pe < (src.borrow().pathEnds.len() as i64) {
            s.borrow_mut().pathEnds.push(src.borrow().pathEnds[pe as usize]);
            pe += 1;
        }
        s.borrow_mut().tableRows = src.borrow().tableRows;
        s.borrow_mut().tableCols = src.borrow().tableCols;
        let mut ci: i64 = 0;
        while ci < (src.borrow().tableCells.len() as i64) {
            let mut cell: Rc<RefCell<PptxTableCell>> = src.borrow().tableCells[ci as usize].clone();
            s.borrow_mut().tableCells.push(PptxClone::tableCell(&cell));
            ci += 1;
        }
        let _tmp_6187 = PptxClone::chart(&src.borrow().chart.clone());
        s.borrow_mut().chart = _tmp_6187;
        s.borrow_mut().chX = src.borrow().chX;
        s.borrow_mut().chY = src.borrow().chY;
        s.borrow_mut().chW = src.borrow().chW;
        s.borrow_mut().chH = src.borrow().chH;
        s.borrow_mut().isDiagram = src.borrow().isDiagram;
        let mut _tmp_6193 = src.borrow().diagramFrame.clone();
        SourceRef::copyFrom(&s.borrow().diagramFrame.clone(), &_tmp_6193);
        s.borrow_mut().diagramSig = src.borrow().diagramSig.clone();
        s.borrow_mut().diagramText = src.borrow().diagramText.clone();
        s.borrow_mut().diagramModelXml = src.borrow().diagramModelXml.clone();
        s.borrow_mut().diagramLayoutXml = src.borrow().diagramLayoutXml.clone();
        s.borrow_mut().diagramLayoutId = src.borrow().diagramLayoutId.clone();
        s.borrow_mut().diagramStyleId = src.borrow().diagramStyleId.clone();
        s.borrow_mut().diagramColorsId = src.borrow().diagramColorsId.clone();
        s.borrow_mut().diagramStyleXml = src.borrow().diagramStyleXml.clone();
        s.borrow_mut().diagramColorsXml = src.borrow().diagramColorsXml.clone();
        s.borrow_mut().diagramHadDrawing = src.borrow().diagramHadDrawing;
        let mut dppi: i64 = 0;
        while dppi < (src.borrow().diagramParaPts.len() as i64) {
            s.borrow_mut().diagramParaPts.push(src.borrow().diagramParaPts[dppi as usize].clone());
            dppi += 1;
        }
        let mut i: i64 = 0;
        while i < (src.borrow().children.len() as i64) {
            let mut child: Rc<RefCell<PptxShape>> = src.borrow().children[i as usize].clone();
            s.borrow_mut().children.push(PptxClone::shape(&child));
            i += 1;
        }
        s
    }
    pub fn sheet(src: &Rc<RefCell<PptxSheet>>) -> Rc<RefCell<PptxSheet>> {
        let mut out: Rc<RefCell<PptxSheet>> = Rc::new(RefCell::new(PptxSheet::new()));
        out.borrow_mut().partPath = src.borrow().partPath.clone();
        out.borrow_mut().dirty = src.borrow().dirty;
        let _tmp_6208 = PptxClone::color(&src.borrow().bg.clone());
        out.borrow_mut().bg = _tmp_6208;
        let mut i: i64 = 0;
        while i < (src.borrow().shapes.len() as i64) {
            out.borrow_mut().shapes.push(PptxClone::shape(&src.borrow().shapes[i as usize].clone()));
            i += 1;
        }
        out
    }
    pub fn section(src: &Rc<RefCell<PptxSection>>) -> Rc<RefCell<PptxSection>> {
        let mut out: Rc<RefCell<PptxSection>> = Rc::new(RefCell::new(PptxSection::new()));
        out.borrow_mut().name = src.borrow().name.clone();
        out.borrow_mut().id = src.borrow().id.clone();
        let mut i: i64 = 0;
        while i < (src.borrow().slideIds.len() as i64) {
            out.borrow_mut().slideIds.push(src.borrow().slideIds[i as usize]);
            i += 1;
        }
        out
    }
    pub fn opaqueInto(src: &Vec<Rc<RefCell<PptxOpaque>>>, mut dst: &mut Vec<Rc<RefCell<PptxOpaque>>>) {
        let mut i: i64 = 0;
        while i < (src.len() as i64) {
            let mut so: Rc<RefCell<PptxOpaque>> = src[i as usize].clone();
            let mut op: Rc<RefCell<PptxOpaque>> = Rc::new(RefCell::new(PptxOpaque::new()));
            op.borrow_mut().name = so.borrow().name.clone();
            op.borrow_mut().afterShapes = so.borrow().afterShapes;
            let mut _tmp_6213 = so.borrow().source.clone();
            SourceRef::copyFrom(&op.borrow().source.clone(), &_tmp_6213);
            dst.push(op.clone());
            i += 1;
        }
    }
    pub fn slide(src: &Rc<RefCell<PptxSlide>>) -> Rc<RefCell<PptxSlide>> {
        let mut sl: Rc<RefCell<PptxSlide>> = Rc::new(RefCell::new(PptxSlide::new()));
        sl.borrow_mut().partPath = src.borrow().partPath.clone();
        sl.borrow_mut().dirty = src.borrow().dirty;
        sl.borrow_mut().revision = src.borrow().revision;
        sl.borrow_mut().width = src.borrow().width;
        sl.borrow_mut().height = src.borrow().height;
        let _tmp_6221 = PptxClone::color(&src.borrow().bg.clone());
        sl.borrow_mut().bg = _tmp_6221;
        sl.borrow_mut().layoutPart = src.borrow().layoutPart.clone();
        sl.borrow_mut().masterPart = src.borrow().masterPart.clone();
        let _tmp_6225 = PptxClone::shape(&src.borrow().notes.clone());
        sl.borrow_mut().notes = _tmp_6225;
        sl.borrow_mut().notesPart = src.borrow().notesPart.clone();
        sl.borrow_mut().slideId = src.borrow().slideId;
        sl.borrow_mut().key = src.borrow().key;
        let mut tr: Rc<RefCell<PptxTransition>> = Rc::new(RefCell::new(PptxTransition::new()));
        tr.borrow_mut().isSet = src.borrow().transition.borrow().isSet;
        tr.borrow_mut().kind = src.borrow().transition.borrow().kind.clone();
        tr.borrow_mut().dir = src.borrow().transition.borrow().dir.clone();
        tr.borrow_mut().seconds = src.borrow().transition.borrow().seconds;
        tr.borrow_mut().onClick = src.borrow().transition.borrow().onClick;
        tr.borrow_mut().afterSeconds = src.borrow().transition.borrow().afterSeconds;
        sl.borrow_mut().transition = tr.clone();
        let mut bi: i64 = 0;
        while bi < (src.borrow().build.len() as i64) {
            let mut b: Rc<RefCell<PptxBuildStep>> = src.borrow().build[bi as usize].clone();
            let mut step: Rc<RefCell<PptxBuildStep>> = Rc::new(RefCell::new(PptxBuildStep::new()));
            step.borrow_mut().editId = b.borrow().editId;
            step.borrow_mut().shapeId = b.borrow().shapeId;
            step.borrow_mut().kind = b.borrow().kind.clone();
            step.borrow_mut().dir = b.borrow().dir.clone();
            step.borrow_mut().seconds = b.borrow().seconds;
            step.borrow_mut().paragraph = b.borrow().paragraph;
            step.borrow_mut().start = b.borrow().start.clone();
            sl.borrow_mut().build.push(step.clone());
            bi += 1;
        }
        let mut i: i64 = 0;
        while i < (src.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = src.borrow().shapes[i as usize].clone();
            sl.borrow_mut().shapes.push(PptxClone::shape(&sh));
            i += 1;
        }
        // What the slide carries and this model does not describe. A snapshot
        // that dropped these would delete a deck's SmartArt on the first undo,
        // which is exactly the failure the source spans exist to prevent.
        PptxClone::opaqueInto(&src.borrow().opaque.clone(), &mut sl.borrow_mut().opaque);
        PptxClone::opaqueInto(&src.borrow().sldHead.clone(), &mut sl.borrow_mut().sldHead);
        PptxClone::opaqueInto(&src.borrow().sldTail.clone(), &mut sl.borrow_mut().sldTail);
        sl
    }
}

/// One point in the undo stack: the slides, which slide was in front of you, and
/// what was selected — restoring geometry without the selection loses your place.
/// What the format painter carries: how a shape is filled, outlined, shadowed
/// and lettered, with none of its geometry and none of its words.
#[derive(Clone)]
pub struct PptxFormat {
    pub fill: Rc<RefCell<PptxColor>>,
    pub grad: Rc<RefCell<PptxGradient>>,
    pub line: Rc<RefCell<PptxColor>>,
    pub lineWidthPt: f64,
    pub shadow: Rc<RefCell<PptxShadow>>,
    pub hasText: bool,
    pub bold: bool,
    pub italic: bool,
    pub sizePt: f64,
    pub sizeSet: bool,
    pub family: String,
    pub color: Rc<RefCell<PptxColor>>,
    pub align: String,
    pub alignSet: bool,
}

impl PptxFormat {
    pub fn new() -> Self {
        Self {
            fill: Rc::new(RefCell::new(PptxColor::new())),
            grad: Rc::new(RefCell::new(PptxGradient::new())),
            line: Rc::new(RefCell::new(PptxColor::new())),
            lineWidthPt: 0.0,
            shadow: Rc::new(RefCell::new(PptxShadow::new())),
            hasText: false,
            bold: false,
            italic: false,
            sizePt: 18.0,
            sizeSet: false,
            family: String::new(),
            color: Rc::new(RefCell::new(PptxColor::new())),
            align: "l".to_string(),
            alignSet: false,
        }
    }
}

#[derive(Clone)]
pub struct PptxEditSnapshot {
    pub label: String,
    pub slideIndex: i64,
    pub slides: Vec<Rc<RefCell<PptxSlide>>>,
    pub selection: Vec<i64>,
    /// The master and layout sheets, because they can be edited too, and the
    /// sections, because reordering the deck moves them.
    pub sheetKeys: Vec<String>,
    pub sheetValues: Vec<Rc<RefCell<PptxSheet>>>,
    pub sheetIsMaster: Vec<bool>,
    pub sections: Vec<Rc<RefCell<PptxSection>>>,
    pub sheetPart: String,
}

impl PptxEditSnapshot {
    pub fn new() -> Self {
        let mut me = Self {
            label: String::new(),
            slideIndex: 0,
            slides: Vec::new(),
            selection: Vec::new(),
            sheetKeys: Vec::new(),
            sheetValues: Vec::new(),
            sheetIsMaster: Vec::new(),
            sections: Vec::new(),
            sheetPart: String::new(),
        };
        let mut s_3: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        me.slides = s_3.clone();
        let mut sel: Vec<i64> = Vec::new();
        me.selection = sel.clone();
        let mut sk: Vec<String> = Vec::new();
        me.sheetKeys = sk.clone();
        let mut sv: Vec<Rc<RefCell<PptxSheet>>> = Vec::new();
        me.sheetValues = sv.clone();
        let mut sm: Vec<bool> = Vec::new();
        me.sheetIsMaster = sm.clone();
        let mut sc: Vec<Rc<RefCell<PptxSection>>> = Vec::new();
        me.sections = sc.clone();
        me
    }
}

#[derive(Clone)]
pub struct PptxEditor {
    pub doc: Rc<RefCell<PptxPresentation>>,
    pub slideIndex: i64,
    pub selection: Vec<i64>,
    /// The snapshots and the cap — `gallery/office/editor`, the same file the
    /// document and the spreadsheet keep their ops in. A deck step is a whole
    /// snapshot rather than an operation, and the shared history does not care
    /// what it holds: nothing is asked of the type parameter.
    pub history: Rc<RefCell<OfficeHistory<Rc<RefCell<PptxEditSnapshot>>>>>,
    pub nextId: i64,
    pub dirty: bool,
    pub attached: bool,
    /// Alignment guides the last snapped move produced, in slide points, for a
    /// host that wants to draw them.
    pub snapEnabled: bool,
    pub snapTolPt: f64,
    pub guideX: Vec<f64>,
    pub guideY: Vec<f64>,
    pub minSizePt: f64,
    /// Text editing. Zero means the deck is being arranged rather than written
    /// in — a shape is picked up and moved, not typed into.
    pub editingId: i64,
    pub caret: Rc<RefCell<PptxTextPos>>,
    /// Where a turning selection started, so every frame of the drag turns from
    /// the same place rather than from the last frame's answer.
    pub rotActive: bool,
    pub rotPivotX: f64,
    pub rotPivotY: f64,
    /// The live angle of a multi-shape rotation, and the box the selection had
    /// when it started.
    ///
    /// The chrome needs both. A single shape's frame turns with the shape,
    /// because the shape IS the selection; several shapes have no such angle,
    /// so the frame stayed square while everything inside it turned — which is
    /// how it was reported. And it cannot be the current bounds turned either:
    /// those are the axis-aligned box of shapes that are themselves turning, so
    /// it grows as the drag goes on and stops hugging anything.
    pub rotDeg: f64,
    pub rotBox: Rc<RefCell<PptxEditRect>>,
    pub rotStartX: Vec<f64>,
    pub rotStartY: Vec<f64>,
    pub rotStartDeg: Vec<f64>,
    pub anchor: Rc<RefCell<PptxTextPos>>,
    /// What was copied, and where a drag on empty canvas started. The clipboard
    /// is the editor's own — a browser tab cannot read the system one without
    /// asking, and a deck's shapes are not text anyway.
    pub clip: Vec<Rc<RefCell<PptxShape>>>,
    pub gridEnabled: bool,
    pub gridPt: f64,
    pub format: Rc<RefCell<PptxFormat>>,
    pub formatSet: bool,
    /// Never reused, so a revision names one state of one slide even after an
    /// undo has thrown a later one away.
    pub nextRevision: i64,
    /// --- history, shared ------------------------------------------------------
    ///
    /// A step used to copy the WHOLE deck, which is right while the operation
    /// set is still moving and wrong once it has stopped: at five hundred slides
    /// a keystroke cost thirty milliseconds, three frames, because four hundred
    /// and ninety-nine slides were copied to record a change to one.
    ///
    /// So a step copies only the slides that CHANGED, and every step shares the
    /// rest. The cache below holds one clone per slide, tagged with the revision
    /// it was taken at; a capture asks it for each slide and only pays for the
    /// ones whose revision has moved on. Nothing in it is ever a live object —
    /// it stores copies — and `restoreSnapshot` copies out of it again, so a
    /// shared clone can never be reached by an edit.
    ///
    /// The invariant the whole thing rests on: **a slide's revision changes
    /// whenever its content does.** `pushSnapshot` and `pushCoalesced` bump the
    /// slide the step is on before capturing, and the two operations that
    /// rewrite EVERY slide — `remergeChrome` and `retheme` — bump all of them.
    pub nextSlideKey: i64,
    pub cloneRev: HashMap<i64,i64>,
    pub cloneVal: HashMap<i64,Rc<RefCell<PptxSlide>>>,
    /// For re-resolving a palette colour; the same mapping the loader used.
    pub resolver: Rc<RefCell<PptxResolver>>,
    /// Editing a master or a layout: the sheet stands in for the slide, so
    /// every operation that knows how to move a shape works on it unchanged.
    pub sheetPart: String,
    pub sheetSlide: Rc<RefCell<PptxSlide>>,
}

impl PptxEditor {
    pub fn new() -> Self {
        Self {
            doc: Rc::new(RefCell::new(PptxPresentation::new())),
            slideIndex: 0,
            selection: Vec::new(),
            history: Rc::new(RefCell::new(OfficeHistory::new())),
            nextId: 1,
            dirty: false,
            attached: false,
            snapEnabled: true,
            snapTolPt: 3.0,
            guideX: Vec::new(),
            guideY: Vec::new(),
            minSizePt: 4.0,
            editingId: 0,
            caret: Rc::new(RefCell::new(PptxTextPos::new())),
            rotActive: false,
            rotPivotX: 0.0,
            rotPivotY: 0.0,
            rotDeg: 0.0,
            rotBox: PptxEditRect::of(0.0, 0.0, 0.0, 0.0),
            rotStartX: Vec::new(),
            rotStartY: Vec::new(),
            rotStartDeg: Vec::new(),
            anchor: Rc::new(RefCell::new(PptxTextPos::new())),
            clip: Vec::new(),
            gridEnabled: false,
            gridPt: 9.0,
            format: Rc::new(RefCell::new(PptxFormat::new())),
            formatSet: false,
            nextRevision: 0,
            nextSlideKey: 1,
            cloneRev: HashMap::new(),
            cloneVal: HashMap::new(),
            resolver: Rc::new(RefCell::new(PptxResolver::new())),
            sheetPart: String::new(),
            sheetSlide: Rc::new(RefCell::new(PptxSlide::new())),
        }
    }
    pub fn findIn(shape: &Rc<RefCell<PptxShape>>, id: i64) -> Option<Rc<RefCell<PptxShape>>> {
        if shape.borrow().editId == id {
            let mut __self: Option<Rc<RefCell<PptxShape>>> = Some(shape.clone());
            return __self;
        }
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            let mut child: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
            let mut hit: Option<Rc<RefCell<PptxShape>>> = PptxEditor::findIn(&child, id);
            if hit.is_some() {
                return hit;
            }
            i += 1;
        }
        let mut __missing: Option<Rc<RefCell<PptxShape>>> = None;
        __missing
    }
    /// --- hit testing ---------------------------------------------------------
    pub fn containsPoint(shape: &Rc<RefCell<PptxShape>>, px: f64, py: f64) -> bool {
        let cx: f64 = shape.borrow().x + shape.borrow().width * 0.5;
        let cy: f64 = shape.borrow().y + shape.borrow().height * 0.5;
        let dx: f64 = px - cx;
        let dy: f64 = py - cy;
        let mut rx: f64 = dx;
        let mut ry: f64 = dy;
        if shape.borrow().rotation > 0.001 || shape.borrow().rotation < -0.001 {
            let rad: f64 = (0.0 - shape.borrow().rotation) * 0.017453292519943295;
            let cs: f64 = rad.cos();
            let sn: f64 = rad.sin();
            rx = dx * cs - dy * sn;
            ry = dx * sn + dy * cs;
        }
        let hw: f64 = shape.borrow().width * 0.5;
        let hh: f64 = shape.borrow().height * 0.5;
        let ax: f64 = rx.abs();
        let ay: f64 = ry.abs();
        if ax > hw {
            return false;
        }
        if ay > hh {
            return false;
        }
        true
    }
    /// The nudge that would put one of `edges` onto one of `lines`, or 0 when
    /// nothing is near enough. `outLine` gets the line that was matched.
    pub fn nearestSnap(edges: &[f64], lines: &[f64], tol: f64) -> f64 {
        let mut best: f64 = 0.0;
        let mut best_dist: f64 = tol;
        let mut found: bool = false;
        let mut i: i64 = 0;
        while i < (edges.len() as i64) {
            let e: f64 = edges[i as usize];
            let mut j: i64 = 0;
            while j < (lines.len() as i64) {
                let l: f64 = lines[j as usize];
                let diff: f64 = l - e;
                let dist: f64 = diff.abs();
                if dist <= best_dist {
                    if !found {
                        best = diff;
                        best_dist = dist;
                        found = true;
                    } else {
                        if dist < best_dist {
                            best = diff;
                            best_dist = dist;
                        }
                    }
                }
                j += 1;
            }
            i += 1;
        }
        best
    }
    /// The line a snapped edge landed on, for drawing. Recomputed rather than
    /// carried out of `nearestSnap`, which returns one value.
    pub fn matchedLine(edges: &[f64], lines: &[f64], tol: f64) -> f64 {
        let mut best_line: f64 = 0.0;
        let mut best_dist: f64 = tol;
        let mut found: bool = false;
        let mut i: i64 = 0;
        while i < (edges.len() as i64) {
            let e: f64 = edges[i as usize];
            let mut j: i64 = 0;
            while j < (lines.len() as i64) {
                let l: f64 = lines[j as usize];
                let dist: f64 = (l - e).abs();
                if dist <= best_dist {
                    if !found {
                        best_line = l;
                        best_dist = dist;
                        found = true;
                    } else {
                        if dist < best_dist {
                            best_line = l;
                            best_dist = dist;
                        }
                    }
                }
                j += 1;
            }
            i += 1;
        }
        best_line
    }
    /// --- turning a selection ---------------------------------------------------
    ///
    /// Turning every shape by the same angle about ITS OWN centre leaves them
    /// all exactly where they were and spins each in place. That is not a
    /// rotation of the selection — it is several rotations that happen to share
    /// an angle, and on screen a group came apart instead of turning.
    ///
    /// A selection turns about the selection's centre: each shape keeps its own
    /// angle relative to the group, and its centre travels round that pivot.
    pub fn spinAbout(shapes: &Vec<Rc<RefCell<PptxShape>>>, pivot_x: f64, pivot_y: f64, delta: f64) {
        let rad: f64 = (delta * 3.141592653589793) / 180.0;
        let co: f64 = rad.cos();
        let si: f64 = rad.sin();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let cx: f64 = (sh.borrow().x + sh.borrow().width * 0.5) - pivot_x;
            let cy: f64 = (sh.borrow().y + sh.borrow().height * 0.5) - pivot_y;
            let nx: f64 = (cx * co - cy * si) + pivot_x;
            let ny: f64 = (cx * si + cy * co) + pivot_y;
            let hoist_1 = nx - sh.borrow().width * 0.5;
            sh.borrow_mut().x = hoist_1;
            let hoist_2 = ny - sh.borrow().height * 0.5;
            sh.borrow_mut().y = hoist_2;
            let hoist_3 = PptxEditor::normalizeDeg(sh.borrow().rotation + delta);
            sh.borrow_mut().rotation = hoist_3;
            i += 1;
        }
    }
    pub fn normalizeDeg(deg: f64) -> f64 {
        let mut d: f64 = deg;
        while d < 0.0 {
            d += 360.0;
        }
        while d >= 360.0 {
            d -= 360.0;
        }
        d
    }
    /// --- text ----------------------------------------------------------------
    pub fn firstRun(body: &Rc<RefCell<PptxTextBody>>) -> Option<Rc<RefCell<PptxTextRun>>> {
        if body.borrow().paragraphs.is_empty() {
            let mut __none: Option<Rc<RefCell<PptxTextRun>>> = None;
            return __none;
        }
        let mut p: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[0].clone();
        if p.borrow().runs.is_empty() {
            let mut __none2: Option<Rc<RefCell<PptxTextRun>>> = None;
            return __none2;
        }
        let mut hit: Option<Rc<RefCell<PptxTextRun>>> = Some(p.borrow().runs[0].clone());
        hit
    }
    /// Replace a body with one paragraph per line, keeping the first run's look
    /// so retyping a title does not reset it to 18pt black.
    pub fn setBodyText(body: &Rc<RefCell<PptxTextBody>>, text: &str) {
        let mut proto: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        let mut existing: Option<Rc<RefCell<PptxTextRun>>> = PptxEditor::firstRun(body);
        if existing.is_some() {
            let mut e: Rc<RefCell<PptxTextRun>> = existing.clone().unwrap();
            proto.borrow_mut().fontFamily = e.borrow().fontFamily.clone();
            proto.borrow_mut().fontSizePt = e.borrow().fontSizePt;
            proto.borrow_mut().sizeSet = e.borrow().sizeSet;
            proto.borrow_mut().boldSet = e.borrow().boldSet;
            proto.borrow_mut().bold = e.borrow().bold;
            proto.borrow_mut().italicSet = e.borrow().italicSet;
            proto.borrow_mut().italic = e.borrow().italic;
            let _tmp_6256 = PptxClone::color(&e.borrow().color.clone());
            proto.borrow_mut().color = _tmp_6256;
        }
        let mut align: String = "l".to_string();
        let mut align_set: bool = false;
        if !body.borrow().paragraphs.is_empty() {
            let mut p0: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[0].clone();
            align = p0.borrow().align.clone();
            align_set = p0.borrow().alignSet;
        }
        let mut paras: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
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
                let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
                para.borrow_mut().align = align.clone();
                para.borrow_mut().alignSet = align_set;
                let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                run.borrow_mut().text = piece.clone();
                run.borrow_mut().fontFamily = proto.borrow().fontFamily.clone();
                run.borrow_mut().fontSizePt = proto.borrow().fontSizePt;
                run.borrow_mut().sizeSet = proto.borrow().sizeSet;
                run.borrow_mut().bold = proto.borrow().bold;
                run.borrow_mut().boldSet = proto.borrow().boldSet;
                run.borrow_mut().italic = proto.borrow().italic;
                run.borrow_mut().italicSet = proto.borrow().italicSet;
                let _tmp_6265 = PptxClone::color(&proto.borrow().color.clone());
                run.borrow_mut().color = _tmp_6265;
                para.borrow_mut().runs.push(run.clone());
                paras.push(para.clone());
                start = i + 1;
            }
            i += 1;
        }
        body.borrow_mut().paragraphs = paras.clone();
    }
    /// --- what a drag over empty canvas selects --------------------------------
    pub fn rectsOverlap(ax: f64, ay: f64, aw: f64, ah: f64, bx: f64, by: f64, bw: f64, bh: f64) -> bool {
        if ax + aw < bx {
            return false;
        }
        if bx + bw < ax {
            return false;
        }
        if ay + ah < by {
            return false;
        }
        if by + bh < ay {
            return false;
        }
        true
    }
    /// --- the grid -------------------------------------------------------------
    pub fn snapToStep(v: f64, step: f64) -> f64 {
        if step <= 0.0 {
            return v;
        }
        // Rounded on the magnitude, sign put back: `to_int` floors, so
        // subtracting a half on the negative side rounds away from zero twice
        // and -11 on a 9pt grid lands on -18.
        let n: f64 = v / step;
        let neg: bool = n < 0.0;
        let mut mag: f64 = n;
        if neg {
            mag = 0.0 - n;
        }
        let k: i64 = (mag + 0.5).floor() as i64;
        let out: f64 = (k as f64) * step;
        if neg {
            return 0.0 - out;
        }
        out
    }
    /// --- the theme ------------------------------------------------------------
    ///
    /// A deck's colours mostly come from its palette, and the resolver keeps
    /// that provenance now, so re-theming is: put a new palette in the theme and
    /// ask every colour that came from one to resolve again.
    pub fn paletteNames() -> Vec<String> {
        vec!["office".to_string(), "forest".to_string(), "berry".to_string(), "slate".to_string(), "sunset".to_string()].clone()
    }
    pub fn paletteColors(name: &str) -> Vec<String> {
        // dk1 lt1 dk2 lt2 accent1..6 hlink folHlink
        if name == "forest" {
            return vec!["1E2B22".to_string(), "FFFFFF".to_string(), "375845".to_string(), "EAF2EC".to_string(), "217346".to_string(), "6BA644".to_string(), "9BC53D".to_string(), "C7D96A".to_string(), "2F6F5E".to_string(), "48A9A6".to_string(), "1F6F54".to_string(), "6B7C74".to_string()].clone();
        }
        if name == "berry" {
            return vec!["2B1A22".to_string(), "FFFFFF".to_string(), "5A2B3F".to_string(), "F6EAF0".to_string(), "B0295E".to_string(), "E0518A".to_string(), "F08CB4".to_string(), "8E44AD".to_string(), "C2185B".to_string(), "6A1B4D".to_string(), "AD1457".to_string(), "7B5E6B".to_string()].clone();
        }
        if name == "slate" {
            return vec!["1B1F24".to_string(), "FFFFFF".to_string(), "36414D".to_string(), "EDF1F5".to_string(), "3D5A80".to_string(), "5C82B2".to_string(), "98C1D9".to_string(), "6C7A89".to_string(), "2F4858".to_string(), "8FA6B8".to_string(), "2C5F8A".to_string(), "6E7B87".to_string()].clone();
        }
        if name == "sunset" {
            return vec!["2A1B12".to_string(), "FFFFFF".to_string(), "5A3924".to_string(), "FBEEE4".to_string(), "E4572E".to_string(), "F5A623".to_string(), "F2C14E".to_string(), "C1440E".to_string(), "8C3503".to_string(), "A44A3F".to_string(), "D2691E".to_string(), "8B6F5E".to_string()].clone();
        }
        vec!["000000".to_string(), "FFFFFF".to_string(), "44546A".to_string(), "E7E6E6".to_string(), "4472C4".to_string(), "ED7D31".to_string(), "A5A5A5".to_string(), "FFC000".to_string(), "5B9BD5".to_string(), "70AD47".to_string(), "0563C1".to_string(), "954F72".to_string()].clone()
    }
    pub fn namedShape(sheet: &Rc<RefCell<PptxSheet>>, name: &str) -> Option<Rc<RefCell<PptxShape>>> {
        let mut i: i64 = 0;
        while i < (sheet.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = sheet.borrow().shapes[i as usize].clone();
            if sh.borrow().name == name {
                let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(sh.clone());
                return hit;
            }
            i += 1;
        }
        let mut __none: Option<Rc<RefCell<PptxShape>>> = None;
        __none
    }
    /// --- deck ----------------------------------------------------------------
    pub fn attach(&mut self, pres: Rc<RefCell<PptxPresentation>>) {
        self.doc = pres.clone();
        self.slideIndex = 0;
        self.nextId = 1;
        self.dirty = false;
        self.attached = true;
        self.assignIds();
        self.clearSelection();
        OfficeHistory::reset(&self.history.clone());
        self.history.borrow_mut().limit = 60;
        self.pushSnapshot("open");
        self.dirty = false;
    }
    pub fn assignIds(&mut self) {
        let mut si: i64 = 0;
        while si < (self.doc.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[si as usize].clone();
            let mut i: i64 = 0;
            while i < (slide.borrow().shapes.len() as i64) {
                let mut sh: Rc<RefCell<PptxShape>> = slide.borrow().shapes[i as usize].clone();
                self.assignIdTo(&sh);
                i += 1;
            }
            self.assignIdTo(&slide.borrow().notes.clone());
            self.linkBuild(&slide);
            if slide.borrow().key == 0 {
                slide.borrow_mut().key = self.nextSlideKey;
                self.nextSlideKey += 1;
            }
            si += 1;
        }
    }
    /// A build step names its shape by the number the FILE uses; everything in
    /// the editor names shapes by `editId`, because a file's number is rewritten
    /// on every save. Link them once, when the deck is attached.
    pub fn linkBuild(&self, slide: &Rc<RefCell<PptxSlide>>) {
        let mut kept: Vec<Rc<RefCell<PptxBuildStep>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (slide.borrow().build.len() as i64) {
            let mut step: Rc<RefCell<PptxBuildStep>> = slide.borrow().build[i as usize].clone();
            let mut k: i64 = 0;
            while k < (slide.borrow().shapes.len() as i64) {
                let mut sh: Rc<RefCell<PptxShape>> = slide.borrow().shapes[k as usize].clone();
                if sh.borrow().shapeId == step.borrow().shapeId {
                    step.borrow_mut().editId = sh.borrow().editId;
                }
                k += 1;
            }
            // A step naming a shape that is not on the slide animates nothing.
            if step.borrow().editId > 0 {
                kept.push(step.clone());
            }
            i += 1;
        }
        slide.borrow_mut().build = kept.clone();
    }
    pub fn assignIdTo(&mut self, shape: &Rc<RefCell<PptxShape>>) {
        shape.borrow_mut().editId = self.nextId;
        self.nextId += 1;
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            let mut child: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
            self.assignIdTo(&child);
            i += 1;
        }
    }
    pub fn slideCount(&self) -> i64 {
        self.doc.borrow().slides.len() as i64
    }
    pub fn currentSlide(&self) -> Option<Rc<RefCell<PptxSlide>>> {
        if !self.sheetPart.is_empty() {
            let mut editing: Option<Rc<RefCell<PptxSlide>>> = Some(self.sheetSlide.clone()).clone();
            return editing;
        }
        if self.slideIndex < 0 {
            let mut __none: Option<Rc<RefCell<PptxSlide>>> = None;
            return __none;
        }
        if self.slideIndex >= (self.doc.borrow().slides.len() as i64) {
            let mut __none2: Option<Rc<RefCell<PptxSlide>>> = None;
            return __none2;
        }
        let mut hit: Option<Rc<RefCell<PptxSlide>>> = Some(self.doc.borrow().slides[(self.slideIndex) as usize].clone());
        hit
    }
    pub fn gotoSlide(&mut self, i: i64) {
        if i < 0 {
            return;
        }
        if i >= (self.doc.borrow().slides.len() as i64) {
            return;
        }
        self.slideIndex = i;
        self.clearSelection();
    }
    pub fn slideWidth(&self) -> f64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_some() {
            let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
            return s.borrow().width;
        }
        self.doc.borrow().slideWidth
    }
    pub fn slideHeight(&self) -> f64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_some() {
            let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
            return s.borrow().height;
        }
        self.doc.borrow().slideHeight
    }
    /// The shapes a user may touch: what the slide itself declares. Master and
    /// layout chrome is drawn on the slide but is not on it — clicking the
    /// footer of a template should not offer to drag the template.
    pub fn editableShapes(&self) -> Vec<Rc<RefCell<PptxShape>>> {
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return out;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            if !sh.borrow().inherited {
                out.push(sh.clone());
            }
            i += 1;
        }
        out
    }
    pub fn findShape(&self, id: i64) -> Option<Rc<RefCell<PptxShape>>> {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            let mut __none: Option<Rc<RefCell<PptxShape>>> = None;
            return __none;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            let mut hit: Option<Rc<RefCell<PptxShape>>> = PptxEditor::findIn(&sh, id);
            if hit.is_some() {
                return hit;
            }
            i += 1;
        }
        // The notes are a shape too, and not one on the slide.
        if s.borrow().notes.borrow().editId == id {
            let mut notes_hit: Option<Rc<RefCell<PptxShape>>> = Some(s.borrow().notes.clone()).clone();
            return notes_hit;
        }
        let mut __missing: Option<Rc<RefCell<PptxShape>>> = None;
        __missing
    }
    /// --- selection -----------------------------------------------------------
    pub fn clearSelection(&mut self) {
        let mut s: Vec<i64> = Vec::new();
        self.selection = s.clone();
        self.editingId = 0;
    }
    pub fn selectionCount(&self) -> i64 {
        self.selection.len() as i64
    }
    pub fn isSelected(&self, id: i64) -> bool {
        let mut i: i64 = 0;
        while i < (self.selection.len() as i64) {
            if self.selection[i as usize] == id {
                return true;
            }
            i += 1;
        }
        false
    }
    pub fn select(&mut self, id: i64) {
        self.clearSelection();
        if id > 0 {
            self.selection.push(id);
        }
    }
    pub fn addToSelection(&mut self, id: i64) {
        if id <= 0 {
            return;
        }
        if self.isSelected(id) {
            return;
        }
        self.selection.push(id);
    }
    pub fn toggleSelection(&mut self, id: i64) {
        if id <= 0 {
            return;
        }
        if self.isSelected(id) {
            let mut out: Vec<i64> = Vec::new();
            let mut i: i64 = 0;
            while i < (self.selection.len() as i64) {
                let v: i64 = self.selection[i as usize];
                if v != id {
                    out.push(v);
                }
                i += 1;
            }
            self.selection = out.clone();
            return;
        }
        self.selection.push(id);
    }
    pub fn selectAll(&mut self) {
        self.clearSelection();
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.editableShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            self.selection.push(sh.borrow().editId);
            i += 1;
        }
    }
    pub fn selectedShapes(&self) -> Vec<Rc<RefCell<PptxShape>>> {
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (self.selection.len() as i64) {
            let id: i64 = self.selection[i as usize];
            let mut hit: Option<Rc<RefCell<PptxShape>>> = self.findShape(id);
            if hit.is_some() {
                out.push(hit.clone().unwrap());
            }
            i += 1;
        }
        out
    }
    pub fn firstSelected(&self) -> Option<Rc<RefCell<PptxShape>>> {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        if shapes.is_empty() {
            let mut __none: Option<Rc<RefCell<PptxShape>>> = None;
            return __none;
        }
        let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(shapes[0].clone());
        hit
    }
    /// Topmost first: the last shape painted is the one under the pointer.
    pub fn hitTest(&self, px: f64, py: f64) -> i64 {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.editableShapes();
        let mut i: i64 = (shapes.len() as i64) - 1;
        while i >= 0 {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if PptxEditor::containsPoint(&sh, px, py) {
                return sh.borrow().editId;
            }
            i -= 1;
        }
        0
    }
    pub fn selectAt(&mut self, px: f64, py: f64, additive: bool) -> bool {
        let id: i64 = self.hitTest(px, py);
        if id == 0 {
            if !additive {
                self.clearSelection();
            }
            return false;
        }
        if additive {
            self.toggleSelection(id);
            return true;
        }
        if self.isSelected(id) {
            return true;
        }
        self.select(id);
        true
    }
    /// --- bounds --------------------------------------------------------------
    pub fn selectionBounds(&self) -> Rc<RefCell<PptxEditRect>> {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        if shapes.is_empty() {
            return PptxEditRect::of(0.0, 0.0, 0.0, 0.0);
        }
        let mut first: Rc<RefCell<PptxShape>> = shapes[0].clone();
        let mut x0: f64 = first.borrow().x;
        let mut y0: f64 = first.borrow().y;
        let mut x1: f64 = first.borrow().x + first.borrow().width;
        let mut y1: f64 = first.borrow().y + first.borrow().height;
        let mut i: i64 = 1;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if sh.borrow().x < x0 {
                x0 = sh.borrow().x;
            }
            if sh.borrow().y < y0 {
                y0 = sh.borrow().y;
            }
            let r: f64 = sh.borrow().x + sh.borrow().width;
            let b: f64 = sh.borrow().y + sh.borrow().height;
            if r > x1 {
                x1 = r;
            }
            if b > y1 {
                y1 = b;
            }
            i += 1;
        }
        PptxEditRect::of(x0, y0, x1 - x0, y1 - y0)
    }
    /// --- history -------------------------------------------------------------
    /// A copy of this slide that the history may share. The cache answers with
    /// the copy it already has when the slide has not moved on since; otherwise
    /// it makes one and remembers it.
    /// A name of its own for a slide that has just come into the deck.
    pub fn freshKey(&mut self, slide: &Rc<RefCell<PptxSlide>>) {
        slide.borrow_mut().key = self.nextSlideKey;
        self.nextSlideKey += 1;
    }
    pub fn sharedClone(&mut self, slide: &Rc<RefCell<PptxSlide>>) -> Rc<RefCell<PptxSlide>> {
        if slide.borrow().key == 0 {
            slide.borrow_mut().key = self.nextSlideKey;
            self.nextSlideKey += 1;
        }
        let have: Option<i64> = self.cloneRev.get(&slide.borrow().key).cloned();
        if have.is_some() {
            if have.unwrap() == slide.borrow().revision {
                let mut cached: Option<Rc<RefCell<PptxSlide>>> = self.cloneVal.get(&slide.borrow().key)
                    .cloned();
                if cached.is_some() {
                    return cached.clone().unwrap();
                }
            }
        }
        let mut copy: Rc<RefCell<PptxSlide>> = PptxClone::slide(slide);
        self.cloneRev.insert(slide.borrow().key, slide.borrow().revision);
        self.cloneVal.insert(slide.borrow().key, copy.clone());
        copy
    }
    /// The slide a step is about has changed, so the copy the history holds of
    /// it is out of date. Called BEFORE the capture, which is what makes the
    /// sharing sound.
    pub fn touchSlide(&mut self, slide: &Rc<RefCell<PptxSlide>>) {
        self.nextRevision += 1;
        slide.borrow_mut().revision = self.nextRevision;
        slide.borrow_mut().dirty = true;
    }
    /// Two operations rewrite every slide in the deck rather than the one in
    /// front of you — re-merging the chrome and re-theming the colours — and
    /// both have to say so, or the history shares copies of slides that have
    /// quietly changed underneath it.
    pub fn touchAllSlides(&mut self) {
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            self.touchSlide(&self.doc.borrow().slides[i as usize].clone().clone());
            i += 1;
        }
    }
    pub fn captureSnapshot(&mut self, label: String) -> Rc<RefCell<PptxEditSnapshot>> {
        let mut snap: Rc<RefCell<PptxEditSnapshot>> = Rc::new(RefCell::new(PptxEditSnapshot::new()));
        snap.borrow_mut().label = label.clone();
        snap.borrow_mut().slideIndex = self.slideIndex;
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            let mut sl: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[i as usize].clone();
            snap.borrow_mut().slides.push(self.sharedClone(&sl));
            i += 1;
        }
        let mut j: i64 = 0;
        while j < (self.selection.len() as i64) {
            snap.borrow_mut().selection.push(self.selection[j as usize]);
            j += 1;
        }
        let mut mkeys: Vec<String> = self.doc.borrow()
            .masters.keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut mi: i64 = 0;
        while mi < (mkeys.len() as i64) {
            let mk: String = mkeys[mi as usize].clone();
            let mut ms: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .masters.get(&mk)
                .cloned();
            if ms.is_some() {
                snap.borrow_mut().sheetKeys.push(mk.clone());
                snap.borrow_mut().sheetValues.push(PptxClone::sheet(&ms.clone().unwrap()));
                snap.borrow_mut().sheetIsMaster.push(true);
            }
            mi += 1;
        }
        let mut lkeys: Vec<String> = self.doc.borrow()
            .layouts.keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut li: i64 = 0;
        while li < (lkeys.len() as i64) {
            let lk: String = lkeys[li as usize].clone();
            let mut ls: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .layouts.get(&lk)
                .cloned();
            if ls.is_some() {
                snap.borrow_mut().sheetKeys.push(lk.clone());
                snap.borrow_mut().sheetValues.push(PptxClone::sheet(&ls.clone().unwrap()));
                snap.borrow_mut().sheetIsMaster.push(false);
            }
            li += 1;
        }
        let mut qi: i64 = 0;
        while qi < (self.doc.borrow().sections.len() as i64) {
            snap.borrow_mut().sections.push(PptxClone::section(&self.doc.borrow().sections[qi as usize].clone()));
            qi += 1;
        }
        snap.borrow_mut().sheetPart = self.sheetPart.clone();
        snap
    }
    pub fn pushSnapshot(&mut self, label: &str) {
        // The slide this step is about has changed, and it has to say so BEFORE
        // the capture: the history shares the copies of the slides that did
        // not, and it tells them apart by revision.
        if label != "open" {
            let mut cur0: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
            if cur0.is_some() {
                self.touchSlide(&cur0.clone().unwrap());
            }
        }
        let mut snap: Rc<RefCell<PptxEditSnapshot>> = self.captureSnapshot(label.to_string());
        // Everything a step does to the stack: the redo tail goes, because a
        // new step makes it unreachable, and the oldest steps fall off the
        // bottom rather than the stack growing without end — a deck copy is
        // not free. Both rules are the shared file's.
        OfficeHistory::record(&self.history.clone(), snap.clone());
        self.dirty = true;
        // An operation inside a master writes through to the sheet: the ops
        // replace the shape array rather than mutating it in place.
        if !self.sheetPart.is_empty() {
            let mut live: Option<Rc<RefCell<PptxSheet>>> = self.sheetFor(&self.sheetPart.clone());
            if live.is_some() {
                let mut ls2: Rc<RefCell<PptxSheet>> = live.clone().unwrap();
                ls2.borrow_mut().shapes = self.sheetSlide.borrow().shapes.clone();
                ls2.borrow_mut().bg = self.sheetSlide.borrow().bg.clone();
                ls2.borrow_mut().dirty = true;
            }
        }
    }
    pub fn restoreSnapshot(&mut self, snap: &Rc<RefCell<PptxEditSnapshot>>) {
        // The other half of the sharing. A snapshot's copy of a slide and the
        // live slide with the same key and the same revision are the same
        // state, so the live one is already what the undo wants and is kept
        // where it stands. Only the slides that actually differ are copied out
        // — and copied they must be, because what the snapshot holds is shared
        // with every other step that did not change them.
        let mut live_by_key: HashMap<i64,Rc<RefCell<PptxSlide>>> = HashMap::new();
        let mut li: i64 = 0;
        while li < (self.doc.borrow().slides.len() as i64) {
            let mut live: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[li as usize].clone();
            if live.borrow().key > 0 {
                live_by_key.insert(live.borrow().key, live.clone());
            }
            li += 1;
        }
        let mut slides: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (snap.borrow().slides.len() as i64) {
            let mut sl: Rc<RefCell<PptxSlide>> = snap.borrow().slides[i as usize].clone();
            let mut same: bool = false;
            if sl.borrow().key > 0 {
                let mut hit: Option<Rc<RefCell<PptxSlide>>> = live_by_key.get(&sl.borrow().key).cloned();
                if hit.is_some() {
                    let mut lv: Rc<RefCell<PptxSlide>> = hit.clone().unwrap();
                    if lv.borrow().revision == sl.borrow().revision {
                        slides.push(lv.clone());
                        same = true;
                    }
                }
            }
            if !same {
                slides.push(PptxClone::slide(&sl));
            }
            i += 1;
        }
        self.doc.borrow_mut().slides = slides.clone();
        // Stay on the slide you are looking at. The snapshot remembers which
        // slide the step was taken on, and that is the fallback — undoing a
        // deleted slide has to go back to it — but undoing an edit should not
        // move the deck under you the way it did when the answer was always
        // the snapshot's own index.
        let mut want: i64 = self.slideIndex;
        if want >= (self.doc.borrow().slides.len() as i64) {
            want = snap.borrow().slideIndex;
        }
        if want < 0 {
            want = snap.borrow().slideIndex;
        }
        self.slideIndex = want;
        if self.slideIndex >= (self.doc.borrow().slides.len() as i64) {
            self.slideIndex = (self.doc.borrow().slides.len() as i64) - 1;
        }
        if self.slideIndex < 0 {
            self.slideIndex = 0;
        }
        // What was selected before the step is the fallback, not the answer.
        // Undoing a move should leave the shape you moved selected — so the
        // live selection is kept for every id the restored deck still has, and
        // the snapshot's own selection is what comes back when it has none
        // (undoing a delete, where those ids exist again).
        let mut keep: Vec<i64> = Vec::new();
        let mut i2: i64 = 0;
        while i2 < (self.selection.len() as i64) {
            let id: i64 = self.selection[i2 as usize];
            let mut hit_1: Option<Rc<RefCell<PptxShape>>> = self.findShape(id);
            if hit_1.is_some() {
                keep.push(id);
            }
            i2 += 1;
        }
        if keep.is_empty() {
            let mut j: i64 = 0;
            while j < (snap.borrow().selection.len() as i64) {
                keep.push(snap.borrow().selection[j as usize]);
                j += 1;
            }
        }
        self.selection = keep.clone();
        // The sheets and the sections come back with the slides.
        let mut masters: HashMap<String,Rc<RefCell<PptxSheet>>> = HashMap::new();
        let mut layouts: HashMap<String,Rc<RefCell<PptxSheet>>> = HashMap::new();
        let mut si: i64 = 0;
        while si < (snap.borrow().sheetKeys.len() as i64) {
            let key: String = snap.borrow().sheetKeys[si as usize].clone();
            let mut sheet: Rc<RefCell<PptxSheet>> = PptxClone::sheet(&snap.borrow().sheetValues[si as usize].clone());
            if snap.borrow().sheetIsMaster[si as usize] {
                masters.insert(key.clone(), sheet.clone());
            } else {
                layouts.insert(key.clone(), sheet.clone());
            }
            si += 1;
        }
        if !snap.borrow().sheetKeys.is_empty() {
            self.doc.borrow_mut().masters = masters.clone();
            self.doc.borrow_mut().layouts = layouts.clone();
        }
        let mut sections: Vec<Rc<RefCell<PptxSection>>> = Vec::new();
        let mut qi: i64 = 0;
        while qi < (snap.borrow().sections.len() as i64) {
            sections.push(PptxClone::section(&snap.borrow().sections[qi as usize].clone()));
            qi += 1;
        }
        self.doc.borrow_mut().sections = sections.clone();
        // Stay in the master you are in, the same way you stay on the slide you
        // are on: the snapshot remembers which sheet the step was taken in, and
        // that is the fallback rather than the answer.
        let mut want_sheet: String = self.sheetPart.clone();
        if want_sheet.is_empty() {
            want_sheet = snap.borrow().sheetPart.clone();
        }
        self.sheetPart = String::new();
        if !want_sheet.is_empty() {
            self.editSheet(want_sheet.clone());
        }
    }
    /// The top of the undo side is the state the deck is IN, so there has to be
    /// one below it to go back to.
    pub fn canUndo(&self) -> bool {
        return OfficeHistory::depth(&self.history) > 1;
    }
    pub fn canRedo(&self) -> bool {
        return OfficeHistory::canRedo(&self.history);
    }
    pub fn undo(&mut self) -> bool {
        if !self.canUndo() {
            return false;
        }
        let mut cur: Rc<RefCell<PptxEditSnapshot>> = OfficeHistory::popUndo(&self.history);
        OfficeHistory::pushRedo(&self.history.clone(), cur.clone());
        let mut snap: Rc<RefCell<PptxEditSnapshot>> = OfficeHistory::peekUndo(&self.history);
        self.restoreSnapshot(&snap);
        self.dirty = true;
        true
    }
    /// The same function with the direction turned around.
    pub fn redo(&mut self) -> bool {
        if !self.canRedo() {
            return false;
        }
        let mut snap: Rc<RefCell<PptxEditSnapshot>> = OfficeHistory::popRedo(&self.history);
        OfficeHistory::pushUndo(&self.history.clone(), snap.clone());
        self.restoreSnapshot(&snap);
        self.dirty = true;
        true
    }
    /// Every step the deck can reach, forwards and back — the stack used to be
    /// one array with a cursor in it, and this is the same number.
    pub fn historyDepth(&self) -> i64 {
        return OfficeHistory::depth(&self.history) + OfficeHistory::redoDepth(&self.history);
    }
    /// --- transforms ----------------------------------------------------------
    pub fn applyMove(&self, dx: f64, dy: f64) {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let hoist_4 = sh.borrow().x + dx;
            sh.borrow_mut().x = hoist_4;
            let hoist_5 = sh.borrow().y + dy;
            sh.borrow_mut().y = hoist_5;
            sh.borrow_mut().hasXfrm = true;
            i += 1;
        }
    }
    pub fn moveSelection(&mut self, dx: f64, dy: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        self.applyMove(dx, dy);
        self.clearGuides();
        self.pushSnapshot("move");
        true
    }
    pub fn clearGuides(&mut self) {
        let mut gx: Vec<f64> = Vec::new();
        self.guideX = gx.clone();
        let mut gy: Vec<f64> = Vec::new();
        self.guideY = gy.clone();
    }
    /// Edges worth lining up against: the slide's own thirds — left, centre,
    /// right — and every edge and centre of the shapes that are staying put.
    pub fn snapLinesX(&self) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        out.push(0.0);
        let sw: f64 = self.slideWidth();
        out.push(sw * 0.5);
        out.push(sw);
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.editableShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if !self.isSelected(sh.borrow().editId.clone()) {
                out.push(sh.borrow().x);
                out.push(sh.borrow().x + sh.borrow().width * 0.5);
                out.push(sh.borrow().x + sh.borrow().width);
            }
            i += 1;
        }
        out
    }
    pub fn snapLinesY(&self) -> Vec<f64> {
        let mut out: Vec<f64> = Vec::new();
        out.push(0.0);
        let sh2: f64 = self.slideHeight();
        out.push(sh2 * 0.5);
        out.push(sh2);
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.editableShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if !self.isSelected(sh.borrow().editId.clone()) {
                out.push(sh.borrow().y);
                out.push(sh.borrow().y + sh.borrow().height * 0.5);
                out.push(sh.borrow().y + sh.borrow().height);
            }
            i += 1;
        }
        out
    }
    /// A drag that lines up with what is already on the slide. The delta the
    /// host asked for is nudged, not replaced — a shape never jumps to a guide
    /// it was not already next to.
    pub fn moveSelectionSnapped(&mut self, dx: f64, dy: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut adj: Rc<RefCell<PptxEditRect>> = self.snapDelta(dx, dy);
        let mut _tmp_6292 = adj.borrow().x.clone();
        self.applyMove(_tmp_6292, adj.borrow().y.clone());
        self.pushSnapshot("move");
        true
    }
    /// x / y of the returned rect are the adjusted delta; w / h are unused.
    /// Guides are left in `guideX` / `guideY` for the host to paint.
    pub fn snapDelta(&mut self, dx: f64, dy: f64) -> Rc<RefCell<PptxEditRect>> {
        self.clearGuides();
        if !self.snapEnabled {
            return PptxEditRect::of(dx, dy, 0.0, 0.0);
        }
        let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
        if b.borrow().w <= 0.0 {
            return PptxEditRect::of(dx, dy, 0.0, 0.0);
        }
        let mut ex: Vec<f64> = Vec::new();
        ex.push(b.borrow().x + dx);
        ex.push(b.borrow().midX() + dx);
        ex.push(b.borrow().right() + dx);
        let mut ey: Vec<f64> = Vec::new();
        ey.push(b.borrow().y + dy);
        ey.push(b.borrow().midY() + dy);
        ey.push(b.borrow().bottom() + dy);
        let mut lines_x: Vec<f64> = self.snapLinesX();
        let mut lines_y: Vec<f64> = self.snapLinesY();
        // With the grid on, the grid line nearest each moving edge is a
        // candidate like any other — so a shape lands on the grid when nothing
        // better is near, and on its neighbour's edge when one is.
        if self.gridEnabled {
            let mut gi: i64 = 0;
            while gi < (ex.len() as i64) {
                lines_x.push(PptxEditor::snapToStep(ex[gi as usize], self.gridPt));
                gi += 1;
            }
            let mut gj: i64 = 0;
            while gj < (ey.len() as i64) {
                lines_y.push(PptxEditor::snapToStep(ey[gj as usize], self.gridPt));
                gj += 1;
            }
        }
        let ax: f64 = PptxEditor::nearestSnap(&ex, &lines_x, self.snapTolPt.clone());
        let ay: f64 = PptxEditor::nearestSnap(&ey, &lines_y, self.snapTolPt.clone());
        if (ax.abs()) > 0.0001 {
            let mut _tmp_6297 = PptxEditor::matchedLine(&ex, &lines_x, self.snapTolPt.clone());
            self.guideX.push(_tmp_6297);
        }
        if (ay.abs()) > 0.0001 {
            let mut _tmp_6299 = PptxEditor::matchedLine(&ey, &lines_y, self.snapTolPt.clone());
            self.guideY.push(_tmp_6299);
        }
        PptxEditRect::of(dx + ax, dy + ay, 0.0, 0.0)
    }
    /// The same, for a handle rather than the whole shape.
    ///
    /// Moving a shape snaps all six of its edges and centres to whatever is
    /// near; resizing snaps only the edges the HANDLE actually moves — the
    /// right-hand handle cannot line the left edge up with anything, and
    /// offering to is how a resize ends up dragging the far side of the box
    /// about. So the candidates here are the one or two edges under the hand.
    ///
    /// Until this existed a resize snapped to nothing at all and drew no guide:
    /// a reader could line two boxes up by dragging them and then lose it again
    /// the moment they made one of them wider, with nothing on screen to say
    /// how far out they were.
    ///
    /// `dx` and `dy` are on the SLIDE's axes, which is the only frame the snap
    /// lines are in. A rotated box is resized in its own frame instead, so its
    /// caller does not ask — see where this is called from.
    ///
    /// Handles, clockwise from the top-left: 0 nw, 1 n, 2 ne, 3 e, 4 se, 5 s,
    /// 6 sw, 7 w — so 0, 6 and 7 move the left edge, 2, 3 and 4 the right, and
    /// 1 and 5 neither. The same reading down: 0, 1 and 2 move the top, 4, 5
    /// and 6 the bottom.
    pub fn snapResize(&mut self, handle: i64, dx: f64, dy: f64) -> Rc<RefCell<PptxEditRect>> {
        self.clearGuides();
        if !self.snapEnabled {
            return PptxEditRect::of(dx, dy, 0.0, 0.0);
        }
        let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
        if b.borrow().w <= 0.0 {
            return PptxEditRect::of(dx, dy, 0.0, 0.0);
        }
        let mut ex: Vec<f64> = Vec::new();
        if (handle == 0 || handle == 6) || handle == 7 {
            ex.push(b.borrow().x + dx);
        }
        if (handle == 2 || handle == 3) || handle == 4 {
            ex.push(b.borrow().right() + dx);
        }
        let mut ey: Vec<f64> = Vec::new();
        if (handle == 0 || handle == 1) || handle == 2 {
            ey.push(b.borrow().y + dy);
        }
        if (handle == 4 || handle == 5) || handle == 6 {
            ey.push(b.borrow().bottom() + dy);
        }
        let mut lines_x: Vec<f64> = self.snapLinesX();
        let mut lines_y: Vec<f64> = self.snapLinesY();
        if self.gridEnabled {
            let mut gi: i64 = 0;
            while gi < (ex.len() as i64) {
                lines_x.push(PptxEditor::snapToStep(ex[gi as usize], self.gridPt));
                gi += 1;
            }
            let mut gj: i64 = 0;
            while gj < (ey.len() as i64) {
                lines_y.push(PptxEditor::snapToStep(ey[gj as usize], self.gridPt));
                gj += 1;
            }
        }
        let mut ax: f64 = 0.0;
        let mut ay: f64 = 0.0;
        if !ex.is_empty() {
            ax = PptxEditor::nearestSnap(&ex, &lines_x, self.snapTolPt);
        }
        if !ey.is_empty() {
            ay = PptxEditor::nearestSnap(&ey, &lines_y, self.snapTolPt);
        }
        if (ax.abs()) > 0.0001 {
            let mut _tmp_6301 = PptxEditor::matchedLine(&ex, &lines_x, self.snapTolPt.clone());
            self.guideX.push(_tmp_6301);
        }
        if (ay.abs()) > 0.0001 {
            let mut _tmp_6303 = PptxEditor::matchedLine(&ey, &lines_y, self.snapTolPt.clone());
            self.guideY.push(_tmp_6303);
        }
        PptxEditRect::of(dx + ax, dy + ay, 0.0, 0.0)
    }
    /// Handles, clockwise from the top-left: 0 nw, 1 n, 2 ne, 3 e, 4 se, 5 s,
    /// 6 sw, 7 w. A multiple selection scales inside its own box, so the shapes
    /// keep their arrangement.
    pub fn resizeSelection(&mut self, handle: i64, dx: f64, dy: f64) -> bool {
        let ok: bool = self.applyResize(handle, dx, dy);
        if !ok {
            return false;
        }
        self.pushSnapshot("resize");
        true
    }
    /// The same arithmetic without a history step, for a drag in progress: a
    /// pointer that moves thirty times between press and release is one edit.
    pub fn applyResize(&self, handle: i64, dx: f64, dy: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
        if b.borrow().w <= 0.0 {
            return false;
        }
        if b.borrow().h <= 0.0 {
            return false;
        }
        let mut nx: f64 = b.borrow().x;
        let mut ny: f64 = b.borrow().y;
        let mut nw: f64 = b.borrow().w;
        let mut nh: f64 = b.borrow().h;
        let mut west: bool = false;
        let mut east: bool = false;
        let mut north: bool = false;
        let mut south: bool = false;
        if handle == 0 {
            west = true;
            north = true;
        }
        if handle == 1 {
            north = true;
        }
        if handle == 2 {
            east = true;
            north = true;
        }
        if handle == 3 {
            east = true;
        }
        if handle == 4 {
            east = true;
            south = true;
        }
        if handle == 5 {
            south = true;
        }
        if handle == 6 {
            west = true;
            south = true;
        }
        if handle == 7 {
            west = true;
        }
        if west {
            nx = b.borrow().x + dx;
            nw = b.borrow().w - dx;
        }
        if east {
            nw = b.borrow().w + dx;
        }
        if north {
            ny = b.borrow().y + dy;
            nh = b.borrow().h - dy;
        }
        if south {
            nh = b.borrow().h + dy;
        }
        if nw < self.minSizePt {
            if west {
                nx = (b.borrow().x + b.borrow().w) - self.minSizePt;
            }
            nw = self.minSizePt;
        }
        if nh < self.minSizePt {
            if north {
                ny = (b.borrow().y + b.borrow().h) - self.minSizePt;
            }
            nh = self.minSizePt;
        }
        let sx: f64 = nw / b.borrow().w;
        let sy: f64 = nh / b.borrow().h;
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let rel_x: f64 = sh.borrow().x - b.borrow().x;
            let rel_y: f64 = sh.borrow().y - b.borrow().y;
            sh.borrow_mut().x = nx + rel_x * sx;
            sh.borrow_mut().y = ny + rel_y * sy;
            let hoist_6 = sh.borrow().width * sx;
            sh.borrow_mut().width = hoist_6;
            let hoist_7 = sh.borrow().height * sy;
            sh.borrow_mut().height = hoist_7;
            sh.borrow_mut().hasXfrm = true;
            i += 1;
        }
        true
    }
    /// Where every shape was when the turn started. A drag reports the pointer's
    /// ABSOLUTE angle every frame, so each frame puts the shapes back and turns
    /// them once from there; adding a delta to what the last frame produced
    /// would compound rounding all the way round the circle.
    pub fn beginRotate(&mut self) {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        let mut sx: Vec<f64> = Vec::new();
        let mut sy: Vec<f64> = Vec::new();
        let mut sr: Vec<f64> = Vec::new();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sx.push(sh.borrow().x);
            sy.push(sh.borrow().y);
            sr.push(sh.borrow().rotation);
            i += 1;
        }
        self.rotStartX = sx.clone();
        self.rotStartY = sy.clone();
        self.rotStartDeg = sr.clone();
        let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
        self.rotPivotX = b.borrow().midX();
        self.rotPivotY = b.borrow().midY();
        self.rotBox = b.clone();
        self.rotDeg = 0.0;
        self.rotActive = true;
    }
    pub fn endRotate(&mut self) {
        self.rotActive = false;
        self.rotDeg = 0.0;
    }
    pub fn applyRotate(&mut self, deg: f64) {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        // One shape turns about itself, and the handle points where the
        // pointer does — the angle IS the shape's angle. With more than one
        // there is no such shape, so the selection as a whole takes the angle
        // and everything in it comes along.
        if (shapes.len() as i64) < 2 {
            let mut i: i64 = 0;
            while i < (shapes.len() as i64) {
                let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
                sh.borrow_mut().rotation = PptxEditor::normalizeDeg(deg);
                i += 1;
            }
            return;
        }
        if !self.rotActive {
            self.beginRotate();
        }
        if (self.rotStartX.len() as i64) != (shapes.len() as i64) {
            self.beginRotate();
        }
        let mut r: i64 = 0;
        while r < (shapes.len() as i64) {
            let mut sh2: Rc<RefCell<PptxShape>> = shapes[r as usize].clone();
            sh2.borrow_mut().x = self.rotStartX[r as usize];
            sh2.borrow_mut().y = self.rotStartY[r as usize];
            sh2.borrow_mut().rotation = self.rotStartDeg[r as usize];
            r += 1;
        }
        self.rotDeg = deg;
        let mut _tmp_6309 = self.rotPivotX.clone();
        PptxEditor::spinAbout(&shapes, _tmp_6309, self.rotPivotY.clone(), deg);
    }
    pub fn rotateSelection(&mut self, deg: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        if (shapes.len() as i64) > 1 {
            let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
            PptxEditor::spinAbout(&shapes, b.borrow().midX(), b.borrow().midY(), deg);
        } else {
            let mut i: i64 = 0;
            while i < (shapes.len() as i64) {
                let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
                let hoist_8 = PptxEditor::normalizeDeg(sh.borrow().rotation + deg);
                sh.borrow_mut().rotation = hoist_8;
                i += 1;
            }
        }
        self.pushSnapshot("rotate");
        true
    }
    pub fn setSelectionRotation(&mut self, deg: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let norm: f64 = PptxEditor::normalizeDeg(deg);
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sh.borrow_mut().rotation = norm;
            i += 1;
        }
        self.pushSnapshot("rotate");
        true
    }
    /// --- the shape list ------------------------------------------------------
    pub fn replaceShapes(&self, list: Vec<Rc<RefCell<PptxShape>>>) {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        s.borrow_mut().shapes = list.clone();
    }
    /// Master and layout chrome is always at the front of a resolved slide, so
    /// the authored shapes are the tail — and the tail is the only part whose
    /// order a user owns.
    pub fn firstAuthoredIndex(&self) -> i64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return 0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            if !sh.borrow().inherited {
                return i;
            }
            i += 1;
        }
        return s.borrow().shapes.len() as i64;
    }
    pub fn addShapeAt(&mut self, preset: String, x: f64, y: f64, w: f64, h: f64) -> i64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return 0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().editId = self.nextId;
        self.nextId += 1;
        let hoist_9 = format!("Shape {}", sh.borrow().editId);
        sh.borrow_mut().name = hoist_9;
        sh.borrow_mut().preset = preset.clone();
        sh.borrow_mut().x = x;
        sh.borrow_mut().y = y;
        sh.borrow_mut().width = w;
        sh.borrow_mut().height = h;
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().fill = PptxColor::srgbOf("4472C4".to_string());
        // A label on a shape sits in the MIDDLE of it, both ways. The empty
        // paragraph is put here rather than when the first character arrives,
        // because by then there is nothing left to say what the shape wanted:
        // text typed into a shape used to land against its top-left corner,
        // which no editor does and nobody means.
        sh.borrow_mut().text.borrow_mut().anchor = "ctr".to_string();
        sh.borrow_mut().text.borrow_mut().anchorSet = true;
        let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
        para.borrow_mut().align = "ctr".to_string();
        para.borrow_mut().alignSet = true;
        sh.borrow_mut().text.borrow_mut().paragraphs.push(para.clone());
        s.borrow_mut().shapes.push(sh.clone());
        self.select(sh.borrow().editId.clone());
        self.pushSnapshot("insert shape");
        return sh.borrow().editId;
    }
    /// A shape whose geometry is an OUTLINE rather than a preset name.
    ///
    /// `pts` are in the shape's own unit square, which is what `pathUnit` holds
    /// and what the writer multiplies back up, and `ends` say where each contour
    /// stops — pass an empty list for a shape that is one ring.
    ///
    /// This is what an emoji is on a slide. A preset name would be simpler and
    /// there is no preset for a panda.
    pub fn addPathShapeAt(&mut self, pts: &[f64], ends: &[i64], x: f64, y: f64, w: f64, h: f64) -> i64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return 0;
        }
        if (pts.len() as i64) < 6 {
            return 0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().editId = self.nextId;
        self.nextId += 1;
        let hoist_10 = format!("Shape {}", sh.borrow().editId);
        sh.borrow_mut().name = hoist_10;
        sh.borrow_mut().preset = "rect".to_string();
        sh.borrow_mut().x = x;
        sh.borrow_mut().y = y;
        sh.borrow_mut().width = w;
        sh.borrow_mut().height = h;
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().hasPath = true;
        // Closed, because an outline encloses an area. An open path is a
        // stroke and the painter fills nothing for one.
        sh.borrow_mut().pathClosed = true;
        let mut i: i64 = 0;
        while i < (pts.len() as i64) {
            sh.borrow_mut().pathUnit.push(pts[i as usize]);
            i += 1;
        }
        let mut e: i64 = 0;
        while e < (ends.len() as i64) {
            sh.borrow_mut().pathEnds.push(ends[e as usize]);
            e += 1;
        }
        sh.borrow_mut().fill = PptxColor::srgbOf("4472C4".to_string());
        s.borrow_mut().shapes.push(sh.clone());
        self.select(sh.borrow().editId.clone());
        self.pushSnapshot("insert shape");
        return sh.borrow().editId;
    }
    pub fn addTextBox(&mut self, x: f64, y: f64, w: f64, h: f64, text: &str) -> i64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return 0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().editId = self.nextId;
        self.nextId += 1;
        let hoist_11 = format!("TextBox {}", sh.borrow().editId);
        sh.borrow_mut().name = hoist_11;
        sh.borrow_mut().preset = "rect".to_string();
        sh.borrow_mut().x = x;
        sh.borrow_mut().y = y;
        sh.borrow_mut().width = w;
        sh.borrow_mut().height = h;
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().hasText = true;
        PptxEditor::setBodyText(&sh.borrow().text.clone(), text);
        s.borrow_mut().shapes.push(sh.clone());
        self.select(sh.borrow().editId.clone());
        self.pushSnapshot("insert text");
        return sh.borrow().editId;
    }
    pub fn addPicture(&mut self, part: String, bytes: Vec<u8>, x: f64, y: f64, w: f64, h: f64) -> i64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return 0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::picture();
        sh.borrow_mut().editId = self.nextId;
        self.nextId += 1;
        let hoist_12 = format!("Picture {}", sh.borrow().editId);
        sh.borrow_mut().name = hoist_12;
        sh.borrow_mut().x = x;
        sh.borrow_mut().y = y;
        sh.borrow_mut().width = w;
        sh.borrow_mut().height = h;
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().imagePart = part.clone();
        sh.borrow_mut().imageBytes = bytes.clone();
        s.borrow_mut().shapes.push(sh.clone());
        self.select(sh.borrow().editId.clone());
        self.pushSnapshot("insert picture");
        return sh.borrow().editId;
    }
    pub fn deleteSelection(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut kept: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut removed: i64 = 0;
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            let mut drop: bool = false;
            if !sh.borrow().inherited {
                if self.isSelected(sh.borrow().editId.clone()) {
                    if !sh.borrow().locked {
                        drop = true;
                    }
                }
            }
            if drop {
                removed += 1;
            } else {
                kept.push(sh.clone());
            }
            i += 1;
        }
        if removed == 0 {
            return false;
        }
        s.borrow_mut().shapes = kept.clone();
        self.clearSelection();
        self.pushSnapshot("delete");
        true
    }
    pub fn duplicateSelection(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut copies: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut copy: Rc<RefCell<PptxShape>> = PptxClone::shape(&sh);
            let hoist_13 = copy.borrow().x + 10.0;
            copy.borrow_mut().x = hoist_13;
            let hoist_14 = copy.borrow().y + 10.0;
            copy.borrow_mut().y = hoist_14;
            self.assignIdTo(&copy);
            s.borrow_mut().shapes.push(copy.clone());
            copies.push(copy.clone());
            i += 1;
        }
        self.clearSelection();
        let mut j: i64 = 0;
        while j < (copies.len() as i64) {
            let mut c: Rc<RefCell<PptxShape>> = copies[j as usize].clone();
            self.selection.push(c.borrow().editId);
            j += 1;
        }
        self.pushSnapshot("duplicate");
        true
    }
    /// --- z-order -------------------------------------------------------------
    pub fn reorder(&mut self, mode: &str) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let base: i64 = self.firstAuthoredIndex();
        let mut head: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut tail: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            if i < base {
                head.push(sh.clone());
            } else {
                tail.push(sh.clone());
            }
            i += 1;
        }
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        if mode == "front" {
            let mut a: i64 = 0;
            while a < (tail.len() as i64) {
                let mut sh2: Rc<RefCell<PptxShape>> = tail[a as usize].clone();
                if !self.isSelected(sh2.borrow().editId.clone()) {
                    out.push(sh2.clone());
                }
                a += 1;
            }
            let mut b: i64 = 0;
            while b < (tail.len() as i64) {
                let mut sh3: Rc<RefCell<PptxShape>> = tail[b as usize].clone();
                if self.isSelected(sh3.borrow().editId.clone()) {
                    out.push(sh3.clone());
                }
                b += 1;
            }
        }
        if mode == "back" {
            let mut c: i64 = 0;
            while c < (tail.len() as i64) {
                let mut sh4: Rc<RefCell<PptxShape>> = tail[c as usize].clone();
                if self.isSelected(sh4.borrow().editId.clone()) {
                    out.push(sh4.clone());
                }
                c += 1;
            }
            let mut d: i64 = 0;
            while d < (tail.len() as i64) {
                let mut sh5: Rc<RefCell<PptxShape>> = tail[d as usize].clone();
                if !self.isSelected(sh5.borrow().editId.clone()) {
                    out.push(sh5.clone());
                }
                d += 1;
            }
        }
        if mode == "forward" {
            out = self.stepOrder(&tail, true);
        }
        if mode == "backward" {
            out = self.stepOrder(&tail, false);
        }
        if (out.len() as i64) != (tail.len() as i64) {
            return false;
        }
        let mut merged: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut h: i64 = 0;
        while h < (head.len() as i64) {
            merged.push(head[h as usize].clone());
            h += 1;
        }
        let mut t: i64 = 0;
        while t < (out.len() as i64) {
            merged.push(out[t as usize].clone());
            t += 1;
        }
        s.borrow_mut().shapes = merged.clone();
        self.pushSnapshot(&format!("order {}", mode));
        true
    }
    /// One place up or down the pile, per selected shape, without letting two
    /// selected shapes swap past each other.
    pub fn stepOrder(&self, tail: &Vec<Rc<RefCell<PptxShape>>>, up: bool) -> Vec<Rc<RefCell<PptxShape>>> {
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (tail.len() as i64) {
            out.push(tail[i as usize].clone());
            i += 1;
        }
        let n: i64 = out.len() as i64;
        if up {
            let mut k: i64 = n - 2;
            while k >= 0 {
                let mut cur: Rc<RefCell<PptxShape>> = out[k as usize].clone();
                let mut above: Rc<RefCell<PptxShape>> = out[(k + 1) as usize].clone();
                if self.isSelected(cur.borrow().editId.clone()) {
                    if !self.isSelected(above.borrow().editId.clone()) {
                        out[k as usize] = above.clone();
                        out[(k + 1) as usize] = cur.clone();
                    }
                }
                k -= 1;
            }
        } else {
            let mut m: i64 = 1;
            while m < n {
                let mut cur2: Rc<RefCell<PptxShape>> = out[m as usize].clone();
                let mut below: Rc<RefCell<PptxShape>> = out[(m - 1) as usize].clone();
                if self.isSelected(cur2.borrow().editId.clone()) {
                    if !self.isSelected(below.borrow().editId.clone()) {
                        out[m as usize] = below.clone();
                        out[(m - 1) as usize] = cur2.clone();
                    }
                }
                m += 1;
            }
        }
        out
    }
    pub fn bringToFront(&mut self) -> bool {
        self.reorder("front")
    }
    pub fn sendToBack(&mut self) -> bool {
        self.reorder("back")
    }
    pub fn bringForward(&mut self) -> bool {
        self.reorder("forward")
    }
    pub fn sendBackward(&mut self) -> bool {
        self.reorder("backward")
    }
    /// --- group / ungroup -----------------------------------------------------
    /// A group's children keep slide coordinates and the group's child box is
    /// its own bounds, so the viewer's chOff / chExt mapping is the identity —
    /// grouping moves nothing on screen, which is the whole contract.
    pub fn groupSelection(&mut self) -> bool {
        if (self.selection.len() as i64) < 2 {
            return false;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
        let mut grp: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        grp.borrow_mut().kind = PptxShapeKind::group();
        grp.borrow_mut().editId = self.nextId;
        self.nextId += 1;
        let hoist_15 = format!("Group {}", grp.borrow().editId);
        grp.borrow_mut().name = hoist_15;
        grp.borrow_mut().x = b.borrow().x;
        grp.borrow_mut().y = b.borrow().y;
        grp.borrow_mut().width = b.borrow().w;
        grp.borrow_mut().height = b.borrow().h;
        grp.borrow_mut().hasXfrm = true;
        grp.borrow_mut().chX = b.borrow().x;
        grp.borrow_mut().chY = b.borrow().y;
        grp.borrow_mut().chW = b.borrow().w;
        grp.borrow_mut().chH = b.borrow().h;
        let mut kept: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut taken: i64 = 0;
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            let mut r#move: bool = false;
            if !sh.borrow().inherited {
                if self.isSelected(sh.borrow().editId.clone()) {
                    r#move = true;
                }
            }
            if r#move {
                grp.borrow_mut().children.push(sh.clone());
                taken += 1;
            } else {
                kept.push(sh.clone());
            }
            i += 1;
        }
        if taken < 2 {
            return false;
        }
        kept.push(grp.clone());
        s.borrow_mut().shapes = kept.clone();
        self.select(grp.borrow().editId.clone());
        self.pushSnapshot("group");
        true
    }
    pub fn ungroupSelection(&mut self) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut freed: Vec<i64> = Vec::new();
        let mut opened: i64 = 0;
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            let mut open: bool = false;
            if sh.borrow().kind == PptxShapeKind::group() {
                if !sh.borrow().inherited {
                    if self.isSelected(sh.borrow().editId.clone()) {
                        open = true;
                    }
                }
            }
            if open {
                opened += 1;
                let mut sx: f64 = 1.0;
                let mut sy: f64 = 1.0;
                if sh.borrow().chW > 0.0 {
                    sx = sh.borrow().width / sh.borrow().chW;
                }
                if sh.borrow().chH > 0.0 {
                    sy = sh.borrow().height / sh.borrow().chH;
                }
                let mut c: i64 = 0;
                while c < (sh.borrow().children.len() as i64) {
                    let mut child: Rc<RefCell<PptxShape>> = sh.borrow().children[c as usize].clone();
                    let hoist_16 = sh.borrow().x + (child.borrow().x - sh.borrow().chX) * sx;
                    child.borrow_mut().x = hoist_16;
                    let hoist_17 = sh.borrow().y + (child.borrow().y - sh.borrow().chY) * sy;
                    child.borrow_mut().y = hoist_17;
                    let hoist_18 = child.borrow().width * sx;
                    child.borrow_mut().width = hoist_18;
                    let hoist_19 = child.borrow().height * sy;
                    child.borrow_mut().height = hoist_19;
                    let hoist_20 = PptxEditor::normalizeDeg(child.borrow().rotation + sh.borrow().rotation);
                    child.borrow_mut().rotation = hoist_20;
                    child.borrow_mut().hasXfrm = true;
                    out.push(child.clone());
                    freed.push(child.borrow().editId);
                    c += 1;
                }
            } else {
                out.push(sh.clone());
            }
            i += 1;
        }
        if opened == 0 {
            return false;
        }
        s.borrow_mut().shapes = out.clone();
        self.clearSelection();
        let mut j: i64 = 0;
        while j < (freed.len() as i64) {
            self.selection.push(freed[j as usize]);
            j += 1;
        }
        self.pushSnapshot("ungroup");
        true
    }
    /// --- align / distribute --------------------------------------------------
    /// One shape lines up against the slide; several line up against each other.
    /// PowerPoint does the same, and it is the only reading that makes "centre"
    /// mean something in both cases.
    pub fn alignSelection(&mut self, mode: &str) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        let mut b: Rc<RefCell<PptxEditRect>> = self.selectionBounds();
        if (shapes.len() as i64) < 2 {
            let mut _tmp_6356 = self.slideWidth();
            let _tmp_6358 = PptxEditRect::of(0.0, 0.0, _tmp_6356, self.slideHeight());
            b = _tmp_6358;
        }
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if mode == "left" {
                sh.borrow_mut().x = b.borrow().x;
            }
            if mode == "hcenter" {
                let hoist_21 = b.borrow().midX() - sh.borrow().width * 0.5;
                sh.borrow_mut().x = hoist_21;
            }
            if mode == "right" {
                let hoist_22 = b.borrow().right() - sh.borrow().width;
                sh.borrow_mut().x = hoist_22;
            }
            if mode == "top" {
                sh.borrow_mut().y = b.borrow().y;
            }
            if mode == "vcenter" {
                let hoist_23 = b.borrow().midY() - sh.borrow().height * 0.5;
                sh.borrow_mut().y = hoist_23;
            }
            if mode == "bottom" {
                let hoist_24 = b.borrow().bottom() - sh.borrow().height;
                sh.borrow_mut().y = hoist_24;
            }
            sh.borrow_mut().hasXfrm = true;
            i += 1;
        }
        self.pushSnapshot(&format!("align {}", mode));
        true
    }
    /// Equal gaps between the outermost two, which is what "distribute" means
    /// once more than two shapes are involved.
    pub fn distributeSelection(&mut self, horizontal: bool) -> bool {
        if (self.selection.len() as i64) < 3 {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let n: i64 = shapes.len() as i64;
        let mut order: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < n {
            order.push(shapes[i as usize].clone());
            i += 1;
        }
        let mut a: i64 = 0;
        while a < n {
            let mut b2: i64 = a + 1;
            while b2 < n {
                let mut sa: Rc<RefCell<PptxShape>> = order[a as usize].clone();
                let mut sb: Rc<RefCell<PptxShape>> = order[b2 as usize].clone();
                let mut ka: f64 = sa.borrow().x;
                let mut kb: f64 = sb.borrow().x;
                if !horizontal {
                    ka = sa.borrow().y;
                    kb = sb.borrow().y;
                }
                if kb < ka {
                    order[a as usize] = sb.clone();
                    order[b2 as usize] = sa.clone();
                }
                b2 += 1;
            }
            a += 1;
        }
        let mut first_shape: Rc<RefCell<PptxShape>> = order[0].clone();
        let mut last_shape: Rc<RefCell<PptxShape>> = order[(n - 1) as usize].clone();
        if horizontal {
            let span: f64 = (last_shape.borrow().x - first_shape.borrow().x) / ((n - 1) as f64);
            let mut k: i64 = 1;
            while k < n - 1 {
                let mut sh: Rc<RefCell<PptxShape>> = order[k as usize].clone();
                sh.borrow_mut().x = first_shape.borrow().x + span * (k as f64);
                sh.borrow_mut().hasXfrm = true;
                k += 1;
            }
        } else {
            let span_y: f64 = (last_shape.borrow().y - first_shape.borrow().y) / ((n - 1) as f64);
            let mut m: i64 = 1;
            while m < n - 1 {
                let mut sh2: Rc<RefCell<PptxShape>> = order[m as usize].clone();
                sh2.borrow_mut().y = first_shape.borrow().y + span_y * (m as f64);
                sh2.borrow_mut().hasXfrm = true;
                m += 1;
            }
        }
        self.pushSnapshot("distribute");
        true
    }
    /// --- style ---------------------------------------------------------------
    pub fn setFillHex(&mut self, hex: &str) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sh.borrow_mut().fill = PptxColor::srgbOf(hex.to_string());
            sh.borrow_mut().fillGrad.borrow_mut().isSet = false;
            i += 1;
        }
        self.pushSnapshot("fill");
        true
    }
    /// --- geometry by the number ----------------------------------------------
    ///
    /// Dragging says "about here". A deck says 2.54 cm from the left, and the
    /// two are not the same instruction: a diagram whose boxes are meant to
    /// line up is lined up by typing the same number twice, not by nudging
    /// until it looks right. This is that half of the editor — the outline
    /// panel's inspector calls it, and so does anything else that knows what
    /// the number should be.
    ///
    /// `field` is "x", "y", "w", "h" or "rot", in slide points and degrees.
    /// Every selected shape takes it, which is what makes "put these four
    /// boxes at the same width" one gesture. Locked shapes are left alone, the
    /// way every other transform leaves them alone.
    pub fn setShapeBox(&mut self, field: &str, value: f64) -> bool {
        self.applyShapeBox(field, value, true)
    }
    /// The same, as a step from where the shape is now: the panel's arrows,
    /// and any host that would rather say "1pt to the right" than work out
    /// where that is.
    pub fn nudgeShapeBox(&mut self, field: &str, delta: f64) -> bool {
        self.applyShapeBox(field, delta, false)
    }
    pub fn applyShapeBox(&mut self, field: &str, value: f64, absolute: bool) -> bool {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        if shapes.is_empty() {
            return false;
        }
        let mut known: bool = false;
        if field == "x" {
            known = true;
        }
        if field == "y" {
            known = true;
        }
        if field == "w" {
            known = true;
        }
        if field == "h" {
            known = true;
        }
        if field == "rot" {
            known = true;
        }
        if !known {
            return false;
        }
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut now: f64 = 0.0;
            if field == "x" {
                now = sh.borrow().x;
            }
            if field == "y" {
                now = sh.borrow().y;
            }
            if field == "w" {
                now = sh.borrow().width;
            }
            if field == "h" {
                now = sh.borrow().height;
            }
            if field == "rot" {
                now = sh.borrow().rotation;
            }
            let mut next: f64 = value;
            if !absolute {
                next = now + value;
            }
            if field == "x" {
                sh.borrow_mut().x = next;
            }
            if field == "y" {
                sh.borrow_mut().y = next;
            }
            if field == "w" {
                // A box has to stay a box. Zero width is exactly the state
                // this editor exists to get a deck OUT of — see
                // `PptxResolver.fallbackBox` — so it is not one the reader
                // can type their way back into.
                if next < self.minSizePt {
                    next = self.minSizePt;
                }
                sh.borrow_mut().width = next;
            }
            if field == "h" {
                if next < self.minSizePt {
                    next = self.minSizePt;
                }
                sh.borrow_mut().height = next;
            }
            if field == "rot" {
                sh.borrow_mut().rotation = next;
            }
            // Stated now, whatever it was before: a size typed into the panel
            // is the document's own and is written out as such, which is what
            // takes a shape off the reader's fallback box for good.
            sh.borrow_mut().hasXfrm = true;
            sh.borrow_mut().hasOff = true;
            sh.borrow_mut().hasExt = true;
            sh.borrow_mut().geomFallback = false;
            i += 1;
        }
        self.pushSnapshot("geometry");
        true
    }
    pub fn clearFill(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut none: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
            sh.borrow_mut().fill = none.clone();
            sh.borrow_mut().fillGrad.borrow_mut().isSet = false;
            i += 1;
        }
        self.pushSnapshot("fill none");
        true
    }
    pub fn setLineHex(&mut self, hex: &str, width_pt: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sh.borrow_mut().line = PptxColor::srgbOf(hex.to_string());
            sh.borrow_mut().lineWidthPt = width_pt;
            i += 1;
        }
        self.pushSnapshot("line");
        true
    }
    /// Take the outline away. `setLineHex` with an empty string would set the
    /// line to black, which is the opposite of what "None" means.
    pub fn clearLine(&mut self) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut none: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
            sh.borrow_mut().line = none.clone();
            sh.borrow_mut().lineWidthPt = 0.0;
            i += 1;
        }
        self.pushSnapshot("line none");
        true
    }
    /// The outline's thickness, leaving its colour alone — which is what a
    /// reader picking a width means, and what `setLineHex` could not express
    /// because it took both at once.
    pub fn setLineWidthPt(&mut self, width_pt: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        if width_pt <= 0.0 {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sh.borrow_mut().lineWidthPt = width_pt;
            // A width with no colour draws nothing, so a shape that never had
            // an outline gets the one a reader plainly just asked for. Black
            // is what every editor gives a new rule.
            if !sh.borrow().line.borrow().isSet {
                sh.borrow_mut().line = PptxColor::srgbOf("000000".to_string());
            }
            i += 1;
        }
        self.pushSnapshot("line width");
        true
    }
    pub fn setOpacity(&mut self, alpha: f64) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sh.borrow_mut().fill.borrow_mut().alpha = alpha;
            i += 1;
        }
        self.pushSnapshot("opacity");
        true
    }
    pub fn setPreset(&mut self, preset: String) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if sh.borrow().kind == PptxShapeKind::shape() {
                sh.borrow_mut().preset = preset.clone();
                sh.borrow_mut().hasPath = false;
            }
            i += 1;
        }
        self.pushSnapshot("preset");
        true
    }
    pub fn setShapeText(&mut self, id: i64, text: &str) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.findShape(id);
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        sh.borrow_mut().hasText = true;
        PptxEditor::setBodyText(&sh.borrow().text.clone(), text);
        self.relayoutDiagramOf(id);
        self.pushSnapshot("text");
        true
    }
    pub fn selectionText(&self) -> String {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.firstSelected();
        if hit.is_none() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if !sh.borrow().hasText {
            return String::new();
        }
        return (sh.borrow().text).borrow().plainText();
    }
    pub fn setTextBold(&mut self, bold: bool) -> bool {
        if self.styleTextRange("bold", bold, 0.0, "") {
            return true;
        }
        self.applyRunStyle("bold", bold, 0.0, String::new())
    }
    pub fn setTextItalic(&mut self, italic: bool) -> bool {
        if self.styleTextRange("italic", italic, 0.0, "") {
            return true;
        }
        self.applyRunStyle("italic", italic, 0.0, String::new())
    }
    pub fn setTextSizePt(&mut self, size: f64) -> bool {
        if self.styleTextRange("size", false, size, "") {
            return true;
        }
        self.applyRunStyle("size", false, size, String::new())
    }
    /// The typeface, for the caret's range or for everything in the selected
    /// shapes — the same two-step every other run style takes.
    pub fn setTextFamily(&mut self, family: &str) -> bool {
        if family.is_empty() {
            return false;
        }
        if self.styleTextRange("family", false, 0.0, family) {
            return true;
        }
        self.applyRunStyle("family", false, 0.0, family.to_string())
    }
    /// What typeface the selection is in, or "" when it is in more than one —
    /// which is what a chooser needs in order to show the current value rather
    /// than a guess. The caret's own run first, then the first run of the first
    /// selected shape.
    pub fn selectionFamily(&self) -> String {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
            if shapes.is_empty() {
                return String::new();
            }
            hit = Some(shapes[0].clone());
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if !sh.borrow().hasText {
            return String::new();
        }
        let mut seen: String = String::new();
        let mut pi: i64 = 0;
        while pi < (sh.borrow().text.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
                .text.borrow()
                .paragraphs[pi as usize].clone();
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                if !run.borrow().fontFamily.is_empty() {
                    if seen.is_empty() {
                        seen = run.borrow().fontFamily.clone();
                    } else {
                        if seen != run.borrow().fontFamily {
                            return String::new();
                        }
                    }
                }
                ri += 1;
            }
            pi += 1;
        }
        seen
    }
    /// What colour the selection's text is, or "" when it is in more than one —
    /// the same question `selectionFamily` answers for the typeface, and for
    /// the same reason: a swatch showing the current value has to have one.
    pub fn selectionColorHex(&self) -> String {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
            if shapes.is_empty() {
                return String::new();
            }
            hit = Some(shapes[0].clone());
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if !sh.borrow().hasText {
            return String::new();
        }
        let mut seen: String = String::new();
        let mut pi: i64 = 0;
        while pi < (sh.borrow().text.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
                .text.borrow()
                .paragraphs[pi as usize].clone();
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                if run.borrow().color.borrow().isSet {
                    if seen.is_empty() {
                        seen = run.borrow().color.borrow().srgb.clone();
                    } else {
                        if seen != run.borrow().color.borrow().srgb {
                            return String::new();
                        }
                    }
                }
                ri += 1;
            }
            pi += 1;
        }
        seen
    }
    pub fn setTextColorHex(&mut self, hex: &str) -> bool {
        if self.styleTextRange("color", false, 0.0, hex) {
            return true;
        }
        self.applyRunStyle("color", false, 0.0, hex.to_string())
    }
    pub fn applyRunStyle(&mut self, what: &str, flag: bool, size: f64, hex: String) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut touched: i64 = 0;
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if sh.borrow().hasText {
                touched += 1;
                let mut pi: i64 = 0;
                while pi < (sh.borrow().text.borrow().paragraphs.len() as i64) {
                    let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
                        .text.borrow()
                        .paragraphs[pi as usize].clone();
                    let mut ri: i64 = 0;
                    while ri < (para.borrow().runs.len() as i64) {
                        let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                        if what == "bold" {
                            run.borrow_mut().bold = flag;
                            run.borrow_mut().boldSet = true;
                        }
                        if what == "italic" {
                            run.borrow_mut().italic = flag;
                            run.borrow_mut().italicSet = true;
                        }
                        if what == "size" {
                            run.borrow_mut().fontSizePt = size;
                            run.borrow_mut().sizeSet = true;
                        }
                        if what == "color" {
                            run.borrow_mut().color = PptxColor::srgbOf(hex.clone());
                            run.borrow_mut().colorStated = true;
                        }
                        // The typeface travels in the same string slot the
                        // colour does — `applyStyle`, the version that works on
                        // a caret range rather than a whole shape, has read
                        // "family" out of it for some time and this one never
                        // did. So changing the font with a whole box selected
                        // and no caret in it did nothing at all.
                        if what == "family" {
                            run.borrow_mut().fontFamily = hex.clone();
                        }
                        ri += 1;
                    }
                    pi += 1;
                }
            }
            i += 1;
        }
        if touched == 0 {
            return false;
        }
        self.pushSnapshot(&format!("text {}", what));
        true
    }
    pub fn setTextAlign(&mut self, align: String) -> bool {
        if self.selection.is_empty() {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut touched: i64 = 0;
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if sh.borrow().hasText {
                touched += 1;
                let mut pi: i64 = 0;
                while pi < (sh.borrow().text.borrow().paragraphs.len() as i64) {
                    let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
                        .text.borrow()
                        .paragraphs[pi as usize].clone();
                    para.borrow_mut().align = align.clone();
                    para.borrow_mut().alignSet = true;
                    pi += 1;
                }
            }
            i += 1;
        }
        if touched == 0 {
            return false;
        }
        self.pushSnapshot("align text");
        true
    }
    /// --- slides --------------------------------------------------------------
    pub fn addSlide(&mut self) -> bool {
        let mut slide: Rc<RefCell<PptxSlide>> = Rc::new(RefCell::new(PptxSlide::new()));
        slide.borrow_mut().dirty = true;
        self.freshKey(&slide);
        slide.borrow_mut().width = self.doc.borrow().slideWidth;
        slide.borrow_mut().height = self.doc.borrow().slideHeight;
        slide.borrow_mut().bg = PptxColor::srgbOf("FFFFFF".to_string());
        let mut out: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        let at: i64 = self.slideIndex + 1;
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            if i == at {
                out.push(slide.clone());
            }
            out.push(self.doc.borrow().slides[i as usize].clone());
            i += 1;
        }
        if at >= (self.doc.borrow().slides.len() as i64) {
            out.push(slide.clone());
        }
        self.doc.borrow_mut().slides = out.clone();
        self.slideIndex = at;
        self.clearSelection();
        self.pushSnapshot("new slide");
        true
    }
    pub fn duplicateSlide(&mut self) -> bool {
        let mut cur: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if cur.is_none() {
            return false;
        }
        let mut src: Rc<RefCell<PptxSlide>> = cur.clone().unwrap();
        let mut copy: Rc<RefCell<PptxSlide>> = PptxClone::slide(&src);
        // A copy is a new slide, not the one it was copied from: it cannot
        // claim the part its original lives in, and it cannot keep its key
        // either — two slides answering to the same name is a history that
        // shares the wrong copy and a panel that draws the wrong thumbnail.
        copy.borrow_mut().partPath = String::new();
        copy.borrow_mut().dirty = true;
        self.freshKey(&copy);
        copy.borrow_mut().revision = 0;
        let mut j: i64 = 0;
        while j < (copy.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = copy.borrow().shapes[j as usize].clone();
            self.assignIdTo(&sh);
            j += 1;
        }
        let mut out: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            out.push(self.doc.borrow().slides[i as usize].clone());
            if i == self.slideIndex {
                out.push(copy.clone());
            }
            i += 1;
        }
        self.doc.borrow_mut().slides = out.clone();
        self.slideIndex += 1;
        self.clearSelection();
        self.pushSnapshot("duplicate slide");
        true
    }
    pub fn deleteSlide(&mut self) -> bool {
        if (self.doc.borrow().slides.len() as i64) <= 1 {
            return false;
        }
        let mut out: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            if i != self.slideIndex {
                out.push(self.doc.borrow().slides[i as usize].clone());
            }
            i += 1;
        }
        self.doc.borrow_mut().slides = out.clone();
        if self.slideIndex >= (self.doc.borrow().slides.len() as i64) {
            self.slideIndex = (self.doc.borrow().slides.len() as i64) - 1;
        }
        self.clearSelection();
        self.pushSnapshot("delete slide");
        true
    }
    /// Straight to an index, which is what a drag in the slide panel means.
    pub fn moveSlideTo(&mut self, target: i64) -> bool {
        let n: i64 = self.doc.borrow().slides.len() as i64;
        if target < 0 {
            return false;
        }
        if target >= n {
            return false;
        }
        if target == self.slideIndex {
            return false;
        }
        let mut moved: Rc<RefCell<PptxSlide>> = self.doc.borrow()
            .slides[(self.slideIndex) as usize].clone();
        let mut out: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        let mut i: i64 = 0;
        while i < n {
            if i != self.slideIndex {
                let at: i64 = out.len() as i64;
                if at == target {
                    out.push(moved.clone());
                }
                out.push(self.doc.borrow().slides[i as usize].clone());
            }
            i += 1;
        }
        if (out.len() as i64) < n {
            out.push(moved.clone());
        }
        self.doc.borrow_mut().slides = out.clone();
        let mut k: i64 = 0;
        while k < (self.doc.borrow().slides.len() as i64) {
            if Rc::ptr_eq(&self.doc.borrow().slides[k as usize], &moved) {
                self.slideIndex = k;
            }
            k += 1;
        }
        self.pushSnapshot("move slide");
        true
    }
    pub fn moveSlide(&mut self, delta: i64) -> bool {
        let target: i64 = self.slideIndex + delta;
        if target < 0 {
            return false;
        }
        if target >= (self.doc.borrow().slides.len() as i64) {
            return false;
        }
        let mut moved: Rc<RefCell<PptxSlide>> = self.doc.borrow()
            .slides[(self.slideIndex) as usize].clone();
        let mut other: Rc<RefCell<PptxSlide>> = self.doc.borrow()
            .slides[target as usize].clone();
        self.doc.borrow_mut().slides[target as usize] = moved.clone();
        self.doc.borrow_mut()
            .slides[(self.slideIndex) as usize] = other.clone();
        self.slideIndex = target;
        self.pushSnapshot("move slide");
        true
    }
    /// --- typing --------------------------------------------------------------
    /// Keystrokes are one edit, not one edit each: a step whose label matches
    /// the one on top of the stack replaces it instead of pushing.
    pub fn pushCoalesced(&mut self, label: &str) {
        if OfficeHistory::depth(&self.history) > 0 {
            let mut top: Rc<RefCell<PptxEditSnapshot>> = OfficeHistory::peekUndo(&self.history);
            if top.borrow().label == label {
                // A coalesced step replaces the one on top rather than
                // adding another, and it still changed a slide — so the
                // slide is marked before the capture, exactly as it is for
                // a step of its own.
                let mut cur: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
                if cur.is_some() {
                    self.touchSlide(&cur.clone().unwrap());
                }
                let mut snap: Rc<RefCell<PptxEditSnapshot>> = self.captureSnapshot(label.to_string());
                OfficeHistory::replaceTop(&self.history.clone(), &snap);
                self.dirty = true;
                return;
            }
        }
        self.pushSnapshot(label);
    }
    /// --- typing into a SELECTED shape, with no caret in it yet ----------------
    ///
    /// These two edit the end of the shape's text, and they used to do it by
    /// taking the whole body out as one plain string, changing that string, and
    /// building the body again from it. Which flattened the shape:
    /// `setBodyText` gives every paragraph ONE run in the style of the FIRST
    /// one, so a box holding an orange link and white prose came back entirely
    /// orange, a bold word came back plain, and the empty paragraphs between
    /// items were gone — on the FIRST backspace, before a single character had
    /// been typed.
    ///
    /// The caret machinery next door has always inserted and deleted by run.
    /// So these do the same thing, at the end of the text rather than at a
    /// caret: `insertAt` and `deleteRange` split and rejoin runs and leave every
    /// other run, and every paragraph's own bullet, level and spacing, alone.
    ///
    /// `setBodyText` is still right where it is used — a new shape's text, a
    /// whole body replaced on purpose — because there the flattening IS the
    /// intent. It is only wrong as a way of editing what is already there.
    pub fn appendText(&mut self, s: &str) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.firstSelected();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        sh.borrow_mut().hasText = true;
        let mut end: Rc<RefCell<PptxTextPos>> = PptxTextEdit::docEnd(&sh.borrow().text.clone());
        PptxTextEdit::insertAt(&sh.borrow().text.clone(), &end, s);
        self.pushCoalesced("typing");
        true
    }
    pub fn backspaceText(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.firstSelected();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if !sh.borrow().hasText {
            return false;
        }
        let mut end: Rc<RefCell<PptxTextPos>> = PptxTextEdit::docEnd(&sh.borrow().text.clone());
        // One character back, not one string unit back: `moveLeft` steps over a
        // surrogate pair, and chopping the last unit off a plain string cut an
        // emoji in half.
        let mut from: Rc<RefCell<PptxTextPos>> = PptxTextEdit::moveLeft(&sh.borrow().text.clone(), &end);
        if from.borrow().isSame(&end) {
            return false;
        }
        PptxTextEdit::deleteRange(&sh.borrow().text.clone(), &from, &end);
        self.pushCoalesced("typing");
        true
    }
    /// --- the caret ------------------------------------------------------------
    pub fn isEditingText(&self) -> bool {
        self.editingId > 0
    }
    pub fn editingShape(&self) -> Option<Rc<RefCell<PptxShape>>> {
        if self.editingId <= 0 {
            let mut __none: Option<Rc<RefCell<PptxShape>>> = None;
            return __none;
        }
        self.findShape(self.editingId.clone())
    }
    pub fn beginTextEdit(&mut self, id: i64) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.findShape(id);
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if sh.borrow().kind == PptxShapeKind::picture() {
            return false;
        }
        if sh.borrow().kind == PptxShapeKind::group() {
            return false;
        }
        // A table and a chart carry their own content. Setting `hasText` on
        // one would give it a second, empty text body that the writer would
        // then have to explain — and a click can now reach this, where before
        // only a deliberate F2 could.
        if sh.borrow().kind == PptxShapeKind::table() {
            return false;
        }
        if sh.borrow().kind == PptxShapeKind::chart() {
            return false;
        }
        sh.borrow_mut().hasText = true;
        if sh.borrow().text.borrow().paragraphs.is_empty() {
            let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            para.borrow_mut().align = sh.borrow().text.borrow().defAlign.clone();
            sh.borrow_mut().text.borrow_mut().paragraphs.push(para.clone());
        }
        // Selecting clears the caret — picking a different shape stops typing
        // into the last one — so the selection is made FIRST and the caret set
        // after it.
        self.select(id);
        self.editingId = id;
        let mut end: Rc<RefCell<PptxTextPos>> = PptxTextEdit::docEnd(&sh.borrow().text.clone());
        self.caret = end.clone();
        self.anchor = end.borrow().copy();
        true
    }
    /// The end of an edit: a shape inside a SmartArt diagram that can be laid
    /// out again (SaEdit), whose text changed, has that text written back to
    /// the diagram's data and the diagram is laid out anew. The new layout is
    /// part of the edit that caused it: it replaces that step, so one undo
    /// takes back the text and the layout together.
    pub fn endTextEdit(&mut self) {
        let was: i64 = self.editingId;
        self.editingId = 0;
        if was <= 0 {
            return;
        }
        let mut dh: Option<Rc<RefCell<PptxShape>>> = self.diagramHolding(was);
        if dh.is_none() {
            return;
        }
        let mut d: Rc<RefCell<PptxShape>> = dh.clone().unwrap();
        if d.borrow().diagramSig == d.borrow().diagramSignature() {
            return;
        }
        if self.relayoutDiagramOf(was) {
            let mut label: String = "diagram".to_string();
            if OfficeHistory::depth(&self.history) > 0 {
                let mut top: Rc<RefCell<PptxEditSnapshot>> = OfficeHistory::peekUndo(&self.history);
                label = top.borrow().label.clone();
            }
            self.pushCoalesced(&label);
        }
    }
    /// The diagram on this slide that holds shape `id`, not being it.
    pub fn diagramHolding(&self, id: i64) -> Option<Rc<RefCell<PptxShape>>> {
        let mut none: Option<Rc<RefCell<PptxShape>>> = None;
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return none;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            if sh.borrow().isDiagram && sh.borrow().editId != id {
                let mut inside: Option<Rc<RefCell<PptxShape>>> = PptxEditor::findIn(&sh, id);
                if inside.is_some() {
                    let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(sh.clone());
                    return hit;
                }
            }
            i += 1;
        }
        none
    }
    /// The diagram holding shape `id`, laid out again from its data with the
    /// text `id` now has. True when it was; false — and nothing changed — for
    /// a shape in no diagram, or in one that holds no data to lay it out from.
    pub fn relayoutDiagramOf(&mut self, id: i64) -> bool {
        let mut dh: Option<Rc<RefCell<PptxShape>>> = self.diagramHolding(id);
        if dh.is_none() {
            return false;
        }
        let mut d: Rc<RefCell<PptxShape>> = dh.clone().unwrap();
        if !SaEdit::canRelayout(&d) {
            return false;
        }
        let mut eh: Option<Rc<RefCell<PptxShape>>> = PptxEditor::findIn(&d, id);
        if eh.is_none() {
            return false;
        }
        let mut r: Rc<RefCell<SaResult>> = SaEdit::relayout(&d, &eh.clone().unwrap());
        if !r.borrow().ok {
            return false;
        }
        let mut g: Rc<RefCell<PptxShape>> = r.borrow().shape.clone().clone();
        let mut _tmp_6392 = self.doc.borrow().theme.clone();
        self.resolver.clone().borrow().resolveShape(&g, &_tmp_6392);
        let mut kids: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (g.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<PptxShape>> = g.borrow().children[i as usize].clone();
            self.assignIdTo(&c);
            kids.push(c.clone());
            i += 1;
        }
        d.borrow_mut().children = kids.clone();
        d.borrow_mut().chX = 0.0;
        d.borrow_mut().chY = 0.0;
        let hoist_25 = d.borrow().width;
        d.borrow_mut().chW = hoist_25;
        let hoist_26 = d.borrow().height;
        d.borrow_mut().chH = hoist_26;
        d.borrow_mut().diagramModelXml = g.borrow().diagramModelXml.clone();
        d.borrow_mut().diagramText = g.borrow().diagramText.clone();
        let hoist_27 = d.borrow().diagramSignature();
        d.borrow_mut().diagramSig = hoist_27;
        // PowerPoint's drawing of it no longer says what it shows
        d.borrow_mut().diagramHadDrawing = false;
        true
    }
    pub fn textSelectionEmpty(&self) -> bool {
        return self.caret.borrow().isSame(&self.anchor.clone());
    }
    pub fn selectedText(&self) -> String {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut _tmp_6401 = sh.borrow().text.clone();
        let mut _tmp_6402 = self.anchor.clone();
        return PptxTextEdit::textInRange(&_tmp_6401, &_tmp_6402, &self.caret.clone());
    }
    pub fn caretText(&self) -> String {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        return (sh.borrow().text).borrow().plainText();
    }
    pub fn setCaretPos(&mut self, para: i64, col: i64, extend: bool) {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut pos: Rc<RefCell<PptxTextPos>> = PptxTextPos::of(para, col);
        PptxTextEdit::clamp(&sh.borrow().text.clone(), &pos);
        self.caret = pos.clone();
        if !extend {
            self.anchor = pos.borrow().copy();
        }
    }
    pub fn selectAllText(&mut self) {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let _tmp_6407 = PptxTextEdit::docStart(&sh.borrow().text.clone());
        self.anchor = _tmp_6407;
        let _tmp_6409 = PptxTextEdit::docEnd(&sh.borrow().text.clone());
        self.caret = _tmp_6409;
    }
    /// The word under the caret, selected — what a double-click takes.
    pub fn selectWord(&mut self) {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut _tmp_6410 = sh.borrow().text.clone();
        let line: String = PptxTextEdit::paraText(&_tmp_6410, self.caret.borrow().para.clone());
        let mut ends: Vec<i64> = Vec::new();
        OfficeCaretMotion::wordAt(&line, self.caret.borrow().col.clone(), &mut ends);
        if (ends.len() as i64) < 2 {
            return;
        }
        let _tmp_6414 = PptxTextPos::of(self.caret.borrow().para.clone(), ends[0]);
        self.anchor = _tmp_6414;
        let _tmp_6416 = PptxTextPos::of(self.caret.borrow().para.clone(), ends[1]);
        self.caret = _tmp_6416;
    }
    /// "left" "right" "up" "down" "home" "end" "wordLeft" "wordRight"
    pub fn moveCaretBy(&mut self, dir: &str, extend: bool) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut body: Rc<RefCell<PptxTextBody>> = sh.borrow().text.clone().clone();
        // An unextended move out of a selection collapses to its near end,
        // which is what a left arrow means when something is selected.
        if !extend {
            if !self.textSelectionEmpty() {
                let mut _tmp_6417 = self.anchor.clone();
                let cmpv: i64 = PptxTextEdit::cmp(&_tmp_6417, &self.caret.clone());
                if dir == "left" {
                    if cmpv < 0 {
                        self.caret = self.anchor.borrow().copy();
                    }
                    self.anchor = self.caret.borrow().copy();
                    return true;
                }
                if dir == "right" {
                    if cmpv > 0 {
                        self.caret = self.anchor.borrow().copy();
                    }
                    self.anchor = self.caret.borrow().copy();
                    return true;
                }
            }
        }
        let mut next: Rc<RefCell<PptxTextPos>> = self.caret.borrow().copy();
        if dir == "left" {
            let _tmp_6429 = PptxTextEdit::moveLeft(&body, &self.caret.clone());
            next = _tmp_6429;
        }
        if dir == "right" {
            let _tmp_6431 = PptxTextEdit::moveRight(&body, &self.caret.clone());
            next = _tmp_6431;
        }
        if dir == "up" {
            let _tmp_6433 = PptxTextEdit::moveUp(&body, &self.caret.clone());
            next = _tmp_6433;
        }
        if dir == "down" {
            let _tmp_6435 = PptxTextEdit::moveDown(&body, &self.caret.clone());
            next = _tmp_6435;
        }
        if dir == "home" {
            let _tmp_6437 = PptxTextEdit::lineStart(&body, &self.caret.clone());
            next = _tmp_6437;
        }
        if dir == "end" {
            let _tmp_6439 = PptxTextEdit::lineEnd(&body, &self.caret.clone());
            next = _tmp_6439;
        }
        if dir == "wordLeft" {
            let _tmp_6441 = PptxTextEdit::wordLeft(&body, &self.caret.clone());
            next = _tmp_6441;
        }
        if dir == "wordRight" {
            let _tmp_6443 = PptxTextEdit::wordRight(&body, &self.caret.clone());
            next = _tmp_6443;
        }
        self.caret = next.clone();
        if !extend {
            self.anchor = next.borrow().copy();
        }
        true
    }
    pub fn dropSelectedText(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        if self.textSelectionEmpty() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut _tmp_6445 = sh.borrow().text.clone();
        let mut _tmp_6446 = self.anchor.clone();
        let mut _tmp_6447 = self.caret.clone();
        let mut at: Rc<RefCell<PptxTextPos>> = PptxTextEdit::deleteRange(
            &_tmp_6445,
            &_tmp_6446,
            &_tmp_6447
        );
        self.caret = at.clone();
        self.anchor = at.borrow().copy();
        true
    }
    pub fn typeAtCaret(&mut self, text: &str) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        if text.is_empty() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        self.dropSelectedText();
        let mut _tmp_6449 = sh.borrow().text.clone();
        let mut at: Rc<RefCell<PptxTextPos>> = PptxTextEdit::insertAt(&_tmp_6449, &self.caret.clone(), text);
        self.caret = at.clone();
        self.anchor = at.borrow().copy();
        self.pushCoalesced("typing");
        true
    }
    /// The paragraph the caret is in, as JSON: which level it sits at and
    /// whether it is a list item. Small, and only what a host needs in order to
    /// check that a key did what it was supposed to — the model itself does not
    /// cross the boundary.
    pub fn caretParagraphJson(&self) -> String {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return "{\"level\":-1,\"bullet\":false}".to_string();
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let pi: i64 = self.caret.borrow().para;
        if pi < 0 {
            return "{\"level\":-1,\"bullet\":false}".to_string();
        }
        if pi >= (sh.borrow().text.borrow().paragraphs.len() as i64) {
            return "{\"level\":-1,\"bullet\":false}".to_string();
        }
        let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
            .text.borrow()
            .paragraphs[pi as usize].clone();
        let mut bullet: String = "false";
        if para.borrow().hasBullet {
            bullet = "true";
        }
        return format!("{}{},\"bullet\":{}{}", "{\"level\":".to_string(), para.borrow().level, bullet, "}".to_string());
    }
    /// Enter, when the caret is in a list.
    ///
    /// Every editor a reader has used ends a list the same way: press Enter on
    /// an item you have not typed anything into, and instead of another empty
    /// bullet you get out. One level at a time, so a sub-list steps back to its
    /// parent before it stops being a list at all — which is what makes a nested
    /// list writable without reaching for the toolbar between every level.
    ///
    /// Without this an empty bullet was simply added, and the only way out of a
    /// list was to press Enter, then find the list button and press it twice.
    /// Reported as "two empty Enters should end the list".
    ///
    /// The name says `enter` rather than `newline` because it is a decision and
    /// not an insertion: it answers what Enter MEANS here, which some of the
    /// time is nothing being typed at all.
    pub fn enterAtCaret(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return self.typeAtCaret("\n");
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        // Only with nothing selected. Enter with a selection replaces it, and
        // a reader who has selected a whole item and pressed Enter means to
        // type over it, not to leave the list.
        if !self.textSelectionEmpty() {
            return self.typeAtCaret("\n");
        }
        let pi: i64 = self.caret.borrow().para;
        if pi < 0 {
            return self.typeAtCaret("\n");
        }
        if pi >= (sh.borrow().text.borrow().paragraphs.len() as i64) {
            return self.typeAtCaret("\n");
        }
        let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
            .text.borrow()
            .paragraphs[pi as usize].clone();
        if !para.borrow().hasBullet {
            return self.typeAtCaret("\n");
        }
        // Empty means empty of TEXT. A paragraph carrying an empty run, which
        // is what backspacing an item down to nothing leaves behind, is still
        // an item a reader sees as blank.
        if PptxTextEdit::paraLen(&sh.borrow().text.clone(), pi) > 0 {
            return self.typeAtCaret("\n");
        }
        if para.borrow().level > 0 {
            return self.indentList(0 - 1);
        }
        // At the outermost level there is nowhere left to go but out of the
        // list. `bulletSet` is what stops the layout's own list style from
        // putting the bullet straight back.
        para.borrow_mut().bulletSet = true;
        para.borrow_mut().hasBullet = false;
        para.borrow_mut().bulletAutoNum = false;
        self.pushSnapshot("end list");
        true
    }
    pub fn newlineAtCaret(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        self.dropSelectedText();
        let mut _tmp_6453 = sh.borrow().text.clone();
        let mut at: Rc<RefCell<PptxTextPos>> = PptxTextEdit::splitAt(&_tmp_6453, &self.caret.clone());
        self.caret = at.clone();
        self.anchor = at.borrow().copy();
        self.pushSnapshot("newline");
        true
    }
    pub fn backspaceAtCaret(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if !self.textSelectionEmpty() {
            self.dropSelectedText();
            self.pushCoalesced("typing");
            return true;
        }
        let mut _tmp_6456 = sh.borrow().text.clone();
        let mut from: Rc<RefCell<PptxTextPos>> = PptxTextEdit::moveLeft(&_tmp_6456, &self.caret.clone());
        if from.borrow().isSame(&self.caret.clone()) {
            return false;
        }
        let mut _tmp_6459 = sh.borrow().text.clone();
        let mut _tmp_6460 = self.caret.clone();
        let mut at: Rc<RefCell<PptxTextPos>> = PptxTextEdit::deleteRange(
            &_tmp_6459,
            &from,
            &_tmp_6460
        );
        self.caret = at.clone();
        self.anchor = at.borrow().copy();
        self.pushCoalesced("typing");
        true
    }
    pub fn deleteAtCaret(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        if !self.textSelectionEmpty() {
            self.dropSelectedText();
            self.pushCoalesced("typing");
            return true;
        }
        let mut _tmp_6462 = sh.borrow().text.clone();
        let mut to: Rc<RefCell<PptxTextPos>> = PptxTextEdit::moveRight(&_tmp_6462, &self.caret.clone());
        if to.borrow().isSame(&self.caret.clone()) {
            return false;
        }
        let mut _tmp_6465 = sh.borrow().text.clone();
        let mut at: Rc<RefCell<PptxTextPos>> = PptxTextEdit::deleteRange(&_tmp_6465, &self.caret.clone(), &to);
        self.caret = at.clone();
        self.anchor = at.borrow().copy();
        self.pushCoalesced("typing");
        true
    }
    /// A style command while a caret is in the text means the SELECTION, and a
    /// style command otherwise means the whole shape. Both are what a user
    /// expects, and the difference is only which one is in front of them.
    /// --- lists ----------------------------------------------------------------
    ///
    /// A bullet and a number are properties of a PARAGRAPH, not of a run, so
    /// these act on whole paragraphs: the ones the caret spans when there is a
    /// caret, and every paragraph of every selected shape when there is not —
    /// which is how a person turns a whole box into a list without first
    /// selecting its text.
    pub fn listParagraphs(&self) -> Vec<Rc<RefCell<PptxParagraph>>> {
        let mut out: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_some() {
            let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
            let mut a: i64 = self.anchor.borrow().para;
            let mut b: i64 = self.caret.borrow().para;
            if a > b {
                let swap: i64 = a;
                a = b;
                b = swap;
            }
            let mut i: i64 = a;
            while i <= b {
                if i < (sh.borrow().text.borrow().paragraphs.len() as i64) {
                    out.push(sh.borrow().text.borrow().paragraphs[i as usize].clone());
                }
                i += 1;
            }
            return out;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut si: i64 = 0;
        while si < (shapes.len() as i64) {
            let mut s2: Rc<RefCell<PptxShape>> = shapes[si as usize].clone();
            if s2.borrow().hasText {
                let mut pi: i64 = 0;
                while pi < (s2.borrow().text.borrow().paragraphs.len() as i64) {
                    out.push(s2.borrow().text.borrow().paragraphs[pi as usize].clone());
                    pi += 1;
                }
            }
            si += 1;
        }
        out
    }
    /// "none", "bullet" or "number", read off the first paragraph the command
    /// would act on — which is what a toggle needs in order to know whether it
    /// is turning a list on or off.
    pub fn listKind(&self) -> String {
        let mut paras: Vec<Rc<RefCell<PptxParagraph>>> = self.listParagraphs();
        if paras.is_empty() {
            return "none".to_string();
        }
        let mut first: Rc<RefCell<PptxParagraph>> = paras[0].clone();
        if !first.borrow().hasBullet {
            return "none".to_string();
        }
        if first.borrow().bulletAutoNum {
            return "number".to_string();
        }
        "bullet".to_string()
    }
    pub fn setListKind(&mut self, kind: &str) -> bool {
        let mut paras: Vec<Rc<RefCell<PptxParagraph>>> = self.listParagraphs();
        if paras.is_empty() {
            return false;
        }
        let mut i: i64 = 0;
        while i < (paras.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = paras[i as usize].clone();
            // Stated on the paragraph, so the layout's inherited list style
            // does not put the bullet back that this just took away.
            para.borrow_mut().bulletSet = true;
            if kind == "none" {
                para.borrow_mut().hasBullet = false;
                para.borrow_mut().bulletAutoNum = false;
            }
            if kind == "bullet" {
                para.borrow_mut().hasBullet = true;
                para.borrow_mut().bulletAutoNum = false;
                if para.borrow().bulletChar.is_empty() {
                    para.borrow_mut().bulletChar = String::new();
                }
            }
            if kind == "number" {
                para.borrow_mut().hasBullet = true;
                para.borrow_mut().bulletAutoNum = true;
            }
            i += 1;
        }
        self.pushSnapshot(&format!("list {}", kind));
        true
    }
    /// One level in or out. The level is what the layout indents by and what
    /// the numbering counts within, so a sub-list starts at one — and when the
    /// deck stated its own `marL`, that moves with the level too, or nothing
    /// on screen would change.
    pub fn indentList(&mut self, delta: i64) -> bool {
        let mut paras: Vec<Rc<RefCell<PptxParagraph>>> = self.listParagraphs();
        if paras.is_empty() {
            return false;
        }
        let mut moved: i64 = 0;
        let mut i: i64 = 0;
        while i < (paras.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = paras[i as usize].clone();
            let mut want: i64 = para.borrow().level + delta;
            if want < 0 {
                want = 0;
            }
            if want > 8 {
                want = 8;
            }
            if want != para.borrow().level {
                if para.borrow().marginSet {
                    let hoist_28 = para.borrow().marginLeftPt + ((want - para.borrow().level) as f64) * 18.0;
                    para.borrow_mut().marginLeftPt = hoist_28;
                    if para.borrow().marginLeftPt < 0.0 {
                        para.borrow_mut().marginLeftPt = 0.0;
                    }
                }
                para.borrow_mut().level = want;
                moved += 1;
            }
            i += 1;
        }
        if moved == 0 {
            return false;
        }
        self.pushSnapshot("list indent");
        true
    }
    pub fn styleTextRange(&mut self, what: &str, flag: bool, size: f64, text: &str) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        if self.textSelectionEmpty() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut _tmp_6469 = sh.borrow().text.clone();
        let mut _tmp_6470 = self.anchor.clone();
        let mut _tmp_6471 = self.caret.clone();
        PptxTextEdit::applyStyle(
            &_tmp_6469,
            &_tmp_6470,
            &_tmp_6471,
            what,
            flag,
            size,
            text.to_string()
        );
        self.pushSnapshot(&format!("text {}", what));
        true
    }
    pub fn selectionIsAll(&self, what: &str) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.editingShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut _tmp_6472 = sh.borrow().text.clone();
        let mut _tmp_6473 = self.anchor.clone();
        return PptxTextEdit::rangeIsAll(&_tmp_6472, &_tmp_6473, &self.caret.clone(), what);
    }
    /// A rubber band takes what it TOUCHES rather than only what it encloses:
    /// the enclosing rule is PowerPoint's and it makes a band across a row of
    /// shapes select nothing at all.
    pub fn selectInRect(&mut self, x: f64, y: f64, w: f64, h: f64, additive: bool) -> i64 {
        if !additive {
            self.clearSelection();
        }
        let mut rx: f64 = x;
        let mut ry: f64 = y;
        let mut rw: f64 = w;
        let mut rh: f64 = h;
        if rw < 0.0 {
            rx = x + w;
            rw = 0.0 - w;
        }
        if rh < 0.0 {
            ry = y + h;
            rh = 0.0 - h;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.editableShapes();
        let mut hits: i64 = 0;
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut _tmp_6475 = sh.borrow().x.clone();
            let mut _tmp_6476 = sh.borrow().y.clone();
            let mut _tmp_6477 = sh.borrow().width.clone();
            if PptxEditor::rectsOverlap(rx, ry, rw, rh, _tmp_6475, _tmp_6476, _tmp_6477, sh.borrow().height.clone()) {
                self.addToSelection(sh.borrow().editId.clone());
                hits += 1;
            }
            i += 1;
        }
        hits
    }
    /// --- the clipboard --------------------------------------------------------
    pub fn copySelection(&mut self) -> i64 {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        if shapes.is_empty() {
            return 0;
        }
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            out.push(PptxClone::shape(&sh));
            i += 1;
        }
        self.clip = out.clone();
        out.len() as i64
    }
    pub fn cutSelection(&mut self) -> i64 {
        let n: i64 = self.copySelection();
        if n == 0 {
            return 0;
        }
        self.deleteSelection();
        n
    }
    pub fn clipboardCount(&self) -> i64 {
        self.clip.len() as i64
    }
    /// Pasting on the slide it was copied from offsets, so the copy is visible;
    /// pasting on another slide puts it where it was, which is what makes
    /// copying a header from one slide to the next useful.
    pub fn pasteClipboard(&mut self, offset: bool) -> bool {
        if self.clip.is_empty() {
            return false;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut pasted: Vec<i64> = Vec::new();
        let mut i: i64 = 0;
        while i < (self.clip.len() as i64) {
            let mut src: Rc<RefCell<PptxShape>> = self.clip[i as usize].clone();
            let mut copy: Rc<RefCell<PptxShape>> = PptxClone::shape(&src);
            if offset {
                let hoist_29 = copy.borrow().x + 10.0;
                copy.borrow_mut().x = hoist_29;
                let hoist_30 = copy.borrow().y + 10.0;
                copy.borrow_mut().y = hoist_30;
            }
            copy.borrow_mut().locked = false;
            self.assignIdTo(&copy);
            s.borrow_mut().shapes.push(copy.clone());
            pasted.push(copy.borrow().editId);
            i += 1;
        }
        self.clearSelection();
        let mut j: i64 = 0;
        while j < (pasted.len() as i64) {
            self.selection.push(pasted[j as usize]);
            j += 1;
        }
        self.pushSnapshot("paste");
        true
    }
    /// --- locking --------------------------------------------------------------
    /// Mirroring, which is a flag rather than a transform: the box stays where
    /// it is and what is drawn in it is read the other way round.
    pub fn flipSelection(&mut self, horizontal: bool) -> bool {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        if shapes.is_empty() {
            return false;
        }
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if horizontal {
                let hoist_31 = !sh.borrow().flipH;
                sh.borrow_mut().flipH = hoist_31;
            } else {
                let hoist_32 = !sh.borrow().flipV;
                sh.borrow_mut().flipV = hoist_32;
            }
            i += 1;
        }
        self.pushSnapshot("flip");
        true
    }
    pub fn setLocked(&mut self, locked: bool) -> bool {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        if shapes.is_empty() {
            return false;
        }
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            sh.borrow_mut().locked = locked;
            i += 1;
        }
        self.pushSnapshot("lock");
        true
    }
    pub fn selectionLocked(&self) -> bool {
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        if shapes.is_empty() {
            return false;
        }
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if !sh.borrow().locked {
                return false;
            }
            i += 1;
        }
        true
    }
    /// The shapes a transform may touch: everything selected that is not locked.
    pub fn movableShapes(&self) -> Vec<Rc<RefCell<PptxShape>>> {
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.selectedShapes();
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if !sh.borrow().locked {
                out.push(sh.clone());
            }
            i += 1;
        }
        out
    }
    pub fn setGrid(&mut self, on: bool) {
        self.gridEnabled = on;
    }
    /// --- the format painter ---------------------------------------------------
    pub fn copyFormat(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.firstSelected();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        let mut f: Rc<RefCell<PptxFormat>> = Rc::new(RefCell::new(PptxFormat::new()));
        let _tmp_6485 = PptxClone::color(&sh.borrow().fill.clone());
        f.borrow_mut().fill = _tmp_6485;
        let _tmp_6487 = PptxClone::gradient(&sh.borrow().fillGrad.clone());
        f.borrow_mut().grad = _tmp_6487;
        let _tmp_6489 = PptxClone::color(&sh.borrow().line.clone());
        f.borrow_mut().line = _tmp_6489;
        f.borrow_mut().lineWidthPt = sh.borrow().lineWidthPt;
        let _tmp_6492 = PptxClone::shadow(&sh.borrow().shadow.clone());
        f.borrow_mut().shadow = _tmp_6492;
        if sh.borrow().hasText {
            if !sh.borrow().text.borrow().paragraphs.is_empty() {
                let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow().text.borrow().paragraphs[0].clone();
                f.borrow_mut().align = para.borrow().align.clone();
                f.borrow_mut().alignSet = para.borrow().alignSet;
                if !para.borrow().runs.is_empty() {
                    let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[0].clone();
                    f.borrow_mut().hasText = true;
                    f.borrow_mut().bold = run.borrow().bold;
                    f.borrow_mut().italic = run.borrow().italic;
                    f.borrow_mut().sizePt = run.borrow().fontSizePt;
                    f.borrow_mut().sizeSet = run.borrow().sizeSet;
                    f.borrow_mut().family = run.borrow().fontFamily.clone();
                    let _tmp_6501 = PptxClone::color(&run.borrow().color.clone());
                    f.borrow_mut().color = _tmp_6501;
                }
            }
        }
        self.format = f.clone();
        self.formatSet = true;
        true
    }
    pub fn hasFormat(&self) -> bool {
        self.formatSet
    }
    /// Painted onto everything selected that is not locked. Text properties
    /// reach a shape only if it HAS text — a picture does not become bold.
    pub fn applyFormat(&mut self) -> bool {
        if !self.formatSet {
            return false;
        }
        let mut shapes: Vec<Rc<RefCell<PptxShape>>> = self.movableShapes();
        if shapes.is_empty() {
            return false;
        }
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            if sh.borrow().kind != PptxShapeKind::picture() {
                let _tmp_6503 = PptxClone::color(&self.format.borrow().fill.clone());
                sh.borrow_mut().fill = _tmp_6503;
                let _tmp_6505 = PptxClone::gradient(&self.format.borrow().grad.clone());
                sh.borrow_mut().fillGrad = _tmp_6505;
            }
            let _tmp_6507 = PptxClone::color(&self.format.borrow().line.clone());
            sh.borrow_mut().line = _tmp_6507;
            sh.borrow_mut().lineWidthPt = self.format.borrow().lineWidthPt;
            let _tmp_6510 = PptxClone::shadow(&self.format.borrow().shadow.clone());
            sh.borrow_mut().shadow = _tmp_6510;
            if sh.borrow().hasText {
                let mut pi: i64 = 0;
                while pi < (sh.borrow().text.borrow().paragraphs.len() as i64) {
                    let mut para: Rc<RefCell<PptxParagraph>> = sh.borrow()
                        .text.borrow()
                        .paragraphs[pi as usize].clone();
                    if self.format.borrow().alignSet {
                        para.borrow_mut().align = self.format.borrow().align.clone();
                        para.borrow_mut().alignSet = true;
                    }
                    if self.format.borrow().hasText {
                        let mut ri: i64 = 0;
                        while ri < (para.borrow().runs.len() as i64) {
                            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[ri as usize].clone();
                            run.borrow_mut().bold = self.format.borrow().bold;
                            run.borrow_mut().boldSet = true;
                            run.borrow_mut().italic = self.format.borrow().italic;
                            run.borrow_mut().italicSet = true;
                            run.borrow_mut().fontSizePt = self.format.borrow().sizePt;
                            run.borrow_mut().sizeSet = self.format.borrow().sizeSet;
                            run.borrow_mut().fontFamily = self.format.borrow().family.clone();
                            let _tmp_6518 = PptxClone::color(&self.format.borrow().color.clone());
                            run.borrow_mut().color = _tmp_6518;
                            ri += 1;
                        }
                    }
                    pi += 1;
                }
            }
            i += 1;
        }
        self.pushSnapshot("format");
        true
    }
    pub fn applyPalette(&mut self, name: &str) -> bool {
        let mut cols: Vec<String> = PptxEditor::paletteColors(name);
        if (cols.len() as i64) < 12 {
            return false;
        }
        let mut slots: Vec<String> = vec!["dk1".to_string(), "lt1".to_string(), "dk2".to_string(), "lt2".to_string(), "accent1".to_string(), "accent2".to_string(), "accent3".to_string(), "accent4".to_string(), "accent5".to_string(), "accent6".to_string(), "hlink".to_string(), "folHlink".to_string()];
        let mut i: i64 = 0;
        while i < (slots.len() as i64) {
            self.doc.borrow().theme.clone().borrow_mut().setColor(&slots[i as usize].clone(), &cols[i as usize].clone());
            i += 1;
        }
        self.retheme();
        self.pushSnapshot(&format!("theme {}", name));
        true
    }
    pub fn setThemeColor(&mut self, slot: &str, hex: &str) -> bool {
        if slot.is_empty() {
            return false;
        }
        self.doc.borrow().theme.clone().borrow_mut().setColor(slot, hex);
        self.retheme();
        self.pushSnapshot("theme colour");
        true
    }
    /// Every colour in the deck that came from the palette, resolved again —
    /// including the master and layout sheets, so a layout applied afterwards
    /// arrives in the new colours too.
    pub fn retheme(&mut self) {
        // As with `remergeChrome`: every slide changes, so every slide says so.
        self.touchAllSlides();
        let mut si: i64 = 0;
        while si < (self.doc.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[si as usize].clone();
            self.rethemeColor(&slide.borrow().bg.clone());
            let mut i: i64 = 0;
            while i < (slide.borrow().shapes.len() as i64) {
                self.rethemeShape(&slide.borrow().shapes[i as usize].clone().clone());
                i += 1;
            }
            self.rethemeShape(&slide.borrow().notes.clone());
            slide.borrow_mut().dirty = true;
            self.nextRevision += 1;
            slide.borrow_mut().revision = self.nextRevision;
            si += 1;
        }
        let mut mkeys: Vec<String> = self.doc.borrow()
            .masters.keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut mi: i64 = 0;
        while mi < (mkeys.len() as i64) {
            let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .masters.get(&mkeys[mi as usize].clone())
                .cloned();
            if sheet.is_some() {
                self.rethemeSheet(&sheet.clone().unwrap());
            }
            mi += 1;
        }
        let mut lkeys: Vec<String> = self.doc.borrow()
            .layouts.keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut li: i64 = 0;
        while li < (lkeys.len() as i64) {
            let mut lsheet: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .layouts.get(&lkeys[li as usize].clone())
                .cloned();
            if lsheet.is_some() {
                self.rethemeSheet(&lsheet.clone().unwrap());
            }
            li += 1;
        }
    }
    pub fn rethemeSheet(&self, sheet: &Rc<RefCell<PptxSheet>>) {
        self.rethemeColor(&sheet.borrow().bg.clone());
        let mut i: i64 = 0;
        while i < (sheet.borrow().shapes.len() as i64) {
            self.rethemeShape(&sheet.borrow().shapes[i as usize].clone().clone());
            i += 1;
        }
    }
    pub fn rethemeShape(&self, shape: &Rc<RefCell<PptxShape>>) {
        self.rethemeColor(&shape.borrow().fill.clone());
        self.rethemeColor(&shape.borrow().line.clone());
        self.rethemeColor(&shape.borrow().fillGrad.borrow().c1.clone());
        self.rethemeColor(&shape.borrow().fillGrad.borrow().c2.clone());
        self.rethemeColor(&shape.borrow().shadow.borrow().color.clone());
        self.rethemeColor(&shape.borrow().text.borrow().defColor.clone());
        let mut pi: i64 = 0;
        while pi < (shape.borrow().text.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = shape.borrow()
                .text.borrow()
                .paragraphs[pi as usize].clone();
            let mut ri: i64 = 0;
            while ri < (para.borrow().runs.len() as i64) {
                self.rethemeColor(&para.borrow().runs[ri as usize].clone().borrow().color.clone());
                ri += 1;
            }
            pi += 1;
        }
        let mut ci: i64 = 0;
        while ci < (shape.borrow().tableCells.len() as i64) {
            self.rethemeColor(&shape.borrow().tableCells[ci as usize].clone().borrow().fill.clone());
            ci += 1;
        }
        let mut k: i64 = 0;
        while k < (shape.borrow().children.len() as i64) {
            self.rethemeShape(&shape.borrow().children[k as usize].clone().clone());
            k += 1;
        }
    }
    pub fn rethemeColor(&self, c: &Rc<RefCell<PptxColor>>) {
        if !c.borrow().isSet {
            return;
        }
        if !c.borrow().isScheme {
            return;
        }
        let key: String = self.resolver.borrow().mapSchemeName(&c.borrow().scheme.clone());
        c.borrow_mut().srgb = (self.doc.borrow().theme).borrow().colorOf(&key);
    }
    pub fn themeColor(&self, slot: &str) -> String {
        return (self.doc.borrow().theme).borrow().colorOf(slot);
    }
    /// --- the layout -----------------------------------------------------------
    pub fn layoutParts(&self) -> Vec<String> {
        self.doc.borrow().layouts.keys().cloned().collect::<Vec<String>>()
    }
    pub fn slideLayout(&self) -> String {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return String::new();
        }
        slide.clone().unwrap().borrow().layoutPart.clone()
    }
    /// Picking a layout swaps the chrome the slide inherits — the master's, then
    /// the new layout's — and leaves everything the slide itself declares where
    /// it is. It is the same merge the resolver does at load, done again with a
    /// different layout.
    pub fn applyLayout(&mut self, part: String) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
            .layouts.get(&part)
            .cloned();
        if sheet.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut layout: Rc<RefCell<PptxSheet>> = sheet.clone().unwrap();
        let mut authored: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (s.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = s.borrow().shapes[i as usize].clone();
            if !sh.borrow().inherited {
                authored.push(sh.clone());
            }
            i += 1;
        }
        let mut merged: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut master: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
            .masters.get(&s.borrow().masterPart)
            .cloned();
        if master.is_none() {
            let mut mkeys: Vec<String> = self.doc.borrow()
                .masters.keys()
                .cloned()
                .collect::<Vec<String>>();
            if !mkeys.is_empty() {
                master = self.doc.borrow().masters.get(&mkeys[0].clone()).cloned();
            }
        }
        if master.is_some() {
            let mut ms: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
            let mut mi: i64 = 0;
            while mi < (ms.borrow().shapes.len() as i64) {
                let mut mshape: Rc<RefCell<PptxShape>> = ms.borrow().shapes[mi as usize].clone();
                if !mshape.borrow().isPlaceholder {
                    let mut mc: Rc<RefCell<PptxShape>> = PptxClone::shape(&mshape);
                    mc.borrow_mut().inherited = true;
                    self.assignIdTo(&mc);
                    merged.push(mc.clone());
                }
                mi += 1;
            }
        }
        let mut li: i64 = 0;
        while li < (layout.borrow().shapes.len() as i64) {
            let mut lshape: Rc<RefCell<PptxShape>> = layout.borrow().shapes[li as usize].clone();
            if !lshape.borrow().isPlaceholder {
                let mut lc: Rc<RefCell<PptxShape>> = PptxClone::shape(&lshape);
                lc.borrow_mut().inherited = true;
                self.assignIdTo(&lc);
                merged.push(lc.clone());
            }
            li += 1;
        }
        let mut j: i64 = 0;
        while j < (authored.len() as i64) {
            merged.push(authored[j as usize].clone());
            j += 1;
        }
        s.borrow_mut().shapes = merged.clone();
        s.borrow_mut().layoutPart = part.clone();
        if layout.borrow().bg.borrow().isSet {
            if !s.borrow().bg.borrow().isSet {
                let _tmp_6535 = PptxClone::color(&layout.borrow().bg.clone());
                s.borrow_mut().bg = _tmp_6535;
            }
        }
        self.clearSelection();
        self.pushSnapshot("layout");
        true
    }
    /// --- the slide's own background -------------------------------------------
    pub fn setSlideBackground(&mut self, hex: &str) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        s.borrow_mut().bg = PptxColor::srgbOf(hex.to_string());
        self.pushSnapshot("background");
        true
    }
    pub fn setSlideBackgroundScheme(&mut self, scheme: String) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut c: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        c.borrow_mut().isSet = true;
        c.borrow_mut().isScheme = true;
        c.borrow_mut().scheme = scheme.clone();
        c.borrow_mut().srgb = (self.doc.borrow().theme).borrow().colorOf(&self.resolver.borrow().mapSchemeName(&scheme));
        s.borrow_mut().bg = c.clone();
        self.pushSnapshot("background");
        true
    }
    pub fn clearSlideBackground(&mut self) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut none: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        s.borrow_mut().bg = none.clone();
        self.pushSnapshot("background");
        true
    }
    /// --- the notes ------------------------------------------------------------
    pub fn notesShape(&mut self) -> Option<Rc<RefCell<PptxShape>>> {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            let mut __none: Option<Rc<RefCell<PptxShape>>> = None;
            return __none;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        if s.borrow().notes.borrow().editId == 0 {
            self.assignIdTo(&s.borrow().notes.clone());
        }
        s.borrow_mut().notes.borrow_mut().hasText = true;
        if s.borrow().notes.borrow().text.borrow().paragraphs.is_empty() {
            let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            s.borrow_mut().notes.borrow_mut().text.borrow_mut().paragraphs.push(para.clone());
        }
        let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(s.borrow().notes.clone()).clone();
        hit
    }
    pub fn notesText(&self) -> String {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return String::new();
        }
        slide.clone().unwrap().notesText().clone()
    }
    pub fn setNotes(&mut self, text: &str) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.notesShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        PptxEditor::setBodyText(&sh.borrow().text.clone(), text);
        self.pushSnapshot("notes");
        true
    }
    pub fn editNotes(&mut self) -> bool {
        let mut hit: Option<Rc<RefCell<PptxShape>>> = self.notesShape();
        if hit.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = hit.clone().unwrap();
        // The notes are not on the slide, so editing them is not a selection:
        // the caret goes there without anything being picked up.
        self.clearSelection();
        self.editingId = sh.borrow().editId;
        let mut end: Rc<RefCell<PptxTextPos>> = PptxTextEdit::docEnd(&sh.borrow().text.clone());
        self.caret = end.clone();
        self.anchor = end.borrow().copy();
        true
    }
    pub fn isEditingNotes(&self) -> bool {
        if self.editingId <= 0 {
            return false;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        return s.borrow().notes.borrow().editId == self.editingId;
    }
    /// --- editing a master or a layout -----------------------------------------
    ///
    /// A sheet is a slide with a different name — the same shapes, the same
    /// geometry — so editing one is standing a slide in front of the editor and
    /// letting every operation work as it always does. What comes back out is
    /// copied into the sheet, and the slides that inherit from it are re-merged.
    pub fn sheetFor(&self, part: &str) -> Option<Rc<RefCell<PptxSheet>>> {
        let mut m: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
            .masters.get(part)
            .cloned();
        if m.is_some() {
            return m;
        }
        let mut l: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
            .layouts.get(part)
            .cloned();
        l
    }
    pub fn editSheet(&mut self, part: String) -> bool {
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.sheetFor(&part);
        if sheet.is_none() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxSheet>> = sheet.clone().unwrap();
        let mut stand: Rc<RefCell<PptxSlide>> = Rc::new(RefCell::new(PptxSlide::new()));
        stand.borrow_mut().partPath = part.clone();
        stand.borrow_mut().width = self.doc.borrow().slideWidth;
        stand.borrow_mut().height = self.doc.borrow().slideHeight;
        stand.borrow_mut().bg = sh.borrow().bg.clone();
        stand.borrow_mut().shapes = sh.borrow().shapes.clone();
        let mut i: i64 = 0;
        while i < (stand.borrow().shapes.len() as i64) {
            let mut s2: Rc<RefCell<PptxShape>> = stand.borrow().shapes[i as usize].clone();
            if s2.borrow().editId == 0 {
                self.assignIdTo(&s2);
            }
            i += 1;
        }
        self.sheetSlide = stand.clone();
        self.sheetPart = part.clone();
        self.clearSelection();
        true
    }
    pub fn isEditingSheet(&self) -> bool {
        !self.sheetPart.is_empty()
    }
    pub fn editingSheetPart(&self) -> String {
        self.sheetPart.clone()
    }
    pub fn masterParts(&self) -> Vec<String> {
        self.doc.borrow().masters.keys().cloned().collect::<Vec<String>>()
    }
    /// Copy what was done back onto the sheet, and give every slide that follows
    /// it its chrome again.
    pub fn exitSheet(&mut self) -> bool {
        if self.sheetPart.is_empty() {
            return false;
        }
        let part: String = self.sheetPart.clone();
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.sheetFor(&part);
        if sheet.is_some() {
            let mut sh: Rc<RefCell<PptxSheet>> = sheet.clone().unwrap();
            sh.borrow_mut().shapes = self.sheetSlide.borrow().shapes.clone();
            sh.borrow_mut().bg = self.sheetSlide.borrow().bg.clone();
            sh.borrow_mut().dirty = true;
        }
        self.sheetPart = String::new();
        self.clearSelection();
        self.remergeChrome();
        true
    }
    /// The resolver's merge, run again for every slide: master chrome, then
    /// layout chrome, then whatever the slide itself declares.
    pub fn remergeChrome(&mut self) {
        // Every slide is rewritten here, not just the one in front of you, and
        // the history has to be told — it shares its copies of the slides that
        // did not change, and tells them apart by revision.
        self.touchAllSlides();
        let mut si: i64 = 0;
        while si < (self.doc.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[si as usize].clone();
            let mut authored: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
            let mut i: i64 = 0;
            while i < (slide.borrow().shapes.len() as i64) {
                let mut sh: Rc<RefCell<PptxShape>> = slide.borrow().shapes[i as usize].clone();
                if !sh.borrow().inherited {
                    authored.push(sh.clone());
                }
                i += 1;
            }
            let mut merged: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
            let mut master: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .masters.get(&slide.borrow().masterPart)
                .cloned();
            if master.is_none() {
                let mut mkeys: Vec<String> = self.doc.borrow()
                    .masters.keys()
                    .cloned()
                    .collect::<Vec<String>>();
                if !mkeys.is_empty() {
                    master = self.doc.borrow().masters.get(&mkeys[0].clone()).cloned();
                }
            }
            if master.is_some() {
                let mut ms: Rc<RefCell<PptxSheet>> = master.clone().unwrap();
                let mut mi: i64 = 0;
                while mi < (ms.borrow().shapes.len() as i64) {
                    let mut mshape: Rc<RefCell<PptxShape>> = ms.borrow().shapes[mi as usize].clone();
                    if !mshape.borrow().isPlaceholder {
                        let mut mc: Rc<RefCell<PptxShape>> = PptxClone::shape(&mshape);
                        mc.borrow_mut().inherited = true;
                        self.assignIdTo(&mc);
                        merged.push(mc.clone());
                    }
                    mi += 1;
                }
            }
            let mut layout: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .layouts.get(&slide.borrow().layoutPart)
                .cloned();
            if layout.is_some() {
                let mut ls: Rc<RefCell<PptxSheet>> = layout.clone().unwrap();
                let mut li: i64 = 0;
                while li < (ls.borrow().shapes.len() as i64) {
                    let mut lshape: Rc<RefCell<PptxShape>> = ls.borrow().shapes[li as usize].clone();
                    if !lshape.borrow().isPlaceholder {
                        let mut lc: Rc<RefCell<PptxShape>> = PptxClone::shape(&lshape);
                        lc.borrow_mut().inherited = true;
                        self.assignIdTo(&lc);
                        merged.push(lc.clone());
                    }
                    li += 1;
                }
            }
            let mut j: i64 = 0;
            while j < (authored.len() as i64) {
                merged.push(authored[j as usize].clone());
                j += 1;
            }
            slide.borrow_mut().shapes = merged.clone();
            si += 1;
        }
    }
    /// --- what every slide says: footers and page numbers -----------------------
    ///
    /// These belong on the master, which is where PowerPoint keeps them: put one
    /// there and every slide that follows the master has it, without a copy per
    /// slide to keep in step.
    pub fn masterSheet(&self) -> Option<Rc<RefCell<PptxSheet>>> {
        let mut mkeys: Vec<String> = self.doc.borrow()
            .masters.keys()
            .cloned()
            .collect::<Vec<String>>();
        if mkeys.is_empty() {
            let mut __none: Option<Rc<RefCell<PptxSheet>>> = None;
            return __none;
        }
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_some() {
            let mut s: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
            let mut own: Option<Rc<RefCell<PptxSheet>>> = self.doc.borrow()
                .masters.get(&s.borrow().masterPart)
                .cloned();
            if own.is_some() {
                return own;
            }
        }
        self.doc.borrow().masters.get(&mkeys[0].clone()).cloned()
    }
    pub fn dropNamed(&self, sheet: &Rc<RefCell<PptxSheet>>, name: &str) {
        let mut kept: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (sheet.borrow().shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = sheet.borrow().shapes[i as usize].clone();
            if sh.borrow().name != name {
                kept.push(sh.clone());
            }
            i += 1;
        }
        sheet.borrow_mut().shapes = kept.clone();
    }
    pub fn showSlideNumbers(&mut self, on: bool) -> bool {
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.masterSheet();
        if sheet.is_none() {
            return false;
        }
        let mut ms: Rc<RefCell<PptxSheet>> = sheet.clone().unwrap();
        self.dropNamed(&ms, "Slide Number Placeholder");
        if on {
            let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            sh.borrow_mut().name = "Slide Number Placeholder".to_string();
            sh.borrow_mut().kind = PptxShapeKind::shape();
            sh.borrow_mut().hasXfrm = true;
            sh.borrow_mut().width = 60.0;
            sh.borrow_mut().height = 20.0;
            let hoist_33 = (self.doc.borrow().slideWidth - sh.borrow().width) - 24.0;
            sh.borrow_mut().x = hoist_33;
            let hoist_34 = (self.doc.borrow().slideHeight - sh.borrow().height) - 12.0;
            sh.borrow_mut().y = hoist_34;
            sh.borrow_mut().hasText = true;
            let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            para.borrow_mut().align = "r".to_string();
            para.borrow_mut().alignSet = true;
            let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
            run.borrow_mut().field = "slidenum".to_string();
            run.borrow_mut().text = (self.slideIndex + 1).to_string();
            run.borrow_mut().fontSizePt = 12.0;
            run.borrow_mut().sizeSet = true;
            run.borrow_mut().color = PptxColor::srgbOf("595959".to_string());
            run.borrow_mut().colorStated = true;
            para.borrow_mut().runs.push(run.clone());
            sh.borrow_mut().text.borrow_mut().paragraphs.push(para.clone());
            self.assignIdTo(&sh);
            ms.borrow_mut().shapes.push(sh.clone());
        }
        ms.borrow_mut().dirty = true;
        self.remergeChrome();
        self.pushSnapshot("slide numbers");
        true
    }
    pub fn slideNumbersShown(&self) -> bool {
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.masterSheet();
        if sheet.is_none() {
            return false;
        }
        let mut hit: Option<Rc<RefCell<PptxShape>>> = PptxEditor::namedShape(&sheet.clone().unwrap(), "Slide Number Placeholder");
        hit.is_some()
    }
    pub fn setFooter(&mut self, text: String) -> bool {
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.masterSheet();
        if sheet.is_none() {
            return false;
        }
        let mut ms: Rc<RefCell<PptxSheet>> = sheet.clone().unwrap();
        self.dropNamed(&ms, "Footer Placeholder");
        if !text.is_empty() {
            let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            sh.borrow_mut().name = "Footer Placeholder".to_string();
            sh.borrow_mut().kind = PptxShapeKind::shape();
            sh.borrow_mut().hasXfrm = true;
            sh.borrow_mut().width = self.doc.borrow().slideWidth * 0.5;
            sh.borrow_mut().height = 20.0;
            let hoist_35 = (self.doc.borrow().slideWidth - sh.borrow().width) * 0.5;
            sh.borrow_mut().x = hoist_35;
            let hoist_36 = (self.doc.borrow().slideHeight - sh.borrow().height) - 12.0;
            sh.borrow_mut().y = hoist_36;
            sh.borrow_mut().hasText = true;
            let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            para.borrow_mut().align = "ctr".to_string();
            para.borrow_mut().alignSet = true;
            let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
            run.borrow_mut().text = text.clone();
            run.borrow_mut().fontSizePt = 12.0;
            run.borrow_mut().sizeSet = true;
            run.borrow_mut().color = PptxColor::srgbOf("595959".to_string());
            run.borrow_mut().colorStated = true;
            para.borrow_mut().runs.push(run.clone());
            sh.borrow_mut().text.borrow_mut().paragraphs.push(para.clone());
            self.assignIdTo(&sh);
            ms.borrow_mut().shapes.push(sh.clone());
        }
        ms.borrow_mut().dirty = true;
        self.remergeChrome();
        self.pushSnapshot("footer");
        true
    }
    pub fn footerText(&self) -> String {
        let mut sheet: Option<Rc<RefCell<PptxSheet>>> = self.masterSheet();
        if sheet.is_none() {
            return String::new();
        }
        let mut hit: Option<Rc<RefCell<PptxShape>>> = PptxEditor::namedShape(&sheet.clone().unwrap(), "Footer Placeholder");
        if hit.is_none() {
            return String::new();
        }
        return hit.clone().unwrap().borrow().text.plainText().clone();
    }
    /// --- the show: transitions and builds -------------------------------------
    ///
    /// What a slide does when it arrives, and what it does once it is there.
    /// Both are per slide and both survive a save, which is the only thing that
    /// makes them worth having in an editor rather than in the player.
    pub fn setTransition(&mut self, kind: String, dir: String, seconds: f64) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut tr: Rc<RefCell<PptxTransition>> = s0.borrow().transition.clone().clone();
        if kind.is_empty() {
            tr.borrow_mut().isSet = false;
            tr.borrow_mut().kind = String::new();
            tr.borrow_mut().dir = String::new();
        } else {
            tr.borrow_mut().isSet = true;
            tr.borrow_mut().kind = kind.clone();
            tr.borrow_mut().dir = dir.clone();
            if seconds > 0.0 {
                tr.borrow_mut().seconds = seconds;
            }
        }
        self.pushSnapshot("transition");
        true
    }
    pub fn transitionKind(&self) -> String {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return String::new();
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        if !s0.borrow().transition.borrow().isSet {
            return String::new();
        }
        return s0.borrow().transition.borrow().kind.clone();
    }
    /// Advance on a timer as well as on a click. Zero seconds turns it off.
    pub fn setAdvanceAfter(&mut self, seconds: f64) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        s0.borrow_mut().transition.borrow_mut().afterSeconds = seconds;
        if seconds > 0.0 {
            s0.borrow_mut().transition.borrow_mut().isSet = true;
            if s0.borrow().transition.borrow().kind.is_empty() {
                s0.borrow_mut().transition.borrow_mut().kind = "fade".to_string();
            }
        }
        self.pushSnapshot("advance");
        true
    }
    pub fn buildCount(&self) -> i64 {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return 0;
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        return s0.borrow().build.len() as i64;
    }
    /// Give every selected shape a step of its own, in the order they were
    /// selected — which is what "animate these, one click each" means.
    pub fn animateSelection(&mut self, kind: String, seconds: f64) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        if self.selection.is_empty() {
            return false;
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let mut i: i64 = 0;
        while i < (self.selection.len() as i64) {
            let id: i64 = self.selection[i as usize];
            let mut already: bool = false;
            let mut k: i64 = 0;
            while k < (s0.borrow().build.len() as i64) {
                let mut old: Rc<RefCell<PptxBuildStep>> = s0.borrow().build[k as usize].clone();
                if old.borrow().editId == id {
                    if old.borrow().paragraph < 0 {
                        already = true;
                        old.borrow_mut().kind = kind.clone();
                        if seconds > 0.0 {
                            old.borrow_mut().seconds = seconds;
                        }
                    }
                }
                k += 1;
            }
            if !already {
                let mut step: Rc<RefCell<PptxBuildStep>> = Rc::new(RefCell::new(PptxBuildStep::new()));
                step.borrow_mut().editId = id;
                step.borrow_mut().kind = kind.clone();
                if seconds > 0.0 {
                    step.borrow_mut().seconds = seconds;
                }
                s0.borrow_mut().build.push(step.clone());
            }
            i += 1;
        }
        self.pushSnapshot("animate");
        true
    }
    pub fn clearAnimation(&mut self) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        if s0.borrow().build.is_empty() {
            return false;
        }
        let mut empty: Vec<Rc<RefCell<PptxBuildStep>>> = Vec::new();
        s0.borrow_mut().build = empty.clone();
        self.pushSnapshot("clear animation");
        true
    }
    /// Move a step earlier or later in the build, which is the whole of an
    /// animation pane that is worth having: the ORDER is the interesting part.
    pub fn moveBuildStep(&mut self, from: i64, to: i64) -> bool {
        let mut slide: Option<Rc<RefCell<PptxSlide>>> = self.currentSlide();
        if slide.is_none() {
            return false;
        }
        let mut s0: Rc<RefCell<PptxSlide>> = slide.clone().unwrap();
        let n: i64 = s0.borrow().build.len() as i64;
        if from < 0 {
            return false;
        }
        if from >= n {
            return false;
        }
        if to < 0 {
            return false;
        }
        if to >= n {
            return false;
        }
        if from == to {
            return false;
        }
        let mut moved: Rc<RefCell<PptxBuildStep>> = s0.borrow().build[from as usize].clone();
        let mut out: Vec<Rc<RefCell<PptxBuildStep>>> = Vec::new();
        let mut i: i64 = 0;
        while i < n {
            if i != from {
                if (out.len() as i64) == to {
                    out.push(moved.clone());
                }
                out.push(s0.borrow().build[i as usize].clone());
            }
            i += 1;
        }
        if (out.len() as i64) < n {
            out.push(moved.clone());
        }
        s0.borrow_mut().build = out.clone();
        self.pushSnapshot("build order");
        true
    }
    /// Which shapes are visible at a given step of the build. Step 0 is the
    /// slide as it arrives: everything that is not animated. Each click after
    /// that turns one more step on.
    pub fn shapeVisibleAt(&self, slide: &Rc<RefCell<PptxSlide>>, shape: &Rc<RefCell<PptxShape>>, step: i64) -> bool {
        let mut i: i64 = 0;
        while i < (slide.borrow().build.len() as i64) {
            let mut b: Rc<RefCell<PptxBuildStep>> = slide.borrow().build[i as usize].clone();
            if b.borrow().editId == shape.borrow().editId {
                if b.borrow().paragraph < 0 {
                    return step > i;
                }
            }
            i += 1;
        }
        true
    }
    /// --- sections -------------------------------------------------------------
    ///
    /// A run of slides under a name. The file names its members by slide id, so
    /// the deck answers "which section is slide 3 in" by asking which section
    /// holds that slide's id.
    pub fn sectionCount(&self) -> i64 {
        self.doc.borrow().sections.len() as i64
    }
    pub fn sectionNameAt(&self, i: i64) -> String {
        if i < 0 {
            return String::new();
        }
        if i >= (self.doc.borrow().sections.len() as i64) {
            return String::new();
        }
        self.doc.borrow().sections[i as usize].clone().borrow().name.clone()
    }
    pub fn ensureSlideIds(&self) {
        let mut next: i64 = 256;
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[i as usize].clone();
            if slide.borrow().slideId >= next {
                next = slide.borrow().slideId + 1;
            }
            i += 1;
        }
        let mut j: i64 = 0;
        while j < (self.doc.borrow().slides.len() as i64) {
            let mut s2: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[j as usize].clone();
            if s2.borrow().slideId == 0 {
                s2.borrow_mut().slideId = next;
                next += 1;
            }
            j += 1;
        }
    }
    /// The section a slide belongs to, or -1. Slides before the first section
    /// belong to none, which is what PowerPoint shows too.
    pub fn sectionOfSlide(&self, index: i64) -> i64 {
        if index < 0 {
            return 0 - 1;
        }
        if index >= (self.doc.borrow().slides.len() as i64) {
            return 0 - 1;
        }
        let id: i64 = self.doc.borrow()
            .slides[index as usize].clone()
            .borrow()
            .slideId;
        if id == 0 {
            return 0 - 1;
        }
        let mut si: i64 = 0;
        while si < (self.doc.borrow().sections.len() as i64) {
            let mut sec: Rc<RefCell<PptxSection>> = self.doc.borrow()
                .sections[si as usize].clone();
            let mut k: i64 = 0;
            while k < (sec.borrow().slideIds.len() as i64) {
                if sec.borrow().slideIds[k as usize] == id {
                    return si;
                }
                k += 1;
            }
            si += 1;
        }
        0 - 1
    }
    /// Where in the deck a section begins, or the slide count when none of its
    /// slides are in the deck any more.
    pub fn sectionStart(&self, sec: &Rc<RefCell<PptxSection>>) -> i64 {
        let mut best: i64 = self.doc.borrow().slides.len() as i64;
        let mut i: i64 = 0;
        while i < (self.doc.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[i as usize].clone();
            let mut k: i64 = 0;
            while k < (sec.borrow().slideIds.len() as i64) {
                if sec.borrow().slideIds[k as usize] == slide.borrow().slideId {
                    if i < best {
                        best = i;
                    }
                }
                k += 1;
            }
            i += 1;
        }
        best
    }
    /// Start a section at the slide in front of you: it runs from there to the
    /// slide before the next section begins, which is what cutting a deck into
    /// parts means — the slides in between change hands.
    pub fn addSection(&mut self, name: String) -> bool {
        self.ensureSlideIds();
        let mut limit: i64 = self.doc.borrow().slides.len() as i64;
        let mut li: i64 = 0;
        while li < (self.doc.borrow().sections.len() as i64) {
            let mut other0: Rc<RefCell<PptxSection>> = self.doc.borrow()
                .sections[li as usize].clone();
            let at: i64 = self.sectionStart(&other0);
            if at > self.slideIndex {
                if at < limit {
                    limit = at;
                }
            }
            li += 1;
        }
        let mut sec: Rc<RefCell<PptxSection>> = Rc::new(RefCell::new(PptxSection::new()));
        sec.borrow_mut().name = name.clone();
        let mut i: i64 = self.slideIndex;
        while i < limit {
            let mut slide: Rc<RefCell<PptxSlide>> = self.doc.borrow()
                .slides[i as usize].clone();
            sec.borrow_mut().slideIds.push(slide.borrow().slideId);
            i += 1;
        }
        // A slide can only be in one section, and a section with nothing left
        // in it is not a section any more.
        let mut kept: Vec<Rc<RefCell<PptxSection>>> = Vec::new();
        let mut si: i64 = 0;
        while si < (self.doc.borrow().sections.len() as i64) {
            let mut other: Rc<RefCell<PptxSection>> = self.doc.borrow()
                .sections[si as usize].clone();
            let mut stay: Vec<i64> = Vec::new();
            let mut k: i64 = 0;
            while k < (other.borrow().slideIds.len() as i64) {
                let id: i64 = other.borrow().slideIds[k as usize];
                let mut claimed: bool = false;
                let mut n: i64 = 0;
                while n < (sec.borrow().slideIds.len() as i64) {
                    if sec.borrow().slideIds[n as usize] == id {
                        claimed = true;
                    }
                    n += 1;
                }
                if !claimed {
                    stay.push(id);
                }
                k += 1;
            }
            other.borrow_mut().slideIds = stay.clone();
            if !stay.is_empty() {
                kept.push(other.clone());
            }
            si += 1;
        }
        kept.push(sec.clone());
        self.doc.borrow_mut().sections = kept.clone();
        self.sortSections();
        self.pushSnapshot("section");
        true
    }
    /// Sections read in deck order, so the panel can write each name once above
    /// the run it names.
    pub fn sortSections(&self) {
        let mut out: Vec<Rc<RefCell<PptxSection>>> = Vec::new();
        let mut used: Vec<bool> = Vec::new();
        let mut i: i64 = 0;
        while i < (self.doc.borrow().sections.len() as i64) {
            used.push(false);
            i += 1;
        }
        let mut taken: i64 = 0;
        while taken < (self.doc.borrow().sections.len() as i64) {
            let mut best: i64 = 0 - 1;
            let mut best_at: i64 = 0;
            let mut k: i64 = 0;
            while k < (self.doc.borrow().sections.len() as i64) {
                if !(used[k as usize]) {
                    let at: i64 = self.sectionStart(&self.doc.borrow().sections[k as usize].clone());
                    if best < 0 {
                        best = k;
                        best_at = at;
                    } else {
                        if at < best_at {
                            best = k;
                            best_at = at;
                        }
                    }
                }
                k += 1;
            }
            if best < 0 {
                taken = self.doc.borrow().sections.len() as i64;
            } else {
                used[best as usize] = true;
                out.push(self.doc.borrow().sections[best as usize].clone());
                taken += 1;
            }
        }
        self.doc.borrow_mut().sections = out.clone();
    }
    pub fn renameSection(&mut self, i: i64, name: String) -> bool {
        if i < 0 {
            return false;
        }
        if i >= (self.doc.borrow().sections.len() as i64) {
            return false;
        }
        let mut sec: Rc<RefCell<PptxSection>> = self.doc.borrow()
            .sections[i as usize].clone();
        sec.borrow_mut().name = name.clone();
        self.pushSnapshot("rename section");
        true
    }
    pub fn removeSection(&mut self, i: i64) -> bool {
        if i < 0 {
            return false;
        }
        if i >= (self.doc.borrow().sections.len() as i64) {
            return false;
        }
        let mut out: Vec<Rc<RefCell<PptxSection>>> = Vec::new();
        let mut k: i64 = 0;
        while k < (self.doc.borrow().sections.len() as i64) {
            if k != i {
                out.push(self.doc.borrow().sections[k as usize].clone());
            }
            k += 1;
        }
        self.doc.borrow_mut().sections = out.clone();
        self.pushSnapshot("remove section");
        true
    }
}
