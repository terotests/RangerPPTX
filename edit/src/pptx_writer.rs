// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pptx::text::PptxText;

use pptx::pptx_model::PptxBuildStep;
use pptx::pptx_model::PptxChart;
use pptx::pptx_model::PptxChartSeries;
use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxOpaque;
use pptx::pptx_model::PptxParagraph;
use pptx::pptx_model::PptxPresentation;
use pptx::pptx_model::PptxSection;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use pptx::pptx_model::PptxSheet;
use pptx::pptx_model::PptxSlide;
use pptx::pptx_model::PptxTableCell;
use pptx::pptx_model::PptxTableLine;
use pptx::pptx_model::PptxTextBody;
use pptx::pptx_model::PptxTextRun;
use pptx::pptx_model::PptxTheme;
use pptx::pptx_model::PptxTransition;
use pptx::rgr::OpcPackage;
use crate::rgr::OpcRelationship;
use pptx::rgr::SourceRef;
use pptx::smartart::sa_edit::SaEdit;
use pptx::smartart::sa_package::SaNumbering;
use pptx::smartart::sa_package::SaPackage;


/// ==============================================================================
/// PptxWriter - PptxModel → an OPC package a PowerPoint can open
/// ==============================================================================
///
/// The other direction. `PptxParser` reads a `.pptx` into the model; this writes
/// the model back out as PresentationML over `gallery/zip`'s ZipWriter, against
/// ECMA-376 rather than against anything another editor does.
///
///   package
///     [Content_Types].xml
///     _rels/.rels
///     ppt/presentation.xml            (+ _rels)
///     ppt/theme/theme1.xml
///     ppt/slideMasters/slideMaster1.xml   (+ _rels)
///     ppt/slideLayouts/slideLayout1.xml   (+ _rels)
///     ppt/slides/slideN.xml               (+ _rels)
///     ppt/media/…
///
/// What comes out is a FLAT deck: the model it is given is the resolved one, in
/// which master and layout chrome has already been merged into each slide, so
/// the written master and layout are blank and every slide carries its own
/// picture. The deck looks like what was on screen — which is the contract an
/// editor's "save" has — and it is not the same file that came in. Keeping the
/// original parts for anything the model does not describe is phase E3b; see
/// PLAN_EDITOR.md.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxWriteMedia {
    /// the model's package path
    pub part: String,
    /// file name inside ppt/media
    pub name: String,
    /// where it lands in the output package
    pub path: String,
    /// A picture that is already in the package we are writing over is
    /// referenced where it lies rather than copied under a new name.
    pub existing: bool,
    pub bytes: Vec<u8>,
    /// What makes two of these the same picture. It is derived from the bytes,
    /// never from `part` — see the note on `mediaIndexFor`.
    pub assetId: String,
}

impl PptxWriteMedia {
    pub fn new() -> Self {
        Self {
            part: String::new(),
            name: String::new(),
            path: String::new(),
            existing: false,
            bytes: vec![0u8; 0],
            assetId: String::new(),
        }
    }
}

/// The links one slide's runs click through to, collected while the slide
/// is written: each distinct address or slide once, by the relationship id
/// its runs name (`rIdLink1`, `rIdJump1`), and written into the slide's
/// relationship part after it. Off for a part that gets no relationships of
/// this kind (notes, a SmartArt drawing): its runs are written without one.
#[derive(Clone)]
pub struct PptxRunLinks {
    pub on: bool,
    /// how many slides the deck written has: a jump past them goes nowhere
    pub slideCount: i64,
    pub urls: Vec<String>,
    pub slides: Vec<i64>,
}

impl PptxRunLinks {
    pub fn new() -> Self {
        Self {
            on: false,
            slideCount: 0,
            urls: Vec::new(),
            slides: Vec::new(),
        }
    }
    /// the relationship id of `run`'s link, "" when it has none (or none
    /// this part can carry)
    pub fn idFor(&mut self, run: &Rc<RefCell<PptxTextRun>>) -> String {
        if !self.on {
            return String::new();
        }
        if run.borrow().linkSlide >= self.slideCount {
            return String::new();
        }
        if run.borrow().linkSlide >= 0 {
            let mut i: i64 = 0;
            while i < (self.slides.len() as i64) {
                if self.slides[i as usize] == run.borrow().linkSlide {
                    return format!("rIdJump{}", i + 1);
                }
                i += 1;
            }
            self.slides.push(run.borrow().linkSlide);
            return format!("rIdJump{}", self.slides.len() as i64);
        }
        if run.borrow().link.is_empty() || (run.borrow().link)[(0) as usize..(1) as usize].to_string() == "#" {
            return String::new();
        }
        let mut j: i64 = 0;
        while j < (self.urls.len() as i64) {
            if self.urls[j as usize].clone() == run.borrow().link {
                return format!("rIdLink{}", j + 1);
            }
            j += 1;
        }
        self.urls.push(run.borrow().link.clone());
        format!("rIdLink{}", self.urls.len() as i64)
    }
}

#[derive(Clone)]
pub struct PptxWriter {
    zip: Rc<RefCell<ZipWriter>>,
    pub media: Vec<Rc<RefCell<PptxWriteMedia>>>,
    pub charts: Vec<String>,
    pub chartPaths: Vec<String>,
    /// SmartArt laid out here, saved as SmartArt: five parts a diagram
    /// (data, layout, quick style, colours, drawing), their paths and bodies in
    /// that order, so diagram k's are at 5k … 5k+4
    pub diagramPaths: Vec<String>,
    pub diagramBodies: Vec<String>,
    /// the slide's relationship id for each, made from the part's own name so
    /// it cannot be one a slide written over still holds
    pub diagramRelIds: Vec<String>,
    pub shapeId: i64,
    /// Which media and which charts the slide being written uses, so its
    /// relationship part names those and not every picture in the deck.
    pub slideMediaUse: Vec<i64>,
    pub slideChartUse: Vec<i64>,
    pub slideDiagramUse: Vec<i64>,
    /// The notes part this slide will point at, if it has any notes.
    pub slideNotesPath: String,
    pub notesPaths: Vec<String>,
    pub notesBodies: Vec<String>,
    /// Parts of the package being written over, when there is one.
    pub sourceParts: HashMap<String,bool>,
    /// Every distinct picture this save has seen, keyed by its content. The
    /// writer used to key on the package path instead, which is how two
    /// different files both called `logo.png` ended up as one picture in the
    /// saved deck.
    assets: Rc<RefCell<OfficeAssetStore>>,
    pub overMode: bool,
    /// In over mode the master's chrome is already in the package, so a slide
    /// writes only what the slide itself declares.
    pub skipInherited: bool,
    /// The package being written over, so that a shape tree can copy the
    /// elements nobody here models — SmartArt, embedded objects, ink, vendor
    /// extensions — out of the part they came from and put them back where they
    /// were. Off unless `buildOver` turned it on: a FLAT save has no original to
    /// copy from, and writing a span out of the wrong file would be worse than
    /// dropping it.
    overPkg: Rc<RefCell<OpcPackage>>,
    pub keepOpaque: bool,
    /// the slide part being written over, while one is: a SmartArt frame is
    /// kept only on the slide whose relationships it names
    pub writingPart: String,
    /// part path → its text. One slide asks for the same part once per opaque
    /// element it holds, and a part is not cheap to decode twice.
    pub opaqueParts: HashMap<String,String>,
    /// the links of the slide being written, and where each slide of the deck
    /// is written (a slide jump's target)
    pub links: Rc<RefCell<PptxRunLinks>>,
    pub slideParts: Vec<String>,
}

