// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::rgr::{SourceRef};



/// ==============================================================================
/// PptxModel - normalized PowerPoint scene (pre-EVG)
/// ==============================================================================
///
/// Units in the model are already converted from EMU:
///   pt = emu / 12700
/// Rotations are degrees (DrawingML stores 1/60000 deg).
///
/// Shape kinds:
///   0 = shape (rect/ellipse/preset + optional text)
///   1 = picture
///   2 = group
///   3 = connector (MVP: treated like a thin shape)
/// ==============================================================================
/// ------------------------------------------------------------------------------
/// What a slide holds that this model does not describe.
///
/// A shape tree can carry things no reader here understands: SmartArt and charts
/// wrapped in `mc:AlternateContent`, embedded objects, ink (`p:contentPart`),
/// and `p:extLst` blocks whose contents belong to whoever wrote them. The parser
/// walks past all of it, and a writer that regenerates the tree from the shapes
/// would therefore delete it.
///
/// So each one is remembered as WHERE it was, and put back exactly there:
///
///   source       the span in the slide part it came out of
///   afterShapes  how many shapes preceded it — its place in z-order
///   name         what it was called, for a person reading a diagnostic
///
/// Nothing is copied and nothing is understood. That is the point: `Ranger does
/// not have to understand something in order to preserve it.`
/// ------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PptxOpaque {
    source: Rc<RefCell<SourceRef>>,
    pub afterShapes: i64,
    pub name: String,
}

impl PptxOpaque {
    pub fn new() -> Self {
        Self {
            source: Rc::new(RefCell::new(SourceRef::new())),
            afterShapes: 0,
            name: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct PptxColor {
    pub isSet: bool,
    pub isScheme: bool,
    pub scheme: String,
    /// RRGGBB
    pub srgb: String,
    pub alpha: f64,
    /// DrawingML puts a colour through a chain of adjustments before it is
    /// drawn: `lumMod`/`lumOff` lighten or darken it, `tint` mixes it towards
    /// white and `shade` towards black, and `satMod` pushes the saturation. A
    /// theme's own fill styles are written entirely in these — the colour is a
    /// placeholder and the STYLE is the modifiers — so a reader that drops them
    /// draws every themed shape in the flat accent colour, or not at all.
    /// Each is a fraction; 1.0 means "not stated".
    pub lumMod: f64,
    pub lumOff: f64,
    pub tint: f64,
    pub shade: f64,
    pub satMod: f64,
    /// The adjustments have already been carried out and `srgb` is the answer.
    /// A colour that came out of a theme STYLE is two chains of modifiers deep,
    /// which no single set of modifiers can express — so it is baked, and the
    /// resolver leaves it alone rather than resolving it back to the flat
    /// palette colour it started from.
    pub baked: bool,
}

impl PptxColor {
    pub fn new() -> Self {
        Self {
            isSet: false,
            isScheme: false,
            scheme: String::new(),
            srgb: String::new(),
            alpha: 1.0,
            lumMod: 1.0,
            lumOff: 0.0,
            tint: 0.0,
            shade: 0.0,
            satMod: 1.0,
            baked: false,
        }
    }
    pub fn srgbOf(hex: String) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        c.borrow_mut().isSet = true;
        c.borrow_mut().isScheme = false;
        c.borrow_mut().srgb = hex.clone();
        c
    }
    pub fn schemeOf(name: String) -> Rc<RefCell<PptxColor>> {
        let mut c: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        c.borrow_mut().isSet = true;
        c.borrow_mut().isScheme = true;
        c.borrow_mut().scheme = name.clone();
        c
    }
    /// A string that changes when anything about this colour does. Used to tell
    /// a value the resolver worked out from one the file stated — see the note
    /// on `styleFillSig`.
    pub fn signature(&self) -> String {
        if !self.isSet {
            return "-".to_string();
        }
        let mut out: String = self.srgb.clone();
        if self.isScheme {
            out = format!("#{}", self.scheme);
        }
        if self.baked {
            out = format!("{}!", out);
        }
        if self.hasMods() {
            out = format!("{}~{},{},", out, (self.lumMod * 1000.0 + 0.5).floor() as i64, ((self.lumOff * 1000.0 + 0.5).floor() as i64).to_string());
            out = format!("{}{},", out, ((self.tint * 1000.0 + 0.5).floor() as i64).to_string());
            out = format!("{}{},", out, ((self.shade * 1000.0 + 0.5).floor() as i64).to_string());
            out = format!("{}{}", out, (self.satMod * 1000.0 + 0.5).floor() as i64);
        }
        if self.alpha < 0.999 {
            out = format!("{}/{}", out, (self.alpha * 1000.0 + 0.5).floor() as i64);
        }
        out
    }
    pub fn hasMods(&self) -> bool {
        if self.lumMod != 1.0 {
            return true;
        }
        if self.lumOff != 0.0 {
            return true;
        }
        if self.tint != 0.0 {
            return true;
        }
        if self.shade != 0.0 {
            return true;
        }
        if self.satMod != 1.0 {
            return true;
        }
        false
    }
    pub fn copyModsFrom(&mut self, src: &Rc<RefCell<PptxColor>>) {
        self.lumMod = src.borrow().lumMod;
        self.lumOff = src.borrow().lumOff;
        self.tint = src.borrow().tint;
        self.shade = src.borrow().shade;
        self.satMod = src.borrow().satMod;
    }
}

/// Two-stop linear gradient (SoftCanvas fillRoundRectGrad).
#[derive(Clone)]
pub struct PptxGradient {
    pub isSet: bool,
    /// 0 = vertical, 1 = horizontal
    pub dir: i64,
    pub c1: Rc<RefCell<PptxColor>>,
    pub c2: Rc<RefCell<PptxColor>>,
}

impl PptxGradient {
    pub fn new() -> Self {
        Self {
            isSet: false,
            dir: 0,
            c1: Rc::new(RefCell::new(PptxColor::new())),
            c2: Rc::new(RefCell::new(PptxColor::new())),
        }
    }
}

/// One entry of the theme's `fmtScheme`. A shape that carries a `<p:style>`
/// rather than its own fill is asking for one of these, with the placeholder
/// colour `phClr` replaced by the colour the reference names — which is how a
/// deck gets a gradient on a shape whose own XML says nothing about gradients.
#[derive(Clone)]
pub struct PptxFillStyle {
    pub isSet: bool,
    /// 0 = solid, 1 = gradient, 2 = none
    pub kind: i64,
    /// For a solid: the modifiers to put the reference colour through.
    pub solid: Rc<RefCell<PptxColor>>,
    /// For a gradient: the two ends, again as modifiers on the reference colour,
    /// and the direction the theme states.
    pub c1: Rc<RefCell<PptxColor>>,
    pub c2: Rc<RefCell<PptxColor>>,
    pub dir: i64,
}

impl PptxFillStyle {
    pub fn new() -> Self {
        Self {
            isSet: false,
            kind: 0,
            solid: Rc::new(RefCell::new(PptxColor::new())),
            c1: Rc::new(RefCell::new(PptxColor::new())),
            c2: Rc::new(RefCell::new(PptxColor::new())),
            dir: 0,
        }
    }
}

#[derive(Clone)]
pub struct PptxLineStyle {
    pub isSet: bool,
    pub widthPt: f64,
    pub color: Rc<RefCell<PptxColor>>,
}

impl PptxLineStyle {
    pub fn new() -> Self {
        Self {
            isSet: false,
            widthPt: 1.0,
            color: Rc::new(RefCell::new(PptxColor::new())),
        }
    }
}

#[derive(Clone)]
pub struct PptxShadow {
    pub isSet: bool,
    pub offsetXPt: f64,
    pub offsetYPt: f64,
    pub blurPt: f64,
    pub color: Rc<RefCell<PptxColor>>,
    pub alpha: f64,
}

impl PptxShadow {
    pub fn new() -> Self {
        Self {
            isSet: false,
            offsetXPt: 3.0,
            offsetYPt: 3.0,
            blurPt: 4.0,
            color: Rc::new(RefCell::new(PptxColor::new())),
            alpha: 0.35,
        }
    }
}

#[derive(Clone)]
pub struct PptxTextRun {
    pub text: String,
    /// `a:highlight` — the colour drawn BEHIND this run. A deck uses it the way
    /// a marker pen is used, and a reader that skips it loses the emphasis
    /// entirely rather than drawing it wrong.
    pub highlight: Rc<RefCell<PptxColor>>,
    /// `a:fld` — a run whose text is computed rather than typed. "slidenum" is
    /// the one that matters: the file carries the number it had when it was
    /// saved, and a viewer has to substitute the one the slide has now.
    pub field: String,
    /// An equation: the children of an `m:oMath`, Office Math as PowerPoint
    /// keeps it. `text` is then the same formula as plain Unicode, which is
    /// what a reader with no equations shows instead — the writer puts both in
    /// the file, the equation as the choice and the text as its fallback.
    pub math: String,
    /// …set as a display (`m:oMathPara`): limits under the sum, a full-size
    /// fraction, the formula centred on its own line.
    pub mathDisplay: bool,
    pub fontFamily: String,
    pub fontSizePt: f64,
    pub sizeSet: bool,
    /// What the RESOLVER put here, for a run that stated nothing of its own —
    /// from the list style, the body default or the theme. Signatures rather
    /// than flags for the reason given on `PptxShape.styleFillSig`: a flag
    /// would stay set after the user changed the font, and the save would then
    /// discard the change. These stop matching the moment anything moves.
    /// Whether the run's own `rPr` carried a colour. Not the same as
    /// `color.isSet`: the resolver fills a colour in from the list style before
    /// the shape's `<p:style><a:fontRef>` gets a turn, and by then a run that
    /// said nothing is indistinguishable from one that asked for black. This is
    /// set by the PARSER and by the editor, and by nothing that infers.
    pub colorStated: bool,
    pub resolvedFontSig: String,
    pub resolvedColorSig: String,
    pub resolvedSizeSig: String,
    /// Whether the run SAID anything about its weight and slant, alongside what
    /// it said. `b="0"` and no `b` at all are different documents: the first
    /// takes a word out of a bold placeholder, the second lets the placeholder
    /// decide. Testing `bold == false` cannot tell them apart, and the
    /// inheritance below used to do exactly that — so a run that un-bolded
    /// itself came back bold. Same idea, and the same spelling, as `sizeSet`.
    pub boldSet: bool,
    pub bold: bool,
    pub italicSet: bool,
    pub italic: bool,
    pub color: Rc<RefCell<PptxColor>>,
    /// A soft edge round the letters (`a:glow`), radius in points; zero for
    /// none. What keeps a heading readable over a busy picture.
    pub glow: Rc<RefCell<PptxColor>>,
    pub glowPt: f64,
    /// `a:hlinkClick`: where a click on this run goes. `link` an address
    /// outside the deck (written as an external hyperlink); `linkSlide` a
    /// slide of this deck, by its index in `pres.slides` (a slide jump), -1
    /// for none. A `#fragment` nobody resolved to a slide is not written:
    /// outside a browser it goes nowhere.
    pub link: String,
    pub linkSlide: i64,
}

impl PptxTextRun {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            highlight: Rc::new(RefCell::new(PptxColor::new())),
            field: String::new(),
            math: String::new(),
            mathDisplay: false,
            fontFamily: String::new(),
            fontSizePt: 18.0,
            sizeSet: false,
            colorStated: false,
            resolvedFontSig: String::new(),
            resolvedColorSig: String::new(),
            resolvedSizeSig: String::new(),
            boldSet: false,
            bold: false,
            italicSet: false,
            italic: false,
            color: Rc::new(RefCell::new(PptxColor::new())),
            glow: Rc::new(RefCell::new(PptxColor::new())),
            glowPt: 0.0,
            link: String::new(),
            linkSlide: -1,
        }
    }
    /// Every style property of this run, onto another one — the whole of it,
    /// and neither the text nor the field.
    ///
    /// Splitting a run used to copy eight of the fourteen fields by hand and
    /// drop the rest, and the three it dropped were the RESOLUTION SIGNATURES:
    /// the record of what this run's size, colour and font were derived from.
    /// A half with no signature is a half the resolver reads as never having
    /// been resolved, so it inherits from the layout again — and a body
    /// placeholder's layout size is not the size the run was showing. Toggling
    /// bold on part of a line split the run and the text jumped to another
    /// size.
    ///
    /// A hand-written list of fields goes stale the first time somebody adds
    /// one, and it had: the signatures were added to make the writer able to
    /// tell an inherited value from an authored one, and nothing taught this
    /// about them. One method, next to the fields.
    pub fn copyStyleTo(&self, o: &Rc<RefCell<PptxTextRun>>) {
        o.borrow_mut().fontFamily = self.fontFamily.clone();
        o.borrow_mut().fontSizePt = self.fontSizePt;
        o.borrow_mut().sizeSet = self.sizeSet;
        o.borrow_mut().boldSet = self.boldSet;
        o.borrow_mut().bold = self.bold;
        o.borrow_mut().italicSet = self.italicSet;
        o.borrow_mut().italic = self.italic;
        o.borrow_mut().resolvedFontSig = self.resolvedFontSig.clone();
        o.borrow_mut().resolvedColorSig = self.resolvedColorSig.clone();
        o.borrow_mut().resolvedSizeSig = self.resolvedSizeSig.clone();
        o.borrow_mut().glow = self.glow.clone();
        o.borrow_mut().glowPt = self.glowPt;
        o.borrow_mut().link = self.link.clone();
        o.borrow_mut().linkSlide = self.linkSlide;
    }
}

