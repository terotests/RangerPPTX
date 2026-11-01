// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::pptx_css::PptxCss;
use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxParagraph;
use pptx::pptx_model::PptxPresentation;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxSlide;
use pptx::pptx_model::PptxTextBody;
use pptx::pptx_model::PptxTextRun;
use pptx::pptx_parser::PptxParser;
use pptx::pptx_resolver::PptxResolver;
use crate::pptx_text_edit::PptxTextEdit;
use crate::pptx_writer::PptxWriter;

/// ==============================================================================
/// PptxApi — open a deck, read it, change it, write it back. No screen involved.
/// ==============================================================================
///
/// This is the published surface of the .pptx reader and writer: the thing a
/// program outside this repository calls, and the thing the generated
/// documentation describes. Everything under it already existed — `PptxParser`,
/// `PptxResolver`, the model in `PptxModel`, `PptxWriter` — and was reachable
/// only through `PptxApp`, which is an EDITOR: it owns a view, a selection, a
/// scroll offset, an undo history and a text caret. A build server that wants to
/// put a customer's name on slide one had to construct all of that and then
/// carefully not use it.
///
/// So this facade is HEADLESS by construction. It imports the parser, the model
/// and the writer, and it imports no renderer, no canvas and no font. That is not
/// tidiness: fonts are megabytes and a rasterizer is a different kind of
/// dependency from a ZIP reader, and a program that only rewrites text should not
/// pay for either. Rendering — PNG, PDF — lives in `PptxRenderApi` beside this
/// file, which imports this one and adds the drawing. Two entry points, and the
/// one you import decides what you carry.
///
/// WHAT A HANDLE IS. `PptxDeck`, `PptxSlideRef` and `PptxShapeRef` hold an INDEX
/// into the model rather than the model object. A reference that held the object
/// would go on describing a shape after the shape was removed, and would write
/// changes into a slide the deck no longer contains. Holding the index means a
/// stale handle answers `false` from `exists()` and refuses to write, which is a
/// thing a caller can check rather than a thing that silently does nothing.
///
/// WHY THERE ARE NO EXCEPTIONS HERE. Ranger's compiled JavaScript has no throw
/// that a C++ target would honour the same way, so a failure is a value: `ok` is
/// false and `error` says what happened. The JavaScript wrapper turns that into
/// a thrown `Error`, because that is what a JavaScript caller expects — the
/// translation belongs in the wrapper, not in code that also compiles to C++.
/// ==============================================================================
/// ------------------------------------------------------------------------------
/// A run of text inside a shape, addressed by paragraph and position.
/// ------------------------------------------------------------------------------
///
/// A deck's text is not a string: it is paragraphs of runs, and a run is the unit
/// that carries a font, a size, a weight and a colour. `setText` on a shape
/// replaces the lot and keeps the FIRST run's formatting, because that is what
/// someone filling in a template means. Reach for this class when that is not
/// what you mean — when the second half of a sentence has to be red and the first
/// half must not change.
#[derive(Clone)]
pub struct PptxRunRef {
    pub shape: Rc<RefCell<PptxShapeRef>>,
    pub para: i64,
    pub index: i64,
}