impl PptxWriter {
    pub fn new() -> Self {
        Self {
            zip: Rc::new(RefCell::new(ZipWriter::new())),
            media: Vec::new(),
            charts: Vec::new(),
            chartPaths: Vec::new(),
            diagramPaths: Vec::new(),
            diagramBodies: Vec::new(),
            diagramRelIds: Vec::new(),
            shapeId: 1,
            slideMediaUse: Vec::new(),
            slideChartUse: Vec::new(),
            slideDiagramUse: Vec::new(),
            slideNotesPath: String::new(),
            notesPaths: Vec::new(),
            notesBodies: Vec::new(),
            sourceParts: HashMap::new(),
            assets: Rc::new(RefCell::new(OfficeAssetStore::new())),
            overMode: false,
            skipInherited: false,
            overPkg: Rc::new(RefCell::new(OpcPackage::new())),
            keepOpaque: false,
            writingPart: String::new(),
            opaqueParts: HashMap::new(),
            links: Rc::new(RefCell::new(PptxRunLinks::new())),
            slideParts: Vec::new(),
        }
    }
    /// --- units ---------------------------------------------------------------
    /// `to_int` FLOORS, so `v - 0.5` on a negative rounds outwards twice and a
    /// hanging indent of exactly -18pt came back as -228601 EMU instead of
    /// -228600 — the same defect this repository has met in the JPEG quantizer
    /// and in the path formatter. Round the magnitude and put the sign back:
    /// that is round-half-away-from-zero whichever way `to_int` goes.
    pub fn emu(pt: f64) -> String {
        let v: f64 = pt * 12700.0;
        if v < 0.0 {
            return format!("-{}", ((0.0 - v) + 0.5).floor() as i64);
        }
        ((v + 0.5).floor() as i64).to_string()
    }
    pub fn hundredths(pt: f64) -> String {
        if pt < 0.0 {
            return format!("-{}", ((0.0 - pt) * 100.0 + 0.5).floor() as i64);
        }
        let i: i64 = (pt * 100.0 + 0.5).floor() as i64;
        i.to_string()
    }
    pub fn pct(v: f64) -> String {
        let mut i: i64 = (v * 100000.0 + 0.5).floor() as i64;
        if i < 0 {
            i = 0;
        }
        if i > 100000 {
            i = 100000;
        }
        i.to_string()
    }
    pub fn deg60k(deg: f64) -> String {
        let mut d: f64 = deg;
        while d < 0.0 {
            d += 360.0;
        }
        while d >= 360.0 {
            d -= 360.0;
        }
        let i: i64 = (d * 60000.0 + 0.5).floor() as i64;
        i.to_string()
    }
    /// --- text ----------------------------------------------------------------
    /// Only the five ASCII specials are replaced; everything between them is
    /// copied as it is with `substring`. Rebuilding the text one `charAt` /
    /// `strfromcode` at a time is wrong on a target whose string unit is a
    /// UTF-8 byte (Go): each byte of "ä" came back as a character of its own.
    pub fn esc(s: &str) -> String {
        let mut out: String = String::new();
        let mut from: i64 = 0;
        let mut i: i64 = 0;
        let n: i64 = s.as_bytes().len() as i64;
        while i < n {
            let c: i64 = (s.as_bytes()[i as usize] as i64);
            let mut rep: String = "";
            if c == 38 {
                rep = "&amp;";
            }
            if c == 60 {
                rep = "&lt;";
            }
            if c == 62 {
                rep = "&gt;";
            }
            if c == 34 {
                rep = "&quot;";
            }
            if c == 39 {
                rep = "&apos;";
            }
            if !rep.is_empty() {
                out = format!("{}{}{}", out, s[(from) as usize..(i) as usize].to_string(), rep);
                from = i + 1;
            }
            i += 1;
        }
        format!("{}{}", out, s[(from) as usize..(n) as usize].to_string())
    }
    /// ZipWriter.addString writes one byte per UTF-16 unit, which is fine for
    /// ASCII and wrong for every deck written in a language that needs more.
    /// The XML parts go in as UTF-8 bytes instead.
    /// The text's code points (`to_chars`, the same on every target), not
    /// `charAt`: on Go that is already a UTF-8 byte, and encoding each byte
    /// again wrote "ä" as "Ã¤".
    pub fn utf8(s: &str) -> Vec<u8> {
        let mut cs: Vec<i64> = s.chars().map(|c| c as i64).collect::<Vec<i64>>();
        let n: i64 = cs.len() as i64;
        let mut tmp: Vec<u8> = vec![0u8; (n * 4) as usize];
        let mut w: i64 = 0;
        let mut i: i64 = 0;
        while i < n {
            let c: i64 = cs[i as usize];
            if c < 128 {
                tmp[w as usize] = (c) as u8;
                w += 1;
            } else {
                if c < 2048 {
                    tmp[w as usize] = (192 + (c / 64)) as u8;
                    tmp[(w + 1) as usize] = (128 + c % 64) as u8;
                    w += 2;
                } else {
                    if c < 65536 {
                        tmp[w as usize] = (224 + (c / 4096)) as u8;
                        tmp[(w + 1) as usize] = (128 + (c / 64) % 64) as u8;
                        tmp[(w + 2) as usize] = (128 + c % 64) as u8;
                        w += 3;
                    } else {
                        tmp[w as usize] = (240 + (c / 262144)) as u8;
                        tmp[(w + 1) as usize] = (128 + (c / 4096) % 64) as u8;
                        tmp[(w + 2) as usize] = (128 + (c / 64) % 64) as u8;
                        tmp[(w + 3) as usize] = (128 + c % 64) as u8;
                        w += 4;
                    }
                }
            }
            i += 1;
        }
        let mut out: Vec<u8> = vec![0u8; (w) as usize];
        let mut k: i64 = 0;
        while k < w {
            out[k as usize] = (tmp[k as usize] as i64) as u8;
            k += 1;
        }
        out
    }
    /// --- colours -------------------------------------------------------------
    pub fn colorXml(c: &Rc<RefCell<PptxColor>>) -> String {
        let mut inner: String = String::new();
        if c.borrow().alpha < 0.999 {
            inner = format!("<a:alpha val=\"{}\"/>", PptxWriter::pct(c.borrow().alpha));
        }
        // A colour that came out of a theme STYLE has already been through two
        // chains of modifiers, which no `schemeClr` can say — writing the slot
        // name would throw the adjustment away and flatten every themed
        // gradient back to its accent colour.
        if c.borrow().isScheme && !c.borrow().baked {
            if !inner.is_empty() {
                return format!("<a:schemeClr val=\"{}\">{}</a:schemeClr>", c.borrow().scheme, inner);
            }
            return format!("<a:schemeClr val=\"{}\"/>", c.borrow().scheme);
        }
        let mut hex: String = c.borrow().srgb.clone();
        if (hex.as_bytes().len() as i64) != 6 {
            hex = "000000".to_string();
        }
        if !inner.is_empty() {
            return format!("<a:srgbClr val=\"{}\">{}</a:srgbClr>", hex, inner);
        }
        format!("<a:srgbClr val=\"{}\"/>", hex)
    }
    pub fn fillXml(shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().fillGrad.borrow().isSet {
            let mut ang: String = "5400000";
            if shape.borrow().fillGrad.borrow().dir == 1 {
                ang = "0";
            }
            let a: String = format!("<a:gs pos=\"0\">{}</a:gs>", PptxWriter::colorXml(&shape.borrow().fillGrad.borrow().c1.clone()));
            let b: String = format!("<a:gs pos=\"100000\">{}</a:gs>", PptxWriter::colorXml(&shape.borrow().fillGrad.borrow().c2.clone()));
            return format!("<a:gradFill rotWithShape=\"1\"><a:gsLst>{}{}</a:gsLst><a:lin ang=\"{}\" scaled=\"0\"/></a:gradFill>", a, b, ang);
        }
        if !shape.borrow().fill.borrow().isSet {
            return "<a:noFill/>".to_string();
        }
        return format!("<a:solidFill>{}</a:solidFill>", PptxWriter::colorXml(&shape.borrow().fill.clone()));
    }
    pub fn lineXml(shape: &Rc<RefCell<PptxShape>>) -> String {
        if !shape.borrow().line.borrow().isSet {
            return String::new();
        }
        let mut w: f64 = shape.borrow().lineWidthPt;
        if w <= 0.0 {
            w = 1.0;
        }
        return format!("<a:ln w=\"{}\"><a:solidFill>{}</a:solidFill></a:ln>", PptxWriter::emu(w), PptxWriter::colorXml(&shape.borrow().line.clone()));
    }
    /// An outer shadow is stated as a distance and a direction, and the model
    /// keeps an offset — so the polar form is computed here rather than stored.
    pub fn effectXml(shape: &Rc<RefCell<PptxShape>>) -> String {
        if !shape.borrow().shadow.borrow().isSet {
            return String::new();
        }
        let dx: f64 = shape.borrow().shadow.borrow().offsetXPt;
        let dy: f64 = shape.borrow().shadow.borrow().offsetYPt;
        let dist: f64 = (dx * dx + dy * dy).sqrt();
        let ang: f64 = dy.atan2(dx);
        let deg: f64 = (ang * 180.0) / 3.141592653589793;
        let mut col: Rc<RefCell<PptxColor>> = Rc::new(RefCell::new(PptxColor::new()));
        col.borrow_mut().isSet = true;
        col.borrow_mut().isScheme = shape.borrow().shadow.borrow().color.borrow().isScheme;
        col.borrow_mut().scheme = shape.borrow().shadow.borrow().color.borrow().scheme.clone();
        col.borrow_mut().srgb = shape.borrow().shadow.borrow().color.borrow().srgb.clone();
        col.borrow_mut().alpha = shape.borrow().shadow.borrow().alpha;
        return format!("<a:effectLst><a:outerShdw blurRad=\"{}\" dist=\"{}\" dir=\"{}\" algn=\"tl\" rotWithShape=\"0\">{}</a:outerShdw></a:effectLst>", PptxWriter::emu(shape.borrow().shadow.borrow().blurPt.clone()), PptxWriter::emu(dist), PptxWriter::deg60k(deg), PptxWriter::colorXml(&col));
    }
    /// --- geometry ------------------------------------------------------------
    pub fn xfrmXml(shape: &Rc<RefCell<PptxShape>>, tag: &str, with_child: bool) -> String {
        let mut rot: String = String::new();
        if shape.borrow().rotation > 0.001 || shape.borrow().rotation < -0.001 {
            rot = format!(" rot=\"{}\"", PptxWriter::deg60k(shape.borrow().rotation));
        }
        if shape.borrow().flipH {
            rot = format!("{} flipH=\"1\"", rot);
        }
        if shape.borrow().flipV {
            rot = format!("{} flipV=\"1\"", rot);
        }
        let mut _tmp_5004 = shape.borrow().x.clone();
        let mut _tmp_5005 = shape.borrow().y.clone();
        let mut _tmp_5006 = shape.borrow().width.clone();
        let mut out: String = format!("<a:xfrm{}><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/>", rot, PptxWriter::emu(_tmp_5004), PptxWriter::emu(_tmp_5005), PptxWriter::emu(_tmp_5006), PptxWriter::emu(shape.borrow().height.clone()));
        if with_child {
            let mut cw: f64 = shape.borrow().chW;
            let mut ch: f64 = shape.borrow().chH;
            if cw <= 0.0 {
                cw = shape.borrow().width;
            }
            if ch <= 0.0 {
                ch = shape.borrow().height;
            }
            out = format!("{}<a:chOff x=\"{}\" y=\"{}\"/><a:chExt cx=\"{}\" cy=\"{}\"/>", out, PptxWriter::emu(shape.borrow().chX), PptxWriter::emu(shape.borrow().chY), PptxWriter::emu(cw), PptxWriter::emu(ch));
        }
        format!("{}</a:xfrm>", out)
    }
    /// A flattened path is written as one, rather than as the preset it stopped
    /// being — the points are what the viewer drew.
    ///
    /// TWO POINTS IS A PATH. The threshold used to be three, and a two-point
    /// path — a hand-drawn straight line, a chart's axis rule — fell through to
    /// whatever preset the shape still carried underneath, which is a rectangle.
    ///
    /// AND IT IS NOT ALWAYS CLOSED. `<a:close/>` used to be written on every
    /// one, which the parser then read back as `pathClosed` — so a stroke that
    /// was open when it was drawn came back joined end to end, and if it had a
    /// fill it came back as a solid blob. `PptxShape.pathClosed` has recorded
    /// the answer since the parser learned to read it; this is the writer
    /// finally asking.
    /// @internal One contour, as one `<a:path>`.
    pub fn pathRingXml(pts: &[f64], from: i64, upto: i64, closed: bool) -> String {
        let n: i64 = (upto - from) / 2;
        if n < 2 {
            return String::new();
        }
        let mut out: String = "<a:path w=\"100000\" h=\"100000\">".to_string();
        let mut i: i64 = 0;
        while i < n {
            let px: f64 = pts[(from + i * 2) as usize];
            let py: f64 = pts[((from + i * 2) + 1) as usize];
            let sx: String = ((px * 100000.0 + 0.5).floor() as i64).to_string();
            let sy: String = ((py * 100000.0 + 0.5).floor() as i64).to_string();
            let pt: String = format!("<a:pt x=\"{}\" y=\"{}\"/>", sx, sy);
            if i == 0 {
                out = format!("{}<a:moveTo>{}</a:moveTo>", out, pt);
            } else {
                out = format!("{}<a:lnTo>{}</a:lnTo>", out, pt);
            }
            i += 1;
        }
        if closed {
            out = format!("{}<a:close/>", out);
        }
        format!("{}</a:path>", out)
    }
    pub fn geomXml(shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().hasPath {
            let n: i64 = (shape.borrow().pathUnit.len() as i64) / 2;
            if n >= 2 {
                let mut out: String = "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"100000\" b=\"100000\"/><a:pathLst>".to_string();
                // ONE `<a:path>` PER CONTOUR.
                //
                // This wrote every point into a single path, which is right for
                // the one-ring shapes a viewer draws and wrong for anything
                // with a hole in it: PowerPoint would join the end of each
                // contour to the start of the next and fill the holes solid.
                // `pathEnds` is empty on every shape that has one contour, so
                // that case still writes exactly what it wrote before.
                let mut start: i64 = 0;
                let mut r: i64 = 0;
                let mut ring_count: i64 = shape.borrow().pathEnds.len() as i64;
                if ring_count == 0 {
                    ring_count = 1;
                }
                //
                // …except that a CLOSED shape with a hole in it is one path:
                // each `<a:path>` is filled on its own, so a glyph's counter
                // (the inside of a 6, an o) written as a second path was
                // filled solid over the letter. Contours that are subpaths of
                // one path — a `moveTo` each, as the preset donut is drawn —
                // are filled together, and the inner one is a hole.
                let one_fill: bool = shape.borrow().pathClosed && ring_count > 1;
                if one_fill {
                    out = format!("{}<a:path w=\"100000\" h=\"100000\">", out);
                }
                while r < ring_count {
                    let mut upto: i64 = shape.borrow().pathUnit.len() as i64;
                    if !shape.borrow().pathEnds.is_empty() {
                        upto = shape.borrow().pathEnds[r as usize];
                    }
                    let mut _tmp_5008 = shape.borrow().pathUnit.clone();
                    let mut ring: String = PptxWriter::pathRingXml(&_tmp_5008, start, upto, shape.borrow().pathClosed.clone());
                    if one_fill && !ring.is_empty() {
                        // the ring without its own `<a:path>` around it
                        let open: i64 = PptxText::index_of(&ring, ">");
                        ring = ring[(open + 1) as usize..((ring.as_bytes().len() as i64) - ("</a:path>".to_string().as_bytes().len() as i64)) as usize].to_string();
                    }
                    out = format!("{}{}", out, ring);
                    start = upto;
                    r += 1;
                }
                if one_fill {
                    out = format!("{}</a:path>", out);
                }
                return format!("{}</a:pathLst></a:custGeom>", out);
            }
        }
        let mut preset: String = shape.borrow().preset.clone();
        if preset.is_empty() {
            preset = "rect".to_string();
        }
        // The shape's adjustment, when it has one — a rounded rectangle whose
        // corners are written away comes back with the default ones.
        let mut av: String = "<a:avLst/>".to_string();
        if shape.borrow().adj >= 0.0 {
            let v: i64 = (shape.borrow().adj * 100000.0 + 0.5).floor() as i64;
            av = format!("<a:avLst><a:gd name=\"adj\" fmla=\"val {}\"/></a:avLst>", v);
        }
        format!("<a:prstGeom prst=\"{}\">{}</a:prstGeom>", preset, av)
    }
    /// --- text bodies ---------------------------------------------------------
    /// Is this still exactly what the resolver put here for a run that stated
    /// nothing? Then the file said nothing, and saying it now would freeze it —
    /// the run would stop following the list style, the placeholder and the
    /// theme's font scheme. Comparing the VALUE rather than trusting a flag is
    /// what makes a later edit come out as an edit; see `PptxShape.styleFillSig`.
    pub fn stillInherited(sig: &str, now: &str, keep_inherited: bool) -> bool {
        if !keep_inherited {
            return false;
        }
        if sig.is_empty() {
            return false;
        }
        sig == now
    }
    /// An equation run: Office Math inside `a14:m`. Every `m:r` of it carries
    /// the run's own properties, so the formula has the colour and the size of
    /// the text around it, in the face PowerPoint sets equations in.
    pub fn mathXml(run: &Rc<RefCell<PptxTextRun>>, attrs: &str, inner: &str) -> String {
        let latin: String = "<a:latin typeface=\"Cambria Math\" panose=\"02040503050406030204\" pitchFamily=\"18\" charset=\"0\"/>";
        // the run's face goes: an equation is set in the math face, which
        // comes last of what `inner` holds (after `a:highlight`)
        let mut body: String = inner.to_string();
        let at: i64 = PptxText::index_of(&body, "<a:latin ");
        if at >= 0 {
            let end: i64 = PptxWriter::indexFrom(&body, "/>", at);
            body = format!("{}{}", body[(0) as usize..(at) as usize].to_string(), body[(end + 2) as usize..(body.as_bytes().len() as i64) as usize].to_string());
        }
        body = format!("{}{}", body, latin);
        let r_pr: String = format!("<a:rPr{}>{}</a:rPr>", attrs, body);
        let src: String = run.borrow().math.clone();
        let mut out: String = String::new();
        let mut from: i64 = 0;
        let mut at2: i64 = PptxWriter::indexFrom(&src, "<m:t", from);
        while at2 >= 0 {
            // `<m:t>` or `<m:t xml:space=…>`, and not `<m:type`
            let after: i64 = (src.as_bytes()[(at2 + 4) as usize] as i64);
            if after == 32 || after == 62 {
                out = format!("{}{}{}", out, src[(from) as usize..(at2) as usize].to_string(), r_pr);
                from = at2;
            }
            at2 = PptxWriter::indexFrom(&src, "<m:t", at2 + 1);
        }
        out = format!("{}{}", out, src[(from) as usize..(src.as_bytes().len() as i64) as usize].to_string());
        let ns: String = "xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"";
        if run.borrow().mathDisplay {
            return format!("<a14:m><m:oMathPara {}><m:oMathParaPr><m:jc m:val=\"centerGroup\"/></m:oMathParaPr><m:oMath>{}</m:oMath></m:oMathPara></a14:m>", ns, out);
        }
        format!("<a14:m><m:oMath {}>{}</m:oMath></a14:m>", ns, out)
    }
    pub fn indexFrom(s: &str, needle: &str, from: i64) -> i64 {
        let at: i64 = PptxText::index_of(&s[(from) as usize..(s.as_bytes().len() as i64) as usize].to_string(), needle);
        if at < 0 {
            return at;
        }
        at + from
    }
    pub fn hasMath(body: &Rc<RefCell<PptxTextBody>>) -> bool {
        let mut i: i64 = 0;
        while i < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[i as usize].clone();
            let mut j: i64 = 0;
            while j < (para.borrow().runs.len() as i64) {
                let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[j as usize].clone();
                if !run.borrow().math.is_empty() {
                    return true;
                }
                j += 1;
            }
            i += 1;
        }
        false
    }
    pub fn runXml(run: &Rc<RefCell<PptxTextRun>>, keep_inherited: bool, with_math: bool, links: &Rc<RefCell<PptxRunLinks>>) -> String {
        let mut attrs: String = " lang=\"en-US\" dirty=\"0\"".to_string();
        let size_now: String = ((run.borrow().fontSizePt * 100.0 + 0.5).floor() as i64).to_string();
        if PptxWriter::stillInherited(&run.borrow().resolvedSizeSig.clone(), &size_now, keep_inherited) {
        } else {
            attrs = format!("{} sz=\"{}\"", attrs, PptxWriter::hundredths(run.borrow().fontSizePt));
        }
        // A run that says it is NOT bold has to say so. Writing nothing lets
        // the placeholder's own weight back in on the next open, which is the
        // same conflation on the way out.
        if run.borrow().boldSet {
            if run.borrow().bold {
                attrs = format!("{} b=\"1\"", attrs);
            } else {
                attrs = format!("{} b=\"0\"", attrs);
            }
        } else {
            if run.borrow().bold {
                attrs = format!("{} b=\"1\"", attrs);
            }
        }
        if run.borrow().italicSet {
            if run.borrow().italic {
                attrs = format!("{} i=\"1\"", attrs);
            } else {
                attrs = format!("{} i=\"0\"", attrs);
            }
        } else {
            if run.borrow().italic {
                attrs = format!("{} i=\"1\"", attrs);
            }
        }
        let mut inner: String = String::new();
        if run.borrow().color.borrow().isSet {
            if PptxWriter::stillInherited(&run.borrow().resolvedColorSig.clone(), &(run.borrow().color).borrow().signature(), keep_inherited) {
            } else {
                inner = format!("<a:solidFill>{}</a:solidFill>", PptxWriter::colorXml(&run.borrow().color));
            }
        }
        // CT_TextCharacterProperties is a sequence: fill, effects, highlight,
        // then the faces. A highlight after `a:latin` is a file PowerPoint
        // repairs.
        if run.borrow().glowPt > 0.0 {
            inner = format!("{}<a:effectLst><a:glow rad=\"{}\">{}</a:glow></a:effectLst>", inner, PptxWriter::emu(run.borrow().glowPt), PptxWriter::colorXml(&run.borrow().glow));
        }
        if run.borrow().highlight.borrow().isSet {
            inner = format!("{}<a:highlight>{}</a:highlight>", inner, PptxWriter::colorXml(&run.borrow().highlight));
        }
        if !run.borrow().fontFamily.is_empty() {
            let mut _tmp_5012 = run.borrow().resolvedFontSig.clone();
            if PptxWriter::stillInherited(&_tmp_5012, &run.borrow().fontFamily.clone(), keep_inherited) {
            } else {
                inner = format!("{}<a:latin typeface=\"{}\"/>", inner, PptxWriter::esc(&run.borrow().fontFamily));
            }
        }
        // after the faces, as the sequence has it
        let rid: String = links.borrow_mut().idFor(run);
        if !rid.is_empty() {
            if run.borrow().linkSlide >= 0 {
                inner = format!("{}<a:hlinkClick r:id=\"{}\" action=\"ppaction://hlinksldjump\"/>", inner, rid);
            } else {
                inner = format!("{}<a:hlinkClick r:id=\"{}\"/>", inner, rid);
            }
        }
        let mut r_pr: String = format!("<a:rPr{}/>", attrs);
        if !inner.is_empty() {
            r_pr = format!("<a:rPr{}>{}</a:rPr>", attrs, inner);
        }
        // xml:space matters: a run of spaces between two words is content.
        if with_math && !run.borrow().math.is_empty() {
            return PptxWriter::mathXml(run, &attrs, &inner);
        }
        if !run.borrow().field.is_empty() {
            // A field keeps its type and the value it last had, which is what
            // every reader shows until it computes its own.
            let mut _tmp_5014 = run.borrow().field.clone();
            return format!("{}{}\">{}<a:t>{}</a:t></a:fld>", "<a:fld id=\"{B4D1F0A1-0000-4000-9000-000000000001}\" type=\"".to_string(), PptxWriter::esc(&_tmp_5014), r_pr, PptxWriter::esc(&run.borrow().text.clone()));
        }
        return format!("<a:r>{}<a:t xml:space=\"preserve\">{}</a:t></a:r>", r_pr, PptxWriter::esc(&run.borrow().text.clone()));
    }
    pub fn paraXml(para: &Rc<RefCell<PptxParagraph>>, keep_inherited: bool, with_math: bool, links: &Rc<RefCell<PptxRunLinks>>) -> String {
        let mut attrs: String = String::new();
        if para.borrow().level > 0 {
            attrs = format!("{} lvl=\"{}\"", attrs, para.borrow().level);
        }
        if para.borrow().alignSet {
            attrs = format!("{} algn=\"{}\"", attrs, para.borrow().align);
        }
        // What the level style resolved to, written down. A deck saved from
        // here has no master to inherit from any more — the flat writer's whole
        // contract — so the indent and the bullet have to be stated, or a list
        // comes back as a column of unindented lines.
        if para.borrow().marginLeftPt > 0.0 {
            attrs = format!("{} marL=\"{}\"", attrs, PptxWriter::emu(para.borrow().marginLeftPt));
        }
        if para.borrow().indentPt != 0.0 {
            attrs = format!("{} indent=\"{}\"", attrs, PptxWriter::emu(para.borrow().indentPt));
        }
        let mut inner: String = String::new();
        // CT_TextParagraphProperties is a sequence: the line spacing comes
        // before the space above and below, and PowerPoint repairs a file
        // that puts it after.
        if para.borrow().lineSpacingPct != 100.0 {
            inner = format!("{}<a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>", inner, (para.borrow().lineSpacingPct * 1000.0 + 0.5).floor() as i64);
        }
        if para.borrow().spaceBeforePt > 0.0 {
            inner = format!("{}<a:spcBef><a:spcPts val=\"{}\"/></a:spcBef>", inner, (para.borrow().spaceBeforePt * 100.0 + 0.5).floor() as i64);
        }
        if para.borrow().spaceAfterPt > 0.0 {
            inner = format!("{}<a:spcAft><a:spcPts val=\"{}\"/></a:spcAft>", inner, (para.borrow().spaceAfterPt * 100.0 + 0.5).floor() as i64);
        }
        // A paragraph that said nothing about its bullet inherits one from the
        // list style. Writing either the resolved `buChar` or a flat `buNone`
        // here decides for it — the first bakes the level's bullet onto the
        // paragraph, the second turns it off — and both stop it following the
        // placeholder. `bulletSet` is the paragraph's own answer, and
        // `<a:buNone/>` IS an answer.
        let mut says_bullet: bool = true;
        if keep_inherited {
            says_bullet = para.borrow().bulletSet;
        }
        if !says_bullet {
        } else {
            if para.borrow().hasBullet {
                if para.borrow().bulletColor.borrow().isSet {
                    inner = format!("{}<a:buClr>{}</a:buClr>", inner, PptxWriter::colorXml(&para.borrow().bulletColor));
                }
                if para.borrow().bulletSizePt > 0.0 {
                    inner = format!("{}<a:buSzPts val=\"{}\"/>", inner, (para.borrow().bulletSizePt * 100.0 + 0.5).floor() as i64);
                } else {
                    if para.borrow().bulletSizePct != 100.0 {
                        inner = format!("{}<a:buSzPct val=\"{}\"/>", inner, (para.borrow().bulletSizePct * 1000.0 + 0.5).floor() as i64);
                    }
                }
                if !para.borrow().bulletFont.is_empty() {
                    inner = format!("{}<a:buFont typeface=\"{}\"/>", inner, PptxWriter::esc(&para.borrow().bulletFont));
                }
                // A numbered item says WHICH numbering, not which number: the
                // count is the paragraph's place in the run of items above it, and
                // writing a `buChar` here would turn every number back into a dot.
                if para.borrow().bulletAutoNum {
                    let mut num_attrs: String = format!(" type=\"{}\"", PptxWriter::esc(&para.borrow().bulletScheme.clone()));
                    if para.borrow().bulletStartAt != 1 {
                        num_attrs = format!("{} startAt=\"{}\"", num_attrs, para.borrow().bulletStartAt);
                    }
                    inner = format!("{}<a:buAutoNum{}/>", inner, num_attrs);
                } else {
                    inner = format!("{}<a:buChar char=\"{}\"/>", inner, PptxWriter::esc(&para.borrow().bulletChar));
                }
            } else {
                inner = format!("{}<a:buNone/>", inner);
            }
        }
        // An empty `<a:pPr></a:pPr>` states nothing, and a paragraph that
        // stated nothing should not grow one on the way out.
        let mut p_pr: String = format!("<a:pPr{}>{}</a:pPr>", attrs, inner);
        if attrs.is_empty() {
            if inner.is_empty() {
                p_pr = String::new();
            }
        }
        let mut out: String = format!("<a:p>{}", p_pr);
        let mut i: i64 = 0;
        while i < (para.borrow().runs.len() as i64) {
            let mut run: Rc<RefCell<PptxTextRun>> = para.borrow().runs[i as usize].clone();
            // A run whose text is a lone newline came from `<a:br/>` and goes
            // back as one: written as a `t` it would be whitespace PowerPoint
            // collapses, and the line break would be lost on the round trip.
            if run.borrow().text == "\n" {
                out = format!("{}<a:br/>", out);
            } else {
                out = format!("{}{}", out, PptxWriter::runXml(&run, keep_inherited, with_math, links));
            }
            i += 1;
        }
        if para.borrow().runs.is_empty() {
            out = format!("{}<a:endParaRPr lang=\"en-US\"/>", out);
        }
        format!("{}</a:p>", out)
    }
    pub fn bodyXml(body: &Rc<RefCell<PptxTextBody>>, keep_inherited: bool, with_math: bool, links: &Rc<RefCell<PptxRunLinks>>) -> String {
        let mut _tmp_5018 = body.borrow().insetL.clone();
        let mut _tmp_5019 = body.borrow().insetT.clone();
        let mut _tmp_5020 = body.borrow().insetR.clone();
        let mut attrs: String = format!(" lIns=\"{}\" tIns=\"{}\" rIns=\"{}\" bIns=\"{}\"", PptxWriter::emu(_tmp_5018), PptxWriter::emu(_tmp_5019), PptxWriter::emu(_tmp_5020), PptxWriter::emu(body.borrow().insetB.clone()));
        attrs = format!("{} anchor=\"{}\"", attrs, body.borrow().anchor);
        if body.borrow().wrap {
            attrs = format!("{} wrap=\"square\"", attrs);
        } else {
            attrs = format!("{} wrap=\"none\"", attrs);
        }
        // Text columns. Written only when there is more than one, so a deck
        // that never asked for them is byte-identical to what it was.
        if body.borrow().columns > 1 {
            attrs = format!("{} numCol=\"{}\"", attrs, body.borrow().columns);
            if body.borrow().columnGapPt > 0.0 {
                attrs = format!("{} spcCol=\"{}\"", attrs, PptxWriter::emu(body.borrow().columnGapPt));
            }
        }
        let mut out: String = format!("<p:txBody><a:bodyPr{}/><a:lstStyle/>", attrs);
        let mut i: i64 = 0;
        while i < (body.borrow().paragraphs.len() as i64) {
            let mut para: Rc<RefCell<PptxParagraph>> = body.borrow().paragraphs[i as usize].clone();
            out = format!("{}{}", out, PptxWriter::paraXml(&para, keep_inherited, with_math, links));
            i += 1;
        }
        if body.borrow().paragraphs.is_empty() {
            out = format!("{}<a:p><a:endParaRPr lang=\"en-US\"/></a:p>", out);
        }
        format!("{}</p:txBody>", out)
    }
    pub fn relTargetFrom(from_dir: &str, path: &str) -> String {
        // Everything in a presentation package lives under ppt/, so a part is
        // named from a slide as "../<rest>".
        if PptxText::index_of(&path, "ppt/") == 0 {
            return format!("../{}", path[(4) as usize..(path.as_bytes().len() as i64) as usize].to_string());
        }
        format!("/{}", path)
    }
    /// The `descr` attribute of a `p:cNvPr`, or nothing.
    ///
    /// A deck's alt text is the only thing a screen reader can say about a
    /// picture, and a writer that leaves it out does not merely fail to add
    /// accessibility — it DELETES the author's, the first time the deck is
    /// edited here. That is worse than never having read it.
    /// `p:cNvPr`, which is either a self-closing tag or one with an extension
    /// list inside it.
    ///
    /// `a:extLst` is the place OOXML sets aside for an application's own data.
    /// A consumer that does not know the URI is required to PRESERVE the
    /// extension rather than understand it, which is what makes it the right
    /// carrier for a diagram's source: PowerPoint round-trips it untouched and
    /// this reader can pick it back up.
    ///
    /// The source is element CONTENT and not an attribute, and that is not a
    /// style choice: XML attribute-value normalisation turns every newline into
    /// a space, so a Mermaid diagram written into an attribute comes back as
    /// one line and no longer parses.
    pub fn cNvPrXml(id: i64, name: &str, shape: &Rc<RefCell<PptxShape>>) -> String {
        let head: String = format!("<p:cNvPr id=\"{}\" name=\"{}\"{}", id, PptxWriter::esc(name), PptxWriter::descrAttr(shape));
        if shape.borrow().embedSource.is_empty() {
            return format!("{}/>", head);
        }
        let uri: String = PptxWriter::embedUri();
        let mut ext: String = format!("<a:extLst><a:ext uri=\"{}\">", uri);
        // `xml:space="preserve"`, because a reader that trims the content
        // gives back a diagram whose first line lost its indentation — and a
        // source that is nearly right is worse than one that is missing.
        ext = format!("{}<rgr:embed xmlns:rgr=\"urn:ranger:embed\" xml:space=\"preserve\" notation=\"{}\">", ext, PptxWriter::esc(&shape.borrow().embedNotation));
        ext = format!("{}{}</rgr:embed></a:ext></a:extLst>", ext, PptxWriter::esc(&shape.borrow().embedSource));
        format!("{}>{}</p:cNvPr>", head, ext)
    }
    pub fn embedUri() -> String {
        "{7A1B9C40-0B9E-4C2E-9E7E-5F2B6D3A1E11}".to_string()
    }
    pub fn descrAttr(shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().altText.is_empty() {
            return String::new();
        }
        return format!(" descr=\"{}\"", PptxWriter::esc(&shape.borrow().altText.clone()));
    }
    /// The extension of a part name, "" when it has none that is a file
    /// type (no dot, or what follows the last one is not letters and digits:
    /// a name such as "three:apps/w.tsx#1:1"), and then the bytes say.
    pub fn extensionOf(part: &str) -> String {
        let n: i64 = part.as_bytes().len() as i64;
        let mut i: i64 = n - 1;
        while i >= 0 {
            let c: i64 = (part.as_bytes()[i as usize] as i64);
            if c == 46 {
                let ext: String = part[(i + 1) as usize..(n) as usize].to_string();
                if ext.is_empty() {
                    return String::new();
                }
                if ext == "jpg" {
                    return "jpeg".to_string();
                }
                return ext;
            }
            let alnum: bool = ((c >= 48 && c <= 57) || (c >= 65 && c <= 90)) || (c >= 97 && c <= 122);
            if !alnum {
                return String::new();
            }
            i -= 1;
        }
        String::new()
    }
    pub fn styleRef(name: &str, idx: i64, color: &Rc<RefCell<PptxColor>>, extra: &str) -> String {
        let mut out: String = format!("<{} idx=\"{}\">", name, idx);
        out = format!("{}{}", out, PptxWriter::refColorXml(color));
        format!("{}</{}>", out, name)
    }
    /// `a:fontRef` numbers its index with a word rather than a number.
    pub fn styleRefNamed(name: &str, idx: &str, color: &Rc<RefCell<PptxColor>>) -> String {
        let mut out: String = format!("<{} idx=\"{}\">", name, PptxWriter::esc(idx));
        out = format!("{}{}", out, PptxWriter::refColorXml(color));
        format!("{}</{}>", out, name)
    }
    /// The colour INSIDE a style reference, which is a bare colour element with
    /// no `a:solidFill` around it — the reference is already the fill.
    pub fn refColorXml(color: &Rc<RefCell<PptxColor>>) -> String {
        if !color.borrow().isSet {
            // `<a:schemeClr val="phClr"/>` is what a style reference means by
            // "whatever the theme says": it is the placeholder colour the
            // format matrix substitutes into. A reference with no colour at
            // all is not what the file said.
            return "<a:schemeClr val=\"phClr\"/>".to_string();
        }
        PptxWriter::colorXml(color)
    }
    /// `a:srcRect` for a picture shown in part, in thousandths of a percent
    /// cut off each side; "" for a whole one.
    pub fn srcRectXml(shape: &Rc<RefCell<PptxShape>>) -> String {
        if ((shape.borrow().cropL + shape.borrow().cropT) + shape.borrow().cropR) + shape.borrow().cropB < 0.00001 {
            return String::new();
        }
        let mut out: String = "<a:srcRect".to_string();
        out = format!("{}{}", out, PptxWriter::cropAttr("l", shape.borrow().cropL));
        out = format!("{}{}", out, PptxWriter::cropAttr("t", shape.borrow().cropT));
        out = format!("{}{}", out, PptxWriter::cropAttr("r", shape.borrow().cropR));
        out = format!("{}{}", out, PptxWriter::cropAttr("b", shape.borrow().cropB));
        format!("{}/>", out)
    }
    pub fn cropAttr(name: &str, v: f64) -> String {
        if v < 0.00001 {
            return String::new();
        }
        let n: i64 = (v * 100000.0 + 0.5).floor() as i64;
        format!(" {}=\"{}\"", name, n)
    }
    /// A blip that is the PNG `pngRid` with the SVG `svgRid` over it
    /// (MS-ODRAWXML 2.3.6.8, `asvg:svgBlip` in the blip's extension list).
    pub fn svgBlipXml(png_rid: &str, svg_rid: &str) -> String {
        format!("<a:blip r:embed=\"{}{}{}\"/></a:ext></a:extLst></a:blip>", png_rid, "\"><a:extLst><a:ext uri=\"{96DAC541-7B7A-43D3-8B79-37D633B846F1}\"><asvg:svgBlip xmlns:asvg=\"http://schemas.microsoft.com/office/drawing/2016/SVG/main\" r:embed=\"".to_string(), svg_rid)
    }
    pub fn cellLineXml(name: &str, line: &Rc<RefCell<PptxTableLine>>) -> String {
        if !line.borrow().isSet {
            return String::new();
        }
        if !line.borrow().color.borrow().isSet {
            return String::new();
        }
        let mut _tmp_5023 = line.borrow().widthPt.clone();
        return format!("<{} w=\"{}\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill>{}</a:solidFill><a:prstDash val=\"solid\"/></{}>", name, PptxWriter::emu(_tmp_5023), PptxWriter::colorXml(&line.borrow().color.clone()), name);
    }
    /// The tree's own opening and its properties. The writer states its own
    /// rather than keeping the original's, because a group transform that no
    /// longer matches the shapes under it is worse than a neutral one.
    pub fn spTreeOpen() -> String {
        "<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>".to_string()
    }
    /// --- notes ---------------------------------------------------------------
    ///
    /// The speaker's notes live in a part of their own, which the slide points
    /// at. Only the body placeholder is what anyone typed — the rest of a notes
    /// slide is a picture of the slide and its number.
    pub fn notesXml(notes: &Rc<RefCell<PptxShape>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:notes xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree>".to_string();
        out = format!("{}<p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>{}", out, PptxWriter::groupXfrm());
        // the picture of the slide, and the notes under it, each where it is
        // drawn: an importer that does not inherit geometry from the master
        // (Keynote) has nothing to go on otherwise
        out = format!("{}<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Slide Image Placeholder 1\"/><p:cNvSpPr><a:spLocks noGrp=\"1\" noRot=\"1\" noChangeAspect=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"sldImg\"/></p:nvPr></p:nvSpPr><p:spPr>{}</p:spPr></p:sp>", out, PptxWriter::notesImageXfrm());
        out = format!("{}<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Notes Placeholder\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr>{}</p:spPr>", out, PptxWriter::notesBodyXfrm());
        out = format!("{}{}", out, PptxWriter::bodyXml(
            &notes.borrow().text,
            false,
            false,
            &Rc::new(RefCell::new(PptxRunLinks::new()))
        ));
        format!("{}</p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>", out)
    }
    /// Where a notes page (the 6858000 x 9144000 one presentation.xml names)
    /// puts the slide and the notes, as PowerPoint's own notes master does.
    pub fn notesImageXfrm() -> String {
        "<a:xfrm><a:off x=\"685800\" y=\"1143000\"/><a:ext cx=\"5486400\" cy=\"3086100\"/></a:xfrm>".to_string()
    }
    pub fn notesBodyXfrm() -> String {
        "<a:xfrm><a:off x=\"685800\" y=\"4400550\"/><a:ext cx=\"5486400\" cy=\"3600450\"/></a:xfrm>".to_string()
    }
    pub fn groupXfrm() -> String {
        "<p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>".to_string()
    }
    /// A notes slide names the slide it belongs to and the notes master it
    /// follows. The master is not optional: ECMA-376 requires it, and Keynote
    /// refuses a package whose notes slides have none ("the file format is
    /// invalid") where PowerPoint quietly repairs it.
    pub fn notesRelsXml(slide_path: &str, master_path: &str) -> String {
        let target: String = PptxWriter::relTargetFrom("ppt/notesSlides", slide_path);
        let mut out: String = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdSlide\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"{}\"/>", target);
        if !master_path.is_empty() {
            out = format!("{}<Relationship Id=\"rIdMaster\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster\" Target=\"{}\"/>", out, PptxWriter::relTargetFrom("ppt/notesSlides", master_path));
        }
        format!("{}</Relationships>", out)
    }
    /// The notes master, written only when some slide has notes: a ground, the
    /// slide-image and body placeholders a notes slide inherits from, and the
    /// text style of the notes. An empty one is valid by the schema and is not
    /// enough for Keynote, which refuses the whole package over it.
    pub fn notesMasterXml() -> String {
        let mut out: String = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:notesMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:bg><p:bgRef idx=\"1001\"><a:schemeClr val=\"bg1\"/></p:bgRef></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>{}", PptxWriter::groupXfrm());
        out = format!("{}<p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Slide Image Placeholder 1\"/><p:cNvSpPr><a:spLocks noGrp=\"1\" noRot=\"1\" noChangeAspect=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"sldImg\" idx=\"2\"/></p:nvPr></p:nvSpPr><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:noFill/><a:ln w=\"12700\"><a:solidFill><a:prstClr val=\"black\"/></a:solidFill></a:ln></p:spPr></p:sp>", out, PptxWriter::notesImageXfrm());
        out = format!("{}<p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Notes Placeholder 2\"/><p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr><p:nvPr><p:ph type=\"body\" sz=\"quarter\" idx=\"3\"/></p:nvPr></p:nvSpPr><p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr vert=\"horz\" lIns=\"91440\" tIns=\"45720\" rIns=\"91440\" bIns=\"45720\" rtlCol=\"0\"/><a:lstStyle/><a:p><a:pPr lvl=\"0\"/><a:r><a:rPr lang=\"en-US\"/><a:t>Notes</a:t></a:r></a:p></p:txBody></p:sp>", out, PptxWriter::notesBodyXfrm());
        out = format!("{}</p:spTree></p:cSld>", out);
        out = format!("{}<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>", out);
        out = format!("{}<p:notesStyle><a:lvl1pPr marL=\"0\" algn=\"l\" defTabSz=\"914400\" rtl=\"0\" eaLnBrk=\"1\" latinLnBrk=\"0\" hangingPunct=\"1\"><a:defRPr sz=\"1200\" kern=\"1200\"><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill><a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/><a:cs typeface=\"+mn-cs\"/></a:defRPr></a:lvl1pPr></p:notesStyle>", out);
        format!("{}</p:notesMaster>", out)
    }
    /// A master owns its theme, so the notes master gets a copy of the deck's.
    pub fn notesMasterRelsXml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme2.xml\"/></Relationships>".to_string()
    }
    /// --- parts ---------------------------------------------------------------
    /// Which shape each build step is about, resolved BEFORE the shape tree is
    /// written — the tree renumbers as it goes, so the answer has to be a shape
    /// and not a number. A step whose shape is not on the slide resolves to
    /// nothing and is left out of the timing.
    pub fn stepTargets(slide: &Rc<RefCell<PptxSlide>>) -> Vec<Rc<RefCell<PptxShape>>> {
        let mut out: Vec<Rc<RefCell<PptxShape>>> = Vec::new();
        let mut i: i64 = 0;
        while i < (slide.borrow().build.len() as i64) {
            let mut step: Rc<RefCell<PptxBuildStep>> = slide.borrow().build[i as usize].clone();
            let mut hit: Option<Rc<RefCell<PptxShape>>> = PptxWriter::findTarget(&slide.borrow().shapes.clone(), &step);
            if hit.is_none() {
                let mut missing: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
                missing.borrow_mut().shapeId = 0;
                out.push(missing.clone());
            } else {
                out.push(hit.clone().unwrap());
            }
            i += 1;
        }
        out
    }
    pub fn findTarget(shapes: &Vec<Rc<RefCell<PptxShape>>>, step: &Rc<RefCell<PptxBuildStep>>) -> Option<Rc<RefCell<PptxShape>>> {
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut r#match: bool = false;
            if step.borrow().editId > 0 {
                if sh.borrow().editId == step.borrow().editId {
                    r#match = true;
                }
            } else {
                if sh.borrow().shapeId == step.borrow().shapeId {
                    r#match = true;
                }
            }
            if r#match {
                let mut hit: Option<Rc<RefCell<PptxShape>>> = Some(sh.clone());
                return hit;
            }
            let mut deeper: Option<Rc<RefCell<PptxShape>>> = PptxWriter::findTarget(&sh.borrow().children.clone(), step);
            if deeper.is_some() {
                return deeper;
            }
            i += 1;
        }
        let mut __none: Option<Rc<RefCell<PptxShape>>> = None;
        __none
    }
    /// --- how a slide arrives, and what is built on it -------------------------
    pub fn transitionXml(slide: &Rc<RefCell<PptxSlide>>) -> String {
        let mut tr: Rc<RefCell<PptxTransition>> = slide.borrow().transition.clone().clone();
        if !tr.borrow().isSet {
            return String::new();
        }
        if tr.borrow().kind.is_empty() {
            return String::new();
        }
        let mut attrs: String = String::new();
        let ms: i64 = PptxWriter::msOf(tr.borrow().seconds.clone());
        if ms >= 1200 {
            attrs = " spd=\"slow\"".to_string();
        } else {
            if ms <= 500 {
                attrs = " spd=\"fast\"".to_string();
            } else {
                attrs = " spd=\"med\"".to_string();
            }
        }
        if !tr.borrow().onClick {
            attrs = format!("{} advClick=\"0\"", attrs);
        }
        if tr.borrow().afterSeconds > 0.0 {
            attrs = format!("{} advTm=\"{}\"", attrs, PptxWriter::msOf(tr.borrow().afterSeconds));
        }
        let mut eff: String = format!("<p:{}", tr.borrow().kind);
        if !tr.borrow().dir.is_empty() {
            eff = format!("{} dir=\"{}\"", eff, PptxWriter::esc(&tr.borrow().dir));
        }
        eff = format!("{}/>", eff);
        format!("<p:transition{}>{}</p:transition>", attrs, eff)
    }
    pub fn msOf(seconds: f64) -> i64 {
        if seconds <= 0.0 {
            return 0;
        }
        let v: f64 = seconds * 1000.0 + 0.5;
        v.floor() as i64
    }
    /// The timing tree PowerPoint writes for a click-through build: one sequence
    /// on the slide, a click group per step that waits for one, and the step
    /// itself as a visibility change plus the effect that plays over it. It is
    /// verbose because the format is — the shape of it is what a reader expects
    /// to find, so this writes that shape rather than a shorter one that only we
    /// would understand.
    pub fn timingXml(slide: &Rc<RefCell<PptxSlide>>) -> String {
        if slide.borrow().build.is_empty() {
            return String::new();
        }
        let mut out: String = "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst><p:seq concurrent=\"1\" nextAc=\"seek\"><p:cTn id=\"2\" dur=\"indefinite\" nodeType=\"mainSeq\"><p:childTnLst>".to_string();
        let mut id: i64 = 3;
        let mut i: i64 = 0;
        while i < (slide.borrow().build.len() as i64) {
            let mut step: Rc<RefCell<PptxBuildStep>> = slide.borrow().build[i as usize].clone();
            if step.borrow().shapeId <= 0 {
                i += 1;
                continue;
            }
            let mut delay: String = "indefinite";
            let mut node_type: String = "clickEffect";
            if step.borrow().start == "with" {
                delay = "0";
                node_type = "withEffect";
            }
            if step.borrow().start == "after" {
                delay = "0";
                node_type = "afterEffect";
            }
            let ms: String = PptxWriter::msOf(step.borrow().seconds.clone()).to_string();
            // The preset numbers and subtypes are the ones PowerPoint writes for
            // the same effect. Keynote reads the preset, not the behaviours: an
            // effect whose preset and behaviours disagree, or a filter that is
            // not in the list, and it refuses the whole file.
            let kind: String = PptxWriter::buildKind(&step.borrow().kind.clone());
            let mut preset: String = "1";
            let mut subtype: String = "0".to_string();
            if kind == "fade" {
                preset = "10";
            }
            if kind == "fly" {
                preset = "2";
                subtype = PptxWriter::flySubtype(&step.borrow().dir);
            }
            // click group
            out = format!("{}<p:par><p:cTn id=\"{}\" fill=\"hold\"><p:stCondLst><p:cond delay=\"{}\"/></p:stCondLst><p:childTnLst>", out, id, delay);
            id += 1;
            out = format!("{}<p:par><p:cTn id=\"{}\" fill=\"hold\"><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst><p:childTnLst>", out, id);
            id += 1;
            out = format!("{}<p:par><p:cTn id=\"{}\" presetID=\"{}\" presetClass=\"entr\" presetSubtype=\"{}\" fill=\"hold\" grpId=\"0\" nodeType=\"{}\"><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst><p:childTnLst>", out, id, preset, subtype, node_type);
            id += 1;
            let tgt: String = PptxWriter::targetXml(&step);
            out = format!("{}<p:set><p:cBhvr><p:cTn id=\"{}\" dur=\"1\" fill=\"hold\"><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst></p:cTn>{}<p:attrNameLst><p:attrName>style.visibility</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val=\"visible\"/></p:to></p:set>", out, id, tgt);
            id += 1;
            if kind == "fade" {
                out = format!("{}<p:animEffect transition=\"in\" filter=\"fade\"><p:cBhvr><p:cTn id=\"{}\" dur=\"{}\"/>{}</p:cBhvr></p:animEffect>", out, id, ms, tgt);
                id += 1;
            }
            if kind == "fly" {
                let mut from_x: String = "#ppt_x";
                let mut from_y: String = "#ppt_y";
                if subtype == "8" {
                    from_x = "0-#ppt_w/2";
                }
                if subtype == "2" {
                    from_x = "1+#ppt_w/2";
                }
                if subtype == "1" {
                    from_y = "0-#ppt_h/2";
                }
                if subtype == "4" {
                    from_y = "1+#ppt_h/2";
                }
                out = format!("{}{}", out, PptxWriter::flyAnim(id, &ms, &tgt, "ppt_x", from_x, "#ppt_x"));
                id += 1;
                out = format!("{}{}", out, PptxWriter::flyAnim(id, &ms, &tgt, "ppt_y", from_y, "#ppt_y"));
                id += 1;
            }
            out = format!("{}</p:childTnLst></p:cTn></p:par>", out);
            out = format!("{}</p:childTnLst></p:cTn></p:par>", out);
            out = format!("{}</p:childTnLst></p:cTn></p:par>", out);
            i += 1;
        }
        out = format!("{}</p:childTnLst></p:cTn><p:prevCondLst><p:cond evt=\"onPrev\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst><p:nextCondLst><p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq>", out);
        out = format!("{}</p:childTnLst></p:cTn></p:par></p:tnLst>", out);
        // …and the build list: which text is built paragraph by paragraph. A
        // paragraph build without it is a timing tree that points at a build
        // nobody declared, and Keynote will not import it.
        let mut bld: String = String::new();
        let mut seen: Vec<i64> = Vec::new();
        let mut j: i64 = 0;
        while j < (slide.borrow().build.len() as i64) {
            let mut st: Rc<RefCell<PptxBuildStep>> = slide.borrow().build[j as usize].clone();
            if st.borrow().shapeId > 0 && st.borrow().paragraph >= 0 {
                let mut known: bool = false;
                let mut q: i64 = 0;
                while q < (seen.len() as i64) {
                    if seen[q as usize] == st.borrow().shapeId {
                        known = true;
                    }
                    q += 1;
                }
                if false == known {
                    seen.push(st.borrow().shapeId);
                    bld = format!("{}<p:bldP spid=\"{}\" grpId=\"0\" build=\"p\"/>", bld, st.borrow().shapeId);
                }
            }
            j += 1;
        }
        if !bld.is_empty() {
            out = format!("{}<p:bldLst>{}</p:bldLst>", out, bld);
        }
        format!("{}</p:timing>", out)
    }
    /// The effects the writer knows by name; anything else enters as a fade.
    pub fn buildKind(kind: &str) -> String {
        if kind == "appear" || (kind == "fly" || kind == "fade") {
            return kind.to_string();
        }
        "fade".to_string()
    }
    /// PowerPoint's subtype for Fly In: the side it comes from.
    pub fn flySubtype(dir: &str) -> String {
        if dir == "l" {
            return "8".to_string();
        }
        if dir == "r" {
            return "2".to_string();
        }
        if dir == "u" {
            return "1".to_string();
        }
        if dir == "t" {
            return "1".to_string();
        }
        "4".to_string()
    }
    pub fn flyAnim(id: i64, ms: &str, tgt: &str, attr: &str, from: &str, to: &str) -> String {
        format!("<p:anim calcmode=\"lin\" valueType=\"num\"><p:cBhvr additive=\"base\"><p:cTn id=\"{}\" dur=\"{}\" fill=\"hold\"/>{}<p:attrNameLst><p:attrName>{}</p:attrName></p:attrNameLst></p:cBhvr><p:tavLst><p:tav tm=\"0\"><p:val><p:strVal val=\"{}\"/></p:val></p:tav><p:tav tm=\"100000\"><p:val><p:strVal val=\"{}\"/></p:val></p:tav></p:tavLst></p:anim>", id, ms, tgt, attr, from, to)
    }
    pub fn targetXml(step: &Rc<RefCell<PptxBuildStep>>) -> String {
        let mut inner: String = String::new();
        if step.borrow().paragraph >= 0 {
            inner = format!("<p:txEl><p:pRg st=\"{}\" end=\"{}\"/></p:txEl>", step.borrow().paragraph, step.borrow().paragraph);
        }
        return format!("<p:tgtEl><p:spTgt spid=\"{}\">{}</p:spTgt></p:tgtEl>", step.borrow().shapeId, inner);
    }
    /// --- SmartArt laid out here ----------------------------------------------
    ///
    /// A diagram the engine laid out (smartart/SaEngine), its shapes untouched
    /// since, saved as SmartArt: a `graphicFrame` naming its data, layout,
    /// quick style and colour parts, and a drawing part with the shapes as
    /// drawn here, which the data part names through `dsp:dataModelExt`.
    /// PowerPoint lays it out again from the first four and it stays a diagram
    /// there; other readers draw the fifth. Empty — and the diagram is written
    /// as the group it is — when there is nothing to write it from.
    pub fn diagramRelKinds() -> Vec<String> {
        let base: String = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagram";
        vec![format!("{}Data", base), format!("{}Layout", base), format!("{}QuickStyle", base), format!("{}Colors", base), "http://schemas.microsoft.com/office/2007/relationships/diagramDrawing".to_string()].clone()
    }
    /// "ppt/diagrams/data3.xml" → "rIdDgm-data3"
    pub fn diagramRelId(path: &str) -> String {
        let at: i64 = PptxText::last_index_of(&path, "/");
        let mut base: String = path[(at + 1) as usize..(path.as_bytes().len() as i64) as usize].to_string();
        let dot: i64 = PptxText::last_index_of(&base, ".");
        if dot > 0 {
            base = base[(0) as usize..(dot) as usize].to_string();
        }
        format!("rIdDgm-{}", base)
    }
    /// The drawing part: the group's shapes in the frame's own space, each a
    /// `dsp:sp` with its geometry, fill, line and text as drawn here.
    pub fn diagramDrawingXml(g: &Rc<RefCell<PptxShape>>, nb: &Rc<RefCell<SaNumbering>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n".to_string();
        out = format!("{}<dsp:drawing xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" xmlns:dsp=\"http://schemas.microsoft.com/office/drawing/2008/diagram\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">", out);
        out = format!("{}<dsp:spTree><dsp:nvGrpSpPr><dsp:cNvPr id=\"0\" name=\"\"/><dsp:cNvGrpSpPr/></dsp:nvGrpSpPr><dsp:grpSpPr/>", out);
        // a shape's model id: the (numbered) data point it shows, so a
        // reader can tell which point its text is; a GUID for a shape that
        // shows none, or one whose point another shape already named
        let mut used: Vec<String> = Vec::new();
        let mut i: i64 = 0;
        while i < (g.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<PptxShape>> = g.borrow().children[i as usize].clone();
            let mut mid: String = SaPackage::shapeModelId(i);
            if !c.borrow().diagramParaPts.is_empty() {
                let pt: String = nb.borrow().mapped(&c.borrow().diagramParaPts[0].clone());
                if SaEdit::indexIn(&used, &pt) < 0 {
                    mid = pt.clone();
                    used.push(pt.clone());
                }
            }
            out = format!("{}{}", out, PptxWriter::dspSpXml(&c, &mid));
            i += 1;
        }
        format!("{}</dsp:spTree></dsp:drawing>", out)
    }
    pub fn dspSpXml(sh: &Rc<RefCell<PptxShape>>, model_id: &str) -> String {
        let mut out: String = format!("<dsp:sp modelId=\"{}\"><dsp:nvSpPr><dsp:cNvPr id=\"0\" name=\"\"/><dsp:cNvSpPr/></dsp:nvSpPr><dsp:spPr>", model_id);
        out = format!("{}{}{}", out, PptxWriter::xfrmXml(
            sh,
            "a:xfrm",
            false
        ), PptxWriter::geomXml(sh));
        if sh.borrow().noFill {
            out = format!("{}<a:noFill/>", out);
        } else {
            out = format!("{}{}", out, PptxWriter::fillXml(sh));
        }
        if sh.borrow().noLine || !sh.borrow().line.borrow().isSet {
            out = format!("{}<a:ln><a:noFill/></a:ln>", out);
        } else {
            let mut w: f64 = sh.borrow().lineWidthPt;
            if w <= 0.0 {
                w = 1.0;
            }
            let mut ends: String = String::new();
            if !sh.borrow().tailEnd.is_empty() {
                ends = format!("<a:tailEnd type=\"{}\"/>", sh.borrow().tailEnd);
            }
            out = format!("{}<a:ln w=\"{}\"><a:solidFill>{}</a:solidFill>{}</a:ln>", out, PptxWriter::emu(w), PptxWriter::colorXml(&sh.borrow().line), ends);
        }
        out = format!("{}</dsp:spPr>", out);
        if sh.borrow().hasText {
            let mut _tmp_5030 = sh.borrow().text.clone();
            let mut body: String = PptxWriter::bodyXml(
                &_tmp_5030,
                false,
                false,
                &Rc::new(RefCell::new(PptxRunLinks::new()))
            );
            body = format!("<dsp:txBody>{}</dsp:txBody>", body[(("<p:txBody>".to_string().as_bytes().len() as i64)) as usize..((body.as_bytes().len() as i64) - ("</p:txBody>".to_string().as_bytes().len() as i64)) as usize].to_string());
            // shrink on overflow: the size was fitted with the fonts here, and a
            // reader drawing with a wider one shrinks it rather than breaking
            // words (PowerPoint lays the diagram out again anyway)
            let bp: i64 = PptxText::index_of(&body, "/><a:lstStyle/>");
            if bp >= 0 {
                body = format!("{}><a:normAutofit/></a:bodyPr><a:lstStyle/>{}", body[(0) as usize..(bp) as usize].to_string(), body[(bp + ("/><a:lstStyle/>".to_string().as_bytes().len() as i64)) as usize..(body.as_bytes().len() as i64) as usize].to_string());
            }
            out = format!("{}{}", out, body);
        }
        format!("{}</dsp:sp>", out)
    }
    pub fn numStr(v: f64) -> String {
        let whole: i64 = v.floor() as i64;
        let frac: f64 = v - (whole as f64);
        if (frac.abs()) < 0.000001 {
            return whole.to_string();
        }
        v.to_string()
    }
    pub fn seriesXml(chart: &Rc<RefCell<PptxChart>>, series: &Rc<RefCell<PptxChartSeries>>, idx: i64) -> String {
        let mut out: String = format!("<c:ser><c:idx val=\"{}\"/><c:order val=\"{}\"/>", idx, idx);
        out = format!("{}<c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>{}</c:v></c:pt></c:strCache></c:strRef></c:tx>", out, PptxWriter::esc(&series.borrow().name));
        let nc: i64 = chart.borrow().categories.len() as i64;
        out = format!("{}<c:cat><c:strRef><c:f>Sheet1!$A$2:$A${}</c:f><c:strCache><c:ptCount val=\"{}\"/>", out, nc + 1, nc);
        let mut i: i64 = 0;
        while i < nc {
            out = format!("{}<c:pt idx=\"{}\"><c:v>{}</c:v></c:pt>", out, i, PptxWriter::esc(&chart.borrow().categories[i as usize].clone()));
            i += 1;
        }
        out = format!("{}</c:strCache></c:strRef></c:cat>", out);
        let nv: i64 = series.borrow().values.len() as i64;
        out = format!("{}<c:val><c:numRef><c:f>Sheet1!$B$2:$B${}</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val=\"{}\"/>", out, nv + 1, nv);
        let mut j: i64 = 0;
        while j < nv {
            out = format!("{}<c:pt idx=\"{}\"><c:v>{}</c:v></c:pt>", out, j, PptxWriter::numStr(series.borrow().values[j as usize]));
            j += 1;
        }
        format!("{}</c:numCache></c:numRef></c:val></c:ser>", out)
    }
    pub fn chartXml(chart: &Rc<RefCell<PptxChart>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><c:chart>".to_string();
        if !chart.borrow().title.is_empty() {
            out = format!("{}<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>{}</a:t></a:r></a:p></c:rich></c:tx><c:overlay val=\"0\"/></c:title><c:autoTitleDeleted val=\"0\"/>", out, PptxWriter::esc(&chart.borrow().title));
        } else {
            out = format!("{}<c:autoTitleDeleted val=\"1\"/>", out);
        }
        out = format!("{}<c:plotArea><c:layout/>", out);
        let mut body: String = String::new();
        let mut si: i64 = 0;
        while si < (chart.borrow().series.len() as i64) {
            let mut ser: Rc<RefCell<PptxChartSeries>> = chart.borrow().series[si as usize].clone();
            body = format!("{}{}", body, PptxWriter::seriesXml(chart, &ser, si));
            si += 1;
        }
        let axes: String = "<c:axId val=\"111111111\"/><c:axId val=\"222222222\"/>";
        if chart.borrow().chartType == "pie" {
            out = format!("{}<c:pieChart><c:varyColors val=\"1\"/>{}</c:pieChart>", out, body);
        } else {
            if chart.borrow().chartType == "line" {
                out = format!("{}<c:lineChart><c:grouping val=\"standard\"/><c:varyColors val=\"0\"/>{}<c:marker val=\"1\"/>{}</c:lineChart>", out, body, axes);
            } else {
                out = format!("{}<c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:varyColors val=\"0\"/>{}<c:gapWidth val=\"150\"/>{}</c:barChart>", out, body, axes);
            }
        }
        if chart.borrow().chartType != "pie" {
            out = format!("{}<c:catAx><c:axId val=\"111111111\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"b\"/><c:crossAx val=\"222222222\"/></c:catAx>", out);
            out = format!("{}<c:valAx><c:axId val=\"222222222\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling><c:delete val=\"0\"/><c:axPos val=\"l\"/><c:crossAx val=\"111111111\"/></c:valAx>", out);
        }
        out = format!("{}</c:plotArea><c:plotVisOnly val=\"1\"/><c:dispBlanksAs val=\"gap\"/></c:chart>", out);
        format!("{}</c:chartSpace>", out)
    }
    pub fn rootRelsXml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/></Relationships>".to_string()
    }
    /// A blank master and a blank layout: the model handed to the writer is the
    /// resolved one, so the chrome is already on the slides.
    pub fn masterXml() -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:sldMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>".to_string();
        out = format!("{}<p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>", out);
        out = format!("{}<p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst>", out);
        format!("{}</p:sldMaster>", out)
    }
    pub fn masterRelsXml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/></Relationships>".to_string()
    }
    pub fn layoutXml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" type=\"blank\" preserve=\"1\"><p:cSld name=\"Blank\"><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>".to_string()
    }
    pub fn layoutRelsXml() -> String {
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/></Relationships>".to_string()
    }
    /// The deck's own theme, so the colours a `schemeClr` still names resolve
    /// the way they did when it was read.
    pub fn themeXml(theme: &Rc<RefCell<PptxTheme>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"Ranger\"><a:themeElements><a:clrScheme name=\"Ranger\">".to_string();
        out = format!("{}<a:dk1><a:srgbClr val=\"{}\"/></a:dk1><a:lt1><a:srgbClr val=\"{}\"/></a:lt1>", out, theme.borrow().colorOf("dk1"), theme.borrow().colorOf("lt1"));
        out = format!("{}<a:dk2><a:srgbClr val=\"{}\"/></a:dk2><a:lt2><a:srgbClr val=\"{}\"/></a:lt2>", out, theme.borrow().colorOf("dk2"), theme.borrow().colorOf("lt2"));
        let mut i: i64 = 1;
        while i <= 6 {
            let nm: String = format!("accent{}", i);
            out = format!("{}<a:{}><a:srgbClr val=\"{}\"/></a:{}>", out, nm, theme.borrow().colorOf(&nm), nm);
            i += 1;
        }
        out = format!("{}<a:hlink><a:srgbClr val=\"{}\"/></a:hlink><a:folHlink><a:srgbClr val=\"{}\"/></a:folHlink></a:clrScheme>", out, theme.borrow().colorOf("hlink"), theme.borrow().colorOf("folHlink"));
        out = format!("{}<a:fontScheme name=\"Ranger\"><a:majorFont><a:latin typeface=\"{}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"{}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:minorFont></a:fontScheme>", out, PptxWriter::esc(&theme.borrow().majorFont), PptxWriter::esc(&theme.borrow().minorFont));
        let solid: String = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
        let line: String = "<a:ln w=\"9525\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/></a:ln>";
        out = format!("{}<a:fmtScheme name=\"Ranger\"><a:fillStyleLst>{}{}{}</a:fillStyleLst>", out, solid, solid, solid);
        out = format!("{}<a:lnStyleLst>{}{}{}</a:lnStyleLst>", out, line, line, line);
        out = format!("{}<a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst>", out);
        out = format!("{}<a:bgFillStyleLst>{}{}{}</a:bgFillStyleLst></a:fmtScheme>", out, solid, solid, solid);
        format!("{}</a:themeElements><a:objectDefaults/><a:extraClrSchemeLst/></a:theme>", out)
    }
    pub fn isSlidePart(part: &str) -> bool {
        if PptxText::index_of(&part, "ppt/slides/slide") != 0 {
            return false;
        }
        PptxText::index_of(&part, ".xml") > 0
    }
    pub fn isSlideRelsPart(part: &str) -> bool {
        PptxText::index_of(&part, "ppt/slides/_rels/slide") == 0
    }
    pub fn slideOfRels(part: &str) -> String {
        let mut base: String = part[(("ppt/slides/_rels/".to_string().as_bytes().len() as i64)) as usize..(part.as_bytes().len() as i64) as usize].to_string();
        let cut: i64 = PptxText::index_of(&base, ".rels");
        if cut > 0 {
            base = base[(0) as usize..(cut) as usize].to_string();
        }
        format!("ppt/slides/{}", base)
    }
    pub fn firstLayout(parts: &[String]) -> String {
        let mut i: i64 = 0;
        while i < (parts.len() as i64) {
            let p: String = parts[i as usize].clone();
            if PptxText::index_of(&p, "ppt/slideLayouts/slideLayout") == 0 {
                if PptxText::index_of(&p, ".rels") < 0 {
                    return PptxWriter::relTargetFrom("ppt/slides", &p);
                }
            }
            i += 1;
        }
        "../slideLayouts/slideLayout1.xml".to_string()
    }
    /// The slide list, and nothing else: a presentation part carries the slide
    /// size, the master list, the default text style and whatever else the
    /// authoring tool put there, and none of that is ours to regenerate.
    pub fn presentationWithSlides(xml: &str, slide_paths: &[String]) -> String {
        let mut list: String = "<p:sldIdLst>".to_string();
        let mut i: i64 = 0;
        while i < (slide_paths.len() as i64) {
            list = format!("{}<p:sldId id=\"{}\" r:id=\"rIdSld{}\"/>", list, 256 + i, i + 1);
            i += 1;
        }
        list = format!("{}</p:sldIdLst>", list);
        PptxWriter::spliceSlideList(xml, &list)
    }
    pub fn spliceSlideList(xml: &str, list: &str) -> String {
        let start: i64 = PptxText::index_of(&xml, "<p:sldIdLst");
        if start < 0 {
            // No list at all: put one after the master list, which every
            // presentation has.
            let anchor: i64 = PptxText::index_of(&xml, "</p:sldMasterIdLst>");
            if anchor < 0 {
                return xml.to_string();
            }
            let cut: i64 = anchor + ("</p:sldMasterIdLst>".to_string().as_bytes().len() as i64);
            return format!("{}{}{}", xml[(0) as usize..(cut) as usize].to_string(), list, xml[(cut) as usize..(xml.as_bytes().len() as i64) as usize].to_string());
        }
        let close_tag: i64 = PptxText::index_of_from(&xml, "</p:sldIdLst>", start);
        if close_tag >= 0 {
            let after: i64 = close_tag + ("</p:sldIdLst>".to_string().as_bytes().len() as i64);
            return format!("{}{}{}", xml[(0) as usize..(start) as usize].to_string(), list, xml[(after) as usize..(xml.as_bytes().len() as i64) as usize].to_string());
        }
        let self_end: i64 = PptxText::index_of_from(&xml, "/>", start);
        if self_end < 0 {
            return xml.to_string();
        }
        format!("{}{}{}", xml[(0) as usize..(start) as usize].to_string(), list, xml[(self_end + 2) as usize..(xml.as_bytes().len() as i64) as usize].to_string())
    }
    pub fn slideIdsFor(pres: &Rc<RefCell<PptxPresentation>>, count: i64) -> Vec<i64> {
        let mut out: Vec<i64> = Vec::new();
        let mut next: i64 = 256;
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            let mut s0: Rc<RefCell<PptxSlide>> = pres.borrow().slides[i as usize].clone();
            let sid: i64 = s0.borrow().slideId;
            if sid >= next {
                next = sid + 1;
            }
            i += 1;
        }
        let mut k: i64 = 0;
        while k < count {
            let mut id: i64 = 0;
            if k < (pres.borrow().slides.len() as i64) {
                let mut slide: Rc<RefCell<PptxSlide>> = pres.borrow().slides[k as usize].clone();
                id = slide.borrow().slideId;
            }
            if id == 0 {
                id = next;
                next += 1;
            }
            out.push(id);
            k += 1;
        }
        out
    }
    /// PowerPoint wants a GUID here. Sections made in this editor have no id of
    /// their own, so one is derived from their place in the deck: unique inside
    /// the file, which is all the id is used for.
    pub fn sectionId(sec: &Rc<RefCell<PptxSection>>, index: i64) -> String {
        if !sec.borrow().id.is_empty() {
            return sec.borrow().id.clone();
        }
        let mut n: String = (index + 1).to_string();
        while (n.as_bytes().len() as i64) < 12 {
            n = format!("0{}", n);
        }
        format!("{}{}{}", "{00000000-0000-0000-0000-".to_string(), n, "}".to_string())
    }
    pub fn sectionLstXml(pres: &Rc<RefCell<PptxPresentation>>, valid: &[i64]) -> String {
        if pres.borrow().sections.is_empty() {
            return String::new();
        }
        let mut out: String = "<p:extLst><p:ext uri=\"{521415D9-36F7-43E2-AB2F-B90AF26B5E84}\"><p14:sectionLst xmlns:p14=\"http://schemas.microsoft.com/office/powerpoint/2010/main\">".to_string();
        let mut i: i64 = 0;
        while i < (pres.borrow().sections.len() as i64) {
            let mut sec: Rc<RefCell<PptxSection>> = pres.borrow().sections[i as usize].clone();
            out = format!("{}<p14:section name=\"{}\" id=\"{}\"><p14:sldIdLst>", out, PptxWriter::esc(&sec.borrow().name), PptxWriter::sectionId(&sec, i));
            let mut k: i64 = 0;
            while k < (sec.borrow().slideIds.len() as i64) {
                let id: i64 = sec.borrow().slideIds[k as usize];
                let mut live: bool = false;
                let mut v: i64 = 0;
                while v < (valid.len() as i64) {
                    if valid[v as usize] == id {
                        live = true;
                    }
                    v += 1;
                }
                if live {
                    out = format!("{}<p14:sldId id=\"{}\"/>", out, id);
                }
                k += 1;
            }
            out = format!("{}</p14:sldIdLst></p14:section>", out);
            i += 1;
        }
        format!("{}</p14:sectionLst></p:ext></p:extLst>", out)
    }
    /// Splice a freshly built section list into a presentation part, replacing
    /// the one that is there. A deck with no sections loses the list it had.
    pub fn presentationWithSections(xml: &str, sections: &str) -> String {
        let start: i64 = PptxText::index_of(&xml, "<p14:sectionLst");
        if start < 0 {
            if sections.is_empty() {
                return xml.to_string();
            }
            let close: i64 = PptxText::index_of(&xml, "</p:presentation>");
            if close < 0 {
                return xml.to_string();
            }
            let existing: i64 = PptxText::index_of(&xml, "<p:extLst>");
            if existing >= 0 {
                let cut: i64 = existing + ("<p:extLst>".to_string().as_bytes().len() as i64);
                let inner: String = sections[(("<p:extLst>".to_string().as_bytes().len() as i64)) as usize..((sections.as_bytes().len() as i64) - ("</p:extLst>".to_string().as_bytes().len() as i64)) as usize].to_string();
                return format!("{}{}{}", xml[(0) as usize..(cut) as usize].to_string(), inner, xml[(cut) as usize..(xml.as_bytes().len() as i64) as usize].to_string());
            }
            return format!("{}{}{}", xml[(0) as usize..(close) as usize].to_string(), sections, xml[(close) as usize..(xml.as_bytes().len() as i64) as usize].to_string());
        }
        // Replace the whole <p:ext> that holds the list, so an emptied deck
        // does not keep a stray wrapper.
        let mut ext_start: i64 = PptxWriter::lastIndexBefore(xml, "<p:ext ", start);
        if ext_start < 0 {
            ext_start = start;
        }
        let ext_end: i64 = PptxText::index_of_from(&xml, "</p:ext>", start);
        if ext_end < 0 {
            return xml.to_string();
        }
        let after: i64 = ext_end + ("</p:ext>".to_string().as_bytes().len() as i64);
        let mut repl: String = String::new();
        if !sections.is_empty() {
            repl = sections[(("<p:extLst>".to_string().as_bytes().len() as i64)) as usize..((sections.as_bytes().len() as i64) - ("</p:extLst>".to_string().as_bytes().len() as i64)) as usize].to_string();
        }
        let head: String = xml[(0) as usize..(ext_start) as usize].to_string();
        let tail: String = xml[(after) as usize..(xml.as_bytes().len() as i64) as usize].to_string();
        if repl.is_empty() {
            // An extLst with nothing left in it is not valid; drop it too.
            let empty_at: i64 = PptxText::index_of(&format!("{}{}", head, tail), "<p:extLst></p:extLst>");
            let joined: String = format!("{}{}", head, tail);
            if empty_at >= 0 {
                return format!("{}{}", joined[(0) as usize..(empty_at) as usize].to_string(), joined[(empty_at + ("<p:extLst></p:extLst>".to_string().as_bytes().len() as i64)) as usize..(joined.as_bytes().len() as i64) as usize].to_string());
            }
            return joined;
        }
        format!("{}{}{}", head, repl, tail)
    }
    pub fn lastIndexBefore(xml: &str, needle: &str, limit: i64) -> i64 {
        let mut found: i64 = 0 - 1;
        let mut at: i64 = PptxText::index_of(&xml, needle);
        while at >= 0 {
            if at >= limit {
                return found;
            }
            found = at;
            at = PptxText::index_of_from(&xml, needle, at + 1);
        }
        found
    }
    /// The slide list written from the deck's own slide ids, plus the section
    /// list that names them.
    pub fn presentationWithDeck(xml: &str, pres: &Rc<RefCell<PptxPresentation>>, slide_paths: &[String]) -> String {
        let mut ids: Vec<i64> = PptxWriter::slideIdsFor(pres, slide_paths.len() as i64);
        let mut list: String = "<p:sldIdLst>".to_string();
        let mut i: i64 = 0;
        while i < (slide_paths.len() as i64) {
            list = format!("{}<p:sldId id=\"{}\" r:id=\"rIdSld{}\"/>", list, ids[i as usize], i + 1);
            i += 1;
        }
        list = format!("{}</p:sldIdLst>", list);
        let out: String = PptxWriter::spliceSlideList(xml, &list);
        PptxWriter::presentationWithSections(&out, &PptxWriter::sectionLstXml(pres, &ids))
    }
    fn presentationRelsWith(pkg: &Rc<RefCell<OpcPackage>>, xml: &str, slide_paths: &[String]) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">".to_string();
        let mut olds: Vec<Rc<RefCell<OpcRelationship>>> = OpcPackage::parseRelationships(xml);
        let mut i: i64 = 0;
        while i < (olds.len() as i64) {
            let mut rel: Rc<RefCell<OpcRelationship>> = olds[i as usize].clone();
            if PptxText::index_of(&rel.borrow().r#type, "/slide") < 0 {
                out = format!("{}<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>", out, rel.borrow().id, rel.borrow().r#type, rel.borrow().target);
            } else {
                // slideMaster and slideLayout also contain "/slide"; only the
                // plain slide relationships are replaced.
                if PptxText::index_of(&rel.borrow().r#type, "/slideMaster") >= 0 {
                    out = format!("{}<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>", out, rel.borrow().id, rel.borrow().r#type, rel.borrow().target);
                } else {
                    if PptxText::index_of(&rel.borrow().r#type, "/slideLayout") >= 0 {
                        out = format!("{}<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>", out, rel.borrow().id, rel.borrow().r#type, rel.borrow().target);
                    }
                }
            }
            i += 1;
        }
        let mut j: i64 = 0;
        while j < (slide_paths.len() as i64) {
            let mut target: String = slide_paths[j as usize].clone();
            if PptxText::index_of(&target, "ppt/") == 0 {
                target = target[(4) as usize..(target.as_bytes().len() as i64) as usize].to_string();
            }
            out = format!("{}<Relationship Id=\"rIdSld{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"{}\"/>", out, j + 1, target);
            j += 1;
        }
        format!("{}</Relationships>", out)
    }
    /// The original content types, minus the parts that left, plus the ones the
    /// edit brought in.
    pub fn contentTypesWith(xml: &str, written: &HashMap<String,bool>, new_parts: &[String]) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">".to_string();
        let mut seen_ext: HashMap<String,bool> = HashMap::new();
        let mut pos: i64 = 0;
        let __len: i64 = xml.as_bytes().len() as i64;
        while pos < __len {
            let idx: i64 = PptxText::index_of_from(&xml, "<Default", pos);
            if idx < 0 {
                pos = __len;
            } else {
                let end: i64 = PptxText::index_of_from(&xml, ">", idx);
                if end < 0 {
                    pos = __len;
                } else {
                    let tag: String = xml[(idx) as usize..(end + 1) as usize].to_string();
                    let ext: String = OpcPackage::attrValue(&tag, "Extension");
                    let ct: String = OpcPackage::attrValue(&tag, "ContentType");
                    if !ext.is_empty() {
                        out = format!("{}<Default Extension=\"{}\" ContentType=\"{}\"/>", out, ext, ct);
                        seen_ext.insert(ext.clone(), true);
                    }
                    pos = end + 1;
                }
            }
        }
        // A picture the edit added may be of a kind the deck had none of.
        let mut needed: Vec<String> = vec!["png".to_string(), "jpeg".to_string(), "gif".to_string(), "svg".to_string()];
        let mut types: Vec<String> = vec!["image/png".to_string(), "image/jpeg".to_string(), "image/gif".to_string(), "image/svg+xml".to_string()];
        let mut ni: i64 = 0;
        while ni < (needed.len() as i64) {
            let ext2: String = needed[ni as usize].clone();
            let have: Option<bool> = seen_ext.get(&ext2).cloned();
            if have.is_none() {
                out = format!("{}<Default Extension=\"{}\" ContentType=\"{}\"/>", out, ext2, types[ni as usize].clone());
                seen_ext.insert(ext2.clone(), true);
            }
            ni += 1;
        }
        let mut kept: HashMap<String,bool> = HashMap::new();
        let mut pos2: i64 = 0;
        while pos2 < __len {
            let idx2: i64 = PptxText::index_of_from(&xml, "<Override", pos2);
            if idx2 < 0 {
                pos2 = __len;
            } else {
                let end2: i64 = PptxText::index_of_from(&xml, ">", idx2);
                if end2 < 0 {
                    pos2 = __len;
                } else {
                    let tag2: String = xml[(idx2) as usize..(end2 + 1) as usize].to_string();
                    let name: String = OpcPackage::attrValue(&tag2, "PartName");
                    let ct2: String = OpcPackage::attrValue(&tag2, "ContentType");
                    let mut part: String = name.clone();
                    if PptxText::index_of(&part, "/") == 0 {
                        part = part[(1) as usize..(part.as_bytes().len() as i64) as usize].to_string();
                    }
                    let exists: Option<bool> = written.get(&part).cloned();
                    if exists.is_some() {
                        out = format!("{}<Override PartName=\"{}\" ContentType=\"{}\"/>", out, name, ct2);
                        kept.insert(part.clone(), true);
                    }
                    pos2 = end2 + 1;
                }
            }
        }
        let mut pi: i64 = 0;
        while pi < (new_parts.len() as i64) {
            let np: String = new_parts[pi as usize].clone();
            let already: Option<bool> = kept.get(&np).cloned();
            if already.is_none() {
                let ct3: String = PptxWriter::contentTypeOf(&np);
                if !ct3.is_empty() {
                    out = format!("{}<Override PartName=\"/{}\" ContentType=\"{}\"/>", out, np, ct3);
                    kept.insert(np.clone(), true);
                }
            }
            pi += 1;
        }
        format!("{}</Types>", out)
    }
    /// The notes master an opened package already has, or nothing.
    fn notesMasterOf(pkg: &Rc<RefCell<OpcPackage>>) -> String {
        let mut found: Vec<String> = OpcPackage::partsOfContentType(&pkg, "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml");
        if !found.is_empty() {
            return found[0].clone();
        }
        String::new()
    }
    pub fn contentTypeOf(part: &str) -> String {
        if PptxWriter::isSlidePart(part) {
            return "application/vnd.openxmlformats-officedocument.presentationml.slide+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/charts/chart") == 0 {
            return "application/vnd.openxmlformats-officedocument.drawingml.chart+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/notesSlides/notesSlide") == 0 {
            return "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/diagrams/data") == 0 {
            return "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/diagrams/layout") == 0 {
            return "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/diagrams/quickStyle") == 0 {
            return "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/diagrams/colors") == 0 {
            return "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml".to_string();
        }
        if PptxText::index_of(&part, "ppt/diagrams/drawing") == 0 {
            return "application/vnd.ms-office.drawingml.diagramDrawing+xml".to_string();
        }
        String::new()
    }
    /// A save is what makes the slides clean again.
    pub fn markSaved(pres: &Rc<RefCell<PptxPresentation>>) {
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            let mut sl: Rc<RefCell<PptxSlide>> = pres.borrow().slides[i as usize].clone();
            sl.borrow_mut().dirty = false;
            i += 1;
        }
    }
    pub fn addXml(&self, name: &str, body: &str) {
        let arg_0 = PptxWriter::utf8(body).clone();
        ZipWriter::addFile(&self.zip.clone(), name, &arg_0);
    }
    /// --- media ---------------------------------------------------------------
    pub fn resetMedia(&mut self) {
        let mut m: Vec<Rc<RefCell<PptxWriteMedia>>> = Vec::new();
        self.media = m.clone();
        self.assets = Rc::new(RefCell::new(OfficeAssetStore::new()));
        let mut c: Vec<String> = Vec::new();
        self.charts = c.clone();
        let mut cp: Vec<String> = Vec::new();
        self.chartPaths = cp.clone();
        let mut dp: Vec<String> = Vec::new();
        self.diagramPaths = dp.clone();
        let mut db: Vec<String> = Vec::new();
        self.diagramBodies = db.clone();
        let mut dri: Vec<String> = Vec::new();
        self.diagramRelIds = dri.clone();
        let mut np: Vec<String> = Vec::new();
        self.notesPaths = np.clone();
        let mut nb: Vec<String> = Vec::new();
        self.notesBodies = nb.clone();
        let mut sp: HashMap<String,bool> = HashMap::new();
        self.sourceParts = sp.clone();
        self.overMode = false;
        self.resetSlideUse();
    }
    pub fn resetSlideUse(&mut self) {
        let mut mu: Vec<i64> = Vec::new();
        self.slideMediaUse = mu.clone();
        let mut cu: Vec<i64> = Vec::new();
        self.slideChartUse = cu.clone();
        let mut du: Vec<i64> = Vec::new();
        self.slideDiagramUse = du.clone();
        self.slideNotesPath = String::new();
        let mut lk: Rc<RefCell<PptxRunLinks>> = Rc::new(RefCell::new(PptxRunLinks::new()));
        lk.borrow_mut().on = true;
        lk.borrow_mut().slideCount = self.slideParts.len() as i64;
        self.links = lk.clone();
    }
    pub fn noteMediaUse(&mut self, idx: i64) {
        let mut i: i64 = 0;
        while i < (self.slideMediaUse.len() as i64) {
            if self.slideMediaUse[i as usize] == idx {
                return;
            }
            i += 1;
        }
        self.slideMediaUse.push(idx);
    }
    pub fn freePart(&self, prefix: &str, suffix: &str, taken: &HashMap<String,bool>) -> String {
        let mut n: i64 = 1;
        while n < 10000 {
            let candidate: String = format!("{}{}{}", prefix, n, suffix);
            let in_source: Option<bool> = self.sourceParts.get(&candidate).cloned();
            let in_taken: Option<bool> = taken.get(&candidate).cloned();
            if in_source.is_none() {
                if in_taken.is_none() {
                    return candidate;
                }
            }
            n += 1;
        }
        format!("{}9999{}", prefix, suffix)
    }
    /// Does the package we are writing over hold exactly this picture at
    /// `part`? The question is asked of the bytes, not of the name. A picture
    /// the user inserted that happens to be called `image1.png` is not the
    /// deck's own `ppt/media/image1.png`, and answering by name is how the
    /// inserted bytes used to be thrown away and the deck's old picture shown
    /// in their place.
    pub fn sourceHolds(&self, part: &str, id: &str) -> bool {
        if !self.overMode {
            return false;
        }
        if part.is_empty() {
            return false;
        }
        let known: Option<bool> = self.sourceParts.get(part).cloned();
        if known.is_none() {
            return false;
        }
        let already: String = OfficeAssetStore::idForPart(&self.assets, part);
        if !already.is_empty() {
            return already == id;
        }
        let mut raw: Vec<u8> = OpcPackage::readBinary(&self.overPkg, part);
        if raw.is_empty() {
            return false;
        }
        let sid: String = OfficeAssetStore::intern(
            &self.assets,
            raw.clone(),
            "",
            part.to_string()
        );
        sid == id
    }
    /// Which entry in the media table this picture is, adding it if it is new.
    ///
    /// Two pictures are the same picture when their bytes are the same, and for
    /// no other reason. Keying this on `part` — the name the picture happens to
    /// be filed under — was wrong in both directions at once: two different
    /// files inserted under one name became one picture, and the same picture
    /// arriving under two names became two. Both go away once the question is
    /// asked of the content.
    pub fn mediaIndexFor(&mut self, part: String, bytes: Vec<u8>) -> i64 {
        // The identity comes from the bytes alone: `part` is deliberately not
        // passed, so a name that has been reused cannot alias one picture on
        // to another.
        let id: String = OfficeAssetStore::intern(
            &self.assets,
            bytes.clone(),
            "",
            String::new()
        );
        let mut i: i64 = 0;
        while i < (self.media.len() as i64) {
            let mut m: Rc<RefCell<PptxWriteMedia>> = self.media[i as usize].clone();
            if m.borrow().assetId == id {
                return i;
            }
            i += 1;
        }
        let mut entry: Rc<RefCell<PptxWriteMedia>> = Rc::new(RefCell::new(PptxWriteMedia::new()));
        entry.borrow_mut().part = part.clone();
        entry.borrow_mut().bytes = bytes.clone();
        entry.borrow_mut().assetId = id.clone();
        if self.sourceHolds(&part, &id) {
            // Already in the package, byte for byte: reference it where it
            // lies rather than writing a second copy of it.
            entry.borrow_mut().existing = true;
            entry.borrow_mut().path = part.clone();
            entry.borrow_mut().name = part.clone();
        } else {
            let mut ext: String = PptxWriter::extensionOf(&part);
            if ext.is_empty() {
                // A picture that never had a package name — one the user just
                // inserted — is named after what its bytes say it is.
                let mime: String = OfficeAssetStore::mimeOf(&self.assets, &id);
                let dotted: String = OfficeAssetStore::extensionForMime(&mime);
                if (dotted.as_bytes().len() as i64) > 1 {
                    ext = dotted[(1) as usize..(dotted.as_bytes().len() as i64) as usize].to_string();
                } else {
                    ext = "png".to_string();
                }
            }
            let mut taken: HashMap<String,bool> = HashMap::new();
            let mut j: i64 = 0;
            while j < (self.media.len() as i64) {
                let mut m2: Rc<RefCell<PptxWriteMedia>> = self.media[j as usize].clone();
                taken.insert(m2.borrow().path.clone(), true);
                j += 1;
            }
            // `freePart` steps over both the names already used in this save
            // and every part the source package brought with it, so a new
            // picture can never land on top of an old one.
            let path: String = self.freePart("ppt/media/image",
                &format!(".{}", ext),
                &taken
            );
            entry.borrow_mut().path = path.clone();
            entry.borrow_mut().name = path.clone();
        }
        self.media.push(entry.clone());
        (self.media.len() as i64) - 1
    }
    /// --- shapes --------------------------------------------------------------
    /// Shapes are renumbered on every write, and a slide's timing tree names
    /// them by that number — so the model is told what number it got, and the
    /// build written after the shape tree names the shapes the file has.
    pub fn nextId(&mut self) -> i64 {
        self.shapeId += 1;
        self.shapeId
    }
    pub fn idFor(&mut self, shape: &Rc<RefCell<PptxShape>>) -> i64 {
        let id: i64 = self.nextId();
        shape.borrow_mut().shapeId = id;
        id
    }
    pub fn shapeXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().isDiagram {
            // PowerPoint's own, untouched: its frame and parts as they were
            if shape.borrow().diagramHadDrawing {
                let kept0: String = self.diagramFrameXml(shape);
                if !kept0.is_empty() {
                    return kept0;
                }
            }
            // laid out here — made here, read from a file that had no drawing
            // for it, or laid out again after an edit: written whole, drawing
            // included, so a reader that cannot lay SmartArt out (anything but
            // PowerPoint) still draws it
            let made: String = self.smartArtXml(shape);
            if !made.is_empty() {
                return made;
            }
            let kept: String = self.diagramFrameXml(shape);
            if !kept.is_empty() {
                return kept;
            }
        }
        if shape.borrow().kind == PptxShapeKind::group() {
            return self.groupXml(shape);
        }
        if shape.borrow().kind == PptxShapeKind::picture() {
            return self.pictureXml(shape);
        }
        if shape.borrow().kind == PptxShapeKind::table() {
            return self.tableXml(shape);
        }
        if shape.borrow().kind == PptxShapeKind::chart() {
            return self.chartFrameXml(shape);
        }
        if shape.borrow().kind == PptxShapeKind::connector() {
            return self.connectorXml(shape);
        }
        self.spXml(shape)
    }
    /// A connector is written as one: a `p:cxnSp` with the line's colour, its
    /// width and the arrowheads on its ends. Written as a plain shape it came
    /// back as a rectangle, which is a different picture.
    pub fn connectorXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        let id: i64 = self.idFor(shape);
        let mut name: String = shape.borrow().name.clone();
        if name.is_empty() {
            name = format!("Connector {}", id);
        }
        let mut out: String = format!("<p:cxnSp><p:nvCxnSpPr><p:cNvPr id=\"{}\" name=\"{}\"{}/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr>", id, PptxWriter::esc(&name), PptxWriter::descrAttr(shape));
        out = format!("{}{}", out, PptxWriter::xfrmXml(shape, "a:xfrm", false));
        let mut preset: String = shape.borrow().preset.clone();
        if preset.is_empty() {
            preset = "line".to_string();
        }
        out = format!("{}<a:prstGeom prst=\"{}\"><a:avLst/></a:prstGeom>", out, PptxWriter::esc(&preset));
        let mut ln: String = "<a:ln".to_string();
        if shape.borrow().lineWidthPt > 0.0 {
            ln = format!("{} w=\"{}\"", ln, PptxWriter::emu(shape.borrow().lineWidthPt));
        }
        ln = format!("{}>", ln);
        if shape.borrow().line.borrow().isSet {
            ln = format!("{}<a:solidFill>{}</a:solidFill>", ln, PptxWriter::colorXml(&shape.borrow().line));
        }
        if !shape.borrow().headEnd.is_empty() {
            ln = format!("{}<a:headEnd type=\"{}\"/>", ln, PptxWriter::esc(&shape.borrow().headEnd));
        }
        if !shape.borrow().tailEnd.is_empty() {
            ln = format!("{}<a:tailEnd type=\"{}\"/>", ln, PptxWriter::esc(&shape.borrow().tailEnd));
        }
        out = format!("{}{}</a:ln>", out, ln);
        format!("{}</p:spPr></p:cxnSp>", out)
    }
    /// Did the style reference put this here rather than the file? Only asked
    /// in save-over mode: a fresh package has no original style block to defer
    /// to, so the resolved value IS what it has to state.
    pub fn derivedFill(&self, shape: &Rc<RefCell<PptxShape>>) -> bool {
        if !self.overMode {
            return false;
        }
        if !shape.borrow().hasStyle {
            return false;
        }
        if shape.borrow().styleFillSig.is_empty() {
            return false;
        }
        // Still exactly what the style produced, so the style reference says
        // it better. Anything the user changed answers no here.
        return shape.borrow().fillSignature() == shape.borrow().styleFillSig;
    }
    pub fn derivedLine(&self, shape: &Rc<RefCell<PptxShape>>) -> bool {
        if !self.overMode {
            return false;
        }
        if !shape.borrow().hasStyle {
            return false;
        }
        if shape.borrow().styleLineSig.is_empty() {
            return false;
        }
        return shape.borrow().lineSignature() == shape.borrow().styleLineSig;
    }
    /// The `<p:style>` block, put back.
    ///
    /// It says the shape follows the theme's format matrix, and the resolver
    /// turns that into a concrete gradient and a concrete font so the shape can
    /// be drawn. Writing the concrete values back and dropping the block LOOKS
    /// identical and is not the same file: the shape has stopped following the
    /// theme, so changing the theme in PowerPoint no longer changes the shape.
    /// Editing one shape on a slide did that to every shape on it.
    ///
    /// Only in save-over mode. A fresh package carries a theme this writer made
    /// up, whose format matrix is three identical solids — a `fillRef` into THAT
    /// would resolve to a flat colour, which is a different bug rather than the
    /// same one avoided.
    pub fn styleXml(&self, shape: &Rc<RefCell<PptxShape>>) -> String {
        if !self.overMode {
            return String::new();
        }
        if !shape.borrow().hasStyle {
            return String::new();
        }
        let mut out: String = "<p:style>".to_string();
        out = format!("{}{}", out, PptxWriter::styleRef(
            "a:lnRef",
            shape.borrow().lineRef,
            &shape.borrow().lineRefColor,
            ""
        ));
        out = format!("{}{}", out, PptxWriter::styleRef(
            "a:fillRef",
            shape.borrow().fillRef,
            &shape.borrow().fillRefColor,
            ""
        ));
        out = format!("{}{}", out, PptxWriter::styleRef(
            "a:effectRef",
            shape.borrow().effectRef,
            &Rc::new(RefCell::new(PptxColor::new())),
            ""
        ));
        let mut font_idx: String = shape.borrow().fontRef.clone();
        if font_idx.is_empty() {
            font_idx = "minor".to_string();
        }
        out = format!("{}{}", out, PptxWriter::styleRefNamed(
            "a:fontRef",
            &font_idx,
            &shape.borrow().fontRefColor
        ));
        format!("{}</p:style>", out)
    }
    /// `<p:nvPr>`, which is where a shape says it IS a placeholder.
    ///
    /// The writer used to emit a bare `<p:nvPr/>` for everything, so a title
    /// placeholder came back as an ordinary text box: out of the outline view,
    /// no longer inheriting from the layout, and "Reset Slide" with nothing to
    /// reset to. It also stops being a title to anything that reads the deck —
    /// including this repository's own accessibility tree, which finds a slide's
    /// title by `p:ph type="title"` and would have started announcing every
    /// saved deck as "Slide 1, Slide 2, Slide 3".
    pub fn nvPrXml(&self, shape: &Rc<RefCell<PptxShape>>) -> String {
        if !shape.borrow().isPlaceholder {
            return "<p:nvPr/>".to_string();
        }
        let mut out: String = "<p:nvPr><p:ph".to_string();
        if !shape.borrow().phType.is_empty() {
            out = format!("{} type=\"{}\"", out, PptxWriter::esc(&shape.borrow().phType));
        }
        if shape.borrow().phIdx >= 0 {
            out = format!("{} idx=\"{}\"", out, shape.borrow().phIdx);
        }
        format!("{}/></p:nvPr>", out)
    }
    /// A shape with an equation in it goes out twice, the way PowerPoint itself
    /// writes one: the equation for a reader that knows Office Math (`a14`),
    /// and the same shape with the formula as plain text for every other —
    /// Keynote, Google Slides, LibreOffice — which would otherwise drop it.
    /// Both are the one shape, so they share its id.
    pub fn spXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        let id: i64 = self.idFor(shape);
        if shape.borrow().hasText && PptxWriter::hasMath(&shape.borrow().text.clone()) {
            let choice: String = self.spXmlAs(shape, id, true);
            let mut fallback: String = String::new();
            if !shape.borrow().mathFallback.is_empty() {
                fallback = self.shapeXml(&shape.borrow().mathFallback[0].clone());
            } else {
                fallback = self.spXmlAs(shape, id, false);
            }
            return format!("<mc:AlternateContent xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\"><mc:Choice xmlns:a14=\"http://schemas.microsoft.com/office/drawing/2010/main\" Requires=\"a14\">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>", choice, fallback);
        }
        self.spXmlAs(shape, id, false)
    }
    pub fn spXmlAs(&self, shape: &Rc<RefCell<PptxShape>>, id: i64, with_math: bool) -> String {
        let mut name: String = shape.borrow().name.clone();
        if name.is_empty() {
            name = format!("Shape {}", id);
        }
        let mut locks: String = "<p:cNvSpPr/>";
        if shape.borrow().locked {
            locks = "<p:cNvSpPr><a:spLocks noMove=\"1\" noResize=\"1\" noRot=\"1\"/></p:cNvSpPr>";
        }
        let mut out: String = format!("<p:sp><p:nvSpPr><p:cNvPr id=\"{}\" name=\"{}\"{}/>{}{}</p:nvSpPr><p:spPr>", id, PptxWriter::esc(&name), PptxWriter::descrAttr(shape), locks, self.nvPrXml(shape));
        // Geometry the placeholder in the LAYOUT supplied is not the slide's.
        // Written here it pins the shape, so moving the title on the layout no
        // longer moves it on the slides.
        let mut stated_xfrm: bool = true;
        if self.overMode {
            if !shape.borrow().sheetXfrmSig.is_empty() {
                // Unmoved since it was inherited. A move — or a resize, or a
                // rotation — changes the signature and is written.
                if shape.borrow().xfrmSignature() == shape.borrow().sheetXfrmSig {
                    stated_xfrm = false;
                }
            }
        }
        if stated_xfrm {
            out = format!("{}{}", out, PptxWriter::xfrmXml(shape, "a:xfrm", false));
        }
        out = format!("{}{}", out, PptxWriter::geomXml(shape));
        // A fill or a line the RESOLVER worked out from the style reference is
        // not the shape's own, and stating it here would override the very
        // reference this writer is about to put back.
        if self.derivedFill(shape) {
        } else {
            out = format!("{}{}", out, PptxWriter::fillXml(shape));
        }
        if self.derivedLine(shape) {
        } else {
            out = format!("{}{}", out, PptxWriter::lineXml(shape));
        }
        out = format!("{}{}", out, PptxWriter::effectXml(shape));
        out = format!("{}</p:spPr>", out);
        out = format!("{}{}", out, self.styleXml(shape));
        // `txBody` is optional on a shape, and writing an empty one turns a
        // shape that has no text into one that has an empty paragraph — which
        // is a different thing to everything that reads the file afterwards.
        if shape.borrow().hasText {
            out = format!("{}{}", out, PptxWriter::bodyXml(
                &shape.borrow().text,
                self.overMode,
                with_math,
                &self.links
            ));
        }
        format!("{}</p:sp>", out)
    }
    pub fn pictureXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().imageBytes.is_empty() {
            // No bytes to write: an outline where the picture was is more
            // honest than a missing relationship.
            let mut ghost: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
            ghost.borrow_mut().kind = 0;
            ghost.borrow_mut().name = shape.borrow().name.clone();
            ghost.borrow_mut().x = shape.borrow().x;
            ghost.borrow_mut().y = shape.borrow().y;
            ghost.borrow_mut().width = shape.borrow().width;
            ghost.borrow_mut().height = shape.borrow().height;
            ghost.borrow_mut().rotation = shape.borrow().rotation;
            ghost.borrow_mut().preset = "rect".to_string();
            ghost.borrow_mut().line = PptxColor::srgbOf("BFBFBF".to_string());
            ghost.borrow_mut().lineWidthPt = 1.0;
            return self.spXml(&ghost);
        }
        let id: i64 = self.idFor(shape);
        let mut name: String = shape.borrow().name.clone();
        if name.is_empty() {
            name = format!("Picture {}", id);
        }
        let mut _tmp_5047 = shape.borrow().imagePart.clone();
        let idx: i64 = self.mediaIndexFor(_tmp_5047, shape.borrow().imageBytes.clone());
        self.noteMediaUse(idx);
        // Relationship ids are names, not numbers: naming the pictures and the
        // charts apart means neither has to know how many of the other there
        // will turn out to be.
        let rid: String = format!("rIdImg{}", idx + 1);
        let mut out: String = format!("<p:pic><p:nvPicPr><p:cNvPr id=\"{}\" name=\"{}\"{}/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>", id, PptxWriter::esc(&name), PptxWriter::descrAttr(shape));
        let mut blip: String = format!("<a:blip r:embed=\"{}\"/>", rid);
        if !shape.borrow().svgBytes.is_empty() {
            // the vector, as the 2016 extension; the PNG above stays the
            // picture for readers that know nothing of it
            let mut sp: String = shape.borrow().svgPart.clone();
            if sp.is_empty() {
                sp = "picture.svg".to_string();
            }
            let sidx: i64 = self.mediaIndexFor(sp.clone(), shape.borrow().svgBytes.clone());
            self.noteMediaUse(sidx);
            blip = PptxWriter::svgBlipXml(&rid, &format!("rIdImg{}", sidx + 1));
        }
        out = format!("{}<p:blipFill>{}{}<a:stretch><a:fillRect/></a:stretch></p:blipFill>", out, blip, PptxWriter::srcRectXml(shape));
        out = format!("{}<p:spPr>{}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>", out, PptxWriter::xfrmXml(
            shape,
            "a:xfrm",
            false
        ));
        format!("{}</p:pic>", out)
    }
    pub fn groupXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        let id: i64 = self.idFor(shape);
        let mut name: String = shape.borrow().name.clone();
        if name.is_empty() {
            name = format!("Group {}", id);
        }
        let mut out: String = format!("<p:grpSp><p:nvGrpSpPr>{}<p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>", PptxWriter::cNvPrXml(
            id,
            &name,
            shape
        ));
        out = format!("{}<p:grpSpPr>{}</p:grpSpPr>", out, PptxWriter::xfrmXml(shape, "a:xfrm", true));
        let mut i: i64 = 0;
        while i < (shape.borrow().children.len() as i64) {
            let mut child: Rc<RefCell<PptxShape>> = shape.borrow().children[i as usize].clone();
            out = format!("{}{}", out, self.shapeXml(&child));
            i += 1;
        }
        format!("{}</p:grpSp>", out)
    }
    pub fn tableXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        let id: i64 = self.idFor(shape);
        let rows: i64 = shape.borrow().tableRows;
        let cols: i64 = shape.borrow().tableCols;
        if rows < 1 {
            return self.spXml(shape);
        }
        if cols < 1 {
            return self.spXml(shape);
        }
        let col_w: f64 = shape.borrow().width / (cols as f64);
        let row_h: f64 = shape.borrow().height / (rows as f64);
        let mut out: String = format!("<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{}\" name=\"Table {}\"{}/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>", id, id, PptxWriter::descrAttr(shape));
        out = format!("{}<p:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm>", out, PptxWriter::emu(shape.borrow().x), PptxWriter::emu(shape.borrow().y), PptxWriter::emu(shape.borrow().width), PptxWriter::emu(shape.borrow().height));
        out = format!("{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr/><a:tblGrid>", out);
        let mut c: i64 = 0;
        while c < cols {
            let mut cw: f64 = col_w;
            if c < (shape.borrow().tableColW.len() as i64) {
                let stated: f64 = shape.borrow().tableColW[c as usize];
                if stated > 0.0 {
                    cw = stated;
                }
            }
            out = format!("{}<a:gridCol w=\"{}\"/>", out, PptxWriter::emu(cw));
            c += 1;
        }
        out = format!("{}</a:tblGrid>", out);
        let mut r: i64 = 0;
        while r < rows {
            let mut rh: f64 = row_h;
            if r < (shape.borrow().tableRowH.len() as i64) {
                let stated_h: f64 = shape.borrow().tableRowH[r as usize];
                if stated_h > 0.0 {
                    rh = stated_h;
                }
            }
            out = format!("{}<a:tr h=\"{}\">", out, PptxWriter::emu(rh));
            let mut cc: i64 = 0;
            while cc < cols {
                let idx: i64 = r * cols + cc;
                let mut cell: Rc<RefCell<PptxTableCell>> = Rc::new(RefCell::new(PptxTableCell::new()));
                if idx < (shape.borrow().tableCells.len() as i64) {
                    cell = shape.borrow().tableCells[idx as usize].clone();
                }
                // The cell's own paragraphs when it has them — a cell is a
                // text body, and flattening it to one run would lose every
                // colour, size and line the deck stated.
                let mut body_xml: String = String::new();
                if !cell.borrow().body.borrow().paragraphs.is_empty() {
                    // A table cell's body is `a:txBody`, not `p:txBody` — the
                    // same content in the drawing namespace.
                    body_xml = PptxWriter::bodyXml(
                        &cell.borrow().body,
                        self.overMode,
                        false,
                        &self.links
                    );
                    let cut: i64 = PptxText::index_of(&body_xml, "<p:txBody");
                    if cut == 0 {
                        body_xml = format!("<a:txBody{}", body_xml[(("<p:txBody".to_string().as_bytes().len() as i64)) as usize..(body_xml.as_bytes().len() as i64) as usize].to_string());
                    }
                    let close: i64 = PptxText::index_of(&body_xml, "</p:txBody>");
                    if close > 0 {
                        body_xml = format!("{}</a:txBody>", body_xml[(0) as usize..(close) as usize].to_string());
                    }
                } else {
                    let mut run: Rc<RefCell<PptxTextRun>> = Rc::new(RefCell::new(PptxTextRun::new()));
                    run.borrow_mut().text = cell.borrow().text.clone();
                    run.borrow_mut().bold = cell.borrow().bold;
                    run.borrow_mut().fontSizePt = cell.borrow().fontSizePt;
                    run.borrow_mut().sizeSet = true;
                    body_xml = format!("<a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr algn=\"{}\"/>{}</a:p></a:txBody>", cell.borrow().align, PptxWriter::runXml(
                        &run,
                        false,
                        false,
                        &self.links
                    ));
                }
                let mut cell_fill: String = "<a:noFill/>".to_string();
                if cell.borrow().fill.borrow().isSet {
                    cell_fill = format!("<a:solidFill>{}</a:solidFill>", PptxWriter::colorXml(&cell.borrow().fill));
                }
                let mut _tmp_5054 = cell.borrow().marLPt.clone();
                let mut _tmp_5055 = cell.borrow().marRPt.clone();
                let mut _tmp_5056 = cell.borrow().marTPt.clone();
                let mut pr_attrs: String = format!(" marL=\"{}\" marR=\"{}\" marT=\"{}\" marB=\"{}\"", PptxWriter::emu(_tmp_5054), PptxWriter::emu(_tmp_5055), PptxWriter::emu(_tmp_5056), PptxWriter::emu(cell.borrow().marBPt.clone()));
                if cell.borrow().anchor != "t" {
                    pr_attrs = format!("{} anchor=\"{}\"", pr_attrs, cell.borrow().anchor);
                }
                let mut edges: String = String::new();
                edges = format!("{}{}", edges, PptxWriter::cellLineXml("a:lnL", &cell.borrow().lnL));
                edges = format!("{}{}", edges, PptxWriter::cellLineXml("a:lnR", &cell.borrow().lnR));
                edges = format!("{}{}", edges, PptxWriter::cellLineXml("a:lnT", &cell.borrow().lnT));
                edges = format!("{}{}", edges, PptxWriter::cellLineXml("a:lnB", &cell.borrow().lnB));
                let mut span_attr: String = String::new();
                if cell.borrow().gridSpan > 1 {
                    span_attr = format!("{} gridSpan=\"{}\"", span_attr, cell.borrow().gridSpan);
                }
                if cell.borrow().rowSpan > 1 {
                    span_attr = format!("{} rowSpan=\"{}\"", span_attr, cell.borrow().rowSpan);
                }
                if cell.borrow().merged {
                    span_attr = format!("{} hMerge=\"1\"", span_attr);
                }
                out = format!("{}<a:tc{}>{}<a:tcPr{}>{}{}</a:tcPr></a:tc>", out, span_attr, body_xml, pr_attrs, edges, cell_fill);
                cc += 1;
            }
            out = format!("{}</a:tr>", out);
            r += 1;
        }
        format!("{}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>", out)
    }
    /// --- master and layout sheets --------------------------------------------
    ///
    /// A sheet that was edited is written back into its OWN part, and only the
    /// shape tree of it: a master carries a colour map, a text style and a list
    /// of its layouts, and none of that is ours to regenerate.
    /// --- what this writer does not understand ---------------------------------
    ///
    /// A shape tree can hold children this model has no room for: SmartArt and
    /// newer chart types wrapped in `mc:AlternateContent`, embedded objects,
    /// ink, `p:extLst` blocks belonging to whoever wrote them. The parser
    /// remembered each one as a span in the part it came out of; here it is
    /// copied back through, byte for byte, at the place in z-order it had.
    ///
    /// Ranger does not have to understand something in order to preserve it.
    fn opaqueText(&mut self, r#ref: &Rc<RefCell<SourceRef>>) -> String {
        if !self.keepOpaque {
            return String::new();
        }
        if !r#ref.borrow().known {
            return String::new();
        }
        if self.opaqueParts.contains_key(&r#ref.borrow().part) {
            return SourceRef::slice(&r#ref, &self.opaqueParts.get(&r#ref.borrow().part).cloned().unwrap());
        }
        let xml: String = OpcPackage::readXml(&self.overPkg, &r#ref.borrow().part.clone());
        self.opaqueParts.insert(r#ref.borrow().part.clone(), xml.clone());
        return SourceRef::slice(&r#ref, &xml);
    }
    /// Everything that sat exactly after `written` shapes. `atLeast` catches the
    /// tail: when an edit deleted shapes the counts no longer line up, and an
    /// element whose slot is past the end of the tree still has to come back.
    pub fn opaqueAt(&mut self, list: &Vec<Rc<RefCell<PptxOpaque>>>, written: i64, at_least: bool) -> String {
        let mut out: String = String::new();
        if !self.keepOpaque {
            return out;
        }
        let mut i: i64 = 0;
        while i < (list.len() as i64) {
            let mut op: Rc<RefCell<PptxOpaque>> = list[i as usize].clone();
            let mut hit: bool = op.borrow().afterShapes == written;
            if at_least {
                hit = op.borrow().afterShapes >= written;
            }
            if hit {
                out.push_str(&self.opaqueText(&op.borrow().source));
            }
            i += 1;
        }
        out
    }
    /// Every span in a list, in the order they were read.
    pub fn opaqueAll(&mut self, list: &Vec<Rc<RefCell<PptxOpaque>>>) -> String {
        let mut out: String = String::new();
        if !self.keepOpaque {
            return out;
        }
        let mut i: i64 = 0;
        while i < (list.len() as i64) {
            let mut op: Rc<RefCell<PptxOpaque>> = list[i as usize].clone();
            out.push_str(&self.opaqueText(&op.borrow().source));
            i += 1;
        }
        out
    }
    pub fn opaqueHas(&self, list: &Vec<Rc<RefCell<PptxOpaque>>>, name: &str) -> bool {
        if !self.keepOpaque {
            return false;
        }
        let mut i: i64 = 0;
        while i < (list.len() as i64) {
            let mut op: Rc<RefCell<PptxOpaque>> = list[i as usize].clone();
            if op.borrow().name == name {
                return true;
            }
            i += 1;
        }
        false
    }
    /// The children of a shape tree: the shapes this writer generates, with the
    /// elements it does not model spliced back in between them.
    pub fn treeChildren(&mut self, shapes: &Vec<Rc<RefCell<PptxShape>>>, opaque: &Vec<Rc<RefCell<PptxOpaque>>>, skip_inherit: bool) -> String {
        let mut out: String = String::new();
        let mut written: i64 = 0;
        let mut i: i64 = 0;
        while i < (shapes.len() as i64) {
            let mut sh: Rc<RefCell<PptxShape>> = shapes[i as usize].clone();
            let mut write: bool = true;
            if skip_inherit {
                if sh.borrow().inherited {
                    write = false;
                }
            }
            if write {
                out.push_str(&self.opaqueAt(opaque, written, false));
                out.push_str(&self.shapeXml(&sh));
                written += 1;
            }
            i += 1;
        }
        out.push_str(&self.opaqueAt(opaque, written, true));
        out
    }
    pub fn spTreeXml(&mut self, shapes: &Vec<Rc<RefCell<PptxShape>>>, opaque: &Vec<Rc<RefCell<PptxOpaque>>>) -> String {
        self.shapeId = 1;
        self.resetSlideUse();
        let mut out: String = PptxWriter::spTreeOpen();
        out.push_str(&self.treeChildren(shapes, opaque, false));
        format!("{}</p:spTree>", out)
    }
    pub fn sheetWithShapes(&mut self, xml: &str, sheet: &Rc<RefCell<PptxSheet>>) -> String {
        let mut _tmp_5061 = sheet.borrow().shapes.clone();
        let tree: String = self.spTreeXml(&_tmp_5061, &sheet.borrow().opaque.clone());
        let start: i64 = PptxText::index_of(&xml, "<p:spTree");
        if start < 0 {
            return xml.to_string();
        }
        let close: i64 = PptxText::index_of_from(&xml, "</p:spTree>", start);
        if close < 0 {
            let self_end: i64 = PptxText::index_of_from(&xml, "/>", start);
            if self_end < 0 {
                return xml.to_string();
            }
            return format!("{}{}{}", xml[(0) as usize..(start) as usize].to_string(), tree, xml[(self_end + 2) as usize..(xml.as_bytes().len() as i64) as usize].to_string());
        }
        let after: i64 = close + ("</p:spTree>".to_string().as_bytes().len() as i64);
        format!("{}{}{}", xml[(0) as usize..(start) as usize].to_string(), tree, xml[(after) as usize..(xml.as_bytes().len() as i64) as usize].to_string())
    }
    pub fn noteSlide(&mut self, slide: &Rc<RefCell<PptxSlide>>, path: String) {
        if slide.borrow().notesText().is_empty() {
            return;
        }
        self.notesPaths.push(path.clone());
        let mut _tmp_5064 = PptxWriter::notesXml(&slide.borrow().notes.clone());
        self.notesBodies.push(_tmp_5064);
        self.slideNotesPath = path.clone();
    }
    pub fn slideXml(&mut self, slide: &Rc<RefCell<PptxSlide>>) -> String {
        self.shapeId = 1;
        self.resetSlideUse();
        let mut targets: Vec<Rc<RefCell<PptxShape>>> = PptxWriter::stepTargets(slide);
        let mut ti: i64 = 0;
        while ti < (targets.len() as i64) {
            let mut zeroed: Rc<RefCell<PptxShape>> = targets[ti as usize].clone();
            zeroed.borrow_mut().shapeId = 0;
            ti += 1;
        }
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld>".to_string();
        // A background the slide did not state is the master's or the layout's,
        // and writing it here would freeze it onto this slide. In a fresh
        // package there is no master to defer to, so the resolved colour IS
        // what has to be stated.
        let mut own_bg: bool = slide.borrow().bg.borrow().isSet;
        if self.overMode {
            if slide.borrow().bgFromSheet {
                own_bg = false;
            }
        }
        // A picture behind the slide outranks a colour, the way
        // `background-image` is painted over `background-color` in a browser.
        if !slide.borrow().bgImage.is_empty() {
            let mut _tmp_5065 = slide.borrow().bgImageName.clone();
            let bg_idx: i64 = self.mediaIndexFor(_tmp_5065, slide.borrow().bgImage.clone());
            // The same note every picture makes, or the relationship it needs
            // is never written and PowerPoint refuses the part.
            self.noteMediaUse(bg_idx);
            let bg_rid: String = format!("rIdImg{}", bg_idx + 1);
            out = format!("{}<p:bg><p:bgPr><a:blipFill><a:blip r:embed=\"{}\"/><a:stretch><a:fillRect/></a:stretch></a:blipFill><a:effectLst/></p:bgPr></p:bg>", out, bg_rid);
        } else {
            if own_bg {
                out = format!("{}<p:bg><p:bgPr><a:solidFill>{}</a:solidFill><a:effectLst/></p:bgPr></p:bg>", out, PptxWriter::colorXml(&slide.borrow().bg));
            }
        }
        out = format!("{}{}", out, PptxWriter::spTreeOpen());
        // `skipInherited` is also what makes the opaque slots line up: it is on
        // exactly when the shapes being written are the ones the slide part
        // itself declared, which is what the parser counted.
        out = format!("{}{}", out, self.treeChildren(&slide.borrow().shapes,
            &slide.borrow().opaque,
            self.skipInherited
        ));
        out = format!("{}</p:spTree></p:cSld>", out);
        // What stood between `</p:cSld>` and the transition. A slide that
        // states its own colour map override keeps it: writing a plain
        // `masterClrMapping` over the top of one is how a deck loses the
        // palette swap a layout gave it.
        if !self.opaqueHas(&slide.borrow().sldHead.clone(), "clrMapOvr") {
            out = format!("{}<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>", out);
        }
        out = format!("{}{}", out, self.opaqueAll(&slide.borrow().sldHead));
        let mut tk: i64 = 0;
        while tk < (targets.len() as i64) {
            let mut step: Rc<RefCell<PptxBuildStep>> = slide.borrow().build[tk as usize].clone();
            let mut sh2: Rc<RefCell<PptxShape>> = targets[tk as usize].clone();
            step.borrow_mut().shapeId = sh2.borrow().shapeId;
            tk += 1;
        }
        out = format!("{}{}", out, PptxWriter::transitionXml(slide));
        out = format!("{}{}", out, PptxWriter::timingXml(slide));
        // And what stood after them — a tool's own extension list, which the
        // schema puts last and which nothing here understands.
        out = format!("{}{}", out, self.opaqueAll(&slide.borrow().sldTail));
        format!("{}</p:sld>", out)
    }
    /// A slide names the layout it follows, whatever the package already said
    /// about it, and the pictures and charts THIS slide uses — not every one in
    /// the deck.
    fn slideRelsXmlFor(&self, layout_target: &str, keep: &Vec<Rc<RefCell<OpcRelationship>>>) -> String {
        let mut notes_rel: String = String::new();
        if !self.slideNotesPath.is_empty() {
            notes_rel = format!("<Relationship Id=\"rIdNotes\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"{}\"/>", PptxWriter::relTargetFrom("ppt/slides", &self.slideNotesPath));
        }
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">".to_string();
        if !layout_target.is_empty() {
            out = format!("{}<Relationship Id=\"rIdLayout\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"{}\"/>", out, layout_target);
        }
        let mut k: i64 = 0;
        while k < (keep.len() as i64) {
            let mut rel: Rc<RefCell<OpcRelationship>> = keep[k as usize].clone();
            out = format!("{}<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>", out, rel.borrow().id, rel.borrow().r#type, rel.borrow().target);
            k += 1;
        }
        let mut i: i64 = 0;
        while i < (self.slideMediaUse.len() as i64) {
            let idx: i64 = self.slideMediaUse[i as usize];
            let mut m: Rc<RefCell<PptxWriteMedia>> = self.media[idx as usize].clone();
            out = format!("{}<Relationship Id=\"rIdImg{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"{}\"/>", out, idx + 1, PptxWriter::relTargetFrom("ppt/slides", &m.borrow().path));
            i += 1;
        }
        let mut j: i64 = 0;
        while j < (self.slideChartUse.len() as i64) {
            let cidx: i64 = self.slideChartUse[j as usize];
            let cpath: String = self.chartPaths[cidx as usize].clone();
            out = format!("{}<Relationship Id=\"rIdChart{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"{}\"/>", out, cidx + 1, PptxWriter::relTargetFrom("ppt/slides", &cpath));
            j += 1;
        }
        let mut d: i64 = 0;
        while d < (self.slideDiagramUse.len() as i64) {
            let didx: i64 = self.slideDiagramUse[d as usize];
            let mut kinds: Vec<String> = PptxWriter::diagramRelKinds();
            let mut kk: i64 = 0;
            while kk < 5 {
                let dpath: String = self.diagramPaths[(didx * 5 + kk) as usize].clone();
                out = format!("{}<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>", out, self.diagramRelIds[(didx * 5 + kk) as usize].clone(), kinds[kk as usize].clone(), PptxWriter::relTargetFrom("ppt/slides", &dpath));
                kk += 1;
            }
            d += 1;
        }
        let mut u: i64 = 0;
        while u < (self.links.borrow().urls.len() as i64) {
            out = format!("{}<Relationship Id=\"rIdLink{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"{}\" TargetMode=\"External\"/>", out, u + 1, PptxWriter::esc(&self.links.borrow().urls[u as usize].clone()));
            u += 1;
        }
        let mut sl: i64 = 0;
        while sl < (self.links.borrow().slides.len() as i64) {
            let to: i64 = self.links.borrow().slides[sl as usize];
            out = format!("{}<Relationship Id=\"rIdJump{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"{}\"/>", out, sl + 1, PptxWriter::relTargetFrom("ppt/slides", &self.slideParts[to as usize].clone()));
            sl += 1;
        }
        format!("{}{}</Relationships>", out, notes_rel)
    }
    /// --- charts --------------------------------------------------------------
    ///
    /// ChartML with the numbers CACHED rather than referenced: a chart part
    /// normally points at an embedded workbook for its data, and a deck written
    /// here has none, so the cache is the data. Every reader looks there first —
    /// it is what makes a chart draw before the workbook is opened.
    /// A SmartArt diagram, saved over the file it came from: its own
    /// `graphicFrame`, byte for byte, with the id this write gave it and the
    /// position and size it has now. The relationships it names (data, layout,
    /// style, colours, drawing) are kept with the slide's, so it is the same
    /// SmartArt when PowerPoint opens it.
    ///
    /// Empty when that cannot be done — a new file, or a diagram whose shapes
    /// were edited here — and the diagram is written as the group it is drawn
    /// as. Shapes, not SmartArt: what was edited is kept, and the diagram stops
    /// being one PowerPoint can lay out again.
    pub fn diagramFrameXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().diagramSig != shape.borrow().diagramSignature() {
            return String::new();
        }
        // only on the slide it came from: that slide's relationships are the
        // ones its frame names, and a copy on another slide would point at
        // nothing
        if shape.borrow().diagramFrame.borrow().part != self.writingPart {
            return String::new();
        }
        let frame: String = self.opaqueText(&shape.borrow().diagramFrame.clone());
        if frame.is_empty() {
            return String::new();
        }
        let xs: i64 = PptxText::index_of(&frame, "<p:xfrm");
        let xe: i64 = PptxText::index_of(&frame, "</p:xfrm>");
        if xs < 0 || xe < xs {
            return String::new();
        }
        let mut _tmp_5070 = shape.borrow().x.clone();
        let mut _tmp_5071 = shape.borrow().y.clone();
        let mut _tmp_5072 = shape.borrow().width.clone();
        let xfrm: String = format!("<p:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm>", PptxWriter::emu(_tmp_5070), PptxWriter::emu(_tmp_5071), PptxWriter::emu(_tmp_5072), PptxWriter::emu(shape.borrow().height.clone()));
        let mut out: String = format!("{}{}{}", frame[(0) as usize..(xs) as usize].to_string(), xfrm, frame[(xe + ("</p:xfrm>".to_string().as_bytes().len() as i64)) as usize..(frame.as_bytes().len() as i64) as usize].to_string());
        // the id: every shape is numbered again on a write
        let cs: i64 = PptxText::index_of(&out, "<p:cNvPr");
        if cs >= 0 {
            let rest: String = out[(cs) as usize..(out.as_bytes().len() as i64) as usize].to_string();
            let ia: i64 = PptxText::index_of(&rest, " id=\"");
            if ia >= 0 {
                let vs: i64 = (cs + ia) + 5;
                let tail: String = out[(vs) as usize..(out.as_bytes().len() as i64) as usize].to_string();
                let q: i64 = PptxText::index_of(&tail, "\"");
                if q >= 0 {
                    let id: i64 = self.idFor(shape);
                    out = format!("{}{}{}", out[(0) as usize..(vs) as usize].to_string(), id, out[(vs + q) as usize..(out.as_bytes().len() as i64) as usize].to_string());
                }
            }
        }
        out
    }
    pub fn smartArtXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        if shape.borrow().diagramModelXml.is_empty() || shape.borrow().diagramLayoutXml.is_empty() {
            return String::new();
        }
        if shape.borrow().diagramSig != shape.borrow().diagramSignature() {
            return String::new();
        }
        let idx: i64 = (((self.diagramPaths.len() as i64) as f64) / 5.0).floor() as i64;
        let mut taken: HashMap<String,bool> = HashMap::new();
        let mut ti: i64 = 0;
        while ti < (self.diagramPaths.len() as i64) {
            taken.insert(self.diagramPaths[ti as usize].clone(), true);
            ti += 1;
        }
        let mut prefixes: Vec<String> = vec!["ppt/diagrams/data".to_string(), "ppt/diagrams/layout".to_string(), "ppt/diagrams/quickStyle".to_string(), "ppt/diagrams/colors".to_string(), "ppt/diagrams/drawing".to_string()];
        let mut paths: Vec<String> = Vec::new();
        let mut k: i64 = 0;
        while k < 5 {
            let mut path: String = format!("{}{}.xml", prefixes[k as usize].clone(), idx + 1);
            if self.overMode {
                path = self.freePart(&prefixes[k as usize].clone(), ".xml", &taken);
            }
            taken.insert(path.clone(), true);
            paths.push(path.clone());
            k += 1;
        }
        let mut nb: Rc<RefCell<SaNumbering>> = SaPackage::number(&shape.borrow().diagramModelXml.clone());
        if !nb.borrow().ok {
            return String::new();
        }
        let data: String = nb.borrow().dataXml(PptxWriter::diagramRelId(&paths[4].clone()));
        let mut bodies: Vec<String> = Vec::new();
        bodies.push(data);
        bodies.push(shape.borrow().diagramLayoutXml.clone());
        let mut style_body: String = shape.borrow().diagramStyleXml.clone();
        if style_body.is_empty() {
            style_body = SaPackage::styleDefXml(&shape.borrow().diagramStyleId);
        }
        let mut colors_body: String = shape.borrow().diagramColorsXml.clone();
        if colors_body.is_empty() {
            colors_body = SaPackage::colorsDefXml(&shape.borrow().diagramColorsId);
        }
        bodies.push(style_body);
        bodies.push(colors_body);
        bodies.push(PptxWriter::diagramDrawingXml(shape, &nb));
        k = 0;
        while k < 5 {
            self.diagramPaths.push(paths[k as usize].clone());
            self.diagramBodies.push(bodies[k as usize].clone());
            self.diagramRelIds.push(PptxWriter::diagramRelId(&paths[k as usize].clone()));
            k += 1;
        }
        self.slideDiagramUse.push(idx);
        let id: i64 = self.idFor(shape);
        let mut name: String = shape.borrow().name.clone();
        if name.is_empty() {
            name = format!("Diagram {}", id);
        }
        let mut out: String = format!("<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{}\" name=\"{}\"{}/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>", id, PptxWriter::esc(&name), PptxWriter::descrAttr(shape));
        out = format!("{}<p:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm>", out, PptxWriter::emu(shape.borrow().x), PptxWriter::emu(shape.borrow().y), PptxWriter::emu(shape.borrow().width), PptxWriter::emu(shape.borrow().height));
        out = format!("{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"><dgm:relIds xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" r:dm=\"{}\" r:lo=\"{}\" r:qs=\"{}\" r:cs=\"{}\"/></a:graphicData></a:graphic>", out, PptxWriter::diagramRelId(&paths[0].clone()), PptxWriter::diagramRelId(&paths[1].clone()), PptxWriter::diagramRelId(&paths[2].clone()), PptxWriter::diagramRelId(&paths[3].clone()));
        format!("{}</p:graphicFrame>", out)
    }
    pub fn chartFrameXml(&mut self, shape: &Rc<RefCell<PptxShape>>) -> String {
        let mut _tmp_5077 = PptxWriter::chartXml(&shape.borrow().chart.clone());
        self.charts.push(_tmp_5077);
        let mut taken: HashMap<String,bool> = HashMap::new();
        let mut ci: i64 = 0;
        while ci < (self.chartPaths.len() as i64) {
            taken.insert(self.chartPaths[ci as usize].clone(), true);
            ci += 1;
        }
        let mut path: String = format!("ppt/charts/chart{}.xml", self.charts.len() as i64);
        if self.overMode {
            path = self.freePart("ppt/charts/chart", ".xml", &taken);
        }
        self.chartPaths.push(path);
        self.slideChartUse.push((self.charts.len() as i64) - 1);
        let rid: String = format!("rIdChart{}", self.charts.len() as i64);
        let id: i64 = self.idFor(shape);
        let mut name: String = shape.borrow().name.clone();
        if name.is_empty() {
            name = format!("Chart {}", id);
        }
        let mut out: String = format!("<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"{}\" name=\"{}\"{}/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>", id, PptxWriter::esc(&name), PptxWriter::descrAttr(shape));
        out = format!("{}<p:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></p:xfrm>", out, PptxWriter::emu(shape.borrow().x), PptxWriter::emu(shape.borrow().y), PptxWriter::emu(shape.borrow().width), PptxWriter::emu(shape.borrow().height));
        out = format!("{}<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"><c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" r:id=\"{}\"/></a:graphicData></a:graphic>", out, rid);
        format!("{}</p:graphicFrame>", out)
    }
    pub fn presentationXml(&self, pres: &Rc<RefCell<PptxPresentation>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst>".to_string();
        if !self.notesPaths.is_empty() {
            out = format!("{}<p:notesMasterIdLst><p:notesMasterId r:id=\"rIdNotesMaster\"/></p:notesMasterIdLst>", out);
        }
        out = format!("{}<p:sldIdLst>", out);
        let mut ids: Vec<i64> = PptxWriter::slideIdsFor(pres, pres.borrow().slides.len() as i64);
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            out = format!("{}<p:sldId id=\"{}\" r:id=\"rId{}\"/>", out, ids[i as usize], i + 2);
            i += 1;
        }
        out = format!("{}</p:sldIdLst><p:sldSz cx=\"{}\" cy=\"{}\"/><p:notesSz cx=\"6858000\" cy=\"9144000\"/>", out, PptxWriter::emu(pres.borrow().slideWidth), PptxWriter::emu(pres.borrow().slideHeight));
        out = format!("{}{}", out, PptxWriter::sectionLstXml(pres, &ids));
        format!("{}</p:presentation>", out)
    }
    pub fn presentationRelsXml(&self, pres: &Rc<RefCell<PptxPresentation>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>".to_string();
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            out = format!("{}<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{}.xml\"/>", out, i + 2, i + 1);
            i += 1;
        }
        let theme_rid: String = format!("rId{}", (pres.borrow().slides.len() as i64) + 2);
        out = format!("{}<Relationship Id=\"{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"theme/theme1.xml\"/>", out, theme_rid);
        if !self.notesPaths.is_empty() {
            out = format!("{}<Relationship Id=\"rIdNotesMaster\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster\" Target=\"notesMasters/notesMaster1.xml\"/>", out);
        }
        format!("{}</Relationships>", out)
    }
    pub fn contentTypesXml(&self, pres: &Rc<RefCell<PptxPresentation>>) -> String {
        let mut out: String = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Default Extension=\"jpeg\" ContentType=\"image/jpeg\"/><Default Extension=\"gif\" ContentType=\"image/gif\"/><Default Extension=\"svg\" ContentType=\"image/svg+xml\"/>".to_string();
        out = format!("{}<Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>", out);
        out = format!("{}<Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>", out);
        out = format!("{}<Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>", out);
        out = format!("{}<Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>", out);
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            out = format!("{}<Override PartName=\"/ppt/slides/slide{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>", out, i + 1);
            i += 1;
        }
        let mut j: i64 = 0;
        while j < (self.charts.len() as i64) {
            out = format!("{}<Override PartName=\"/ppt/charts/chart{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/>", out, j + 1);
            j += 1;
        }
        let mut dg: i64 = 0;
        while dg < (self.diagramPaths.len() as i64) {
            let dgp: String = self.diagramPaths[dg as usize].clone();
            out = format!("{}<Override PartName=\"/{}\" ContentType=\"{}\"/>", out, dgp, PptxWriter::contentTypeOf(&dgp));
            dg += 1;
        }
        let mut n: i64 = 0;
        while n < (self.notesPaths.len() as i64) {
            out = format!("{}<Override PartName=\"/{}\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>", out, self.notesPaths[n as usize].clone());
            n += 1;
        }
        if !self.notesPaths.is_empty() {
            out = format!("{}<Override PartName=\"/ppt/notesMasters/notesMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml\"/>", out);
            out = format!("{}<Override PartName=\"/ppt/theme/theme2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>", out);
        }
        format!("{}</Types>", out)
    }
    /// The flat writer's blank master, with the deck's own master chrome in it
    /// when the deck has one — otherwise a slide written flat would lose the
    /// banner it inherits.
    pub fn masterWithSheet(&mut self, pres: &Rc<RefCell<PptxPresentation>>) -> String {
        let xml: String = PptxWriter::masterXml();
        let mut mkeys: Vec<String> = pres.borrow().masters.keys().cloned().collect::<Vec<String>>();
        if mkeys.is_empty() {
            return xml;
        }
        let mut ms: Option<Rc<RefCell<PptxSheet>>> = pres.borrow()
            .masters.get(&mkeys[0].clone())
            .cloned();
        if ms.is_none() {
            return xml;
        }
        let mut sheet: Rc<RefCell<PptxSheet>> = ms.clone().unwrap();
        if sheet.borrow().shapes.is_empty() {
            return xml;
        }
        if !self.skipInherited {
            // The flat route already wrote the chrome onto every slide.
            return xml;
        }
        self.sheetWithShapes(&xml, &sheet)
    }
    pub fn layoutWithSheet(&mut self, pres: &Rc<RefCell<PptxPresentation>>) -> String {
        let xml: String = PptxWriter::layoutXml();
        let mut lkeys: Vec<String> = pres.borrow().layouts.keys().cloned().collect::<Vec<String>>();
        if lkeys.is_empty() {
            return xml;
        }
        let mut ls: Option<Rc<RefCell<PptxSheet>>> = pres.borrow()
            .layouts.get(&lkeys[0].clone())
            .cloned();
        if ls.is_none() {
            return xml;
        }
        let mut sheet: Rc<RefCell<PptxSheet>> = ls.clone().unwrap();
        if sheet.borrow().shapes.is_empty() {
            return xml;
        }
        if !self.skipInherited {
            return xml;
        }
        self.sheetWithShapes(&xml, &sheet)
    }
    /// --- the package ---------------------------------------------------------
    pub fn build(&mut self, pres: &Rc<RefCell<PptxPresentation>>) {
        self.resetMedia();
        ZipWriter::create(&self.zip.clone());
        self.skipInherited = false;
        // Slides first: writing them is what discovers the media and the
        // charts, and each slide's relationship part names what that slide
        // turned out to use.
        let mut slide_bodies: Vec<String> = Vec::new();
        let mut slide_rels: Vec<String> = Vec::new();
        // The slide each notes part belongs to. Only slides WITH notes get a
        // part, so the n-th part is not the n-th slide's.
        let mut notes_owners: Vec<String> = Vec::new();
        let mut parts: Vec<String> = Vec::new();
        let mut pi: i64 = 0;
        while pi < (pres.borrow().slides.len() as i64) {
            parts.push(format!("ppt/slides/slide{}.xml", pi + 1));
            pi += 1;
        }
        self.slideParts = parts.clone();
        let mut i: i64 = 0;
        while i < (pres.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = pres.borrow().slides[i as usize].clone();
            slide_bodies.push(self.slideXml(&slide));
            let had_notes: i64 = self.notesPaths.len() as i64;
            self.noteSlide(&slide, format!("ppt/notesSlides/notesSlide{}.xml", i + 1));
            if (self.notesPaths.len() as i64) > had_notes {
                notes_owners.push(format!("ppt/slides/slide{}.xml", i + 1));
            }
            let mut keep: Vec<Rc<RefCell<OpcRelationship>>> = Vec::new();
            slide_rels.push(self.slideRelsXmlFor("../slideLayouts/slideLayout1.xml",
                &keep
            ));
            i += 1;
        }
        self.addXml("_rels/.rels", &PptxWriter::rootRelsXml().clone());
        self.addXml("ppt/presentation.xml", &self.presentationXml(pres));
        self.addXml("ppt/_rels/presentation.xml.rels", &self.presentationRelsXml(pres));
        self.addXml("ppt/theme/theme1.xml", &PptxWriter::themeXml(&pres.borrow().theme).clone());
        self.addXml("ppt/slideMasters/slideMaster1.xml", &self.masterWithSheet(pres));
        self.addXml("ppt/slideMasters/_rels/slideMaster1.xml.rels", &PptxWriter::masterRelsXml().clone());
        self.addXml("ppt/slideLayouts/slideLayout1.xml", &self.layoutWithSheet(pres));
        self.addXml("ppt/slideLayouts/_rels/slideLayout1.xml.rels", &PptxWriter::layoutRelsXml().clone());
        let mut j: i64 = 0;
        while j < (slide_bodies.len() as i64) {
            let nm: String = format!("ppt/slides/slide{}.xml", j + 1);
            self.addXml(&nm, &slide_bodies[j as usize].clone());
            self.addXml(&format!("ppt/slides/_rels/slide{}.xml.rels", j + 1),
                &slide_rels[j as usize].clone()
            );
            j += 1;
        }
        let mut k: i64 = 0;
        while k < (self.media.len() as i64) {
            let mut m: Rc<RefCell<PptxWriteMedia>> = self.media[k as usize].clone();
            let mut _tmp_5087 = m.borrow().path.clone();
            let mut _tmp_5088 = m.borrow().bytes.clone();
            ZipWriter::addFile(&self.zip.clone(), &_tmp_5087, &_tmp_5088);
            k += 1;
        }
        let mut c: i64 = 0;
        while c < (self.charts.len() as i64) {
            let arg_0 = self.chartPaths[c as usize].clone().clone();
            self.addXml(&arg_0, &self.charts[c as usize].clone().clone());
            c += 1;
        }
        let mut dgi: i64 = 0;
        while dgi < (self.diagramPaths.len() as i64) {
            let arg_0 = self.diagramPaths[dgi as usize].clone().clone();
            self.addXml(&arg_0, &self.diagramBodies[dgi as usize].clone().clone());
            dgi += 1;
        }
        let mut n: i64 = 0;
        while n < (self.notesPaths.len() as i64) {
            let npath: String = self.notesPaths[n as usize].clone();
            self.addXml(&npath, &self.notesBodies[n as usize].clone().clone());
            let arg_0 = OpcPackage::relsPathFor(&npath).clone();
            self.addXml(&arg_0, &PptxWriter::notesRelsXml(&notes_owners[n as usize].clone(), "ppt/notesMasters/notesMaster1.xml").clone());
            n += 1;
        }
        if !self.notesPaths.is_empty() {
            self.addXml("ppt/notesMasters/notesMaster1.xml", &PptxWriter::notesMasterXml().clone());
            self.addXml("ppt/notesMasters/_rels/notesMaster1.xml.rels", &PptxWriter::notesMasterRelsXml().clone());
            self.addXml("ppt/theme/theme2.xml", &PptxWriter::themeXml(&pres.borrow().theme).clone());
        }
        self.addXml("[Content_Types].xml", &self.contentTypesXml(pres));
    }
    /// --------------------------------------------------------------------------
    /// Saving the file that was opened
    /// --------------------------------------------------------------------------
    ///
    /// `build` writes a deck from the model alone, which loses everything the
    /// model does not describe — the master, the layouts, notes, SmartArt, the
    /// document properties. This one keeps the package it was read from: every
    /// part is copied through byte for byte, and only the slides that were
    /// TOUCHED are written again, along with the three parts that have to agree
    /// with the slide list (the presentation, its relationships, and the content
    /// types). A deck of forty slides with one edited word changes one part.
    ///
    /// Because the master and the layouts survive, a slide writes only what the
    /// slide itself declares: the resolved chrome would otherwise be baked in on
    /// top of the master that still draws it.
    fn buildOver(&mut self, pres: &Rc<RefCell<PptxPresentation>>, pkg: Rc<RefCell<OpcPackage>>) {
        self.resetMedia();
        ZipWriter::create(&self.zip.clone());
        self.overMode = true;
        self.skipInherited = true;
        // The package is kept, so the parts this writer cannot generate can be
        // read back out of it.
        self.overPkg = pkg.clone();
        self.keepOpaque = true;
        let mut fresh_opaque: HashMap<String,String> = HashMap::new();
        self.opaqueParts = fresh_opaque.clone();
        let mut parts: Vec<String> = OpcPackage::listParts(&pkg);
        let mut p0: i64 = 0;
        while p0 < (parts.len() as i64) {
            self.sourceParts.insert(parts[p0 as usize].clone(), true);
            p0 += 1;
        }
        // --- where each slide will live
        let mut slide_paths: Vec<String> = Vec::new();
        let mut claimed: HashMap<String,bool> = HashMap::new();
        let mut si: i64 = 0;
        while si < (pres.borrow().slides.len() as i64) {
            let mut slide: Rc<RefCell<PptxSlide>> = pres.borrow().slides[si as usize].clone();
            let mut path: String = slide.borrow().partPath.clone();
            if !path.is_empty() {
                let already: Option<bool> = claimed.get(&path).cloned();
                if already.is_some() {
                    path = String::new();
                }
            }
            if path.is_empty() {
                path = self.freePart("ppt/slides/slide", ".xml", &claimed);
            }
            claimed.insert(path.clone(), true);
            slide_paths.push(path.clone());
            si += 1;
        }
        self.slideParts = slide_paths.clone();
        // --- the slides themselves
        let mut written: HashMap<String,bool> = HashMap::new();
        let mut new_parts: Vec<String> = Vec::new();
        let default_layout: String = PptxWriter::firstLayout(&parts);
        let mut sj: i64 = 0;
        while sj < (pres.borrow().slides.len() as i64) {
            let mut slide2: Rc<RefCell<PptxSlide>> = pres.borrow().slides[sj as usize].clone();
            let path2: String = slide_paths[sj as usize].clone();
            let mut is_new: bool = true;
            let known: Option<bool> = self.sourceParts.get(&path2).cloned();
            if known.is_some() {
                is_new = false;
            }
            let mut rewrite: bool = is_new;
            if slide2.borrow().dirty {
                rewrite = true;
            }
            if rewrite {
                self.writingPart = String::new();
                if !is_new {
                    self.writingPart = path2.clone();
                }
                let body: String = self.slideXml(&slide2);
                self.writingPart = String::new();
                let mut notes_path: String = slide2.borrow().notesPart.clone();
                if !slide2.borrow().notesText().is_empty() {
                    if notes_path.is_empty() {
                        let mut taken_notes: HashMap<String,bool> = HashMap::new();
                        let mut ni: i64 = 0;
                        while ni < (self.notesPaths.len() as i64) {
                            taken_notes.insert(self.notesPaths[ni as usize].clone(), true);
                            ni += 1;
                        }
                        notes_path = self.freePart("ppt/notesSlides/notesSlide",
                            ".xml",
                            &taken_notes
                        );
                    }
                    // The model learns where its notes went, so a second save
                    // rewrites that part rather than inventing another one.
                    slide2.borrow_mut().notesPart = notes_path.clone();
                    self.noteSlide(&slide2, notes_path.clone());
                }
                let mut keep: Vec<Rc<RefCell<OpcRelationship>>> = Vec::new();
                let mut layout_target: String = String::new();
                if !is_new {
                    let rels_path: String = OpcPackage::relsPathFor(&path2);
                    let rel_xml: String = OpcPackage::readXml(&pkg, &rels_path);
                    let mut olds: Vec<Rc<RefCell<OpcRelationship>>> = OpcPackage::parseRelationships(&rel_xml);
                    let mut ri: i64 = 0;
                    while ri < (olds.len() as i64) {
                        let mut rel: Rc<RefCell<OpcRelationship>> = olds[ri as usize].clone();
                        // Pictures and charts are named again from what the
                        // slide now holds; everything else the slide pointed at
                        // — its layout, its notes, a hyperlink — is kept.
                        let mut drop: bool = false;
                        if PptxText::index_of(&rel.borrow().r#type, "/image") >= 0 {
                            drop = true;
                        }
                        if PptxText::index_of(&rel.borrow().r#type, "/chart") >= 0 {
                            drop = true;
                        }
                        // The notes are named again from what the slide holds,
                        // and a layout the editor changed is named again too.
                        if PptxText::index_of(&rel.borrow().r#type, "/notesSlide") >= 0 {
                            drop = true;
                        }
                        if PptxText::index_of(&rel.borrow().r#type, "/slideLayout") >= 0 {
                            if !slide2.borrow().layoutPart.is_empty() {
                                drop = true;
                                layout_target = PptxWriter::relTargetFrom("ppt/slides", &slide2.borrow().layoutPart);
                            }
                        }
                        if !drop {
                            keep.push(rel.clone());
                        }
                        ri += 1;
                    }
                }
                if is_new {
                    layout_target = default_layout.clone();
                    if !slide2.borrow().layoutPart.is_empty() {
                        layout_target = PptxWriter::relTargetFrom("ppt/slides", &slide2.borrow().layoutPart);
                    }
                }
                self.addXml(&path2, &body);
                written.insert(path2.clone(), true);
                new_parts.push(path2.clone());
                let rels_out: String = OpcPackage::relsPathFor(&path2);
                self.addXml(&rels_out, &self.slideRelsXmlFor(&layout_target, &keep));
                written.insert(rels_out.clone(), true);
            }
            sj += 1;
        }
        // --- an edited master or layout, rewritten into its own part
        let mut sheet_keys: Vec<String> = pres.borrow()
            .masters.keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut ski: i64 = 0;
        while ski < (sheet_keys.len() as i64) {
            let key: String = sheet_keys[ski as usize].clone();
            let mut ms: Option<Rc<RefCell<PptxSheet>>> = pres.borrow().masters.get(&key).cloned();
            if ms.is_some() {
                let mut sheet: Rc<RefCell<PptxSheet>> = ms.clone().unwrap();
                if sheet.borrow().dirty {
                    let original: String = OpcPackage::readXml(&pkg, &key);
                    if !original.is_empty() {
                        self.addXml(&key, &self.sheetWithShapes(&original, &sheet));
                        written.insert(key.clone(), true);
                    }
                }
            }
            ski += 1;
        }
        let mut layout_keys: Vec<String> = pres.borrow()
            .layouts.keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut lki: i64 = 0;
        while lki < (layout_keys.len() as i64) {
            let lkey: String = layout_keys[lki as usize].clone();
            let mut ls: Option<Rc<RefCell<PptxSheet>>> = pres.borrow().layouts.get(&lkey).cloned();
            if ls.is_some() {
                let mut lsheet: Rc<RefCell<PptxSheet>> = ls.clone().unwrap();
                if lsheet.borrow().dirty {
                    let loriginal: String = OpcPackage::readXml(&pkg, &lkey);
                    if !loriginal.is_empty() {
                        self.addXml(&lkey, &self.sheetWithShapes(&loriginal, &lsheet));
                        written.insert(lkey.clone(), true);
                    }
                }
            }
            lki += 1;
        }
        // --- everything else, byte for byte
        // unused:  let mut kept_overrides: Vec<String> = Vec::new();
        let mut ci: i64 = 0;
        while ci < (parts.len() as i64) {
            let part: String = parts[ci as usize].clone();
            let mut copy_it: bool = true;
            if part == "[Content_Types].xml" {
                copy_it = false;
            }
            if part == "ppt/presentation.xml" {
                copy_it = false;
            }
            if part == "ppt/_rels/presentation.xml.rels" {
                copy_it = false;
            }
            let already2: Option<bool> = written.get(&part).cloned();
            if already2.is_some() {
                copy_it = false;
            }
            // A slide that is no longer in the deck, and its relationships,
            // leave with it.
            if PptxWriter::isSlidePart(&part) {
                let mut still_used: bool = false;
                let mut ui: i64 = 0;
                while ui < (slide_paths.len() as i64) {
                    if slide_paths[ui as usize].clone() == part {
                        still_used = true;
                    }
                    ui += 1;
                }
                if !still_used {
                    copy_it = false;
                }
            }
            if PptxWriter::isSlideRelsPart(&part) {
                let owner: String = PptxWriter::slideOfRels(&part);
                let mut still_used2: bool = false;
                let mut uj: i64 = 0;
                while uj < (slide_paths.len() as i64) {
                    if slide_paths[uj as usize].clone() == owner {
                        still_used2 = true;
                    }
                    uj += 1;
                }
                if !still_used2 {
                    copy_it = false;
                }
            }
            if copy_it {
                let arg_0 = OpcPackage::readBinary(&pkg, &part).clone();
                ZipWriter::addFile(&self.zip.clone(), &part, &arg_0);
                written.insert(part.clone(), true);
            }
            ci += 1;
        }
        // --- media and charts the edit introduced
        let mut mi: i64 = 0;
        while mi < (self.media.len() as i64) {
            let mut m: Rc<RefCell<PptxWriteMedia>> = self.media[mi as usize].clone();
            if !m.borrow().existing {
                let mut _tmp_5099 = m.borrow().path.clone();
                let mut _tmp_5100 = m.borrow().bytes.clone();
                ZipWriter::addFile(&self.zip.clone(), &_tmp_5099, &_tmp_5100);
                written.insert(m.borrow().path.clone(), true);
                new_parts.push(m.borrow().path.clone());
            }
            mi += 1;
        }
        let mut nwi: i64 = 0;
        while nwi < (self.notesPaths.len() as i64) {
            let npath2: String = self.notesPaths[nwi as usize].clone();
            self.addXml(&npath2, &self.notesBodies[nwi as usize].clone().clone());
            written.insert(npath2.clone(), true);
            new_parts.push(npath2.clone());
            let nrels: String = OpcPackage::relsPathFor(&npath2);
            let mut owner_1: String = String::new();
            let mut oi: i64 = 0;
            while oi < (pres.borrow().slides.len() as i64) {
                let mut cand: Rc<RefCell<PptxSlide>> = pres.borrow().slides[oi as usize].clone();
                if cand.borrow().notesPart == npath2 {
                    owner_1 = slide_paths[oi as usize].clone();
                }
                oi += 1;
            }
            if owner_1.is_empty() {
                if nwi < (slide_paths.len() as i64) {
                    owner_1 = slide_paths[nwi as usize].clone();
                }
            }
            self.addXml(&nrels, &PptxWriter::notesRelsXml(&owner_1, &PptxWriter::notesMasterOf(&pkg)).clone());
            written.insert(nrels.clone(), true);
            nwi += 1;
        }
        let mut chi: i64 = 0;
        while chi < (self.charts.len() as i64) {
            let cpath: String = self.chartPaths[chi as usize].clone();
            let seen: Option<bool> = written.get(&cpath).cloned();
            if seen.is_none() {
                self.addXml(&cpath, &self.charts[chi as usize].clone().clone());
                written.insert(cpath.clone(), true);
                new_parts.push(cpath.clone());
            }
            chi += 1;
        }
        let mut dgo: i64 = 0;
        while dgo < (self.diagramPaths.len() as i64) {
            let dgpath: String = self.diagramPaths[dgo as usize].clone();
            self.addXml(&dgpath, &self.diagramBodies[dgo as usize].clone().clone());
            written.insert(dgpath.clone(), true);
            new_parts.push(dgpath.clone());
            dgo += 1;
        }
        // --- the three parts that have to agree with the slide list
        let pres_xml: String = OpcPackage::readXml(&pkg, "ppt/presentation.xml");
        self.addXml("ppt/presentation.xml", &PptxWriter::presentationWithDeck(&pres_xml, pres, &slide_paths).clone());
        let pres_rels_xml: String = OpcPackage::readXml(&pkg, "ppt/_rels/presentation.xml.rels");
        self.addXml("ppt/_rels/presentation.xml.rels", &PptxWriter::presentationRelsWith(&pkg, &pres_rels_xml, &slide_paths).clone());
        // Written, so the content types keep their entries. The presentation's
        // Override used to be dropped here — the main part of the package with
        // no content type, which LibreOffice refuses to open at all and
        // PowerPoint offers to repair.
        written.insert("ppt/presentation.xml".to_string(), true);
        written.insert("ppt/_rels/presentation.xml.rels".to_string(), true);
        let ct_xml: String = OpcPackage::readXml(&pkg, "[Content_Types].xml");
        self.addXml("[Content_Types].xml", &PptxWriter::contentTypesWith(&ct_xml, &written, &new_parts).clone());
    }
    pub fn toBuffer(&mut self, pres: &Rc<RefCell<PptxPresentation>>) -> Vec<u8> {
        self.build(pres);
        return ZipWriter::toBuffer(&self.zip);
    }
    pub fn save(&mut self, pres: &Rc<RefCell<PptxPresentation>>, path: &str, name: &str) {
        self.build(pres);
        ZipWriter::save(&self.zip.clone(), path, name);
    }
    fn saveOver(&mut self, pres: &Rc<RefCell<PptxPresentation>>, pkg: Rc<RefCell<OpcPackage>>, path: &str, name: &str) {
        self.buildOver(pres, pkg.clone());
        ZipWriter::save(&self.zip.clone(), path, name);
    }
    fn toBufferOver(&mut self, pres: &Rc<RefCell<PptxPresentation>>, pkg: Rc<RefCell<OpcPackage>>) -> Vec<u8> {
        self.buildOver(pres, pkg.clone());
        return ZipWriter::toBuffer(&self.zip);
    }
}