#[derive(Clone)]
pub struct PptxParagraph {
    /// l / ctr / r / just
    pub align: String,
    pub alignSet: bool,
    /// `<a:pPr rtl="1">`: which way the paragraph runs. -1 nobody said, 0 left
    /// to right, 1 right to left. Three states because a paragraph inherits
    /// from the list style and from the layout, and "unset" has to be
    /// distinguishable from "explicitly left to right" for either to be
    /// overridable.
    ///
    /// It decides which edge an unaligned paragraph is set against and — where
    /// the text alone would answer wrongly, a line that opens with a Latin
    /// product name and is otherwise Arabic — which direction the line reads.
    pub rtl: i64,
    pub spaceBeforePt: f64,
    pub spaceAfterPt: f64,
    /// 100 = single
    pub lineSpacingPct: f64,
    pub level: i64,
    pub hasBullet: bool,
    /// A NUMBERED list rather than a dotted one. `<a:buAutoNum>` says the
    /// scheme — "arabicPeriod" is 1. 2. 3., "alphaLcParenR" is a) b) c) — and
    /// where the count starts. The number itself is not in the file: it is the
    /// paragraph's position among the ones like it, which is why a viewer that
    /// drew the stored character instead put a dot in front of every item.
    pub bulletAutoNum: bool,
    pub bulletScheme: String,
    pub bulletStartAt: i64,
    pub bulletChar: String,
    /// Where the level's indent puts this paragraph, and what the bullet in
    /// front of it looks like — both inherited from the list style unless the
    /// paragraph states them itself.
    pub marginLeftPt: f64,
    pub marginSet: bool,
    pub indentPt: f64,
    pub indentSet: bool,
    /// The paragraph said what its bullet is — including that it has none. A
    /// level style must not put one back: `<a:buNone/>` on the paragraph is how
    /// a deck turns a list style's bullet off for one line.
    pub bulletSet: bool,
    pub bulletColor: Rc<RefCell<PptxColor>>,
    pub bulletSizePct: f64,
    /// A bullet is sized either as a share of the text or in points, and a deck
    /// picks one; zero here means it did not pick this one.
    pub bulletSizePt: f64,
    pub bulletFont: String,
    pub runs: Vec<Rc<RefCell<PptxTextRun>>>,
}

impl PptxParagraph {
    pub fn new() -> Self {
        let mut me = Self {
            align: "l".to_string(),
            alignSet: false,
            rtl: -1,
            spaceBeforePt: 0.0,
            spaceAfterPt: 0.0,
            lineSpacingPct: 100.0,
            level: 0,
            hasBullet: false,
            bulletAutoNum: false,
            bulletScheme: "arabicPeriod".to_string(),
            bulletStartAt: 1,
            bulletChar: "•".to_string(),
            marginLeftPt: 0.0,
            marginSet: false,
            indentPt: 0.0,
            indentSet: false,
            bulletSet: false,
            bulletColor: Rc::new(RefCell::new(PptxColor::new())),
            bulletSizePct: 100.0,
            bulletSizePt: 0.0,
            bulletFont: String::new(),
            runs: Vec::new(),
        };
        let mut r: Vec<Rc<RefCell<PptxTextRun>>> = Vec::new();
        me.runs = r.clone();
        me
    }
    /// Every formatting property of this paragraph, onto another one — the
    /// whole of it, and none of the text.
    ///
    /// Splitting a paragraph used to copy seven of these by hand and drop the
    /// rest, so pressing Enter at the end of a bulleted line gave a paragraph
    /// that had lost its bullet COLOUR — the dash went from the deck's white to
    /// a default black — along with its bullet font, its indent, its numbering
    /// scheme and its direction.
    ///
    /// A hand-written list of fields is a list that goes stale the first time
    /// somebody adds one, and it had: `rtl` was added for right-to-left
    /// paragraphs and was never copied, so splitting an Arabic paragraph turned
    /// the second half around. One method, next to the fields, is the only
    /// version of this that stays true.
    pub fn copyFormatTo(&self, o: &Rc<RefCell<PptxParagraph>>) {
        o.borrow_mut().align = self.align.clone();
        o.borrow_mut().alignSet = self.alignSet;
        o.borrow_mut().rtl = self.rtl;
        o.borrow_mut().spaceBeforePt = self.spaceBeforePt;
        o.borrow_mut().spaceAfterPt = self.spaceAfterPt;
        o.borrow_mut().lineSpacingPct = self.lineSpacingPct;
        o.borrow_mut().level = self.level;
        o.borrow_mut().hasBullet = self.hasBullet;
        o.borrow_mut().bulletAutoNum = self.bulletAutoNum;
        o.borrow_mut().bulletScheme = self.bulletScheme.clone();
        o.borrow_mut().bulletStartAt = self.bulletStartAt;
        o.borrow_mut().bulletChar = self.bulletChar.clone();
        o.borrow_mut().marginLeftPt = self.marginLeftPt;
        o.borrow_mut().marginSet = self.marginSet;
        o.borrow_mut().indentPt = self.indentPt;
        o.borrow_mut().indentSet = self.indentSet;
        o.borrow_mut().bulletSet = self.bulletSet;
        // The colour is copied field by field rather than by reference: two
        // paragraphs sharing one colour object means recolouring one bullet
        // recolours the other, which is a bug that only shows up later and
        // somewhere else.
        o.borrow_mut().bulletColor.borrow_mut().isSet = self.bulletColor.borrow().isSet;
        o.borrow_mut().bulletColor.borrow_mut().isScheme = self.bulletColor.borrow().isScheme;
        o.borrow_mut().bulletColor.borrow_mut().scheme = self.bulletColor.borrow().scheme.clone();
        o.borrow_mut().bulletColor.borrow_mut().srgb = self.bulletColor.borrow().srgb.clone();
        o.borrow_mut().bulletColor.borrow_mut().alpha = self.bulletColor.borrow().alpha;
        let mut _tmp_15 = self.bulletColor.clone();
        o.borrow().bulletColor.clone().borrow_mut().copyModsFrom(&_tmp_15);
        o.borrow_mut().bulletSizePct = self.bulletSizePct;
        o.borrow_mut().bulletSizePt = self.bulletSizePt;
        o.borrow_mut().bulletFont = self.bulletFont.clone();
    }
}

