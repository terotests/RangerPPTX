// SPDX-License-Identifier: AGPL-3.0-or-later

use ranger::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use pptx::rgr::EVGElement;
use pptx::rgr::EVGUnit;
use crate::rgr::{EVGLayout, EVGStyleSheet, EVGToolbar, ToolItem, ToolTab};
use pptx::rgr::{EVGElement, EVGUnit};


/// ==============================================================================
/// PptxStripUi — the editor's strip, built the way the showcase builds one
/// ==============================================================================
///
/// The previous attempt at modernising the strip restyled the hand-painted view:
/// a subclass that drew rounded rectangles instead of square ones. It looked
/// better and it was the wrong move, because it left the strip outside
/// everything the gallery's components have — CSS layout, hover and press as
/// states rather than as branches in a painter, a stylesheet a host can replace,
/// and an accessible tree.
///
/// This is the strip as `ToolbarDemo` builds one: a `treefactory` whose tags
/// carry the class AND the role, a stylesheet beside it, `EVGLayout` for the
/// boxes and `EVGDisplayList` for the pixels. `EVGA11yFromTree` reads the roles
/// off the same tree the painter walks, so the strip has an accessible tree for
/// the first time — the hand-painted one published nothing, and a screen reader
/// met a canvas.
///
/// WHAT IT DOES NOT TOUCH, and this is what makes it safe:
///
///   THE MODEL. `EVGToolbar` still owns the items, the tabs, the commands and
///   the hot index. This reads that model and builds boxes from it.
///
///   THE HIT TESTING. `toolbar.hit` and `toolbar.hitTab` answer from the x/y/w/h
///   stored on each item, and those are written back here from what the layout
///   computed. The tree becomes the layout engine and the geometry contract is
///   unchanged, so every click, drag and keyboard path keeps working without
///   knowing anything happened.
///
///   THE ICONS. They are procedural — stacks of rectangles in `EVGToolbarView`,
///   not path data — so they cannot go in a tree yet. They are drawn into the
///   boxes the layout computed, which is the same hybrid the dashboard uses for
///   its chart: the tree says where, something else says what. Turning them into
///   SVG paths the way the dashboard's sidebar icons are is the next step and a
///   separate one.
///
/// The look is `dashboard.css`, to the number: 8px radius, `#f4f4f5` for hover
/// and for the thing you are on, `#27272a` / `#09090b` / `#71717a` for the three
/// text weights.
///
///   npm run pptx:web:test   (the strip is hit-tested by the browser selftests)
/// ==============================================================================
/// The tags, each carrying what it looks like and what it MEANS. One
/// declaration, two readers: the painter walks it for pixels and
/// `EVGA11yFromTree` walks it for the accessible tree.
pub struct PptxStripUi {
    /// The two rows. Named here as well as in the sheet because the app asks how
    /// tall the band is before there is a tree to measure.
    /// THE BAND MUST NOT GROW. Padding and gap are what the strip was short of,
    /// not height: the chrome sits above the slide and every pixel it takes is a
    /// pixel of the page. Taking it from 74 to 80 pushed the thumbnails and the
    /// slide down far enough that four of the browser's click tests missed what
    /// they were aiming at, which is the same thing a reader would have found.
    pub tabsH: i64,
    pub rowH: i64,
    /// The laid-out tree, kept so the app can paint it and read an a11y tree off
    /// it without building it twice.
    root: Option<Rc<RefCell<EVGElement>>>,
    /// ITS OWN SHEET, parsed once. Handing it the chrome's was the mistake that
    /// produced a strip stacked down the left-hand edge: that sheet knows
    /// `.slide-panel` and `.pptx-stage` and has never heard of `.ps-bar`, so
    /// every box fell back to a default block and a row of tabs became a column
    /// of them. A component brings its own stylesheet.
    sheet: Rc<RefCell<EVGStyleSheet>>,
    pub parsed: bool,
}

