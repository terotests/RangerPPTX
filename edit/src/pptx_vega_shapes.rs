// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use pptx::pptx_model::PptxColor;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxShapeKind;
use crate::pptx_shape_ink::PptxShapeInk;
use pptx::rgr::EVGColor;
use crate::rgr::PathCommand;
use pptx::rgr::PathRing;
use pptx::rgr::SVGPathParser;
use pptx::rgr::VlCommandList;
use pptx::rgr::VlCompile;
use pptx::rgr::VlDrawCommand;
use pptx::rgr::VlJson;
use pptx::rgr::VlJsonParser;
use pptx::rgr::VlRuntime;
use pptx::rgr::VlSceneCommands;
use pptx::rgr::VlSceneMark;
use crate::rgr::{PathBuilder, PathCommand};
use pptx::rgr::{EVGColor, PathRing, SVGPathParser};


/// ==============================================================================
/// PptxVegaShapes.rgr — a Vega specification becomes DrawingML, not a picture
/// ==============================================================================
///
/// The usual way to put a chart on a slide is to draw it and paste the pixels in.
/// That is what every "export chart to PowerPoint" path does, and the deck then
/// carries a PNG: a quarter of a megabyte for something that is forty rectangles,
/// blurred at any zoom the author did not anticipate, and dead — nobody can
/// recolour a bar, fix a typo in an axis title or read the numbers out of it
/// again.
///
/// None of that is necessary. Vela already compiles a Vega or Vega-Lite
/// specification down to `VlCommandList`: a flat list of absolute-coordinate
/// drawing commands — rects, lines, text, outlines. And PresentationML's shape
/// tree is also rects, lines, text and outlines. The two meet directly, with no
/// raster step between them, so this file is a translation rather than a
/// renderer. What comes out is a `p:grpSp` of ordinary `p:sp`s that PowerPoint,
/// Keynote and Google Slides treat as shapes they drew themselves.
///
/// WHAT THAT BUYS, MEASURED. A four-bar Vega-Lite chart with both axes is 35
/// shapes: 14.5 KB of slide XML, which deflates to 1.4 KB. Drawn instead as a
/// picture of the same slide it is a 30.7 KB PNG at 1280x720 — twenty times the
/// compressed size, and fixed at that resolution. It stays sharp when the deck
/// is shown on a projector. Every bar is selectable, every label is editable,
/// and the text is in the slide's text — so search, alt text and a screen reader
/// all reach it, which no image of a chart offers.
///
/// One caveat on the first number: the OPC writer stores its parts rather than
/// deflating them, so today a chart adds its full 14.5 KB to the .pptx. The
/// deflated figure is what the same bytes cost in a package written by
/// PowerPoint, and what they will cost here when ZipWriter learns method 8.
///
/// HOW EACH COMMAND IS SPELLED. The choice is always the smallest DrawingML that
/// says the same thing, because the alternative — one `custGeom` per command —
/// is both larger and unhelpful to somebody who then wants to nudge a bar:
///
///   rect, square corners  -> prstGeom "rect"        (a bar, a legend swatch)
///   rect, rounded corners -> prstGeom "roundRect"   (with the adjust value)
///   line, two points      -> prstGeom "line"        (gridlines, ticks, rules)
///   symbol "circle"       -> prstGeom "ellipse"     (a scatter point)
///   text                  -> a text box            (labels, titles, legends)
///   anything else         -> custGeom              (areas, wedges, curves)
///
/// The last row is where the curves go. `VlShape` — the same file the SVG and
/// EVG backends ask for their outlines, so a wedge here is the wedge there —
/// builds the outline, and it is FLATTENED to a polyline at `steps` segments per
/// curve. DrawingML has cubic segments and could carry the curves as curves; the
/// model's path is a point list, and widening it is a change to the parser, the
/// editor and the round trip rather than to this file. Sixteen segments on a
/// chart-sized arc is under a tenth of a point of error, which is below what a
/// projector can show.
///
/// WHAT THIS DOES NOT DO. A ring with a hole in it — an even-odd fill — becomes
/// one shape per ring, so a donut's hole is filled rather than cut. No chart in
/// the Vega-Lite examples produces one; a Vega specification with a hand-written
/// `path` mark can. Gradients, dash patterns and clip paths are dropped rather
/// than approximated: each is a visible lie about what the specification said,
/// and the fill, the stroke and the geometry — which are what the picture is —
/// are carried exactly.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxVegaShapes {
    /// Curve subdivision, and the same budget VlEvgList uses: a chart is small
    /// and its curves are short, so this is fixed rather than derived from the
    /// drawn size.
    pub steps: i64,
    /// What "sans-serif" is called on a slide. Vega names a CSS family and a
    /// deck names a font; writing "sans-serif" into a run gives every reader a
    /// different chart, and picking the theme's minor font gives every DECK a
    /// different chart. A stated name is the only one of the three that is the
    /// same everywhere, and it is settable for a house font.
    pub sansFamily: String,
    /// Below this a label is a smudge at any zoom, and a smudge that is still
    /// selectable is worse than one that is not there.
    pub minFontPt: f64,
    /// How it went. There are no exceptions here for the same reason there are
    /// none in PptxApi: a failure is a value, so a caller can look at it.
    pub ok: bool,
    pub error: String,
    /// What was emitted, for a test that wants to assert a chart drew something
    /// without owning a rasterizer.
    pub shapeCount: i64,
    pub textCount: i64,
    /// What the specification asked to be, before it was fitted to the box.
    pub viewWidth: f64,
    pub viewHeight: f64,
    /// Vega-Lite `config` applied to every specification this object compiles,
    /// as JSON text. Empty until a caller sets one.
    ///
    /// A deck of six charts states the same thing six times otherwise — the
    /// same axis colours, the same absent domain line, the same view with no
    /// border round it — and the interesting half of each specification, which
    /// is its DATA, is buried in it. Vega-Lite already has the place for this
    /// and calls it `config`; all this adds is somewhere to say it once.
    pub defaultConfig: String,
}

