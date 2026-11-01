// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaWhole — a whole PowerPoint SmartArt as one file
//! ==============================================================================
//!
//! A diagram in a .pptx is five parts: its data, its layout definition, its
//! quick style, its colour set, and the drawing PowerPoint made of it. One
//! file holds them all as a Flat OPC package — Microsoft's single-XML form of
//! a package (`pkg:package`, namespace
//! http://schemas.microsoft.com/office/2006/xmlPackage), each part a
//! `pkg:part` with its `pkg:contentType` and its XML in `pkg:xmlData`:
//!
//!   <pkg:package xmlns:pkg="http://schemas.microsoft.com/office/2006/xmlPackage">
//!     <pkg:part pkg:name="/ppt/diagrams/data1.xml"
//!               pkg:contentType="application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml">
//!       <pkg:xmlData><dgm:dataModel …>…</dgm:dataModel></pkg:xmlData>
//!     </pkg:part>
//!     … diagramLayout+xml, diagramStyle+xml, diagramColors+xml,
//!       application/vnd.ms-office.drawingml.diagramDrawing+xml
//!   </pkg:package>
//!
//! Parts are found by content type; names do not matter. Only the data part
//! is required.
//!
//! How it is drawn (`layout`):
//!
//!   nothing overridden, a drawing there
//!           as PowerPoint drew it: the drawing's shapes, fitted to the box
//!           with their proportions kept and centred
//!   otherwise
//!           laid out by the engine from the data, with the file's own layout
//!           definition, colour set and quick style — each unless the
//!           Markdown names another (`layout=`, `colors=`, `style=`); a
//!           layout definition the engine refuses gives way to the engine's
//!           own for that id, then to `default`, and the result says so
//!
//! Either way the result carries the file's parts, so a PowerPoint export
//! writes the diagram back with its own layout, style and colours.
//!
//! A file whose root is a `dgm:dataModel` (the data alone) goes straight to
//! the engine, as before.
//! ==============================================================================

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::text::PptxText;

use crate::pptx_model::PptxShape;
use crate::pptx_model::PptxShapeKind;
use crate::pptx_parser::PptxParser;
use crate::rgr::XmlNode;
use crate::rgr::XmlParser;
use crate::smartart::sa_builtins::SaBuiltins;
use crate::smartart::sa_data_reader::SaDataReader;
use crate::smartart::sa_data_reader::SaReadResult;
use crate::smartart::sa_engine::SaEngine;
use crate::smartart::sa_engine::SaOptions;
use crate::smartart::sa_engine::SaResult;
use crate::smartart::sa_layout_def::SaLayoutDef;
use crate::smartart::sa_layout_reader::SaLayoutReadResult;
use crate::smartart::sa_layout_reader::SaLayoutReader;
use crate::smartart::sa_model::SaModel;
use crate::smartart::sa_model::SaPoint;

#[derive(Clone)]
pub struct SaWholeFile {
    pub ok: bool,
    pub error: String,
    pub dataXml: String,
    pub layoutXml: String,
    pub styleXml: String,
    pub colorsXml: String,
    pub drawingXml: String,
}