/// One indentation level of a list style. DrawingML states nine of them, in
/// `a:lvl1pPr` … `a:lvl9pPr`, and a deck's whole typographic scheme lives in
/// them: the size, the colour, the indent and the bullet a paragraph gets when
/// it says nothing itself. Reading only the first level — which is what this did
/// — draws every sub-bullet in the top level's size and colour.
///
/// Every field is paired with a `…Set` flag rather than defaulting, because the
/// levels are MERGED down a chain (master → layout → shape → paragraph) and a
/// value that was never stated must not overwrite one that was.
#[derive(Clone)]
pub struct PptxLevelStyle {
    pub align: String,
    pub alignSet: bool,
    pub marginLeftPt: f64,
    pub marginSet: bool,
    pub indentPt: f64,
    pub indentSet: bool,
    pub fontSizePt: f64,
    pub sizeSet: bool,
    pub bold: bool,
    pub boldSet: bool,
    pub italic: bool,
    pub italicSet: bool,
    pub fontFamily: String,
    pub color: Rc<RefCell<PptxColor>>,
    pub lineSpacingPct: f64,
    pub lineSpacingSet: bool,
    pub spaceBeforePt: f64,
    pub spaceBeforeSet: bool,
    pub spaceAfterPt: f64,
    pub spaceAfterSet: bool,
    /// The bullet: "" nothing stated, "none", "char", "autonum".
    pub bulletKind: String,
    pub bulletScheme: String,
    pub bulletStartAt: i64,
    pub bulletChar: String,
    pub bulletFont: String,
    pub bulletColor: Rc<RefCell<PptxColor>>,
    /// A bullet is sized either as a percentage of the text or in points.
    pub bulletSizePct: f64,
    pub bulletSizePctSet: bool,
    pub bulletSizePt: f64,
    pub bulletSizePtSet: bool,
}

impl PptxLevelStyle {
    pub fn new() -> Self {
        Self {
            align: "l".to_string(),
            alignSet: false,
            marginLeftPt: 0.0,
            marginSet: false,
            indentPt: 0.0,
            indentSet: false,
            fontSizePt: 18.0,
            sizeSet: false,
            bold: false,
            boldSet: false,
            italic: false,
            italicSet: false,
            fontFamily: String::new(),
            color: Rc::new(RefCell::new(PptxColor::new())),
            lineSpacingPct: 100.0,
            lineSpacingSet: false,
            spaceBeforePt: 0.0,
            spaceBeforeSet: false,
            spaceAfterPt: 0.0,
            spaceAfterSet: false,
            bulletKind: String::new(),
            bulletScheme: "arabicPeriod".to_string(),
            bulletStartAt: 1,
            bulletChar: "•".to_string(),
            bulletFont: String::new(),
            bulletColor: Rc::new(RefCell::new(PptxColor::new())),
            bulletSizePct: 100.0,
            bulletSizePctSet: false,
            bulletSizePt: 0.0,
            bulletSizePtSet: false,
        }
    }
    /// Take everything the other level states, leaving what it does not alone.
    /// This is the whole of inheritance: the more specific style is applied over
    /// the less specific one, field by field.
    pub fn mergeFrom(&mut self, o: &Rc<RefCell<PptxLevelStyle>>) {
        if o.borrow().alignSet {
            self.align = o.borrow().align.clone();
            self.alignSet = true;
        }
        if o.borrow().marginSet {
            self.marginLeftPt = o.borrow().marginLeftPt;
            self.marginSet = true;
        }
        if o.borrow().indentSet {
            self.indentPt = o.borrow().indentPt;
            self.indentSet = true;
        }
        if o.borrow().sizeSet {
            self.fontSizePt = o.borrow().fontSizePt;
            self.sizeSet = true;
        }
        if o.borrow().boldSet {
            self.bold = o.borrow().bold;
            self.boldSet = true;
        }
        if o.borrow().italicSet {
            self.italic = o.borrow().italic;
            self.italicSet = true;
        }
        if !o.borrow().fontFamily.is_empty() {
            self.fontFamily = o.borrow().fontFamily.clone();
        }
        if o.borrow().color.borrow().isSet {
            self.color = o.borrow().color.clone();
        }
        if o.borrow().lineSpacingSet {
            self.lineSpacingPct = o.borrow().lineSpacingPct;
            self.lineSpacingSet = true;
        }
        if o.borrow().spaceBeforeSet {
            self.spaceBeforePt = o.borrow().spaceBeforePt;
            self.spaceBeforeSet = true;
        }
        if o.borrow().spaceAfterSet {
            self.spaceAfterPt = o.borrow().spaceAfterPt;
            self.spaceAfterSet = true;
        }
        if !o.borrow().bulletKind.is_empty() {
            self.bulletKind = o.borrow().bulletKind.clone();
            self.bulletScheme = o.borrow().bulletScheme.clone();
            self.bulletStartAt = o.borrow().bulletStartAt;
            self.bulletChar = o.borrow().bulletChar.clone();
        }
        if !o.borrow().bulletFont.is_empty() {
            self.bulletFont = o.borrow().bulletFont.clone();
        }
        if o.borrow().bulletColor.borrow().isSet {
            self.bulletColor = o.borrow().bulletColor.clone();
        }
        if o.borrow().bulletSizePctSet {
            self.bulletSizePct = o.borrow().bulletSizePct;
            self.bulletSizePctSet = true;
        }
        if o.borrow().bulletSizePtSet {
            self.bulletSizePt = o.borrow().bulletSizePt;
            self.bulletSizePtSet = true;
        }
    }
}

/// The nine levels together. A shape has one, a layout placeholder has one, and
/// the master has THREE — one for titles, one for bodies and one for everything
/// else — which is where a deck's typography actually lives.
#[derive(Clone)]
pub struct PptxListStyle {
    pub isSet: bool,
    pub levels: Vec<Rc<RefCell<PptxLevelStyle>>>,
}

impl PptxListStyle {
    pub fn new() -> Self {
        let mut me = Self {
            isSet: false,
            levels: Vec::new(),
        };
        let mut l: Vec<Rc<RefCell<PptxLevelStyle>>> = Vec::new();
        me.levels = l.clone();
        let mut i: i64 = 0;
        while i < 9 {
            me.levels.push(Rc::new(RefCell::new(PptxLevelStyle::new())));
            i += 1;
        }
        me
    }
    pub fn at(&self, level: i64) -> Rc<RefCell<PptxLevelStyle>> {
        let mut i: i64 = level;
        if i < 0 {
            i = 0;
        }
        if i > 8 {
            i = 8;
        }
        self.levels[i as usize].clone()
    }
    pub fn mergeFrom(&mut self, o: &Rc<RefCell<PptxListStyle>>) {
        if !o.borrow().isSet {
            return;
        }
        self.isSet = true;
        let mut i: i64 = 0;
        while i < 9 {
            let mut mine: Rc<RefCell<PptxLevelStyle>> = self.levels[i as usize].clone();
            mine.borrow_mut().mergeFrom(&o.borrow().levels[i as usize].clone());
            i += 1;
        }
    }
}