impl PptxVegaShapes {
    pub fn new() -> Self {
        Self {
            steps: 16,
            sansFamily: "Calibri".to_string(),
            minFontPt: 5.0,
            ok: false,
            error: String::new(),
            shapeCount: 0,
            textCount: 0,
            viewWidth: 0.0,
            viewHeight: 0.0,
            defaultConfig: String::new(),
        }
    }
    /// `over` wins, `under` fills in. One level of nesting deep, which is the
    /// shape a Vega-Lite config has: blocks of properties, not a tree.
    pub fn underlay(under: &Rc<RefCell<VlJson>>, over: &Rc<RefCell<VlJson>>) -> Rc<RefCell<VlJson>> {
        if !VlJson::isObject(&under) {
            return over;
        }
        if !VlJson::isObject(&over) {
            return under;
        }
        let mut out: Rc<RefCell<VlJson>> = VlJson::objectValue();
        let mut keys: Vec<String> = under.borrow().keys.clone();
        let mut i: i64 = 0;
        while i < (keys.len() as i64) {
            let key: String = keys[i as usize].clone();
            let mut mine: Rc<RefCell<VlJson>> = VlJson::get(&under, &key);
            if VlJson::has(&over, &key) {
                let mut theirs: Rc<RefCell<VlJson>> = VlJson::get(&over, &key);
                let mut both_objects: bool = false;
                if VlJson::isObject(&mine) {
                    both_objects = VlJson::isObject(&theirs);
                }
                if both_objects {
                    VlJson::setMember(&out, &key, PptxVegaShapes::underlay(&mine, &theirs));
                } else {
                    VlJson::setMember(&out, &key, theirs.clone());
                }
            } else {
                VlJson::setMember(&out, &key, mine.clone());
            }
            i += 1;
        }
        let mut their_keys: Vec<String> = over.borrow().keys.clone();
        let mut k: i64 = 0;
        while k < (their_keys.len() as i64) {
            let key2: String = their_keys[k as usize].clone();
            if !VlJson::has(&under, &key2) {
                VlJson::setMember(&out, &key2, VlJson::get(&over, &key2));
            }
            k += 1;
        }
        out
    }
    /// The outline, in the command's own coordinates. Lifted from VlEvgList
    /// deliberately: one of these two drawing a wedge differently would be a bug
    /// nothing catches, so they call the same VlShape.
    fn appendGeometry(builder: &Rc<RefCell<PathBuilder>>, cmd: &Rc<RefCell<VlDrawCommand>>) {
        if cmd.borrow().op == "rect" {
            let mut _tmp_5326 = cmd.borrow().x.clone();
            let mut _tmp_5327 = cmd.borrow().y.clone();
            let mut _tmp_5328 = cmd.borrow().width.clone();
            let mut _tmp_5329 = cmd.borrow().height.clone();
            let mut _tmp_5330 = cmd.borrow().cornerTL.clone();
            let mut _tmp_5331 = cmd.borrow().cornerTR.clone();
            let mut _tmp_5332 = cmd.borrow().cornerBR.clone();
            let mut _tmp_5333 = cmd.borrow().cornerBL.clone();
            VlShape::appendRect(
                builder,
                _tmp_5326,
                _tmp_5327,
                _tmp_5328,
                _tmp_5329,
                _tmp_5330,
                _tmp_5331,
                _tmp_5332,
                _tmp_5333
            );
            return;
        }
        if cmd.borrow().op == "line" {
            let mut _tmp_5334 = cmd.borrow().x.clone();
            let mut _tmp_5335 = cmd.borrow().y.clone();
            let mut _tmp_5336 = cmd.borrow().x2.clone();
            PathBuilder::addLine(&builder, _tmp_5334, _tmp_5335, _tmp_5336, cmd.borrow().y2.clone());
            return;
        }
        if cmd.borrow().op == "symbol" {
            let mut _tmp_5338 = cmd.borrow().shape.clone();
            let mut _tmp_5339 = cmd.borrow().size.clone();
            let mut _tmp_5340 = cmd.borrow().x.clone();
            VlShape::appendSymbol(builder, &_tmp_5338, _tmp_5339, _tmp_5340, cmd.borrow().y.clone());
            return;
        }
        if cmd.borrow().op == "polyline" || cmd.borrow().op == "polygon" {
            let mut _tmp_5342 = cmd.borrow().points.clone();
            let mut _tmp_5343 = cmd.borrow().breaks.clone();
            let mut _tmp_5344 = cmd.borrow().interpolate.clone();
            let mut _tmp_5345 = cmd.borrow().tension.clone();
            VlShape::appendBrokenCurve(
                builder,
                &_tmp_5342,
                &_tmp_5343,
                &_tmp_5344,
                _tmp_5345,
                cmd.borrow().op == "polygon"
            );
            return;
        }
        if cmd.borrow().op == "trail" {
            let mut _tmp_5346 = cmd.borrow().points.clone();
            let mut _tmp_5347 = cmd.borrow().widths.clone();
            VlShape::appendBrokenTrail(builder, &_tmp_5346, &_tmp_5347, &cmd.borrow().breaks.clone());
            return;
        }
        if cmd.borrow().op == "arc" {
            let mut _tmp_5349 = cmd.borrow().x.clone();
            let mut _tmp_5350 = cmd.borrow().y.clone();
            let mut _tmp_5351 = cmd.borrow().startAngle.clone();
            let mut _tmp_5352 = cmd.borrow().endAngle.clone();
            let mut _tmp_5353 = cmd.borrow().innerRadius.clone();
            let mut _tmp_5354 = cmd.borrow().outerRadius.clone();
            VlShape::appendArc(
                builder,
                _tmp_5349,
                _tmp_5350,
                _tmp_5351,
                _tmp_5352,
                _tmp_5353,
                _tmp_5354
            );
            return;
        }
        if cmd.borrow().op == "path" {
            PathBuilder::addPathData(&builder, &cmd.borrow().pathData.clone());
        }
    }
    pub fn largest4(a: f64, b: f64, c: f64, d: f64) -> f64 {
        let mut m: f64 = a;
        if b > m {
            m = b;
        }
        if c > m {
            m = c;
        }
        if d > m {
            m = d;
        }
        m
    }
    /// --- entry -----------------------------------------------------------
    /// Compile `specJson` and lay the result out inside the box, in points.
    ///
    /// The view keeps its aspect ratio and is centred: a specification that says
    /// 200×200 put into a 400×200 box is a square chart in the middle of it,
    /// rather than a square chart stretched into an oblong. Ask for the box you
    /// want the chart to have and let the specification state the size.
    pub fn build(&mut self, spec_json: &str, x: f64, y: f64, w: f64, h: f64) -> Rc<RefCell<PptxShape>> {
        self.ok = false;
        self.error = String::new();
        self.shapeCount = 0;
        self.textCount = 0;
        let mut group: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        group.borrow_mut().kind = PptxShapeKind::group();
        group.borrow_mut().name = "Chart".to_string();
        group.borrow_mut().x = x;
        group.borrow_mut().y = y;
        group.borrow_mut().width = w;
        group.borrow_mut().height = h;
        group.borrow_mut().hasXfrm = true;
        // The child coordinate space is the group's own box, so a child holds
        // slide points and nothing is scaled twice. A group whose chExt differs
        // from its ext scales its children — including, in some readers, their
        // text and in others not, which is exactly the sort of difference that
        // shows up only on the projector.
        group.borrow_mut().chX = x;
        group.borrow_mut().chY = y;
        group.borrow_mut().chW = w;
        group.borrow_mut().chH = h;
        let mut parser: Rc<RefCell<VlJsonParser>> = Rc::new(RefCell::new(VlJsonParser::new()));
        let mut vl: Rc<RefCell<VlJson>> = VlJsonParser::parse(&parser, spec_json.to_string());
        if !parser.borrow().ok {
            self.error = format!("the specification is not JSON: {}", parser.borrow().err);
            return group;
        }
        // The deck's config under the specification's own, key by key and one
        // level down, so `config: {view: {…}}` on a chart replaces the view
        // block and leaves the axis block that came from the deck standing.
        // Replacing the whole `config` instead would make setting one property
        // on one chart silently drop every default it also wanted.
        if !self.defaultConfig.is_empty() {
            let mut base_parser: Rc<RefCell<VlJsonParser>> = Rc::new(RefCell::new(VlJsonParser::new()));
            let mut base: Rc<RefCell<VlJson>> = VlJsonParser::parse(&base_parser, self.defaultConfig.clone());
            if !base_parser.borrow().ok {
                self.error = format!("the chart defaults are not JSON: {}", base_parser.borrow().err);
                return group;
            }
            let mut merged: Rc<RefCell<VlJson>> = PptxVegaShapes::underlay(&base, &VlJson::get(&vl, "config"));
            VlJson::setMember(&vl, "config", merged.clone());
        }
        let mut compiler: Rc<RefCell<VlCompile>> = Rc::new(RefCell::new(VlCompile::new()));
        // `compileSpec`, not `compile`. The inner one is called again for every
        // layer and every pane, so the things that belong to the CHART rather
        // than to a piece of it — the configuration above all — are attached by
        // the outer one. Calling the inner one directly dropped `config`
        // entirely, which is why `view: {stroke: null}` drew a border anyway
        // and every chart in a deck had to repeat on each encoding what it had
        // already said once at the top.
        let mut vg: Rc<RefCell<VlJson>> = VlCompile::compileSpec(&compiler, vl.clone());
        let mut rt: Rc<RefCell<VlRuntime>> = Rc::new(RefCell::new(VlRuntime::new()));
        let mut scene: Rc<RefCell<VlSceneMark>> = VlRuntime::run(&rt, vg.clone());
        let errs: i64 = VlRuntime::errorCount(&rt);
        if errs > 0 {
            self.error = "the specification did not compile".to_string();
            return group;
        }
        let mut list: Rc<RefCell<VlCommandList>> = VlSceneCommands::build(&scene);
        let view_w: f64 = VlRuntime::viewWidth(&rt);
        let view_h: f64 = VlRuntime::viewHeight(&rt);
        self.viewWidth = view_w;
        self.viewHeight = view_h;
        if view_w < 1.0 {
            self.error = "the specification has no width".to_string();
            return group;
        }
        if view_h < 1.0 {
            self.error = "the specification has no height".to_string();
            return group;
        }
        let sx: f64 = w / view_w;
        let sy: f64 = h / view_h;
        let mut scale: f64 = sx;
        if sy < scale {
            scale = sy;
        }
        if scale <= 0.0 {
            self.error = "the box has no room in it".to_string();
            return group;
        }
        // The plot's own origin sits inside the view, with the axis chrome
        // around it at negative coordinates; every command is relative to that
        // origin. See VlRuntime.plotOriginX.
        let origin_x: f64 = VlRuntime::plotOriginX(&rt);
        let origin_y: f64 = VlRuntime::plotOriginY(&rt);
        let ox: f64 = (x + (w - view_w * scale) * 0.5) + origin_x * scale;
        let oy: f64 = (y + (h - view_h * scale) * 0.5) + origin_y * scale;
        let n: i64 = list.borrow().commands.len() as i64;
        let mut i: i64 = 0;
        while i < n {
            let mut cmd: Rc<RefCell<VlDrawCommand>> = list.borrow().commands[i as usize].clone();
            self.emit(&group, &cmd, ox, oy, scale);
            i += 1;
        }
        self.ok = true;
        group
    }
    /// --- one command ------------------------------------------------------
    fn emit(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ox: f64, oy: f64, scale: f64) {
        if cmd.borrow().opacity <= 0.0 {
            return;
        }
        if cmd.borrow().op == "text" {
            self.emitText(group, cmd, ox, oy, scale);
            return;
        }
        // Groups are layout, not ink: the coordinates are already absolute by
        // the time they reach here, so a group marker has nothing to say.
        if cmd.borrow().op == "group-begin" {
            return;
        }
        if cmd.borrow().op == "group-end" {
            return;
        }
        // An image mark is the one thing here that IS a picture, and the point
        // of this file is that a chart is not. Dropped rather than smuggled in.
        if cmd.borrow().op == "image" {
            return;
        }
        let mut want_fill: bool = false;
        if cmd.borrow().hasFill {
            want_fill = PptxShapeInk::paints(&cmd.borrow().fill);
        }
        let mut want_stroke: bool = false;
        if cmd.borrow().hasStroke {
            if cmd.borrow().strokeWidth > 0.0 {
                want_stroke = PptxShapeInk::paints(&cmd.borrow().stroke);
            }
        }
        if !want_fill {
            if !want_stroke {
                return;
            }
        }
        if cmd.borrow().op == "rect" {
            self.emitRect(group, cmd, ox, oy, scale, want_fill, want_stroke);
            return;
        }
        if cmd.borrow().op == "line" {
            self.emitLine(group, cmd, ox, oy, scale);
            return;
        }
        if cmd.borrow().op == "symbol" {
            if cmd.borrow().shape == "circle" {
                self.emitCircle(group, cmd, ox, oy, scale, want_fill, want_stroke);
                return;
            }
        }
        self.emitOutline(group, cmd, ox, oy, scale, want_fill, want_stroke);
    }
    /// A bar. The commonest command in the list by a distance, and the one it is
    /// most worth spelling as a preset: `<a:prstGeom prst="rect"/>` is eleven
    /// bytes larger than nothing and it is a rectangle an author can drag.
    fn emitRect(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ox: f64, oy: f64, scale: f64, want_fill: bool, want_stroke: bool) {
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().x = ox + cmd.borrow().x * scale;
        sh.borrow_mut().y = oy + cmd.borrow().y * scale;
        sh.borrow_mut().width = cmd.borrow().width * scale;
        sh.borrow_mut().height = cmd.borrow().height * scale;
        sh.borrow_mut().preset = "rect".to_string();
        // A stack rounds only the end that is exposed, so the four corners can
        // differ. DrawingML's roundRect has ONE adjust value; the largest is
        // the one a reader would notice going missing.
        let mut _tmp_5365 = cmd.borrow().cornerTL.clone();
        let mut _tmp_5366 = cmd.borrow().cornerTR.clone();
        let mut _tmp_5367 = cmd.borrow().cornerBR.clone();
        let r: f64 = PptxVegaShapes::largest4(_tmp_5365, _tmp_5366, _tmp_5367, cmd.borrow().cornerBL.clone());
        if r > 0.0 {
            sh.borrow_mut().preset = "roundRect".to_string();
            let mut shortest: f64 = sh.borrow().width;
            if sh.borrow().height < shortest {
                shortest = sh.borrow().height;
            }
            if shortest > 0.0 {
                let mut adj: f64 = (r * scale) / shortest;
                if adj > 0.5 {
                    adj = 0.5;
                }
                sh.borrow_mut().adj = adj;
            }
        }
        sh.borrow_mut().name = format!("Bar {}", (group.borrow().children.len() as i64) + 1);
        self.paint(&sh, cmd, want_fill, want_stroke, scale);
        self.attach(group, sh.clone());
    }
    /// A gridline, a tick, an axis rule. `prstGeom "line"` runs from the top
    /// left of the box to the bottom right, so the direction of the segment is
    /// carried by the flips rather than by the geometry — which is how
    /// PowerPoint itself writes a straight connector, and why a zero-height box
    /// is legal here and would not be for a custGeom.
    fn emitLine(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ox: f64, oy: f64, scale: f64) {
        let x1: f64 = ox + cmd.borrow().x * scale;
        let y1: f64 = oy + cmd.borrow().y * scale;
        let x2: f64 = ox + cmd.borrow().x2 * scale;
        let y2: f64 = oy + cmd.borrow().y2 * scale;
        let mut sh: Rc<RefCell<PptxShape>> = PptxShapeInk::segment(x1, y1, x2, y2);
        sh.borrow_mut().name = format!("Line {}", (group.borrow().children.len() as i64) + 1);
        let mut col: Rc<RefCell<EVGColor>> = EVGColor::parse(&cmd.borrow().stroke.clone());
        sh.borrow_mut().line = PptxShapeInk::colorOf(&col, cmd.borrow().strokeOpacity * cmd.borrow().opacity);
        sh.borrow_mut().lineWidthPt = cmd.borrow().strokeWidth * scale;
        if sh.borrow().lineWidthPt < 0.25 {
            sh.borrow_mut().lineWidthPt = 0.25;
        }
        self.attach(group, sh.clone());
    }
    /// A scatter point. `size` is an AREA — VlShape's convention, taken from the
    /// reference — so the circle's diameter is its square root.
    fn emitCircle(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ox: f64, oy: f64, scale: f64, want_fill: bool, want_stroke: bool) {
        let side: f64 = cmd.borrow().size.sqrt();
        let d: f64 = side * scale;
        let mut sh: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        sh.borrow_mut().kind = PptxShapeKind::shape();
        sh.borrow_mut().hasXfrm = true;
        sh.borrow_mut().preset = "ellipse".to_string();
        sh.borrow_mut().x = (ox + cmd.borrow().x * scale) - d * 0.5;
        sh.borrow_mut().y = (oy + cmd.borrow().y * scale) - d * 0.5;
        sh.borrow_mut().width = d;
        sh.borrow_mut().height = d;
        sh.borrow_mut().name = format!("Point {}", (group.borrow().children.len() as i64) + 1);
        self.paint(&sh, cmd, want_fill, want_stroke, scale);
        self.attach(group, sh.clone());
    }
    /// Everything with a curve or a corner in it: an area, a pie wedge, a line
    /// mark, a diamond, a hand-written `path`. The outline comes from VlShape —
    /// the same file the SVG backend asks — and is flattened to rings, one shape
    /// per ring.
    fn emitOutline(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ox: f64, oy: f64, scale: f64, want_fill: bool, want_stroke: bool) {
        let mut builder: Rc<RefCell<PathBuilder>> = Rc::new(RefCell::new(PathBuilder::new()));
        PptxVegaShapes::appendGeometry(&builder, cmd);
        let empty: bool = PathBuilder::isEmpty(&builder);
        if empty {
            return;
        }
        let mut outline: Vec<Rc<RefCell<PathCommand>>> = PathBuilder::getCommands(&builder);
        let mut parser: Rc<RefCell<SVGPathParser>> = SVGPathParser::fromCommands(outline.clone());
        let mut _tmp_5378 = self.steps.clone();
        let mut rings: Vec<Rc<RefCell<PathRing>>> = SVGPathParser::flattenRings(
            &parser,
            _tmp_5378,
            scale,
            0.0,
            0.0,
            scale,
            ox,
            oy
        );
        let rn: i64 = rings.len() as i64;
        let mut i: i64 = 0;
        while i < rn {
            let mut ring: Rc<RefCell<PathRing>> = rings[i as usize].clone();
            self.emitRing(group, cmd, &ring, want_fill, want_stroke, scale);
            i += 1;
        }
    }
    /// One ring becomes one `custGeom`. The path is stated in the shape's own
    /// unit square, which is what `PptxShape.pathUnit` holds and what the writer
    /// multiplies back up by 100000 — so the box has to be the ring's bounding
    /// box, and a flat ring (a horizontal line mark with one datum) has to be
    /// given a box with room in it or every point divides by zero.
    fn emitRing(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ring: &Rc<RefCell<PathRing>>, want_fill: bool, want_stroke: bool, scale: f64) {
        let n: i64 = ring.borrow().pts.len() as i64;
        if n < 4 {
            return;
        }
        // A stroke-only command is a POLYLINE, not an outline — see
        // `PptxShapeInk.ring`, which is where that distinction is written down.
        let mut sh: Rc<RefCell<PptxShape>> = PptxShapeInk::ring(&ring.borrow().pts.clone(), 0, n, want_fill);
        sh.borrow_mut().name = format!("Mark {}", (group.borrow().children.len() as i64) + 1);
        self.paint(&sh, cmd, want_fill, want_stroke, scale);
        self.attach(group, sh.clone());
    }
    /// --- text --------------------------------------------------------------
    /// A label, an axis title, a legend entry. The anchor-to-box translation is
    /// `PptxShapeInk.label`, which both converters share; what is decided here
    /// is only what Vega means by the command's own fields.
    fn emitText(&mut self, group: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, ox: f64, oy: f64, scale: f64) {
        let body: String = cmd.borrow().text.clone();
        if body.is_empty() {
            return;
        }
        let size: f64 = cmd.borrow().fontSize * scale;
        if size < self.minFontPt {
            return;
        }
        let ax: f64 = ox + (cmd.borrow().x + cmd.borrow().dx) * scale;
        let ay: f64 = oy + (cmd.borrow().y + cmd.borrow().dy) * scale;
        let mut col: Rc<RefCell<EVGColor>> = EVGColor::rgb(0, 0, 0);
        if cmd.borrow().hasFill {
            let paints: bool = PptxShapeInk::paints(&cmd.borrow().fill.clone());
            if paints {
                let _tmp_5383 = EVGColor::parse(&cmd.borrow().fill.clone());
                col = _tmp_5383;
            }
        }
        let mut ink: Rc<RefCell<PptxColor>> = PptxShapeInk::colorOf(&col, cmd.borrow().fillOpacity * cmd.borrow().opacity);
        let bold: bool = PptxShapeInk::isBold(&cmd.borrow().fontWeight.clone());
        let mut _tmp_5385 = self.sansFamily.clone();
        let mut _tmp_5386 = cmd.borrow().align.clone();
        let mut _tmp_5387 = cmd.borrow().baseline.clone();
        let mut _tmp_5388 = cmd.borrow().angle.clone();
        let mut sh: Rc<RefCell<PptxShape>> = PptxShapeInk::label(
            &body,
            size,
            &_tmp_5385,
            bold,
            &_tmp_5386,
            &_tmp_5387,
            _tmp_5388,
            ax,
            ay,
            ink.clone()
        );
        sh.borrow_mut().name = format!("Label {}", (group.borrow().children.len() as i64) + 1);
        self.textCount += 1;
        self.attach(group, sh.clone());
    }
    /// --- shared -------------------------------------------------------------
    pub fn attach(&mut self, group: &Rc<RefCell<PptxShape>>, sh: Rc<RefCell<PptxShape>>) {
        group.borrow_mut().children.push(sh.clone());
        self.shapeCount += 1;
    }
    fn paint(&self, sh: &Rc<RefCell<PptxShape>>, cmd: &Rc<RefCell<VlDrawCommand>>, want_fill: bool, want_stroke: bool, scale: f64) {
        if want_fill {
            let mut col: Rc<RefCell<EVGColor>> = EVGColor::parse(&cmd.borrow().fill.clone());
            sh.borrow_mut().fill = PptxShapeInk::colorOf(&col, cmd.borrow().fillOpacity * cmd.borrow().opacity);
        } else {
            sh.borrow_mut().noFill = true;
        }
        if want_stroke {
            let mut scol: Rc<RefCell<EVGColor>> = EVGColor::parse(&cmd.borrow().stroke.clone());
            sh.borrow_mut().line = PptxShapeInk::colorOf(&scol, cmd.borrow().strokeOpacity * cmd.borrow().opacity);
            sh.borrow_mut().lineWidthPt = cmd.borrow().strokeWidth * scale;
            if sh.borrow().lineWidthPt < 0.25 {
                sh.borrow_mut().lineWidthPt = 0.25;
            }
        } else {
            sh.borrow_mut().noLine = true;
        }
    }
}
