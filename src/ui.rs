//! The switcher panel: plain AppKit views, no custom GPU renderer, no thumbnails.
//! Tiles are created when the panel opens and dropped when it closes.

use crate::windows::{self, Win};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::*;
use objc2_foundation::*;

const TILE_W: f64 = 120.0;
const TILE_H: f64 = 116.0;
const ICON: f64 = 72.0;
const PAD: f64 = 14.0;
const CAPTION_H: f64 = 30.0;

pub struct Ui {
    mtm: MainThreadMarker,
    panel: Retained<NSPanel>,
    highlight: Retained<NSView>,
    grid: Retained<NSView>,
    caption: Retained<NSTextField>,
    cols: usize,
    count: usize,
    pub visible: bool,
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}

fn rounded(view: &NSView, radius: f64) {
    view.setWantsLayer(true);
    if let Some(layer) = view.layer() {
        layer.setCornerRadius(radius);
        layer.setMasksToBounds(true);
    }
}

fn label(text: &str, size: f64, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let l = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    l.setFont(Some(&NSFont::systemFontOfSize(size)));
    l.setTextColor(Some(&NSColor::colorWithWhite_alpha(1.0, 0.92)));
    l.setAlignment(NSTextAlignment::Center);
    l.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    l.setMaximumNumberOfLines(1);
    l
}