#[derive(Clone)]
pub struct PptxTextBody {
    pub paragraphs: Vec<Rc<RefCell<PptxParagraph>>>,
    /// `a:bodyPr` insets, in points, and the four are NOT the same by default.
    /// ECMA-376 gives lIns/rIns = 91440 EMU (7.2pt = 0.1") and tIns/bIns =
    /// 45720 EMU (3.6pt = 0.05"). All four were 7.2 here, which reserved 7.2pt
    /// too much height in every text box that does not state its own insets: a
    /// 40pt title in the 55.7pt box PowerPoint had auto-fitted to it no longer
    /// fitted, so it overflowed downward from a top anchor and the title came
    /// out sitting on the bottom edge of its box.
    pub insetL: f64,
    pub insetT: f64,
    pub insetR: f64,
    pub insetB: f64,
    pub insetsSet: bool,
    /// t / ctr / b
    pub anchor: String,
    pub anchorSet: bool,
    pub wrap: bool,
    /// `a:bodyPr@numCol` / `@spcCol` — PowerPoint's own text columns. A text
    /// body flows down the first column and into the next, which is what a
    /// multi-column list on a slide IS; the alternative is one shape per
    /// column, and three shapes are not one list.
    pub columns: i64,
    pub columnGapPt: f64,
    /// Default run props from a:lstStyle / a:defRPr (placeholder style inheritance)
    pub hasDefRPr: bool,
    pub defFontSizePt: f64,
    /// The same distinction one level up: a `defRPr` that says `b="0"` turns
    /// bold off for the body, and one that says nothing lets the placeholder
    /// through.
    pub defBoldSet: bool,
    pub defBold: bool,
    pub defItalicSet: bool,
    pub defItalic: bool,
    pub defFontFamily: String,
    pub defColor: Rc<RefCell<PptxColor>>,
    /// Whether `defColor` came from the shape's own `<p:style><a:fontRef>`
    /// rather than from inheritance. It matters because that one outranks the
    /// master's default text style, and nothing else about a default does.
    pub defColorFromStyle: bool,
    pub defAlign: String,
    /// The nine levels this body states, before anything is inherited into
    /// them. The resolver merges the master's and the layout's over it.
    pub listStyle: Rc<RefCell<PptxListStyle>>,
    /// `a:normAutofit` — PowerPoint shrank the text to make it fit, and wrote
    /// down by how much. A reader that ignores it draws the text at full size in
    /// a box that was sized for the shrunken version, which overflows.
    pub fontScale: f64,
    pub lineScale: f64,
}

impl PptxTextBody {
    pub fn new() -> Self {
        let mut me = Self {
            paragraphs: Vec::new(),
            insetL: 7.2,
            insetT: 3.6,
            insetR: 7.2,
            insetB: 3.6,
            insetsSet: false,
            anchor: "t".to_string(),
            anchorSet: false,
            wrap: true,
            columns: 1,
            columnGapPt: 0.0,
            hasDefRPr: false,
            defFontSizePt: 18.0,
            defBoldSet: false,
            defBold: false,
            defItalicSet: false,
            defItalic: false,
            defFontFamily: String::new(),
            defColor: Rc::new(RefCell::new(PptxColor::new())),
            defColorFromStyle: false,
            defAlign: "l".to_string(),
            listStyle: Rc::new(RefCell::new(PptxListStyle::new())),
            fontScale: 1.0,
            lineScale: 1.0,
        };
        let mut p: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
        me.paragraphs = p.clone();
        me
    }
    pub fn plainText(&self) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (self.paragraphs.len() as i64) {
            if i > 0 {
                out = format!("{}\n", out);
            }
            let mut para: Rc<RefCell<PptxParagraph>> = self.paragraphs[i as usize].clone();
            let mut j: i64 = 0;
            while j < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[j as usize].clone();
                out = format!("{}{}", out, run.borrow().text);
                j += 1;
            }
            i += 1;
        }
        out
    }
}

/// One edge of a cell. A table style says most of what a table looks like
/// through these, and a table drawn with a grey box round every cell instead of
/// the edges it actually states looks nothing like the deck it came from.
#[derive(Clone)]
pub struct PptxTableLine {
    pub isSet: bool,
    pub widthPt: f64,
    pub color: Rc<RefCell<PptxColor>>,
}

impl PptxTableLine {
    pub fn new() -> Self {
        Self {
            isSet: false,
            widthPt: 1.0,
            color: Rc::new(RefCell::new(PptxColor::new())),
        }
    }
}

#[derive(Clone)]
pub struct PptxTableCell {
    /// The cell's words, as a text body — so wrapping, runs, colours, sizes and
    /// alignment are the ones the rest of this renderer already knows how to do.
    /// `text` is the same content flattened, which is what a harness reads.
    pub text: String,
    pub body: Rc<RefCell<PptxTextBody>>,
    pub fill: Rc<RefCell<PptxColor>>,
    pub bold: bool,
    pub fontSizePt: f64,
    pub align: String,
    /// `a:tcPr` — where the words sit in the cell and how far from its edges.
    pub anchor: String,
    pub marLPt: f64,
    pub marRPt: f64,
    pub marTPt: f64,
    pub marBPt: f64,
    pub lnL: Rc<RefCell<PptxTableLine>>,
    pub lnR: Rc<RefCell<PptxTableLine>>,
    pub lnT: Rc<RefCell<PptxTableLine>>,
    pub lnB: Rc<RefCell<PptxTableLine>>,
    /// A merged cell: how many columns and rows it covers, and whether it is one
    /// of the covered ones (which draws nothing).
    pub gridSpan: i64,
    pub rowSpan: i64,
    pub merged: bool,
}

impl PptxTableCell {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            body: Rc::new(RefCell::new(PptxTextBody::new())),
            fill: Rc::new(RefCell::new(PptxColor::new())),
            bold: false,
            fontSizePt: 14.0,
            align: "l".to_string(),
            anchor: "t".to_string(),
            marLPt: 7.2,
            marRPt: 7.2,
            marTPt: 3.6,
            marBPt: 3.6,
            lnL: Rc::new(RefCell::new(PptxTableLine::new())),
            lnR: Rc::new(RefCell::new(PptxTableLine::new())),
            lnT: Rc::new(RefCell::new(PptxTableLine::new())),
            lnB: Rc::new(RefCell::new(PptxTableLine::new())),
            gridSpan: 1,
            rowSpan: 1,
            merged: false,
        }
    }
}

#[derive(Clone)]
pub struct PptxChartSeries {
    pub name: String,
    pub values: Vec<f64>,
}

impl PptxChartSeries {
    pub fn new() -> Self {
        let mut me = Self {
            name: String::new(),
            values: Vec::new(),
        };
        let mut v: Vec<f64> = Vec::new();
        me.values = v.clone();
        me
    }
}

#[derive(Clone)]
pub struct PptxChart {
    /// bar / col / line / pie
    pub chartType: String,
    pub title: String,
    pub categories: Vec<String>,
    pub series: Vec<Rc<RefCell<PptxChartSeries>>>,
}

impl PptxChart {
    pub fn new() -> Self {
        let mut me = Self {
            chartType: "bar".to_string(),
            title: String::new(),
            categories: Vec::new(),
            series: Vec::new(),
        };
        let mut c: Vec<String> = Vec::new();
        me.categories = c.clone();
        let mut s: Vec<Rc<RefCell<PptxChartSeries>>> = Vec::new();
        me.series = s.clone();
        me
    }
}

/// ------------------------------------------------------------------------------
/// What a shape on a slide is.
///
/// The comment that used to sit here said the same thing, and a comment is not
/// something the compiler checks. These are read in the parser, the resolver,
/// the editor, the painter and the writer — six places, all of which had the
/// table written out as bare numbers, and any one of them could drift from the
/// others without anything noticing until a shape stopped being drawn.
/// ------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PptxShapeKind {
}

impl PptxShapeKind {
    pub fn new() -> Self {
        Self {
        }
    }
    /// `p:sp` — an autoshape, a placeholder, a text box.
    pub fn shape() -> i64 {
        0
    }
    /// `p:pic` — a picture. `imagePart` and `imageBytes` carry it.
    pub fn picture() -> i64 {
        1
    }
    /// `p:grpSp` — a group. Its `children` are shapes in their own right.
    pub fn group() -> i64 {
        2
    }
    /// `p:cxnSp` — a connector.
    pub fn connector() -> i64 {
        3
    }
    /// A `p:graphicFrame` holding `a:tbl`.
    pub fn table() -> i64 {
        4
    }
    /// A `p:graphicFrame` holding a chart reference.
    pub fn chart() -> i64 {
        5
    }
}