impl PptxRunRef {
    pub fn new() -> Self {
        Self {
            shape: Rc::new(RefCell::new(PptxShapeRef::new())),
            para: 0,
            index: 0,
        }
    }
    /// Is this run still there? A handle taken before an edit that removed
    /// paragraphs is a handle to nothing, and every writer below checks this
    /// first rather than reaching past the end of an array.
    pub fn exists(&self) -> bool {
        if !self.shape.borrow().exists() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.shape.borrow().model();
        if !sh.borrow().hasText {
            return false;
        }
        if self.para < 0 || self.para >= (sh.borrow().text.borrow().paragraphs.len() as i64) {
            return false;
        }
        let mut p: Rc<RefCell<PptxParagraph>> = sh.borrow()
            .text.borrow()
            .paragraphs[(self.para) as usize].clone();
        return self.index >= 0 && self.index < (p.borrow().runs.len() as i64);
    }
    /// @internal The model object behind the handle. Callers use the methods.
    pub fn model(&self) -> Rc<RefCell<PptxTextRun>> {
        let mut sh: Rc<RefCell<PptxShape>> = self.shape.borrow().model();
        let mut p: Rc<RefCell<PptxParagraph>> = sh.borrow()
            .text.borrow()
            .paragraphs[(self.para) as usize].clone();
        return p.borrow().runs[(self.index) as usize].clone();
    }
    pub fn text(&self) -> String {
        if !self.exists() {
            return String::new();
        }
        let mut r: Rc<RefCell<PptxTextRun>> = self.model();
        return r.borrow().text.clone();
    }
    pub fn setText(&self, s: String) -> Rc<RefCell<PptxRunRef>> {
        if self.exists() {
            let mut r: Rc<RefCell<PptxTextRun>> = self.model();
            r.borrow_mut().text = s.clone();
            self.shape.clone().borrow().touch();
        }
        self
    }
    /// The font this run NAMES. It is not necessarily the font it is drawn in —
    /// a renderer substitutes what it has — but it is what the file says and
    /// what PowerPoint will use.
    pub fn font(&self, family: String, size_pt: f64) -> Rc<RefCell<PptxRunRef>> {
        if self.exists() {
            let mut r: Rc<RefCell<PptxTextRun>> = self.model();
            if !family.is_empty() {
                r.borrow_mut().fontFamily = family.clone();
            }
            if size_pt > 0.0 {
                r.borrow_mut().fontSizePt = size_pt;
                r.borrow_mut().sizeSet = true;
            }
            self.shape.clone().borrow().touch();
        }
        self
    }
    pub fn bold(&self, on: bool) -> Rc<RefCell<PptxRunRef>> {
        if self.exists() {
            let mut r: Rc<RefCell<PptxTextRun>> = self.model();
            r.borrow_mut().bold = on;
            self.shape.clone().borrow().touch();
        }
        self
    }
    pub fn italic(&self, on: bool) -> Rc<RefCell<PptxRunRef>> {
        if self.exists() {
            let mut r: Rc<RefCell<PptxTextRun>> = self.model();
            r.borrow_mut().italic = on;
            self.shape.clone().borrow().touch();
        }
        self
    }
    /// "4472C4" — six hexadecimal digits, with or without a leading '#'.
    pub fn color(&self, hex: &str) -> Rc<RefCell<PptxRunRef>> {
        if self.exists() {
            let mut r: Rc<RefCell<PptxTextRun>> = self.model();
            r.borrow_mut().color = PptxApi::colorOf(hex);
            self.shape.clone().borrow().touch();
        }
        self
    }
}

/// ------------------------------------------------------------------------------
/// One shape on one slide: a box, a picture, a group, a text placeholder.
/// ------------------------------------------------------------------------------
///
/// `index` addresses the slide's TOP-LEVEL shapes. A shape inside a group is not
/// reachable from here, and that is a stated limit rather than an oversight: a
/// group's children live in the group's own coordinate space, and handing them
/// out through the same handle as a slide's shapes would make `at(x y)` mean two
/// different things depending on where the shape came from.
#[derive(Clone)]
pub struct PptxShapeRef {
    pub slide: Rc<RefCell<PptxSlideRef>>,
    pub index: i64,
}