impl PptxStripUi {
    pub fn new() -> Self {
        Self {
            tabsH: 34,
            rowH: 40,
            root: None,
            sheet: Rc::new(RefCell::new(EVGStyleSheet::new())),
            parsed: false,
        }
    }
    /// `dashboard.css`, applied to a strip. Every number in here is one of that
    /// file's: the radius, the two greys, the three text colours, the 8px gap.
    pub fn css() -> String {
        format!("{} flex-wrap: nowrap; align-items: stretch;{}{} flex-wrap: nowrap; align-items: center; height: 34px;{}{} flex-wrap: nowrap; align-items: center; justify-content: center; border-radius: 8px; padding: 0 10px; height: 26px;{}{}{}{}{} flex-wrap: nowrap; align-items: center; height: 40px;{}{} flex-wrap: nowrap; align-items: center; justify-content: center;{}{}{}{}{}{}{} margin-left: 10px; margin-right: 10px;{}{}", ".ps-bar { display: flex; flex-direction: column;".to_string(), " background-color: #ffffff }".to_string(), " .ps-tabs { display: flex; flex-direction: row;".to_string(), " padding: 0 10px; gap: 4px }".to_string(), " .ps-tab { display: flex; flex-direction: row;".to_string(), " width: fit-content; color: #71717a; font-size: 13px }".to_string(), " .ps-tab:hover { background-color: #f4f4f5 }".to_string(), " .ps-tab-on { background-color: #f4f4f5; color: #09090b;".to_string(), " font-weight: 500 }".to_string(), " .ps-band { display: flex; flex-direction: row;".to_string(), " padding: 0 10px; gap: 4px }".to_string(), " .ps-btn { display: flex; flex-direction: row;".to_string(), " border-radius: 8px; height: 34px; width: 38px }".to_string(), " .ps-btn:hover { background-color: #f4f4f5 }".to_string(), " .ps-btn-on { background-color: #e4e4e7 }".to_string(), " .ps-btn-wide { width: fit-content; padding: 0 10px; gap: 4px }".to_string(), " .ps-label { color: #27272a; font-size: 13px;".to_string(), " width: fit-content }".to_string(), " .ps-sep { width: 1px; height: 24px;".to_string(), " background-color: #e4e4e7 }".to_string(), " .ps-bar-rule { height: 1px; background-color: #e4e4e7 }".to_string())
    }
    /// The layout puts the tree at the origin; the band sits under the window's
    /// top-left corner or wherever the shell put it.
    fn offsetTree(el: &Rc<RefCell<EVGElement>>, dx: f64, dy: f64) {
        let hoist_7 = el.borrow().calculatedX + dx;
        el.borrow_mut().calculatedX = hoist_7;
        let hoist_8 = el.borrow().calculatedY + dy;
        el.borrow_mut().calculatedY = hoist_8;
        let mut i: i64 = 0;
        while i < EVGElement::getChildCount(&el) {
            PptxStripUi::offsetTree(&EVGElement::getChild(&el, i), dx, dy);
            i += 1;
        }
    }
    pub fn heightFor(&self, w: i64) -> i64 {
        self.tabsH + self.rowH
    }
    /// Build the tree from the toolbar's model.
    fn build(&self, bar: &Rc<RefCell<EVGToolbar>>) -> Rc<RefCell<EVGElement>> {
        let mut _tree1: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
        _tree1.borrow_mut().className = "ps-bar".to_string();
        let mut bare: Rc<RefCell<EVGElement>> = _tree1.clone();
        let mut _tree2: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
        _tree2.borrow_mut().className = "ps-tabs".to_string();
        _tree2.borrow_mut().role = "tablist".to_string();
        _tree2.borrow_mut().a11yLabel = "Ribbon".to_string();
        let mut tabs: Rc<RefCell<EVGElement>> = _tree2.clone();
        let mut i: i64 = 0;
        while i < EVGToolbar::tabCount(&bar) {
            let mut t: Rc<RefCell<ToolTab>> = EVGToolbar::tabAt(&bar, i);
            let mut cls: String = "ps-tab";
            if i == bar.borrow().activeTab {
                cls = "ps-tab ps-tab-on";
            }
            let mut _tree3: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
            _tree3.borrow_mut().className = "ps-tab".to_string();
            _tree3.borrow_mut().elementType = 1;
            _tree3.borrow_mut().role = "tab".to_string();
            _tree3.borrow_mut().a11yFocusable = true;
            _tree3.borrow_mut().className = cls.to_string();
            _tree3.borrow_mut().id = format!("ps-tab-{}", i);
            _tree3.borrow_mut().a11yLabel = t.borrow().name.clone();
            _tree3.borrow_mut().textContent = t.borrow().name.clone();
            let mut el: Rc<RefCell<EVGElement>> = _tree3.clone();
            EVGElement::addChild(&tabs, el.clone());
            i += 1;
        }
        EVGElement::addChild(&bare, tabs.clone());
        let mut _tree4: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
        _tree4.borrow_mut().className = "ps-band".to_string();
        _tree4.borrow_mut().role = "toolbar".to_string();
        _tree4.borrow_mut().a11yLabel = "Slide tools".to_string();
        let mut band: Rc<RefCell<EVGElement>> = _tree4.clone();
        let mut j: i64 = 0;
        while j < EVGToolbar::count(&bar) {
            let mut it: Rc<RefCell<ToolItem>> = EVGToolbar::at(&bar, j);
            // ONLY THE PAGE THAT IS SHOWING. The model holds every tab's
            // buttons in one list and `itemOnActiveTab` says which belong to
            // the page in front of you. Building all of them put five tabs'
            // worth of boxes in one row: they wrapped down the window, and a
            // pointer over the status line came back as button thirty.
            // AN ITEM UNDER A DROPDOWN IS NOT ON THE BAND. `menuOwner` names
            // the button it hangs beneath, and it is drawn in the list that
            // opens rather than in the row — the font chooser's faces were
            // standing in the strip beside the chooser itself, each with its
            // own icon, because a row lays out every child it is given.
            if it.borrow().menuOwner >= 0 {
                it.borrow_mut().x = 0;
                it.borrow_mut().y = 0;
                it.borrow_mut().w = 0;
                it.borrow_mut().h = 0;
                j += 1;
                continue;
            }
            if !EVGToolbar::itemOnActiveTab(&bar, j) {
                it.borrow_mut().x = 0;
                it.borrow_mut().y = 0;
                it.borrow_mut().w = 0;
                it.borrow_mut().h = 0;
                j += 1;
                continue;
            }
            if it.borrow().isSeparator {
                let mut _tree5: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
                _tree5.borrow_mut().className = "ps-sep".to_string();
                _tree5.borrow_mut().role = "separator".to_string();
                EVGElement::addChild(&band, _tree5.clone());
            } else {
                let mut cls_1: String = "ps-btn".to_string();
                if it.borrow().pressed {
                    cls_1 = "ps-btn ps-btn-on".to_string();
                }
                // A word beside the picture — Save, Print — or a value, like
                // the font name. Either way the box is as wide as it needs.
                let mut wide: bool = false;
                if it.borrow().textW > 0 {
                    wide = true;
                }
                if it.borrow().showLabel {
                    wide = true;
                }
                if wide {
                    cls_1 = format!("{} ps-btn-wide", cls_1);
                }
                let mut name: String = it.borrow().label.clone();
                if name.is_empty() {
                    name = it.borrow().command.clone();
                }
                let mut _tree6: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
                _tree6.borrow_mut().className = "ps-btn".to_string();
                _tree6.borrow_mut().elementType = 1;
                _tree6.borrow_mut().role = "button".to_string();
                _tree6.borrow_mut().a11yFocusable = true;
                _tree6.borrow_mut().className = cls_1.clone();
                _tree6.borrow_mut().id = format!("ps-btn-{}", j);
                _tree6.borrow_mut().a11yLabel = name.clone();
                let mut b: Rc<RefCell<EVGElement>> = _tree6.clone();
                // THE POINTER, HANDED TO THE STYLESHEET.
                //
                // `.ps-btn:hover` has been in the sheet since this was written
                // and never once fired, because a rule only applies to an
                // element that says it is hovered and nothing here said so. The
                // strip therefore had a hover state in CSS, no hover on screen,
                // and no way to tell the two apart by reading either file.
                //
                // `bar.hot` is the model's answer to "which button is the
                // pointer over", maintained by `EVGToolbar` from the same
                // coordinates the hit test uses. Copying it onto the element is
                // the whole of the wiring: press already comes through as the
                // `ps-btn-on` class, and the two now arrive by the same route.
                if bar.borrow().hot == j {
                    b.borrow_mut().isHovered = true;
                }
                // THE PICTURE IS A HOLE. The icon is drawn into this box
                // afterwards, so the box has to be there and empty — an icon
                // that is a text glyph would be measured as text and centred on
                // a baseline, which is the bug the dashboard's sidebar icons
                // were written up for.
                if it.borrow().icon > 0 {
                    let mut _tree7: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
                    _tree7.borrow_mut().className = "ps-label".to_string();
                    _tree7.borrow_mut().elementType = 1;
                    _tree7.borrow_mut().className = "ps-icon".to_string();
                    let mut slot: Rc<RefCell<EVGElement>> = _tree7.clone();
                    // The same eighteen the view draws at, so the word beside
                    // it starts one gap away from the picture rather than one
                    // gap away from a box the picture rattles around in.
                    slot.borrow_mut().width = Some(EVGUnit::px(18.0));
                    slot.borrow_mut().height = Some(EVGUnit::px(18.0));
                    EVGElement::addChild(&b, slot.clone());
                }
                if wide {
                    let lab: String = it.borrow().label.clone();
                    if !lab.is_empty() {
                        let mut _tree8: Rc<RefCell<EVGElement>> = Rc::new(RefCell::new(EVGElement::new()));
                        _tree8.borrow_mut().className = "ps-label".to_string();
                        _tree8.borrow_mut().elementType = 1;
                        _tree8.borrow_mut().textContent = lab.clone();
                        EVGElement::addChild(&b, _tree8.clone());
                    }
                }
                EVGElement::addChild(&band, b.clone());
            }
            j += 1;
        }
        EVGElement::addChild(&bare, band.clone());
        bare
    }
    /// Lay it out at the band's box, then WRITE THE BOXES BACK onto the model.
    ///
    /// That last step is what keeps every other path working: `toolbar.hit` and
    /// `toolbar.hitTab` answer out of these numbers, and so do the tooltip, the
    /// dropdown's anchor and the icon drawing. The tree is the layout engine
    /// now; the geometry contract it fills in is the one that was always there.
    fn layout(&mut self, bar: &Rc<RefCell<EVGToolbar>>, x0: i64, y0: i64, w: i64) {
        if !self.parsed {
            let arg_0 = PptxStripUi::css().clone();
            EVGStyleSheet::parse(&self.sheet.clone(), &arg_0);
            self.parsed = true;
        }
        let mut el: Rc<RefCell<EVGElement>> = self.build(bar);
        el.borrow_mut().width = Some(EVGUnit::px(w as f64));
        el.borrow_mut().height = Some(EVGUnit::px((self.heightFor(w)) as f64));
        EVGStyleSheet::applyTree(&self.sheet.clone(), &el, "");
        let mut lay: Rc<RefCell<EVGLayout>> = Rc::new(RefCell::new(EVGLayout::new()));
        EVGLayout::setPageSize(&lay, w as f64, (self.heightFor(w)) as f64);
        EVGLayout::layout(&lay, el.clone());
        PptxStripUi::offsetTree(&el, x0 as f64, y0 as f64);
        self.root = Some(el.clone());
        // The tabs, in the order they were built.
        let mut tabs_el: Rc<RefCell<EVGElement>> = EVGElement::getChild(&el, 0);
        let mut i: i64 = 0;
        while i < EVGToolbar::tabCount(&bar) {
            if i < EVGElement::getChildCount(&tabs_el) {
                let mut te: Rc<RefCell<EVGElement>> = EVGElement::getChild(&tabs_el, i);
                let mut t: Rc<RefCell<ToolTab>> = EVGToolbar::tabAt(&bar, i);
                t.borrow_mut().x = te.borrow().calculatedX.floor() as i64;
                t.borrow_mut().y = te.borrow().calculatedY.floor() as i64;
                t.borrow_mut().w = te.borrow().calculatedWidth.floor() as i64;
                t.borrow_mut().h = te.borrow().calculatedHeight.floor() as i64;
            }
            i += 1;
        }
        // The band holds only the active page's boxes, so the walk counts its
        // own way through and the items off the page keep the zeroes `build`
        // gave them.
        let mut band_el: Rc<RefCell<EVGElement>> = EVGElement::getChild(&el, 1);
        let mut slot: i64 = 0;
        let mut j: i64 = 0;
        while j < EVGToolbar::count(&bar) {
            let mut on_band: bool = EVGToolbar::itemOnActiveTab(&bar, j);
            let mut bit: Rc<RefCell<ToolItem>> = EVGToolbar::at(&bar, j);
            if bit.borrow().menuOwner >= 0 {
                on_band = false;
            }
            if on_band {
                if slot < EVGElement::getChildCount(&band_el) {
                    let mut be: Rc<RefCell<EVGElement>> = EVGElement::getChild(&band_el, slot);
                    let mut it: Rc<RefCell<ToolItem>> = EVGToolbar::at(&bar, j);
                    it.borrow_mut().x = be.borrow().calculatedX.floor() as i64;
                    it.borrow_mut().y = be.borrow().calculatedY.floor() as i64;
                    it.borrow_mut().w = be.borrow().calculatedWidth.floor() as i64;
                    it.borrow_mut().h = be.borrow().calculatedHeight.floor() as i64;
                }
                slot += 1;
            }
            j += 1;
        }
    }
}