impl Ui {
    pub fn new(mtm: MainThreadMarker) -> Self {
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            rect(0.0, 0.0, 300.0, 150.0),
            NSWindowStyleMask::NonactivatingPanel | NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            true,
        );
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setLevel(101); // NSPopUpMenuWindowLevel
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(true);
        panel.setIgnoresMouseEvents(true);
        panel.setHidesOnDeactivate(false);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Transient,
        );

        let root = NSVisualEffectView::new(mtm);
        root.setMaterial(NSVisualEffectMaterial::HUDWindow);
        root.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        root.setState(NSVisualEffectState::Active);
        rounded(&root, 18.0);
        panel.setContentView(Some(&root));

        let highlight = NSView::new(mtm);
        rounded(&highlight, 12.0);
        if let Some(layer) = highlight.layer() {
            layer.setBackgroundColor(Some(&NSColor::colorWithWhite_alpha(1.0, 0.16).CGColor()));
        }
        root.addSubview(&highlight);

        let grid = NSView::new(mtm);
        root.addSubview(&grid);

        let caption = label("", 13.0, mtm);
        root.addSubview(&caption);

        Ui {
            mtm,
            panel,
            highlight,
            grid,
            caption,
            cols: 1,
            count: 0,
            visible: false,
        }
    }

    fn screen_frame(&self) -> NSRect {
        let mouse = NSEvent::mouseLocation();
        let screens = NSScreen::screens(self.mtm);
        for s in screens.iter() {
            let f = s.frame();
            if mouse.x >= f.origin.x
                && mouse.x <= f.origin.x + f.size.width
                && mouse.y >= f.origin.y
                && mouse.y <= f.origin.y + f.size.height
            {
                return s.visibleFrame();
            }
        }
        NSScreen::mainScreen(self.mtm)
            .map(|s| s.visibleFrame())
            .unwrap_or(rect(0.0, 0.0, 1440.0, 900.0))
    }

    /// How many tiles fit on the current screen.
    pub fn capacity(&self) -> usize {
        let vis = self.screen_frame();
        let cols = (((vis.size.width * 0.9) - 2.0 * PAD) / TILE_W)
            .floor()
            .max(1.0) as usize;
        let rows = (((vis.size.height * 0.8) - 2.0 * PAD - CAPTION_H) / TILE_H)
            .floor()
            .max(1.0) as usize;
        cols * rows
    }

    pub fn show(&mut self, items: &[Win], sel: usize) {
        if items.is_empty() {
            return;
        }
        let vis = self.screen_frame();
        let n = items.len();
        let max_cols = (((vis.size.width * 0.9) - 2.0 * PAD) / TILE_W)
            .floor()
            .max(1.0) as usize;
        let cols = n.min(max_cols);
        let rows = n.div_ceil(cols);
        self.cols = cols;
        self.count = n;
        let w = cols as f64 * TILE_W + 2.0 * PAD;
        let h = rows as f64 * TILE_H + 2.0 * PAD + CAPTION_H;

        self.grid.setFrame(rect(0.0, 0.0, w, h));
        for (i, win) in items.iter().enumerate() {
            let (x, y) = self.tile_origin(i, h);
            if let Some(img) = windows::icon(win.pid) {
                let iv = NSImageView::imageViewWithImage(&img, self.mtm);
                iv.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
                iv.setFrame(rect(x + (TILE_W - ICON) / 2.0, y + 32.0, ICON, ICON));
                if win.minimized {
                    iv.setAlphaValue(0.55);
                }
                self.grid.addSubview(&iv);
            }
            let l = label(&win.app_name, 11.0, self.mtm);
            l.setFrame(rect(x + 6.0, y + 10.0, TILE_W - 12.0, 16.0));
            self.grid.addSubview(&l);
        }
        self.caption
            .setFrame(rect(PAD, PAD - 2.0, w - 2.0 * PAD, 20.0));

        let origin = NSPoint::new(
            (vis.origin.x + (vis.size.width - w) / 2.0).round(),
            (vis.origin.y + (vis.size.height - h) / 2.0).round(),
        );
        self.panel
            .setFrame_display(NSRect::new(origin, NSSize::new(w, h)), false);
        self.select(items, sel);
        self.panel.orderFrontRegardless();
        self.panel.invalidateShadow();
        self.visible = true;
    }

    fn tile_origin(&self, i: usize, h: f64) -> (f64, f64) {
        let row = i / self.cols;
        let col = i % self.cols;
        (
            PAD + col as f64 * TILE_W,
            h - PAD - (row + 1) as f64 * TILE_H,
        )
    }

    pub fn select(&self, items: &[Win], sel: usize) {
        let Some(win) = items.get(sel) else { return };
        let h = self.grid.frame().size.height;
        let (x, y) = self.tile_origin(sel, h);
        self.highlight
            .setFrame(rect(x + 2.0, y + 2.0, TILE_W - 4.0, TILE_H - 4.0));
        let text = match (win.title.is_empty(), win.minimized) {
            (true, _) => win.app_name.clone(),
            (false, false) => format!("{} — {}", win.title, win.app_name),
            (false, true) => format!("{} — {} (minimized)", win.title, win.app_name),
        };
        self.caption.setStringValue(&NSString::from_str(&text));
    }

    /// Which tile is under a point given in CG (top-left origin) coordinates.
    /// `Err(())` means the point is outside the panel.
    pub fn tile_at(&self, x: f64, y: f64) -> Result<Option<usize>, ()> {
        if !self.visible {
            return Err(());
        }
        let screens = NSScreen::screens(self.mtm);
        let Some(primary) = screens.iter().next() else {
            return Err(());
        };
        let y = primary.frame().size.height - y; // flip to Cocoa coordinates
        let f = self.panel.frame();
        let (lx, ly) = (x - f.origin.x, y - f.origin.y);
        if lx < 0.0 || ly < 0.0 || lx > f.size.width || ly > f.size.height {
            return Err(());
        }
        Ok((0..self.count).find(|&i| {
            let (tx, ty) = self.tile_origin(i, f.size.height);
            lx >= tx && lx < tx + TILE_W && ly >= ty && ly < ty + TILE_H
        }))
    }

    pub fn hide(&mut self) {
        self.panel.orderOut(None);
        // Drop every tile (and its icon reference) until next time.
        self.grid.setSubviews(&NSArray::new());
        self.visible = false;
    }
}
