// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::pptx_api::PptxShapeRef;
use crate::pptx_api::PptxSlideRef;
use pptx::pptx_model::PptxShape;
use pptx::pptx_model::PptxSlide;
use crate::pptx_vega_shapes::PptxVegaShapes;

/// ==============================================================================
/// PptxChartApi — a Vega specification onto a slide, as shapes
/// ==============================================================================
///
/// The third entry point, and separate from the other two for the same reason
/// they are separate from each other: in Ranger an import is not lazy, so the
/// only way not to carry something is not to name it. `PptxApi` is a ZIP reader,
/// an XML parser and a writer. `PptxRenderApi` adds a rasterizer and fonts. This
/// adds Vela — a Vega and Vega-Lite compiler, its expression language, its
/// scales, its time zones. A build server filling in a template pays for none of
/// it, and a program that wants a chart names this file and gets all three.
///
/// WHAT IT PUTS ON THE SLIDE. Not a picture. `PptxVegaShapes` translates the
/// compiled scene into a group of ordinary DrawingML shapes — `prstGeom` for the
/// bars, the gridlines and the points, `custGeom` for the areas and the wedges,
/// text boxes for the labels — so what lands on the slide is what PowerPoint
/// would have produced if somebody had drawn it there. It scales without
/// blurring, every bar is selectable, every label is editable, and the numbers
/// are in the slide's TEXT, which means search and a screen reader reach them.
/// An exported PNG of the same chart offers none of that and is an order of
/// magnitude larger.
///
/// WHAT IT IS NOT. It is not a chart part: PowerPoint's own charts are a
/// `c:chart` package part with the data behind them, and double-clicking one
/// opens a spreadsheet. This makes shapes, and a reader cannot get the numbers
/// back out of it by clicking. That is the trade — a chart part means carrying
/// ChartML, an embedded workbook and the subset of chart types the format has,
/// and Vega's grammar is wider than that list.
/// ==============================================================================
/// Puts a Vega or Vega-Lite chart on a slide.
///
/// A failure is a value here, exactly as in `PptxApi`: a specification with a
/// typo in it leaves `ok` false and says why in `error`, rather than throwing
/// through a compiled boundary that has no exceptions to throw.
#[derive(Clone)]
pub struct PptxVega {
    pub shapes: Rc<RefCell<PptxVegaShapes>>,
    /// Whether the last `addTo` worked.
    pub ok: bool,
    /// Why it did not, when it did not.
    pub error: String,
    /// How many shapes the last chart became. A test can assert a chart drew
    /// something without owning a rasterizer, and a caller can tell a chart
    /// from an empty box.
    pub shapeCount: i64,
}

impl PptxVega {
    pub fn new() -> Self {
        Self {
            shapes: Rc::new(RefCell::new(PptxVegaShapes::new())),
            ok: false,
            error: String::new(),
            shapeCount: 0,
        }
    }
    /// What "sans-serif" is called on this deck.
    ///
    /// Vega names a CSS family. A slide names a font. Writing "sans-serif" into
    /// a run leaves every reader to pick, which is the one outcome that makes
    /// two people looking at the same deck see different charts. The default is
    /// Calibri; name the deck's own font here.
    pub fn font(&self, family: String) -> Rc<RefCell<PptxVega>> {
        self.shapes.borrow_mut().sansFamily = family.clone();
        self
    }
    /// Vega-Lite `config` for every chart this object draws.
    ///
    /// A deck of six charts otherwise states the same thing six times — the same
    /// axis colours, the same absent domain line, the same view with no border
    /// round it — and the interesting half of each specification, which is its
    /// DATA, ends up buried in the half that never changes. Say it once here and
    /// the specifications carry only what differs.
    ///
    /// A chart's own `config` still wins, key by key and one level down: setting
    /// `view` on one chart replaces the view block and leaves the axis block
    /// from here standing, rather than dropping every default it also wanted.
    pub fn config(&self, json: String) -> Rc<RefCell<PptxVega>> {
        self.shapes.borrow_mut().defaultConfig = json.clone();
        self
    }
    /// How finely a curve is subdivided — an area's top edge, a pie wedge, a
    /// smoothed line. Sixteen segments is under a tenth of a point of error at
    /// chart size; raise it for a chart that fills the slide, lower it for a
    /// sparkline in a table cell.
    pub fn curveSteps(&self, n: i64) -> Rc<RefCell<PptxVega>> {
        if n >= 2 {
            self.shapes.borrow_mut().steps = n;
        }
        self
    }
    /// Compile `spec` and put it on `slide`, inside the box, in points.
    ///
    /// The chart keeps the aspect ratio the specification asked for and is
    /// centred in the box: a 200×200 specification in a 400×200 box is a square
    /// chart in the middle of it rather than a stretched one. State the size in
    /// the specification and the room in the box.
    ///
    /// The handle that comes back is the GROUP. Moving it moves the chart;
    /// ungrouping it in PowerPoint leaves the bars and the labels behind as
    /// shapes, which is the point of not shipping a picture.
    pub fn addTo(&mut self, slide: &Rc<RefCell<PptxSlideRef>>, spec: &str, x: f64, y: f64, w: f64, h: f64) -> Rc<RefCell<PptxShapeRef>> {
        self.ok = false;
        self.error = String::new();
        self.shapeCount = 0;
        let here: bool = slide.borrow().exists();
        if !here {
            self.error = "there is no such slide".to_string();
            return slide.borrow().shapeAt(0 - 1);
        }
        let mut group: Rc<RefCell<PptxShape>> = self.shapes.borrow_mut().build(spec, x, y, w, h);
        self.ok = self.shapes.borrow().ok;
        self.error = self.shapes.borrow().error.clone();
        self.shapeCount = self.shapes.borrow().shapeCount;
        if !self.ok {
            return slide.borrow().shapeAt(0 - 1);
        }
        // A chart nobody can read is a chart nobody can read: the group's alt
        // text is the only thing a screen reader has for a shape tree, and
        // every label inside it is separately readable as text.
        group.borrow_mut().altText = format!("Chart, {} shapes", self.shapeCount);
        let mut sl: Rc<RefCell<PptxSlide>> = slide.borrow().model();
        sl.borrow_mut().shapes.push(group.clone());
        sl.borrow_mut().dirty = true;
        let hoist_4 = sl.borrow().revision + 1;
        sl.borrow_mut().revision = hoist_4;
        let at: i64 = (sl.borrow().shapes.len() as i64) - 1;
        return slide.borrow().shapeAt(at);
    }
}