impl SaWholeFile {
    pub fn new() -> Self {
        Self {
            ok: false,
            error: String::new(),
            dataXml: String::new(),
            layoutXml: String::new(),
            styleXml: String::new(),
            colorsXml: String::new(),
            drawingXml: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct SaWhole {
}

impl SaWhole {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn pkgNs() -> String {
        "http://schemas.microsoft.com/office/2006/xmlPackage".to_string()
    }
    /// Whether this text is a Flat OPC package (from its first bytes).
    pub fn isWhole(head: &str) -> bool {
        PptxText::index_of(&head, &SaWhole::pkgNs()) >= 0
    }
    pub fn read(xml: &str) -> Rc<RefCell<SaWholeFile>> {
        let mut res: Rc<RefCell<SaWholeFile>> = Rc::new(RefCell::new(SaWholeFile::new()));
        let mut parser: Rc<RefCell<XmlParser>> = Rc::new(RefCell::new(XmlParser::new()));
        let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&parser, xml.to_string());
        if root_opt.is_none() {
            res.borrow_mut().error = "not XML: no element could be read".to_string();
            return res;
        }
        let mut root: Rc<RefCell<XmlNode>> = root_opt.clone().unwrap();
        if root.borrow().name != "package" {
            res.borrow_mut().error = format!("the root element is <{}>, not <pkg:package>", root.borrow().name);
            return res;
        }
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&root) {
            let mut part: Rc<RefCell<XmlNode>> = XmlNode::childAt(&root, i);
            if !part.borrow().isText && part.borrow().name == "part" {
                let mut ct: String = XmlNode::attr(&part, "pkg:contentType");
                if ct.is_empty() {
                    ct = XmlNode::attr(&part, "contentType");
                }
                let mut data_opt: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&part, "xmlData");
                if data_opt.is_some() {
                    let body: String = SaWhole::innerXml(xml, &data_opt.clone().unwrap());
                    if PptxText::index_of(&ct, "drawingml.diagramData") >= 0 {
                        res.borrow_mut().dataXml = body.clone();
                    }
                    if PptxText::index_of(&ct, "drawingml.diagramLayout") >= 0 {
                        res.borrow_mut().layoutXml = SaWhole::withNs(&body);
                    }
                    if PptxText::index_of(&ct, "drawingml.diagramStyle") >= 0 {
                        res.borrow_mut().styleXml = SaWhole::withNs(&body);
                    }
                    if PptxText::index_of(&ct, "drawingml.diagramColors") >= 0 {
                        res.borrow_mut().colorsXml = SaWhole::withNs(&body);
                    }
                    if PptxText::index_of(&ct, "drawingml.diagramDrawing") >= 0 {
                        res.borrow_mut().drawingXml = body.clone();
                    }
                }
            }
            i += 1;
        }
        if res.borrow().dataXml.is_empty() {
            res.borrow_mut().error = "the package has no diagram data part (content type …drawingml.diagramData+xml)".to_string();
            return res;
        }
        res.borrow_mut().ok = true;
        res
    }
    /// the first element inside `n`, as it is in the source
    fn innerXml(src: &str, n: &Rc<RefCell<XmlNode>>) -> String {
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&n) {
            let mut c: Rc<RefCell<XmlNode>> = XmlNode::childAt(&n, i);
            if !c.borrow().isText {
                return src[(c.borrow().srcStart) as usize..(c.borrow().srcEnd) as usize].to_string();
            }
            i += 1;
        }
        String::new()
    }
    /// A part lifted out of the package may lean on namespaces the package
    /// declared: its root gets the two a diagram part uses, when it has not.
    pub fn withNs(xml: &str) -> String {
        let lt: i64 = PptxText::index_of(&xml, "<");
        if lt < 0 {
            return xml.to_string();
        }
        let gt: i64 = PptxText::index_of_from(&xml, ">", lt);
        if gt < 0 {
            return xml.to_string();
        }
        let tag: String = xml[(lt) as usize..(gt) as usize].to_string();
        let mut add: String = String::new();
        if PptxText::index_of(&tag, "xmlns:dgm=") < 0 {
            add = format!("{} xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\"", add);
        }
        if PptxText::index_of(&tag, "xmlns:a=") < 0 {
            add = format!("{} xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"", add);
        }
        if add.is_empty() {
            return xml.to_string();
        }
        let mut at: i64 = gt;
        if (xml.as_bytes()[(gt - 1) as usize] as i64) == 47 {
            at = gt - 1;
        }
        format!("{}{}{}", xml[(0) as usize..(at) as usize].to_string(), add, xml[(at) as usize..(xml.as_bytes().len() as i64) as usize].to_string())
    }
    /// --- drawing it ------------------------------------------------------------
    /// A SmartArt file — the data alone or a whole diagram — drawn into a box
    /// of w × h.
    pub fn layout(text: &str, opts: &Rc<RefCell<SaOptions>>, w: f64, h: f64) -> Rc<RefCell<SaResult>> {
        if !SaWhole::isWhole(&text[(0) as usize..(SaWhole::headLen(text)) as usize].to_string()) {
            return SaEngine::layout(text, opts, w, h);
        }
        let mut f: Rc<RefCell<SaWholeFile>> = SaWhole::read(text);
        if !f.borrow().ok {
            let mut bad: Rc<RefCell<SaResult>> = Rc::new(RefCell::new(SaResult::new()));
            bad.borrow_mut().error = f.borrow().error.clone();
            return bad;
        }
        let overridden: bool = !opts.borrow().layout.is_empty() || (!opts.borrow().colors.is_empty() || !opts.borrow().style.is_empty());
        if !f.borrow().drawingXml.is_empty() && !overridden {
            let mut drawn: Rc<RefCell<SaResult>> = SaWhole::fromDrawing(&f, opts, w, h);
            if drawn.borrow().ok {
                return drawn;
            }
        }
        SaWhole::engine(&f, opts, w, h)
    }
    pub fn headLen(text: &str) -> i64 {
        if (text.as_bytes().len() as i64) > 2048 {
            return 2048;
        }
        (text.as_bytes().len() as i64)
    }
    /// laid out by the engine with the file's own definitions
    pub fn engine(f: &Rc<RefCell<SaWholeFile>>, opts: &Rc<RefCell<SaOptions>>, w: f64, h: f64) -> Rc<RefCell<SaResult>> {
        let mut o: Rc<RefCell<SaOptions>> = Rc::new(RefCell::new(SaOptions::new()));
        o.borrow_mut().layout = opts.borrow().layout.clone();
        o.borrow_mut().colors = opts.borrow().colors.clone();
        o.borrow_mut().style = opts.borrow().style.clone();
        o.borrow_mut().measure = Rc::clone(&opts.borrow().measure);
        o.borrow_mut().wrap = opts.borrow().wrap;
        let mut lost_layout: String = String::new();
        if opts.borrow().layout.is_empty() {
            if !f.borrow().layoutXml.is_empty() {
                let mut lr: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&f.borrow().layoutXml.clone());
                if lr.borrow().ok {
                    o.borrow_mut().layoutXml = f.borrow().layoutXml.clone();
                } else {
                    lost_layout = lr.borrow().error.clone();
                }
            }
        }
        if opts.borrow().colors.is_empty() {
            o.borrow_mut().colorsXml = f.borrow().colorsXml.clone();
        }
        if opts.borrow().style.is_empty() {
            o.borrow_mut().styleXml = f.borrow().styleXml.clone();
        }
        let mut r: Rc<RefCell<SaResult>> = SaEngine::layout(&f.borrow().dataXml.clone(), &o, w, h);
        if !lost_layout.is_empty() {
            r.borrow_mut().warnings.push(format!("the file's own layout definition could not be used ({}); the engine's own was used instead", lost_layout));
        }
        if r.borrow().ok {
            SaWhole::keepParts(&r.borrow().shape.clone(), f, opts);
        }
        r
    }
    /// what the file holds, so a PowerPoint export writes its own parts back
    pub fn keepParts(g: &Rc<RefCell<PptxShape>>, f: &Rc<RefCell<SaWholeFile>>, opts: &Rc<RefCell<SaOptions>>) {
        if opts.borrow().layout.is_empty() && !f.borrow().layoutXml.is_empty() {
            let mut lr: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&f.borrow().layoutXml.clone());
            if lr.borrow().ok {
                g.borrow_mut().diagramLayoutXml = f.borrow().layoutXml.clone();
                if !lr.borrow().layout.borrow().uniqueId.is_empty() {
                    g.borrow_mut().diagramLayoutId = lr.borrow().layout.borrow().uniqueId.clone();
                }
            }
        }
        if opts.borrow().colors.is_empty() {
            g.borrow_mut().diagramColorsXml = f.borrow().colorsXml.clone();
        }
        if opts.borrow().style.is_empty() {
            g.borrow_mut().diagramStyleXml = f.borrow().styleXml.clone();
        }
    }
    /// PowerPoint's own drawing, fitted to the box
    pub fn fromDrawing(f: &Rc<RefCell<SaWholeFile>>, opts: &Rc<RefCell<SaOptions>>, w: f64, h: f64) -> Rc<RefCell<SaResult>> {
        let mut res: Rc<RefCell<SaResult>> = Rc::new(RefCell::new(SaResult::new()));
        let mut parser: Rc<RefCell<PptxParser>> = Rc::new(RefCell::new(PptxParser::new()));
        let mut root_opt: Option<Rc<RefCell<XmlNode>>> = XmlParser::parse(&Rc::new(RefCell::new(XmlParser::new())), f.borrow().drawingXml.clone());
        if root_opt.is_none() {
            res.borrow_mut().error = "the drawing part does not read".to_string();
            return res;
        }
        let mut tree_opt: Option<Rc<RefCell<XmlNode>>> = XmlNode::firstChild(&root_opt.clone().unwrap(), "spTree");
        if tree_opt.is_none() {
            res.borrow_mut().error = "the drawing part has no shape tree".to_string();
            return res;
        }
        let mut read: Rc<RefCell<SaReadResult>> = SaDataReader::read(&f.borrow().dataXml.clone());
        if !read.borrow().ok {
            res.borrow_mut().error = read.borrow().error.clone();
            return res;
        }
        let mut model: Rc<RefCell<SaModel>> = read.borrow().model.clone().clone();
        let mut tree: Rc<RefCell<XmlNode>> = tree_opt.clone().unwrap();
        let mut g: Rc<RefCell<PptxShape>> = Rc::new(RefCell::new(PptxShape::new()));
        g.borrow_mut().kind = PptxShapeKind::group();
        g.borrow_mut().isDiagram = true;
        g.borrow_mut().name = "Diagram".to_string();
        let mut i: i64 = 0;
        while i < XmlNode::childCount(&tree) {
            let mut child: Rc<RefCell<XmlNode>> = XmlNode::childAt(&tree, i);
            if !child.borrow().isText {
                if child.borrow().name != "nvGrpSpPr" && child.borrow().name != "grpSpPr" {
                    let mut parsed: Option<Rc<RefCell<PptxShape>>> = parser.borrow().parseTreeChild(&child, "");
                    if parsed.is_some() {
                        let mut ps: Rc<RefCell<PptxShape>> = parsed.clone().unwrap();
                        let _tmp_4774 = PptxParser::paraPtsOf(
                            &model,
                            &XmlNode::attr(&child, "modelId"),
                            ps.borrow().text.borrow().paragraphs.len() as i64
                        );
                        ps.borrow_mut().diagramParaPts = _tmp_4774;
                        g.borrow_mut().children.push(ps.clone());
                    }
                }
            }
            i += 1;
        }
        if g.borrow().children.is_empty() {
            res.borrow_mut().error = "the drawing part has no shapes".to_string();
            return res;
        }
        // the shapes' bounds, widened to the box's proportions, centred
        let mut c0: Rc<RefCell<PptxShape>> = g.borrow().children[0].clone();
        let mut min_x: f64 = c0.borrow().x;
        let mut min_y: f64 = c0.borrow().y;
        let mut max_x: f64 = c0.borrow().x + c0.borrow().width;
        let mut max_y: f64 = c0.borrow().y + c0.borrow().height;
        i = 1;
        while i < (g.borrow().children.len() as i64) {
            let mut c: Rc<RefCell<PptxShape>> = g.borrow().children[i as usize].clone();
            if c.borrow().x < min_x {
                min_x = c.borrow().x;
            }
            if c.borrow().y < min_y {
                min_y = c.borrow().y;
            }
            if c.borrow().x + c.borrow().width > max_x {
                max_x = c.borrow().x + c.borrow().width;
            }
            if c.borrow().y + c.borrow().height > max_y {
                max_y = c.borrow().y + c.borrow().height;
            }
            i += 1;
        }
        let bw: f64 = max_x - min_x;
        let bh: f64 = max_y - min_y;
        if bw <= 0.0 || (bh <= 0.0 || (w <= 0.0 || h <= 0.0)) {
            res.borrow_mut().error = "the drawing has no size".to_string();
            return res;
        }
        let mut cw: f64 = bw;
        let mut chh: f64 = bh;
        if w / h > bw / bh {
            cw = bh * (w / h);
        } else {
            chh = bw * (h / w);
        }
        g.borrow_mut().chX = min_x - (cw - bw) / 2.0;
        g.borrow_mut().chY = min_y - (chh - bh) / 2.0;
        g.borrow_mut().chW = cw;
        g.borrow_mut().chH = chh;
        g.borrow_mut().x = 0.0;
        g.borrow_mut().y = 0.0;
        g.borrow_mut().width = w;
        g.borrow_mut().height = h;
        g.borrow_mut().hasXfrm = true;
        g.borrow_mut().hasOff = true;
        g.borrow_mut().hasExt = true;
        // what it was made from
        g.borrow_mut().diagramText = model.borrow().outline();
        g.borrow_mut().diagramModelXml = f.borrow().dataXml.clone();
        let mut d0: Rc<RefCell<SaPoint>> = model.borrow().doc();
        g.borrow_mut().diagramLayoutId = d0.borrow().layoutId.clone();
        g.borrow_mut().diagramLayoutXml = f.borrow().layoutXml.clone();
        if !f.borrow().layoutXml.is_empty() {
            let mut lr: Rc<RefCell<SaLayoutReadResult>> = SaLayoutReader::read(&f.borrow().layoutXml.clone());
            if !lr.borrow().ok {
                let _tmp_4781 = SaBuiltins::layoutXml(&SaLayoutDef::shortOf(&d0.borrow().layoutId.clone()));
                g.borrow_mut().diagramLayoutXml = _tmp_4781;
            }
        } else {
            let _tmp_4783 = SaBuiltins::layoutXml(&SaLayoutDef::shortOf(&d0.borrow().layoutId.clone()));
            g.borrow_mut().diagramLayoutXml = _tmp_4783;
        }
        g.borrow_mut().diagramStyleId = d0.borrow().styleId.clone();
        g.borrow_mut().diagramColorsId = d0.borrow().colorsId.clone();
        g.borrow_mut().diagramStyleXml = f.borrow().styleXml.clone();
        g.borrow_mut().diagramColorsXml = f.borrow().colorsXml.clone();
        let hoist_26 = g.borrow().diagramSignature();
        g.borrow_mut().diagramSig = hoist_26;
        res.borrow_mut().shape = g.clone();
        let _tmp_4790 = SaLayoutDef::shortOf(&d0.borrow().layoutId.clone());
        res.borrow_mut().layoutUsed = _tmp_4790;
        res.borrow_mut().ok = true;
        res
    }
}