#[derive(Clone)]
pub struct PptxShape {
    pub kind: i64,
    pub name: String,
    /// The author's alt text, from `p:cNvPr/@descr`. It is the ONLY thing a
    /// screen reader has to go on for a picture, a chart or a diagram — the
    /// pixels tell it nothing — and it used to be read past and then written
    /// back out as nothing, so a deck whose author had written proper alt text
    /// lost it the first time it was edited here.
    pub altText: String,
    /// What this shape was GENERATED FROM, when something generated it.
    ///
    /// A diagram written as a ```mermaid fence reaches a slide as a group of
    /// paths and text runs. Nothing can read that back: a class diagram and a
    /// sequence diagram draw the same rectangles, so intent cannot be
    /// reconstructed from strokes. Without the source, a deck can be moved and
    /// resized and never edited again.
    ///
    /// Carried in `p:cNvPr/a:extLst`, which is the place OOXML sets aside for
    /// exactly this: an application's own data, which other applications are
    /// required to preserve rather than understand. Alt text was the other
    /// candidate and is the wrong one — it is the accessibility field, and a
    /// screen reader would read a Mermaid source aloud.
    pub embedNotation: String,
    pub embedSource: String,
    /// A heading this shape was built from, as offsets into the markdown.
    /// -1 means it has no heading to write back to. `mdLastText` is the
    /// words that were last synced, so a retype can be told from a look.
    pub mdSrcStart: i64,
    pub mdSrcEnd: i64,
    pub mdLastText: String,
    /// `p:cNvPr/@id`, which is the name the slide's own timing tree uses to say
    /// which shape an animation is about.
    pub shapeId: i64,
    /// Stable handle for an editor. The parser never sets it — `PptxEditor`
    /// hands one out per shape when a deck is attached, because an index into
    /// `shapes` stops naming the same shape the moment z-order changes.
    pub editId: i64,
    /// pt
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// true when spPr/xfrm supplied geometry
    pub hasXfrm: bool,
    /// WHICH HALF of the xfrm the file actually stated.
    ///
    /// `a:off` and `a:ext` are separate elements and a shape can state one
    /// without the other — a placeholder that has been moved on the slide but
    /// still takes its size from the layout states only `a:off`. Reading the
    /// pair as one flag made "the file said something about geometry" mean "the
    /// file said everything about geometry", so the half it did not state was
    /// inherited as zero: a text box no wider than nothing, its words laid out
    /// one per line down the left edge, and nothing to click because a box of
    /// no width contains no point. That is the report these two exist for.
    /// the xfrm stated a:off
    pub hasOff: bool,
    /// …and a:ext, with a positive size
    pub hasExt: bool,
    /// The box is this READER's, not the file's: nothing in the slide, the
    /// layout or the master ever gave this shape a size, and it carries text
    /// that has to be readable and selectable anyway. `PptxResolver.fallbackBox`
    /// is the only thing that sets it, and the writer never states such a box —
    /// `sheetXfrmSig` covers it the same way it covers inherited geometry.
    pub geomFallback: bool,
    /// degrees
    pub rotation: f64,
    /// `a:xfrm flipH` / `flipV`: the shape drawn mirrored in its own box.
    pub flipH: bool,
    pub flipV: bool,
    /// rect / ellipse / roundRect / …
    pub preset: String,
    /// `a:avLst/a:gd name="adj"` — the shape's own adjustment handle, as a
    /// fraction. A rounded rectangle's corner radius is this times the shorter
    /// side, which is why every roundRect used to have the same corners
    /// whatever the file said.
    pub adj: f64,
    /// Every adjustment the file states, by name.
    ///
    /// `adj` above is one value under one name, which is what the reader used
    /// to keep, and it is enough for exactly the shapes that have one
    /// adjustment. A callout has four. An arrow has two — head length and
    /// shaft width — and reading only the first gives every arrow in a deck
    /// the author's head with the default shaft.
    pub adjNames: Vec<String>,
    pub adjValues: Vec<f64>,
    /// `<p:style>`: which theme fill, line, effect and font this shape asks for,
    /// and the colour each reference names. Zero means the shape said nothing.
    /// `<a:noFill/>` stated on the shape itself, which beats whatever the
    /// theme's style reference would have given it.
    pub noFill: bool,
    pub noLine: bool,
    /// `a:headEnd` / `a:tailEnd` — what is drawn at the ends of a line. "none"
    /// or empty means nothing; "triangle", "arrow", "stealth", "oval" and
    /// "diamond" are the shapes DrawingML names.
    pub headEnd: String,
    pub tailEnd: String,
    /// `<p:style>` — the shape's link to the theme's format matrix. A shape
    /// that has one follows the theme: change the theme in PowerPoint and the
    /// shape changes with it. The writer used to drop the block and state the
    /// RESOLVED gradient and font instead, so editing one shape on a slide
    /// unlinked every shape on it — the deck still looked right and had stopped
    /// being a themed deck.
    pub hasStyle: bool,
    pub fillRef: i64,
    pub fillRefColor: Rc<RefCell<PptxColor>>,
    pub lineRef: i64,
    pub lineRefColor: Rc<RefCell<PptxColor>>,
    pub effectRef: i64,
    /// `a:fontRef@idx` — "minor" or "major". A string rather than an int
    /// because that is what the attribute holds.
    pub fontRef: String,
    pub fontRefColor: Rc<RefCell<PptxColor>>,
    /// What the RESOLVER worked out, recorded so the writer can tell it from
    /// what the file said.
    ///
    /// These are SIGNATURES rather than flags, and the difference is the whole
    /// point. A flag says "this fill was inherited", and stays true after the
    /// user changes the fill — so the save discards the edit. That is the
    /// failure mode that matters here: writing an inherited value back is a
    /// fidelity loss, and dropping a stated one is data loss.
    ///
    /// A signature cannot be wrong that way. The writer asks "does this still
    /// look exactly like what was inherited?", and any change to any part of it
    /// answers no. There is no mutation site to remember to update.
    pub styleFillSig: String,
    pub styleLineSig: String,
    pub sheetXfrmSig: String,
    pub fill: Rc<RefCell<PptxColor>>,
    pub fillGrad: Rc<RefCell<PptxGradient>>,
    pub line: Rc<RefCell<PptxColor>>,
    pub lineWidthPt: f64,
    pub shadow: Rc<RefCell<PptxShadow>>,
    pub hasText: bool,
    pub text: Rc<RefCell<PptxTextBody>>,
    /// Picture
    pub imageRid: String,
    /// resolved package path
    pub imagePart: String,
    pub imageBytes: Vec<u8>,
    /// decoded pixels available in viewer cache
    pub imageOk: bool,
    /// the part of the picture shown (`a:srcRect`): how much is cut off each
    /// side, as fractions of its width / height (0 = nothing)
    pub cropL: f64,
    pub cropT: f64,
    pub cropR: f64,
    pub cropB: f64,
    /// The same picture as an SVG, written beside `imageBytes` as the blip's
    /// `asvg:svgBlip` extension: PowerPoint (2016 and later) and LibreOffice
    /// draw the vector, older readers the PNG — which can then be a small one.
    pub svgBytes: Vec<u8>,
    pub svgPart: String,
    /// Placeholder (PresentationML p:ph)
    pub isPlaceholder: bool,
    /// title / body / ctrTitle / subTitle / …
    pub phType: String,
    pub phIdx: i64,
    /// true when shape came from master/layout chrome (not authored on the slide)
    pub inherited: bool,
    /// `a:spLocks noMove/noResize`: a shape that is on the slide to be looked at
    /// rather than dragged. It can still be selected — a lock you cannot select
    /// is a lock you cannot undo.
    pub locked: bool,
    /// --- CSS identity --------------------------------------------------------
    ///
    /// What a stylesheet can address this shape by. Deliberately NOT `name`:
    /// `name` is the PowerPoint object name, the one an author sees in the
    /// selection pane and renames without thinking about styling, and letting a
    /// rename silently restyle a box would be a surprise nobody could debug.
    /// `styleId` is a styling concept and is set on purpose.
    ///
    /// Empty on every shape that came out of a file. Nothing reads these unless
    /// a sheet was added, so a deck that never mentions CSS pays a string and an
    /// empty array per shape and no behaviour at all.
    pub styleId: String,
    pub styleClasses: Vec<String>,
    /// Properties the fluent API stated on this shape, which no rule may
    /// override — see `CssCascade.resolve`. Names and values in step.
    pub inlineStyleNames: Vec<String>,
    pub inlineStyleValues: Vec<String>,
    /// Custom / flattened path in shape-local unit coords (optional)
    pub hasPath: bool,
    pub pathUnit: Vec<f64>,
    /// Where each contour of that path ENDS, as an index into `pathUnit`.
    ///
    /// Empty means the whole array is one contour, which is what every shape
    /// that came out of a file or a freehand stroke is — so nothing that
    /// existed before this field has to set it.
    ///
    /// It exists because a shape is not always one ring. A mushroom's spots, a
    /// panda's eye patches, the counter in a letter O: an outline with holes is
    /// several contours filled together under one rule, and flattening them
    /// into one list joins the end of each to the start of the next with a line
    /// that is not in the shape. The holes then fill solid.
    pub pathEnds: Vec<i64>,
    /// Whether that path ends with `<a:close/>`, and so encloses an area.
    ///
    /// Only meaningful when `hasPath`. Flattening the path to points throws
    /// the answer away, and it is the difference between a shape and a stroke:
    /// a hand-drawn line is a custom path with no close under it, and filling
    /// one draws a solid blob over half the slide.
    pub pathClosed: bool,
    /// Table (kind 4)
    pub tableRows: i64,
    /// The grid the file states, in points. Equal division is what a table looks
    /// like when nobody read `a:gridCol` and `a:tr@h`.
    pub tableColW: Vec<f64>,
    pub tableRowH: Vec<f64>,
    pub tableCols: i64,
    pub tableCells: Vec<Rc<RefCell<PptxTableCell>>>,
    /// Chart (kind 5)
    pub chart: Rc<RefCell<PptxChart>>,
    /// Group children
    pub children: Vec<Rc<RefCell<PptxShape>>>,
    /// What a reader with no Office Math shows instead of this shape, when the
    /// shape holds an equation: a drawing of it (the first one is used). With
    /// none, the shape itself with each formula as plain text.
    pub mathFallback: Vec<Rc<RefCell<PptxShape>>>,
    /// Group child coordinate space (chOff / chExt), in pt
    pub chX: f64,
    pub chY: f64,
    pub chW: f64,
    pub chH: f64,
    /// A SmartArt diagram (`graphicFrame` holding `dgm:relIds`), drawn from the
    /// drawing part PowerPoint saved beside its data. It is a GROUP to everything
    /// that draws, selects and moves shapes — its children are the drawing's
    /// shapes in the frame's own space (chOff 0,0, chExt = the frame) — and a
    /// FRAME to the writer: saved over its own file, the original `graphicFrame`
    /// goes back byte for byte with only its position and size restated, so
    /// PowerPoint opens it as the SmartArt it was. `diagramSig` is what the
    /// children said when they were read; a diagram whose text was edited here
    /// no longer matches it and is written as the shapes it now is.
    pub isDiagram: bool,
    diagramFrame: Rc<RefCell<SourceRef>>,
    pub diagramSig: String,
    /// the diagram's text, one line per node, indented by level — what a screen
    /// reader is given when the author wrote no alt text
    pub diagramText: String,
    /// What a diagram LAID OUT HERE (smartart/SaEngine) was made from: its data
    /// model, complete, as `dgm:dataModel` XML; the layout definition it was
    /// run with and that layout's id; the quick style and colour set ids. The
    /// writer saves such a diagram as SmartArt PowerPoint can lay out again
    /// (data, layout, style, colours and this drawing as its parts), and the
    /// editor lays it out again when its text changes. Empty for a diagram
    /// that was only read from a drawing part.
    pub diagramModelXml: String,
    pub diagramLayoutXml: String,
    pub diagramLayoutId: String,
    pub diagramStyleId: String,
    pub diagramColorsId: String,
    /// a diagram's own quick style and colour set, as its file held them (a
    /// whole diagram as one file): written back as they are; empty, the
    /// built-in ones the ids name are written
    pub diagramStyleXml: String,
    pub diagramColorsXml: String,
    /// On a shape INSIDE such a diagram: for each of its paragraphs, the data
    /// point that paragraph's text came from, so an edit to the text can be
    /// written back to the data and the diagram laid out again.
    pub diagramParaPts: Vec<String>,
    /// read from a file with PowerPoint's own drawing of it, and not laid out
    /// here since: saved, its original frame and parts are kept
    pub diagramHadDrawing: bool,
}