impl PptxShapeRef {
    pub fn new() -> Self {
        Self {
            slide: Rc::new(RefCell::new(PptxSlideRef::new())),
            index: 0,
        }
    }
    pub fn exists(&self) -> bool {
        if !self.slide.borrow().exists() {
            return false;
        }
        let mut sl: Rc<RefCell<PptxSlide>> = self.slide.borrow().model();
        return self.index >= 0 && self.index < (sl.borrow().shapes.len() as i64);
    }
    /// @internal The model object behind the handle. Callers use the methods.
    pub fn model(&self) -> Rc<RefCell<PptxShape>> {
        let mut sl: Rc<RefCell<PptxSlide>> = self.slide.borrow().model();
        return sl.borrow().shapes[(self.index) as usize].clone();
    }
    /// @internal Every writer calls this. A slide that is not marked dirty is COPIED
    /// through byte for byte when the deck is saved over the file it came from —
    /// which is the whole point of that path, and also the way an edit silently
    /// fails to appear in the output if nothing says the slide changed.
    pub fn touch(&self) {
        if self.slide.borrow().exists() {
            let mut sl: Rc<RefCell<PptxSlide>> = self.slide.borrow().model();
            sl.borrow_mut().dirty = true;
            let hoist_1 = sl.borrow().revision + 1;
            sl.borrow_mut().revision = hoist_1;
        }
    }
    pub fn name(&self) -> String {
        if !self.exists() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().name.clone();
    }
    pub fn setName(&self, s: String) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().name = s.clone();
            self.touch();
        }
        self
    }
    /// The preset geometry: "rect", "ellipse", "roundRect", "star5" — one of the
    /// 187 names ECMA-376 defines. `gallery/office/geom/assets/presets.txt` is
    /// the catalogue, and `gallery/pptx/docs/PRESET_SHAPES.md` lists them.
    pub fn preset(&self) -> String {
        if !self.exists() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().preset.clone();
    }
    pub fn setPreset(&self, name: String) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().preset = name.clone();
            self.touch();
        }
        self
    }
    pub fn x(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().x;
    }
    pub fn y(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().y;
    }
    pub fn width(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().width;
    }
    pub fn height(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().height;
    }
    pub fn rotation(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().rotation;
    }
    /// Points, from the top left of the slide. A deck stores English Metric
    /// Units — 12700 to the point — and nothing above this line ever sees one.
    ///
    /// `hasXfrm` is set here because a shape that states no transform inherits
    /// its box from the layout: moving such a shape without saying it now has
    /// its own geometry writes a position the file does not carry, and the shape
    /// reopens where the layout put it.
    pub fn at(&self, nx: f64, ny: f64) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().x = nx;
            sh.borrow_mut().y = ny;
            sh.borrow_mut().hasXfrm = true;
            self.touch();
        }
        self
    }
    pub fn size(&self, w: f64, h: f64) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().width = w;
            sh.borrow_mut().height = h;
            sh.borrow_mut().hasXfrm = true;
            self.touch();
        }
        self
    }
    /// Degrees clockwise. The file stores sixtieths of a thousandth of a degree;
    /// the writer converts, and a negative angle wraps rather than being written
    /// as a negative number, which PowerPoint does not read.
    pub fn rotate(&self, deg: f64) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().rotation = deg;
            sh.borrow_mut().hasXfrm = true;
            self.touch();
        }
        self
    }
    pub fn fill(&self, hex: &str) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().fill = PptxApi::colorOf(hex);
            sh.borrow_mut().noFill = false;
            self.touch();
        }
        self
    }
    pub fn noFill(&self) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().noFill = true;
            self.touch();
        }
        self
    }
    pub fn line(&self, hex: &str, width_pt: f64) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().line = PptxApi::colorOf(hex);
            sh.borrow_mut().noLine = false;
            if width_pt > 0.0 {
                sh.borrow_mut().lineWidthPt = width_pt;
            }
            self.touch();
        }
        self
    }
    pub fn noLine(&self) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().noLine = true;
            self.touch();
        }
        self
    }
    /// Every character in the shape, paragraphs joined by a newline. This is what
    /// a search reads; it is not what a writer should round-trip, because the
    /// runs and their formatting are not in it.
    ///
    /// A GROUP ANSWERS FOR ITS CHILDREN. This used to stop at the group and
    /// return nothing, because a group carries no text body of its own — so a
    /// deck whose text boxes had been grouped, which is most decks anybody has
    /// arranged, read back as blank. Nothing failed; the search index was simply
    /// empty and there was no way to tell that from a deck with no words in it.
    pub fn text(&self) -> String {
        if !self.exists() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        PptxApi::textOf(&sh)
    }
    /// --- the CSS identity ----------------------------------------------------
    /// Give this shape a class a stylesheet can address.
    ///
    /// Chainable and repeatable: `addClass("card").addClass("wide")` carries
    /// both, and a class added twice is carried once.
    pub fn addClass(&self, name: &str) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            PptxCss::addShapeClass(&sh, name);
            self.touch();
        }
        self
    }
    pub fn removeClass(&self, name: &str) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            PptxCss::removeShapeClass(&sh, name);
            self.touch();
        }
        self
    }
    pub fn hasClass(&self, name: &str) -> bool {
        if !self.exists() {
            return false;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        PptxCss::hasShapeClass(&sh, name)
    }
    /// The id a stylesheet addresses with `#name`.
    ///
    /// NOT `setName`. That is the PowerPoint object name — what an author sees
    /// in the selection pane and renames without thinking about styling — and
    /// conflating the two would mean a rename silently restyles the box. Set
    /// both when you want both.
    pub fn setStyleId(&self, id: String) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            sh.borrow_mut().styleId = id.clone();
            self.touch();
        }
        self
    }
    pub fn styleId(&self) -> String {
        if !self.exists() {
            return String::new();
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        return sh.borrow().styleId.clone();
    }
    /// One property, stated here rather than in a sheet.
    ///
    /// This is CSS's inline style and it behaves like it: nothing in any sheet
    /// can override it, `!important` included. The fluent setters beside it —
    /// `fill`, `noLine`, `run(0,0).bold()` — write the model directly and are
    /// not affected by the cascade either, so the two spellings agree.
    pub fn style(&self, name: &str, value: &str) -> Rc<RefCell<PptxShapeRef>> {
        if self.exists() {
            let mut sh: Rc<RefCell<PptxShape>> = self.model();
            PptxCss::setShapeStyle(&sh, name, value);
            self.touch();
        }
        self
    }
    /// Replace all of the text, keeping the first run's formatting.
    ///
    /// A newline starts a new paragraph, and each paragraph keeps the FIRST
    /// paragraph's properties — alignment, bullet, indent. Filling a template is
    /// what this is for: the box was styled by whoever made the template, and a
    /// caller who replaces the words means to keep that styling. Building text
    /// whose parts differ from one another is `runAt` and `addRun`.
    pub fn setText(&self, s: &str) -> Rc<RefCell<PptxShapeRef>> {
        if !self.exists() {
            return self;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        let mut proto_run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        let mut proto_para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
        if sh.borrow().hasText && !sh.borrow().text.borrow().paragraphs.is_empty() {
            let mut p0: Rc<RefCell<PptxParagraph>> = sh.borrow().text.borrow().paragraphs[0].clone();
            proto_para = p0.clone();
            if !p0.borrow().runs.is_empty() {
                proto_run = p0.borrow().runs[0].clone();
            }
        }
        let mut body: Rc<RefCell<PptxTextBody>> = Rc::new(RefCell::new(PptxTextBody::new()));
        if sh.borrow().hasText {
            body = sh.borrow().text.clone();
        }
        let mut fresh: Vec<Rc<RefCell<PptxParagraph>>> = Vec::new();
        let mut lines: Vec<String> = PptxApi::lines(s);
        let mut i: i64 = 0;
        while i < (lines.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            proto_para.borrow().copyFormatTo(&para);
            let mut run: Rc<RefCell<PptxTextRun>> = PptxTextEdit::emptyLike(&proto_run);
            run.borrow_mut().text = lines[i as usize].clone();
            para.borrow_mut().runs.push(run.clone());
            fresh.push(para.clone());
            i += 1;
        }
        body.borrow_mut().paragraphs = fresh.clone();
        sh.borrow_mut().text = body.clone();
        sh.borrow_mut().hasText = true;
        self.touch();
        self
    }
    pub fn paragraphCount(&self) -> i64 {
        if !self.exists() {
            return 0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        if !sh.borrow().hasText {
            return 0;
        }
        return sh.borrow().text.borrow().paragraphs.len() as i64;
    }
    pub fn runCount(&self, para: i64) -> i64 {
        if !self.exists() {
            return 0;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        if !sh.borrow().hasText {
            return 0;
        }
        if para < 0 || para >= (sh.borrow().text.borrow().paragraphs.len() as i64) {
            return 0;
        }
        let mut p: Rc<RefCell<PptxParagraph>> = sh.borrow()
            .text.borrow()
            .paragraphs[para as usize].clone();
        return p.borrow().runs.len() as i64;
    }
    pub fn runAt(&self, para: i64, index: i64) -> Rc<RefCell<PptxRunRef>> {
        let mut r: Rc<RefCell<PptxRunRef>> = Rc::new(RefCell::new(PptxRunRef::new()));
        r.borrow_mut().shape = self.clone();
        r.borrow_mut().para = para;
        r.borrow_mut().index = index;
        r
    }
    /// Add a run to the end of a paragraph, formatted like the one before it.
    /// A shape with no text at all gets its first paragraph here.
    pub fn addRun(&self, para: i64, text: String) -> Rc<RefCell<PptxRunRef>> {
        if !self.exists() {
            return self.runAt(0, 0);
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        if !sh.borrow().hasText {
            let mut body: Rc<RefCell<PptxTextBody>> = Rc::new(RefCell::new(PptxTextBody::new()));
            sh.borrow_mut().text = body.clone();
            sh.borrow_mut().hasText = true;
        }
        while (sh.borrow().text.borrow().paragraphs.len() as i64) <= para {
            let mut p: Rc<RefCell<PptxParagraph>> = Rc::new(RefCell::new(PptxParagraph::new()));
            sh.borrow_mut().text.borrow_mut().paragraphs.push(p.clone());
        }
        let mut target: Rc<RefCell<PptxParagraph>> = sh.borrow()
            .text.borrow()
            .paragraphs[para as usize].clone();
        let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
        if !target.borrow().runs.is_empty() {
            let last_at: i64 = (target.borrow().runs.len() as i64) - 1;
            let mut last: Rc<RefCell<PptxTextRun>> = target.borrow().runs[last_at as usize].clone();
            run = PptxTextEdit::emptyLike(&last);
        }
        run.borrow_mut().text = text.clone();
        target.borrow_mut().runs.push(run.clone());
        self.touch();
        return self.runAt(para, (target.borrow().runs.len() as i64) - 1);
    }
    /// Horizontal alignment of every paragraph: "l", "ctr", "r", "just".
    pub fn align(&self, how: String) -> Rc<RefCell<PptxShapeRef>> {
        if !self.exists() {
            return self;
        }
        let mut sh: Rc<RefCell<PptxShape>> = self.model();
        if !sh.borrow().hasText {
            return self;
        }
        let mut i: i64 = 0;
        while i < (sh.borrow().text.borrow().paragraphs.len() as i64) {
            let mut p: Rc<RefCell<PptxParagraph>> = sh.borrow()
                .text.borrow()
                .paragraphs[i as usize].clone();
            p.borrow_mut().align = how.clone();
            p.borrow_mut().alignSet = true;
            i += 1;
        }
        self.touch();
        self
    }
}

/// ------------------------------------------------------------------------------
/// One slide.
/// ------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PptxSlideRef {
    pub deck: Rc<RefCell<PptxDeck>>,
    pub index: i64,
}

impl PptxSlideRef {
    pub fn new() -> Self {
        Self {
            deck: Rc::new(RefCell::new(PptxDeck::new())),
            index: 0,
        }
    }
    pub fn exists(&self) -> bool {
        if !self.deck.borrow().ok {
            return false;
        }
        self.index >= 0 && self.index < (self.deck.borrow().pres.borrow().slides.len() as i64)
    }
    /// @internal The model object behind the handle. Callers use the methods.
    pub fn model(&self) -> Rc<RefCell<PptxSlide>> {
        self.deck.borrow()
            .pres.borrow()
            .slides[(self.index) as usize].clone()
            .clone()
    }
    pub fn width(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = self.model();
        return s.borrow().width;
    }
    pub fn height(&self) -> f64 {
        if !self.exists() {
            return 0.0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = self.model();
        return s.borrow().height;
    }
    pub fn shapeCount(&self) -> i64 {
        if !self.exists() {
            return 0;
        }
        let mut s: Rc<RefCell<PptxSlide>> = self.model();
        return s.borrow().shapes.len() as i64;
    }
    pub fn shapeAt(&self, i: i64) -> Rc<RefCell<PptxShapeRef>> {
        let mut r: Rc<RefCell<PptxShapeRef>> = Rc::new(RefCell::new(PptxShapeRef::new()));
        r.borrow_mut().slide = self.clone();
        r.borrow_mut().index = i;
        r
    }
    /// The first shape whose name matches, or a handle that says it does not
    /// exist. A template names its boxes — "Title", "Subtitle", "Footer" — and
    /// filling one in by name is what most callers want; by index is what breaks
    /// the moment somebody reorders the slide in PowerPoint.
    pub fn shapeNamed(&self, name: &str) -> Rc<RefCell<PptxShapeRef>> {
        let mut i: i64 = 0;
        while i < self.shapeCount() {
            let mut sh: Rc<RefCell<PptxShapeRef>> = self.shapeAt(i);
            let got: String = sh.borrow().name();
            if got == name {
                return sh;
            }
            i += 1;
        }
        self.shapeAt(0 - 1)
    }
    /// Every character on the slide, shape by shape in the order they are drawn.
    pub fn text(&self) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < self.shapeCount() {
            let mut sh: Rc<RefCell<PptxShapeRef>> = self.shapeAt(i);
            let t: String = sh.borrow().text();
            if !t.is_empty() {
                if !out.is_empty() {
                    out = format!("{}\n", out);
                }
                out = format!("{}{}", out, t);
            }
            i += 1;
        }
        out
    }
    /// --- the CSS identity ----------------------------------------------------
    /// Give this slide a class a stylesheet can address: `slide.review { … }`.
    pub fn addClass(&self, name: &str) -> Rc<RefCell<PptxSlideRef>> {
        if self.exists() {
            let mut s: Rc<RefCell<PptxSlide>> = self.model();
            PptxCss::addSlideClass(&s, name);
        }
        self
    }
    /// The id a stylesheet addresses with `#name`. See `PptxShapeRef.setStyleId`
    /// for why this is separate from anything the file calls the slide.
    pub fn setStyleId(&self, id: String) -> Rc<RefCell<PptxSlideRef>> {
        if self.exists() {
            let mut s: Rc<RefCell<PptxSlide>> = self.model();
            s.borrow_mut().styleId = id.clone();
        }
        self
    }
    /// One property, stated here rather than in a sheet. No rule can beat it.
    pub fn style(&self, name: &str, value: &str) -> Rc<RefCell<PptxSlideRef>> {
        if self.exists() {
            let mut s: Rc<RefCell<PptxSlide>> = self.model();
            PptxCss::setSlideStyle(&s, name, value);
        }
        self
    }
    pub fn addTextBox(&self, x: f64, y: f64, w: f64, h: f64, text: &str) -> Rc<RefCell<PptxShapeRef>> {
        let mut made: Rc<RefCell<PptxShapeRef>> = self.addShape("rect".to_string(),
            x,
            y,
            w,
            h
        );
        if made.borrow().exists() {
            let mut sh: Rc<RefCell<PptxShape>> = made.borrow().model();
            sh.borrow_mut().noFill = true;
            sh.borrow_mut().noLine = true;
            made.borrow().setText(text);
        }
        made
    }
    pub fn addShape(&self, preset: String, x: f64, y: f64, w: f64, h: f64) -> Rc<RefCell<PptxShapeRef>> {
        if !self.exists() {
            return self.shapeAt(0 - 1);
        }
        let mut s: Rc<RefCell<PptxSlide>> = self.model();
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().preset = preset.clone();
        sh.borrow_mut().x = x;
        sh.borrow_mut().y = y;
        sh.borrow_mut().width = w;
        sh.borrow_mut().height = h;
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().name = format!("{} {}", preset, (s.borrow().shapes.len() as i64) + 1);
        s.borrow_mut().shapes.push(sh.clone());
        s.borrow_mut().dirty = true;
        let hoist_2 = s.borrow().revision + 1;
        s.borrow_mut().revision = hoist_2;
        return self.shapeAt((s.borrow().shapes.len() as i64) - 1);
    }
    pub fn removeShape(&self, i: i64) -> bool {
        if !self.exists() {
            return false;
        }
        let mut s: Rc<RefCell<PptxSlide>> = self.model();
        if i < 0 || i >= (s.borrow().shapes.len() as i64) {
            return false;
        }
        let mut kept: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut k: i64 = 0;
        while k < (s.borrow().shapes.len() as i64) {
            if k != i {
                kept.push(s.borrow().shapes[k as usize].clone());
            }
            k += 1;
        }
        s.borrow_mut().shapes = kept.clone();
        s.borrow_mut().dirty = true;
        let hoist_3 = s.borrow().revision + 1;
        s.borrow_mut().revision = hoist_3;
        true
    }
    pub fn background(&self, hex: &str) -> Rc<RefCell<PptxSlideRef>> {
        if self.exists() {
            let mut s: Rc<RefCell<PptxSlide>> = self.model();
            s.borrow_mut().bg = PptxApi::colorOf(hex);
            s.borrow_mut().dirty = true;
        }
        self
    }
}

/// ------------------------------------------------------------------------------
/// The deck.
/// ------------------------------------------------------------------------------
///
/// Two ways out, and the difference matters more than it looks:
///
///   `save()`     writes over the package the deck was OPENED from. Every part
///                nobody here models — animations, embedded workbooks, custom
///                XML, the parts a future version of PowerPoint adds — is copied
///                through byte for byte, and only the slides marked dirty are
///                rewritten. This is what "edit a document" should mean.
///   `saveNew()`  writes a package built entirely from the model. Everything not
///                modelled is gone. It is the right answer for a deck this API
///                CREATED, and the wrong one for a deck it opened.
///
/// `save()` on a created deck falls back to `saveNew()`, because there is no
/// package to write over — so the common case needs no thought.
#[derive(Clone)]
pub struct PptxDeck {
    /// false when opening failed. `error` says why; nothing else is usable.
    pub ok: bool,
    pub error: String,
    pub pres: Rc<RefCell<PptxPresentation>>,
    /// The package this deck was read from, kept open so `save` can copy the
    /// parts nothing here models.
    pub parser: Rc<RefCell<PptxParser>>,
    pub opened: bool,
    /// The stylesheet, empty until `addStyleSheet` is called. Resolving an empty
    /// sheet over a deck sets nothing and costs one walk, so a deck that never
    /// mentions CSS is unaffected by any of this.
    pub css: Rc<RefCell<PptxCss>>,
    pub hasCss: bool,
}

impl PptxDeck {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            pres: Rc::new(RefCell::new(PptxPresentation::new())),
            parser: Rc::new(RefCell::new(PptxParser::new())),
            opened: false,
            css: Rc::new(RefCell::new(PptxCss::new())),
            hasCss: false,
        }
    }
    /// Add CSS over the deck.
    ///
    /// The rules are resolved when the deck is SAVED — or when `applyStyles` is
    /// called — rather than here, because a sheet is declarative and the boxes
    /// it describes are usually created after it. Called more than once, the
    /// later rules simply come later in source order, which is what a second
    /// sheet should mean.
    ///
    /// The selector subset is small on purpose: `.class`, `#id`, an element name
    /// (`slide`, `shape`, `textBox`, `picture`, `group`, `table`, `chart`,
    /// `paragraph`, `run`), the two combined, and the descendant combinator.
    /// `font-family`, `font-size`, `font-weight`, `font-style` and `color`
    /// inherit down to the runs; `fill`, `stroke` and `stroke-width` do not.
    /// Anything a slide cannot honour — `display: flex` and the rest of the
    /// layout properties — is reported through `styleWarnings` rather than
    /// ignored in silence.
    pub fn addStyleSheet(&mut self, text: &str) -> Rc<RefCell<PptxDeck>> {
        self.css.clone().borrow().addStyleSheet(text);
        self.hasCss = true;
        self
    }
    /// Resolve the sheet over every slide, shape, paragraph and run.
    ///
    /// Called for you by `save` and `saveNew`, and by the renderer before it
    /// draws — so the only reason to call it yourself is to READ a styled value
    /// back off the model before doing either.
    ///
    /// Idempotent: the cascade computes the same answer from the same sheet and
    /// the same classes, so calling it twice sets the same values twice.
    pub fn applyStyles(&self) -> Rc<RefCell<PptxDeck>> {
        if self.hasCss {
            let mut _tmp_6018 = self.pres.clone();
            self.css.clone().borrow_mut().applyTo(&_tmp_6018);
        }
        self
    }
    /// Property names the sheet used that a slide cannot honour, and the
    /// selectors this layer could not read. Empty when everything landed.
    pub fn styleWarnings(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut i: i64 = 0;
        while i < self.css.borrow().errorCount() {
            out.push(self.css.borrow().errorAt(i));
            i += 1;
        }
        let mut k: i64 = 0;
        while k < (self.css.borrow().unsupported.len() as i64) {
            out.push(format!("a slide has nowhere to put `{}`", self.css.borrow().unsupported[k as usize].clone()));
            k += 1;
        }
        out
    }
    pub fn slideCount(&self) -> i64 {
        self.pres.borrow().slides.len() as i64
    }
    pub fn slideAt(&self, i: i64) -> Rc<RefCell<PptxSlideRef>> {
        let mut r: Rc<RefCell<PptxSlideRef>> = Rc::new(RefCell::new(PptxSlideRef::new()));
        r.borrow_mut().deck = self.clone();
        r.borrow_mut().index = i;
        r
    }
    /// Points. A 16:9 deck is 960 by 540; the 4:3 default is 720 by 540.
    pub fn width(&self) -> f64 {
        self.pres.borrow().slideWidth
    }
    pub fn height(&self) -> f64 {
        self.pres.borrow().slideHeight
    }
    pub fn setSize(&self, w: f64, h: f64) -> Rc<RefCell<PptxDeck>> {
        self.pres.borrow_mut().slideWidth = w;
        self.pres.borrow_mut().slideHeight = h;
        let mut i: i64 = 0;
        while i < (self.pres.borrow().slides.len() as i64) {
            let mut s: Rc<RefCell<PptxSlide>> = self.pres.borrow()
                .slides[i as usize].clone();
            s.borrow_mut().width = w;
            s.borrow_mut().height = h;
            s.borrow_mut().dirty = true;
            i += 1;
        }
        self
    }
    pub fn addSlide(&self) -> Rc<RefCell<PptxSlideRef>> {
        let mut s: Rc<RefCell<PptxSlide>> = Rc::new(RefCell::new(PptxSlide::new()));
        s.borrow_mut().width = self.pres.borrow().slideWidth;
        s.borrow_mut().height = self.pres.borrow().slideHeight;
        s.borrow_mut().dirty = true;
        self.pres.borrow_mut().slides.push(s.clone());
        self.slideAt((self.pres.borrow().slides.len() as i64) - 1)
    }
    pub fn removeSlide(&self, i: i64) -> bool {
        if i < 0 || i >= (self.pres.borrow().slides.len() as i64) {
            return false;
        }
        let mut kept: Vec<Rc<RefCell<PptxSlide>>> = Vec::new();
        let mut k: i64 = 0;
        while k < (self.pres.borrow().slides.len() as i64) {
            if k != i {
                kept.push(self.pres.borrow().slides[k as usize].clone());
            }
            k += 1;
        }
        self.pres.borrow_mut().slides = kept.clone();
        true
    }
    /// Every character in the deck, slide by slide. What a search index reads.
    pub fn text(&self) -> String {
        let mut out: String = String::new();
        let mut i: i64 = 0;
        while i < (self.pres.borrow().slides.len() as i64) {
            let mut sl: Rc<RefCell<PptxSlideRef>> = self.slideAt(i);
            let t: String = sl.borrow().text();
            if !t.is_empty() {
                if !out.is_empty() {
                    out = format!("{}\n", out);
                }
                out = format!("{}{}", out, t);
            }
            i += 1;
        }
        out
    }
    pub fn save(&self) -> Vec<u8> {
        self.applyStyles();
        if !self.opened {
            return self.saveNew();
        }
        let mut w: Rc<RefCell<PptxWriter>> = Rc::new(RefCell::new(PptxWriter::new()));
        let mut _tmp_6022 = self.pres.clone();
        return w.borrow_mut().toBufferOver(&_tmp_6022, self.parser.borrow().pkg.clone());
    }
    pub fn saveNew(&self) -> Vec<u8> {
        self.applyStyles();
        let mut w: Rc<RefCell<PptxWriter>> = Rc::new(RefCell::new(PptxWriter::new()));
        return w.borrow_mut().toBuffer(&self.pres.clone());
    }
}

/// ------------------------------------------------------------------------------
/// The front door.
/// ------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PptxApi {
}

