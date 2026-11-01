// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::pptx_model::PptxChart;
use crate::pptx_model::PptxChartSeries;
use crate::rgr::{EVGColor, EVGDisplayList, EVGDrawCmd};


/// ==============================================================================
/// PptxChartVela - PptxChart → Vega-Lite → Vela commands → EVGDrawCmd rects
/// ==============================================================================
///
/// Proof-of-concept only: bar/column charts with categories + one series.
/// EVG still never sees ChartML — only resolved draw commands.
/// ==============================================================================
#[derive(Clone)]
pub struct PptxChartVela {
}

impl PptxChartVela {
    pub fn new() -> Self {
        Self {
        }
    }
    pub fn jsonEscape(s: &str) -> String {
        let mut out: String = String::new();
        let mut from: i64 = 0;
        let mut i: i64 = 0;
        let n: i64 = s.as_bytes().len() as i64;
        while i < n {
            let c: i64 = (s.as_bytes()[i as usize] as i64);
            if c == 34 || c == 92 {
                out = format!("{}{}\\{}", out, s[(from) as usize..(i) as usize].to_string(), char::from_u32(c as u32).unwrap_or('\0'));
                from = i + 1;
            }
            i += 1;
        }
        out = format!("{}{}", out, s[(from) as usize..(n) as usize].to_string());
        out
    }
    pub fn buildVlJson(chart: &Rc<RefCell<PptxChart>>) -> String {
        let mut values: String = String::new();
        let n: i64 = chart.borrow().categories.len() as i64;
        let mut series: Rc<RefCell<PptxChartSeries>> = Rc::new(RefCell::new(PptxChartSeries::new()));
        if !chart.borrow().series.is_empty() {
            series = chart.borrow().series[0].clone();
        }
        let mut i: i64 = 0;
        while i < n {
            if i > 0 {
                values = format!("{},", values);
            }
            let cat: String = chart.borrow().categories[i as usize].clone();
            let mut val: f64 = 0.0;
            if i < (series.borrow().values.len() as i64) {
                val = series.borrow().values[i as usize];
            }
            values = format!("{}{}{}\",\"b\":{}{}", values, "{\"a\":\"".to_string(), PptxChartVela::jsonEscape(&cat), val, "}".to_string());
            i += 1;
        }
        let mut mark: String = "bar";
        let x_field: String = "{\"field\":\"a\",\"type\":\"nominal\",\"sort\":null,\"title\":null}";
        let y_field: String = "{\"field\":\"b\",\"type\":\"quantitative\",\"title\":null}";
        if chart.borrow().chartType == "col" {
            mark = "bar";
        }
        if chart.borrow().chartType == "line" {
            mark = "line";
        }
        let mut title: String = String::new();
        if !chart.borrow().title.is_empty() {
            title = format!(",\"title\":\"{}\"", PptxChartVela::jsonEscape(&chart.borrow().title));
        }
        format!("{}{}{}{}{}{},\"y\":{}{}{}{}", "{\"data\":{\"values\":[".to_string(), values, "]},\"mark\":\"".to_string(), mark, "\",\"encoding\":{\"x\":".to_string(), x_field, y_field, "}".to_string(), title, "}".to_string())
    }
    fn parseHexColor(fill: &str) -> Rc<RefCell<EVGColor>> {
        if (fill.as_bytes().len() as i64) >= 7 {
            if (fill.as_bytes()[0 as usize] as i64) == 35 {
                let hex: String = fill[(1) as usize..(7) as usize].to_string();
                return EVGColor::parse(&format!("#{}", hex));
            }
        }
        EVGColor::rgb(68, 114, 196)
    }
    /// Emit chart into an existing display list inside the given pixel box.
    fn emitToDisplayList(dl: &Rc<RefCell<EVGDisplayList>>, chart: &Rc<RefCell<PptxChart>>, x: f64, y: f64, w: f64, h: f64) {
        if chart.borrow().categories.is_empty() {
            // Fallback empty frame
            let mut c: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
            c.borrow_mut().kind = 0;
            c.borrow_mut().x = x;
            c.borrow_mut().y = y;
            c.borrow_mut().w = w;
            c.borrow_mut().h = h;
            c.borrow_mut().r = 240;
            c.borrow_mut().g = 240;
            c.borrow_mut().b = 245;
            c.borrow_mut().a = 1.0;
            dl.borrow_mut().cmds.push(c.clone());
            return;
        }
        let vl_text: String = PptxChartVela::buildVlJson(chart);
        let mut parser: Rc<RefCell<VlJsonParser>> = Rc::new(RefCell::new(VlJsonParser::new()));
        let mut vl: Rc<RefCell<VlJson>> = VlJsonParser::parse(&parser, vl_text.clone());
        if !parser.borrow().ok {
            return;
        }
        let mut compiler: Rc<RefCell<VlCompile>> = Rc::new(RefCell::new(VlCompile::new()));
        // The outer one: see PptxVegaShapes. The inner `compile` drops the
        // chart's whole configuration on the floor.
        let mut vg: Rc<RefCell<VlJson>> = VlCompile::compileSpec(&compiler, vl.clone());
        let mut rt: Rc<RefCell<VlRuntime>> = Rc::new(RefCell::new(VlRuntime::new()));
        let mut scene: Rc<RefCell<VlSceneMark>> = VlRuntime::run(&rt, vg.clone());
        let mut list: Rc<RefCell<VlCommandList>> = VlSceneCommands::build(&scene);
        let mut view_w: f64 = VlRuntime::viewWidth(&rt);
        let mut view_h: f64 = VlRuntime::viewHeight(&rt);
        if view_w < 1.0 {
            view_w = 400.0;
        }
        if view_h < 1.0 {
            view_h = 300.0;
        }
        let sx: f64 = w / view_w;
        let sy: f64 = h / view_h;
        let pad: f64 = 4.0;
        // Background
        let mut bg: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
        bg.borrow_mut().kind = 0;
        bg.borrow_mut().x = x;
        bg.borrow_mut().y = y;
        bg.borrow_mut().w = w;
        bg.borrow_mut().h = h;
        bg.borrow_mut().r = 250;
        bg.borrow_mut().g = 250;
        bg.borrow_mut().b = 252;
        bg.borrow_mut().a = 1.0;
        dl.borrow_mut().cmds.push(bg.clone());
        let mut i: i64 = 0;
        while i < (list.borrow().commands.len() as i64) {
            let mut cmd: Rc<RefCell<VlDrawCommand>> = list.borrow().commands[i as usize].clone();
            if cmd.borrow().op == "rect" {
                if cmd.borrow().hasFill {
                    let mut col: Rc<RefCell<EVGColor>> = PptxChartVela::parseHexColor(&cmd.borrow().fill.clone());
                    let mut r: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
                    r.borrow_mut().kind = 0;
                    r.borrow_mut().x = (x + pad) + cmd.borrow().x * sx;
                    r.borrow_mut().y = (y + pad) + cmd.borrow().y * sy;
                    r.borrow_mut().w = cmd.borrow().width * sx;
                    r.borrow_mut().h = cmd.borrow().height * sy;
                    if r.borrow().w < 1.0 {
                        r.borrow_mut().w = 1.0;
                    }
                    if r.borrow().h < 1.0 {
                        r.borrow_mut().h = 1.0;
                    }
                    r.borrow_mut().r = EVGColor::red(&col);
                    r.borrow_mut().g = EVGColor::green(&col);
                    r.borrow_mut().b = EVGColor::blue(&col);
                    r.borrow_mut().a = 1.0;
                    dl.borrow_mut().cmds.push(r.clone());
                }
            }
            if cmd.borrow().op == "text" {
                let mut tcol: Rc<RefCell<EVGColor>> = EVGColor::rgb(40, 40, 50);
                if cmd.borrow().hasFill {
                    let _tmp_4158 = PptxChartVela::parseHexColor(&cmd.borrow().fill.clone());
                    tcol = _tmp_4158;
                }
                let mut t: Rc<RefCell<EVGDrawCmd>> = Rc::new(RefCell::new(EVGDrawCmd::new()));
                t.borrow_mut().kind = 3;
                t.borrow_mut().x = (x + pad) + cmd.borrow().x * sx;
                t.borrow_mut().y = (y + pad) + cmd.borrow().y * sy;
                t.borrow_mut().text = cmd.borrow().text.clone();
                t.borrow_mut().fontSize = cmd.borrow().fontSize * sy;
                if t.borrow().fontSize < 9.0 {
                    t.borrow_mut().fontSize = 9.0;
                }
                if t.borrow().fontSize > 18.0 {
                    t.borrow_mut().fontSize = 18.0;
                }
                t.borrow_mut().fontFamily = "Open Sans".to_string();
                t.borrow_mut().r = EVGColor::red(&tcol);
                t.borrow_mut().g = EVGColor::green(&tcol);
                t.borrow_mut().b = EVGColor::blue(&tcol);
                t.borrow_mut().a = 1.0;
                dl.borrow_mut().cmds.push(t.clone());
            }
            i += 1;
        }
    }
}