impl PptxShape {
    pub fn new() -> Self {
        let mut me = Self {
            kind: PptxShapeKind::shape(),
            name: String::new(),
            altText: String::new(),
            embedNotation: String::new(),
            embedSource: String::new(),
            mdSrcStart: -1,
            mdSrcEnd: -1,
            mdLastText: String::new(),
            shapeId: 0,
            editId: 0,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            hasXfrm: false,
            hasOff: false,
            hasExt: false,
            geomFallback: false,
            rotation: 0.0,
            flipH: false,
            flipV: false,
            preset: "rect".to_string(),
            adj: 0.0 - 1.0,
            adjNames: Vec::new(),
            adjValues: Vec::new(),
            noFill: false,
            noLine: false,
            headEnd: String::new(),
            tailEnd: String::new(),
            hasStyle: false,
            fillRef: 0,
            fillRefColor: Rc::new(RefCell::new(PptxColor::new())),
            lineRef: 0,
            lineRefColor: Rc::new(RefCell::new(PptxColor::new())),
            effectRef: 0,
            fontRef: String::new(),
            fontRefColor: Rc::new(RefCell::new(PptxColor::new())),
            styleFillSig: String::new(),
            styleLineSig: String::new(),
            sheetXfrmSig: String::new(),
            fill: Rc::new(RefCell::new(PptxColor::new())),
            fillGrad: Rc::new(RefCell::new(PptxGradient::new())),
            line: Rc::new(RefCell::new(PptxColor::new())),
            lineWidthPt: 0.0,
            shadow: Rc::new(RefCell::new(PptxShadow::new())),
            hasText: false,
            text: Rc::new(RefCell::new(PptxTextBody::new())),
            imageRid: String::new(),
            imagePart: String::new(),
            imageBytes: vec![0u8; 0],
            imageOk: false,
            cropL: 0.0,
            cropT: 0.0,
            cropR: 0.0,
            cropB: 0.0,
            svgBytes: vec![0u8; 0],
            svgPart: String::new(),
            isPlaceholder: false,
            phType: String::new(),
            phIdx: -1,
            inherited: false,
            locked: false,
            styleId: String::new(),
            styleClasses: Vec::new(),
            inlineStyleNames: Vec::new(),
            inlineStyleValues: Vec::new(),
            hasPath: false,
            pathUnit: Vec::new(),
            pathEnds: Vec::new(),
            pathClosed: false,
            tableRows: 0,
            tableColW: Vec::new(),
            tableRowH: Vec::new(),
            tableCols: 0,
            tableCells: Vec::new(),
            chart: Rc::new(RefCell::new(PptxChart::new())),
            children: Vec::new(),
            mathFallback: Vec::new(),
            chX: 0.0,
            chY: 0.0,
            chW: 0.0,
            chH: 0.0,
            isDiagram: false,
            diagramFrame: Rc::new(RefCell::new(SourceRef::new())),
            diagramSig: String::new(),
            diagramText: String::new(),
            diagramModelXml: String::new(),
            diagramLayoutXml: String::new(),
            diagramLayoutId: String::new(),
            diagramStyleId: String::new(),
            diagramColorsId: String::new(),
            diagramStyleXml: String::new(),
            diagramColorsXml: String::new(),
            diagramParaPts: Vec::new(),
            diagramHadDrawing: false,
        };
        let mut c_1: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        me.children = c_1.clone();
        let mut dpp: Vec<String> = Vec::new();
        me.diagramParaPts = dpp.clone();
        let mut sc: Vec<String> = Vec::new();
        me.styleClasses = sc.clone();
        let mut isn: Vec<String> = Vec::new();
        me.inlineStyleNames = isn.clone();
        let mut isv: Vec<String> = Vec::new();
        me.inlineStyleValues = isv.clone();
        let mut tc: Vec<Rc<RefCell<PptxTableCell>>> = Vec::new();
        me.tableCells = tc.clone();
        let mut cw: Vec<f64> = Vec::new();
        me.tableColW = cw.clone();
        let mut rh: Vec<f64> = Vec::new();
        me.tableRowH = rh.clone();
        let mut pu: Vec<f64> = Vec::new();
        me.pathUnit = pu.clone();
        let mut pe: Vec<i64> = Vec::new();
        me.pathEnds = pe.clone();
        me
    }
    pub fn tenths(v: f64) -> String {
        let mut t: i64 = (v * 10.0 + 0.5).floor() as i64;
        if v < 0.0 {
            t = (v * 10.0 - 0.5).floor() as i64;
        }
        t.to_string()
    }
    /// The shape's fill, as a string that changes when anything about it does.
    pub fn fillSignature(&self) -> String {
        if self.noFill {
            return "none".to_string();
        }
        if self.fillGrad.borrow().isSet {
            let mut a: Rc<RefCell<PptxColor>> = self.fillGrad.borrow().c1.clone().clone();
            let mut b: Rc<RefCell<PptxColor>> = self.fillGrad.borrow().c2.clone().clone();
            return format!("grad:{}:{}:{}", self.fillGrad.borrow().dir, a.borrow().signature(), b.borrow().signature());
        }
        if self.fill.borrow().isSet {
            return format!("solid:{}", self.fill.borrow().signature());
        }
        String::new()
    }
    pub fn lineSignature(&self) -> String {
        if self.noLine {
            return "none".to_string();
        }
        let w: String = ((self.lineWidthPt * 100.0 + 0.5).floor() as i64).to_string();
        if self.line.borrow().isSet {
            return format!("line:{}:{}", self.line.borrow().signature(), w);
        }
        format!(":{}", w)
    }
    pub fn xfrmSignature(&self) -> String {
        if !self.hasXfrm {
            return String::new();
        }
        let mut parts: String = ((self.x * 100.0 + 0.5).floor() as i64).to_string();
        parts = format!("{},{}", parts, (self.y * 100.0 + 0.5).floor() as i64);
        parts = format!("{},{}", parts, (self.width * 100.0 + 0.5).floor() as i64);
        parts = format!("{},{}", parts, (self.height * 100.0 + 0.5).floor() as i64);
        parts = format!("{},{}", parts, (self.rotation * 100.0 + 0.5).floor() as i64);
        if self.flipH {
            parts = format!("{},h", parts);
        }
        if self.flipV {
            parts = format!("{},v", parts);
        }
        parts
    }
    /// What a diagram's shapes say and where they are, to a tenth of a point:
    /// equal to `diagramSig` while nothing inside the diagram was changed.
    pub fn diagramSignature(&self) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (self.children.len() as i64) {
            let mut c: Rc<RefCell<PptxShape>> = self.children[i as usize].clone();
            out = format!("{}{}|{}|{},{},{},{};", out, c.borrow().preset, (c.borrow().text).borrow().plainText(), PptxShape::tenths(c.borrow().x), PptxShape::tenths(c.borrow().y), PptxShape::tenths(c.borrow().width), PptxShape::tenths(c.borrow().height));
            if !c.borrow().children.is_empty() {
                out = format!("{}{}{}{}", out, "{".to_string(), c.borrow().diagramSignature(), "}".to_string());
            }
            i += 1;
        }
        out
    }
}