impl PptxApi {
    pub fn new() -> Self {
        Self {
        }
    }
    /// The version of this SURFACE, not of the repository. It changes when a
    /// method changes meaning, so a caller can tell what it is talking to.
    ///
    /// A function rather than an `sdef`: the compiler inlines a static constant
    /// at its use sites and emits nothing for it, so a constant no Ranger code
    /// reads does not exist in the compiled module at all — `Pptx.version` came
    /// out `undefined` in JavaScript and nothing had failed to build.
    pub fn version() -> String {
        "1".to_string()
    }
    /// @internal Every character in a shape and in everything under it.
    ///
    /// Recursive because a group is a shape with no text body and a list of
    /// shapes that have one. `PptxShapeRef.text` and `PptxSlideRef.text` both
    /// come here so that a grouped text box counts exactly once and reads the
    /// same whichever handle asked.
    pub fn textOf(sh: &Rc<RefCell<PptxShape>>) -> String {
        let mut out: String = String::new();
        if sh.borrow().hasText {
            out = (sh.borrow().text).borrow().plainText();
        }
        let mut i: i64 = 0;
        while i < (sh.borrow().children.len() as i64) {
            let mut child: Rc<RefCell<PptxShape>> = sh.borrow().children[i as usize].clone();
            let t: String = PptxApi::textOf(&child);
            if !t.is_empty() {
                if !out.is_empty() {
                    out = format!("{}\n", out);
                }
                out = format!("{}{}", out, t);
            }
            i += 1;
        }
        out
    }
    /// Open a .pptx from its bytes.
    ///
    /// Bytes rather than a path on purpose: this has to work in a browser, where
    /// there is no file system, and in a build step, where the deck arrived over
    /// HTTP. A caller with a path reads it and hands the bytes over.
    pub fn open(bytes: &[u8]) -> Rc<RefCell<PptxDeck>> {
        let mut deck: Rc<RefCell<PptxDeck>> = Rc::new(RefCell::new(PptxDeck::new()));
        let mut parser: Rc<RefCell<PptxParser>> = Rc::new(RefCell::new(PptxParser::new()));
        let got: bool = parser.borrow().openBytes(bytes, "deck.pptx");
        if !got {
            deck.borrow_mut().ok = false;
            deck.borrow_mut().error = "the bytes are not a readable .pptx package".to_string();
            return deck;
        }
        let mut raw: Rc<RefCell<PptxPresentation>> = parser.borrow().parsePresentation();
        let mut resolver: Rc<RefCell<PptxResolver>> = Rc::new(RefCell::new(PptxResolver::new()));
        deck.borrow_mut().pres = resolver.borrow().resolvePresentation(&raw);
        deck.borrow_mut().parser = parser.clone();
        deck.borrow_mut().opened = true;
        deck.borrow_mut().ok = true;
        deck
    }
    /// A deck with nothing in it: one 16:9 page size, no slides. `addSlide` is
    /// the next call.
    pub fn create() -> Rc<RefCell<PptxDeck>> {
        let mut deck: Rc<RefCell<PptxDeck>> = Rc::new(RefCell::new(PptxDeck::new()));
        deck.borrow_mut().pres.borrow_mut().slideWidth = 960.0;
        deck.borrow_mut().pres.borrow_mut().slideHeight = 540.0;
        deck.borrow_mut().opened = false;
        deck.borrow_mut().ok = true;
        deck
    }
    /// @internal Split on newlines. The language has no `split` operator and the three
    /// editors here each grew their own; this one is the API's, and it keeps
    /// empty lines — a caller who writes "a\n\nb" means a blank paragraph
    /// between them, and dropping it silently renumbers everything after.
    pub fn lines(s: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut cur: String = String::new();
        let mut i: i64 = 0;
        let n: i64 = s.as_bytes().len() as i64;
        while i < n {
            let cp: i64 = (s.as_bytes()[i as usize] as i64);
            if cp == 10 {
                out.push(cur.clone());
                cur = String::new();
            } else {
                if cp != 13 {
                    cur = format!("{}{}", cur, s[(i) as usize..(i + 1) as usize].to_string());
                }
            }
            i += 1;
        }
        out.push(cur);
        out
    }
    /// @internal "4472C4", "#4472C4" — and a name nobody can parse gives a colour that is
    /// not set, which the writer leaves out rather than writing black.
    pub fn colorOf(hex: &str) -> Rc<RefCell<PptxColor>> {
        let mut s: String = hex.trim().to_string();
        if s.is_empty() {
            let mut none: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
            return none;
        }
        // `charAt` answers a CODEPOINT, not a one-character string — 35 is '#'.
        // Comparing it against "#" compiles and is never true, which would have
        // sent "#4472C4" to the parser with the hash still on it.
        if (s.as_bytes()[0 as usize] as i64) == 35 {
            s = s[(1) as usize..(s.as_bytes().len() as i64) as usize].to_string();
        }
        PptxColor::srgbOf(s.clone())
    }
}