#[derive(Clone)]
pub struct PptxSlide {
    /// What a stylesheet addresses this slide by — `slide.review` and
    /// `#opening`. See PptxShape.styleId for why this is not the slide's name.
    pub styleId: String,
    pub styleClasses: Vec<String>,
    pub inlineStyleNames: Vec<String>,
    pub inlineStyleValues: Vec<String>,
    /// The part this slide came out of, and whether anything has been done to it
    /// since. A save that keeps the file it opened rewrites only the slides that
    /// were touched — everything else is copied through byte for byte, which is
    /// how the parts nobody here models survive being edited around.
    pub partPath: String,
    pub dirty: bool,
    /// The id `p:sldIdLst` gave it, which is how sections name their slides.
    pub slideId: i64,
    /// The editor's own name for this slide, handed out on attach and never
    /// reused — the same idea as a shape's `editId`, and for the same reason: an
    /// index stops naming the same slide the moment the deck is reordered. The
    /// history uses it to tell whether the slide in front of it is the one it
    /// already has a copy of.
    pub key: i64,
    /// Bumped by every edit on this slide, and never reused, so anything that
    /// caches a picture of a slide can tell whether it is looking at the same
    /// slide it drew.
    pub revision: i64,
    /// pt (default 10")
    pub width: f64,
    /// pt (default 7.5")
    pub height: f64,
    pub shapes: Vec<Rc<RefCell<PptxShape>>>,
    /// Children of the slide's shape tree that nothing here models — SmartArt,
    /// embedded objects, ink, vendor extensions — kept as source spans so that
    /// editing the slide around them does not delete them.
    /// Set by the RESOLVER, never by the parser: this background came from the
    /// layout, the master or the theme's default rather than from the slide.
    /// The writer used to state it as the slide's own, so editing one shape on
    /// a slide froze its background — change the master and every slide that
    /// had ever been touched stopped following it.
    pub bgFromSheet: bool,
    pub opaque: Vec<Rc<RefCell<PptxOpaque>>>,
    /// And the same one level up: children of `<p:sld>` itself that this model
    /// does not describe. `sldHead` is what stood before the transition and the
    /// timing — the colour map override among them, which the writer used to
    /// replace with a plain one whatever the slide said — and `sldTail` what
    /// stood after, which is where a tool's own extension list lives. Both are
    /// positions in the schema's order, so putting them back where they were is
    /// what keeps the part valid.
    pub sldHead: Vec<Rc<RefCell<PptxOpaque>>>,
    pub sldTail: Vec<Rc<RefCell<PptxOpaque>>>,
    pub bg: Rc<RefCell<PptxColor>>,
    /// A PICTURE behind the slide, rather than a colour. Bytes and not a name:
    /// a slide background is a package part like any other picture, and a name
    /// would be a second thing for the writer to resolve at save time.
    ///
    /// A background image is the property readers ask for first and the one a
    /// company template is mostly made of, and until now this model could only
    /// say a colour.
    pub bgImage: Vec<u8>,
    pub bgImageName: String,
    pub layoutPart: String,
    pub masterPart: String,
    /// The speaker's notes, as a shape rather than a string: everything that
    /// knows how to lay out, measure, caret and edit a shape's text then works
    /// on them without knowing what notes are.
    pub notes: Rc<RefCell<PptxShape>>,
    pub notesPart: String,
    /// How this slide arrives, and what is built on it once it has.
    pub transition: Rc<RefCell<PptxTransition>>,
    pub build: Vec<Rc<RefCell<PptxBuildStep>>>,
}

impl PptxSlide {
    pub fn new() -> Self {
        let mut me = Self {
            styleId: String::new(),
            styleClasses: Vec::new(),
            inlineStyleNames: Vec::new(),
            inlineStyleValues: Vec::new(),
            partPath: String::new(),
            dirty: false,
            slideId: 0,
            key: 0,
            revision: 0,
            width: 720.0,
            height: 540.0,
            shapes: Vec::new(),
            bgFromSheet: false,
            opaque: Vec::new(),
            sldHead: Vec::new(),
            sldTail: Vec::new(),
            bg: Rc::new(RefCell::new(PptxColor::new())),
            bgImage: vec![0u8; 0],
            bgImageName: String::new(),
            layoutPart: String::new(),
            masterPart: String::new(),
            notes: Rc::new(RefCell::new(PptxShape::new())),
            notesPart: String::new(),
            transition: Rc::new(RefCell::new(PptxTransition::new())),
            build: Vec::new(),
        };
        let mut s_1: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        me.shapes = s_1.clone();
        let mut b: Vec<Rc<RefCell<PptxBuildStep>>> = Vec::new();
        me.build = b.clone();
        let mut sc_1: Vec<String> = Vec::new();
        me.styleClasses = sc_1.clone();
        let mut isn_1: Vec<String> = Vec::new();
        me.inlineStyleNames = isn_1.clone();
        let mut isv_1: Vec<String> = Vec::new();
        me.inlineStyleValues = isv_1.clone();
        me
    }
    pub fn notesText(&self) -> String {
        if !self.notes.borrow().hasText {
            return String::new();
        }
        return (self.notes.borrow().text).borrow().plainText();
    }
}

/// Cached master / layout sheet (same shape model as a slide).
#[derive(Clone)]
pub struct PptxSheet {
    pub partPath: String,
    pub shapes: Vec<Rc<RefCell<PptxShape>>>,
    /// Children of this sheet's shape tree that nothing here models, kept as
    /// source spans so a rewrite puts them back rather than dropping them.
    pub opaque: Vec<Rc<RefCell<PptxOpaque>>>,
    pub bg: Rc<RefCell<PptxColor>>,
    /// …and a picture, the same as a slide's. A layout or a master is where a
    /// company template usually puts one, so a reader that could only see a
    /// colour here saw a blank deck.
    pub bgImage: Vec<u8>,
    pub bgImageName: String,
    /// `p:txStyles` — the master's own typography, in three list styles. A
    /// placeholder with no formatting of its own gets everything from here, and
    /// that is most placeholders in most decks.
    pub titleStyle: Rc<RefCell<PptxListStyle>>,
    pub bodyStyle: Rc<RefCell<PptxListStyle>>,
    pub otherStyle: Rc<RefCell<PptxListStyle>>,
    /// Edited since it was read, so a save knows to rewrite its part.
    pub dirty: bool,
}

impl PptxSheet {
    pub fn new() -> Self {
        let mut me = Self {
            partPath: String::new(),
            shapes: Vec::new(),
            opaque: Vec::new(),
            bg: Rc::new(RefCell::new(PptxColor::new())),
            bgImage: vec![0u8; 0],
            bgImageName: String::new(),
            titleStyle: Rc::new(RefCell::new(PptxListStyle::new())),
            bodyStyle: Rc::new(RefCell::new(PptxListStyle::new())),
            otherStyle: Rc::new(RefCell::new(PptxListStyle::new())),
            dirty: false,
        };
        let mut s_2: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        me.shapes = s_2.clone();
        me
    }
}

#[derive(Clone)]
pub struct PptxTheme {
    /// scheme name → RRGGBB
    pub colors: HashMap<String,String>,
    pub majorFont: String,
    pub minorFont: String,
    /// `a:fmtScheme` — the fills, outlines and effects a `<p:style>` reference
    /// indexes into. One-based in the file; stored here from index 0.
    pub fillStyles: Vec<Rc<RefCell<PptxFillStyle>>>,
    pub lineStyles: Vec<Rc<RefCell<PptxLineStyle>>>,
    /// How many effect styles carry a shadow, by index — the whole of what this
    /// renderer does with `effectRef`.
    pub effectShadows: Vec<bool>,
}

impl PptxTheme {
    pub fn new() -> Self {
        let mut me = Self {
            colors: HashMap::new(),
            majorFont: "Calibri Light".to_string(),
            minorFont: "Calibri".to_string(),
            fillStyles: Vec::new(),
            lineStyles: Vec::new(),
            effectShadows: Vec::new(),
        };
        let mut f: Vec<Rc<RefCell<PptxFillStyle>>> = Vec::new();
        me.fillStyles = f.clone();
        let mut l_1: Vec<Rc<RefCell<PptxLineStyle>>> = Vec::new();
        me.lineStyles = l_1.clone();
        let mut e: Vec<bool> = Vec::new();
        me.effectShadows = e.clone();
        me
    }
    pub fn defaultColor(name: &str) -> String {
        if name == "dk1" {
            return "000000".to_string();
        }
        if name == "lt1" {
            return "FFFFFF".to_string();
        }
        if name == "dk2" {
            return "44546A".to_string();
        }
        if name == "lt2" {
            return "E7E6E6".to_string();
        }
        if name == "accent1" {
            return "4472C4".to_string();
        }
        if name == "accent2" {
            return "ED7D31".to_string();
        }
        if name == "accent3" {
            return "A5A5A5".to_string();
        }
        if name == "accent4" {
            return "FFC000".to_string();
        }
        if name == "accent5" {
            return "5B9BD5".to_string();
        }
        if name == "accent6" {
            return "70AD47".to_string();
        }
        if name == "hlink" {
            return "0563C1".to_string();
        }
        if name == "folHlink" {
            return "954F72".to_string();
        }
        "000000".to_string()
    }
    pub fn fillStyleAt(&self, idx: i64) -> Option<Rc<RefCell<PptxFillStyle>>> {
        if idx < 1 {
            let mut __none: Option<Rc<RefCell<PptxFillStyle>>> = None;
            return __none;
        }
        if idx > (self.fillStyles.len() as i64) {
            let mut __none2: Option<Rc<RefCell<PptxFillStyle>>> = None;
            return __none2;
        }
        let mut hit: Option<Rc<RefCell<PptxFillStyle>>> = Some(self.fillStyles[(idx - 1) as usize].clone());
        hit
    }
    pub fn lineStyleAt(&self, idx: i64) -> Option<Rc<RefCell<PptxLineStyle>>> {
        if idx < 1 {
            let mut __none: Option<Rc<RefCell<PptxLineStyle>>> = None;
            return __none;
        }
        if idx > (self.lineStyles.len() as i64) {
            let mut __none2: Option<Rc<RefCell<PptxLineStyle>>> = None;
            return __none2;
        }
        let mut hit: Option<Rc<RefCell<PptxLineStyle>>> = Some(self.lineStyles[(idx - 1) as usize].clone());
        hit
    }
    pub fn effectShadowAt(&self, idx: i64) -> bool {
        if idx < 1 {
            return false;
        }
        if idx > (self.effectShadows.len() as i64) {
            return false;
        }
        self.effectShadows[(idx - 1) as usize]
    }
    pub fn setColor(&mut self, name: &str, hex: &str) {
        self.colors.insert(name.to_string(), hex.to_string());
    }
    /// A scheme colour this theme did not state.
    ///
    /// It used to be black, for all twelve of them — including `lt1`, which a
    /// master's colour map sends `bg1` to and which is therefore THE SLIDE'S
    /// BACKGROUND. A deck built from scratch states none of them, so every
    /// deck this repository wrote came out black on black in any reader that
    /// paints the theme's background, which is every reader. It was reported
    /// from Keynote.
    ///
    /// The fallbacks below are Office's own default scheme, which is what a
    /// deck nobody themed is supposed to look like. A theme READ from a file
    /// fills `colors` and none of this applies to it.
    pub fn colorOf(&self, name: &str) -> String {
        let v: Option<String> = self.colors.get(name).cloned();
        if v.is_some() {
            return v.clone().unwrap();
        }
        PptxTheme::defaultColor(name)
    }
}

/// How a slide arrives when the deck is being shown. `kind` is the effect's own
/// name from PresentationML (`fade`, `push`, `wipe`, `cut`, …), `dir` the
/// direction word it carries, and `seconds` how long it runs — a transition is a
/// time-varying picture, so the show needs a length, not just a name.
#[derive(Clone)]
pub struct PptxTransition {
    pub isSet: bool,
    pub kind: String,
    pub dir: String,
    pub seconds: f64,
    /// Advance on a click, on a timer, or on either. `afterSeconds` is 0 when
    /// nothing is timed.
    pub onClick: bool,
    pub afterSeconds: f64,
}

impl PptxTransition {
    pub fn new() -> Self {
        Self {
            isSet: false,
            kind: String::new(),
            dir: String::new(),
            seconds: 0.75,
            onClick: true,
            afterSeconds: 0.0,
        }
    }
}

/// One step of a slide's build: a shape that is not there until the show reaches
/// it. `editId` names the shape (an index would not survive a reorder), `kind`
/// is the effect and `paragraph` is -1 for the whole shape or the paragraph the
/// step reveals when the text builds a line at a time.
#[derive(Clone)]
pub struct PptxBuildStep {
    pub editId: i64,
    pub shapeId: i64,
    pub kind: String,
    pub dir: String,
    pub seconds: f64,
    pub paragraph: i64,
    pub start: String,
}

impl PptxBuildStep {
    pub fn new() -> Self {
        Self {
            editId: 0,
            shapeId: 0,
            kind: "appear".to_string(),
            dir: String::new(),
            seconds: 0.5,
            paragraph: 0 - 1,
            start: "click".to_string(),
        }
    }
}

/// A run of slides under a name, which is how a long deck is navigated.
#[derive(Clone)]
pub struct PptxSection {
    pub name: String,
    pub id: String,
    /// Slide ids, as `p:sldIdLst` names them; the deck resolves them to indices.
    pub slideIds: Vec<i64>,
}

impl PptxSection {
    pub fn new() -> Self {
        let mut me = Self {
            name: String::new(),
            id: String::new(),
            slideIds: Vec::new(),
        };
        let mut s_3: Vec<i64> = Vec::new();
        me.slideIds = s_3.clone();
        me
    }
}

#[derive(Clone)]
pub struct PptxPresentation {
    pub slideWidth: f64,
    pub slideHeight: f64,
    pub sections: Vec<Rc<RefCell<PptxSection>>>,
    pub slideParts: Vec<String>,
    pub slides: Vec<Rc<RefCell<PptxSlide>>>,
    pub theme: Rc<RefCell<PptxTheme>>,
    pub themePart: String,
    /// partPath → parsed master / layout (shared across slides)
    pub masters: HashMap<String,Rc<RefCell<PptxSheet>>>,
    pub layouts: HashMap<String,Rc<RefCell<PptxSheet>>>,
}

impl PptxPresentation {
    pub fn new() -> Self {
        let mut me = Self {
            slideWidth: 720.0,
            slideHeight: 540.0,
            sections: Vec::new(),
            slideParts: Vec::new(),
            slides: Vec::new(),
            theme: Rc::new(RefCell::new(PptxTheme::new())),
            themePart: String::new(),
            masters: HashMap::new(),
            layouts: HashMap::new(),
        };
        let mut sec: Vec<Rc<RefCell<PptxSection>>> = Vec::new();
        me.sections = sec.clone();
        let mut sp: Vec<String> = Vec::new();
        me.slideParts = sp.clone();
        let mut sl: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        me.slides = sl.clone();
        let mut m: HashMap<String,Rc<RefCell<PptxSheet>>> = HashMap::new();
        me.masters = m.clone();
        let mut l_2: HashMap<String,Rc<RefCell<PptxSheet>>> = HashMap::new();
        me.layouts = l_2.clone();
        me
    }
    pub fn slideCount(&self) -> i64 {
        self.slides.len() as i64
    }
}

/// Shared EMU helpers — DrawingML uses English Metric Units.
#[derive(Clone)]
pub struct PptxUnits {
}

impl PptxUnits {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn emuToPt(emu: f64) -> f64 {
        emu / 12700.0
    }
    pub fn emuToPx(emu: f64) -> f64 {
        // CSS reference pixel at 96dpi: 1in = 96px = 914400 EMU
        emu / 9525.0
    }
    pub fn rotToDeg(rot: f64) -> f64 {
        // DrawingML rotation is 1/60000 of a degree
        rot / 60000.0
    }
    pub fn hundredthsPtToPt(sz: f64) -> f64 {
        // a:rPr sz is in hundredths of a point
        sz / 100.0
    }
}
