//! Flick: a lean ⌥Tab window switcher for macOS.
//!
//! Hold ⌥ and press Tab to cycle windows (⇧ to go back, arrows also work),
//! release ⌥ to switch, Esc to cancel.

#![allow(non_upper_case_globals)]

mod ffi;
mod ui;
mod windows;

use dispatch2::{DispatchQueue, DispatchTime};
use ffi::*;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::*;
use objc2_foundation::*;
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering};
use std::time::Duration;

const KEY_TAB: i64 = 48;
const KEY_RETURN: i64 = 36;
const KEY_ESC: i64 = 53;
const KEY_LEFT: i64 = 123;
const KEY_RIGHT: i64 = 124;
const KEY_DOWN: i64 = 125;
const KEY_UP: i64 = 126;

/// Delay before the panel appears, so a quick ⌥Tab tap switches without a flash.
const SHOW_DELAY: Duration = Duration::from_millis(90);

static OPEN: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// A mouse-down was swallowed, so swallow its matching mouse-up too.
static SWALLOW_MOUSE_UP: AtomicBool = AtomicBool::new(false);
static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

struct State {
    ui: ui::Ui,
    items: Vec<windows::Win>,
    sel: isize,
    /// Window ids, most recently used first. macOS raises every window of an
    /// app when it activates, so z-order alone can't tell which window you
    /// were in before; this list can.
    mru: Vec<u32>,
}

impl State {
    fn touch(&mut self, wid: u32) {
        if wid == 0 {
            return;
        }
        self.mru.retain(|&w| w != wid);
        self.mru.insert(0, wid);
        self.mru.truncate(64);
    }
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Open {
        back: bool,
    },
    Move(isize),
    Commit,
    Cancel,
    Show(u64),
    /// Mouse button pressed at a CG point while the switcher is open.
    Click(f64, f64),
    /// Mouse moved to a CG point while the switcher is open.
    Hover(f64, f64),
}

fn post(a: Action) {
    DispatchQueue::main().exec_async(move || handle(a));
}

fn debug() -> bool {
    std::env::var_os("FLICK_DEBUG").is_some()
}

fn handle(a: Action) {
    if debug() {
        eprintln!("action {:?}", a);
    }
    STATE.with_borrow_mut(|st| {
        let Some(st) = st.as_mut() else { return };
        let len = st.items.len() as isize;
        let idx = |sel: isize| {
            if len == 0 {
                0
            } else {
                sel.rem_euclid(len) as usize
            }
        };
        match a {
            Action::Open { back } => {
                let mut items = windows::list();
                // The front window is the one you're in now, however you got there.
                if let Some(front) = items.first().filter(|w| !w.minimized) {
                    let wid = front.wid;
                    st.touch(wid);
                }
                let mru = &st.mru;
                items.sort_by_key(|w| mru.iter().position(|&m| m == w.wid).unwrap_or(usize::MAX));
                items.truncate(st.ui.capacity());
                st.items = items;
                st.sel = if back { -1 } else { 1 };
                let g = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
                let when = DispatchTime::try_from(SHOW_DELAY).unwrap_or(DispatchTime::NOW);
                let _ = DispatchQueue::main().after(when, move || handle(Action::Show(g)));
            }
            Action::Show(g) => {
                if OPEN.load(Ordering::SeqCst)
                    && g == GENERATION.load(Ordering::SeqCst)
                    && !st.ui.visible
                {
                    st.ui.show(&st.items, idx(st.sel));
                }
            }
            Action::Move(d) => {
                st.sel += d;
                if st.ui.visible {
                    st.ui.select(&st.items, idx(st.sel));
                }
            }
            Action::Commit => {
                st.ui.hide();
                if debug() {
                    for (i, w) in st.items.iter().enumerate() {
                        eprintln!(
                            "  {}{} {} — {}",
                            if i == idx(st.sel) { ">" } else { " " },
                            i,
                            w.app_name,
                            w.title
                        );
                    }
                }
                if len > 0 {
                    let i = idx(st.sel);
                    let wid = st.items[i].wid;
                    windows::focus(&st.items[i]);
                    st.touch(wid);
                }
                st.items.clear();
            }
            Action::Cancel => {
                st.ui.hide();
                st.items.clear();
            }
            Action::Click(x, y) => {
                if !OPEN.load(Ordering::SeqCst) {
                    return;
                }
                match st.ui.tile_at(x, y) {
                    // Clicked a tile: switch to it.
                    Ok(Some(i)) => {
                        OPEN.store(false, Ordering::SeqCst);
                        st.sel = i as isize;
                        post(Action::Commit);
                    }
                    // Clicked the panel between tiles: ignore.
                    Ok(None) => {}
                    // Clicked outside a visible panel: close without switching.
                    Err(()) if st.ui.visible => {
                        OPEN.store(false, Ordering::SeqCst);
                        post(Action::Cancel);
                    }
                    Err(()) => {}
                }
            }
            Action::Hover(x, y) => {
                if let Ok(Some(i)) = st.ui.tile_at(x, y)
                    && idx(st.sel) != i
                {
                    st.sel = i as isize;
                    st.ui.select(&st.items, i);
                }
            }
        }
    });
}

extern "C" fn on_event(
    _proxy: *mut c_void,
    ty: u32,
    event: CGEventRef,
    _info: *mut c_void,
) -> CGEventRef {
    let swallow = std::ptr::null_mut();
    unsafe {
        if ty == kCGEventTapDisabledByTimeout || ty == kCGEventTapDisabledByUserInput {
            let tap = TAP.load(Ordering::SeqCst);
            if !tap.is_null() {
                CGEventTapEnable(tap, true);
            }
            return event;
        }
        let flags = CGEventGetFlags(event);
        let alt = flags & kCGEventFlagMaskAlternate != 0
            && flags & (kCGEventFlagMaskCommand | kCGEventFlagMaskControl) == 0;
        let shift = flags & kCGEventFlagMaskShift != 0;
        let open = OPEN.load(Ordering::SeqCst);
        match ty {
            kCGEventKeyDown => {
                let key = CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode);
                if key == KEY_TAB && alt {
                    if open {
                        post(Action::Move(if shift { -1 } else { 1 }));
                    } else {
                        OPEN.store(true, Ordering::SeqCst);
                        post(Action::Open { back: shift });
                    }
                    return swallow;
                }
                if open {
                    match key {
                        KEY_ESC => {
                            OPEN.store(false, Ordering::SeqCst);
                            post(Action::Cancel);
                        }
                        KEY_RETURN => {
                            OPEN.store(false, Ordering::SeqCst);
                            post(Action::Commit);
                        }
                        KEY_LEFT | KEY_UP => post(Action::Move(-1)),
                        KEY_RIGHT | KEY_DOWN => post(Action::Move(1)),
                        _ => {}
                    }
                    return swallow;
                }
                event
            }
            kCGEventKeyUp => {
                let key = CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode);
                if open || (key == KEY_TAB && alt) {
                    swallow
                } else {
                    event
                }
            }
            kCGEventFlagsChanged => {
                if open && flags & kCGEventFlagMaskAlternate == 0 {
                    OPEN.store(false, Ordering::SeqCst);
                    post(Action::Commit);
                }
                event
            }
            // While the switcher is open no click reaches the windows behind it.
            // Otherwise a click while holding ⌥ is an ⌥-click, which macOS
            // treats as "hide this app" or "minimize all".
            kCGEventLeftMouseDown | kCGEventRightMouseDown | kCGEventOtherMouseDown if open => {
                let p = CGEventGetLocation(event);
                SWALLOW_MOUSE_UP.store(true, Ordering::SeqCst);
                post(Action::Click(p.x, p.y));
                swallow
            }
            kCGEventLeftMouseUp | kCGEventRightMouseUp | kCGEventOtherMouseUp => {
                if SWALLOW_MOUSE_UP.swap(false, Ordering::SeqCst) {
                    swallow
                } else {
                    event
                }
            }
            kCGEventMouseMoved if open => {
                let p = CGEventGetLocation(event);
                post(Action::Hover(p.x, p.y));
                event
            }
            _ => event,
        }
    }
}

fn install_tap() -> bool {
    if debug() {
        eprintln!("installing tap, trusted={}", unsafe {
            AXIsProcessTrusted()
        });
    }
    unsafe {
        let mask = [
            kCGEventKeyDown,
            kCGEventKeyUp,
            kCGEventFlagsChanged,
            kCGEventLeftMouseDown,
            kCGEventLeftMouseUp,
            kCGEventRightMouseDown,
            kCGEventRightMouseUp,
            kCGEventOtherMouseDown,
            kCGEventOtherMouseUp,
            kCGEventMouseMoved,
        ]
        .iter()
        .fold(0u64, |m, t| m | (1u64 << t));
        let tap = CGEventTapCreate(
            kCGSessionEventTap,
            kCGHeadInsertEventTap,
            kCGEventTapOptionDefault,
            mask,
            on_event,
            std::ptr::null_mut(),
        );
        if tap.is_null() {
            return false;
        }
        TAP.store(tap as *mut c_void, Ordering::SeqCst);
        let src = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        CFRunLoopAddSource(CFRunLoopGetMain(), src, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        true
    }
}

/// Retry until the user grants Accessibility in System Settings.
fn install_tap_when_trusted() {
    if unsafe { AXIsProcessTrusted() } && install_tap() {
        return;
    }
    let when = DispatchTime::try_from(Duration::from_secs(1)).unwrap_or(DispatchTime::NOW);
    let _ = DispatchQueue::main().after(when, install_tap_when_trusted);
}

fn prompt_for_trust() {
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let vals = [kCFBooleanTrue];
        let opts = Cf(CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            vals.as_ptr(),
            1,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        ));
        AXIsProcessTrustedWithOptions(opts.0);
    }
}

fn status_item(mtm: MainThreadMarker) -> Retained<NSStatusItem> {
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = item.button(mtm) {
        button.setTitle(&NSString::from_str("⌥⇥"));
    }
    let menu = NSMenu::new(mtm);
    let quit = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str("Quit Flick"),
            Some(sel!(terminate:)),
            &NSString::from_str("q"),
        )
    };
    menu.addItem(&quit);
    item.setMenu(Some(&menu));
    item
}

fn main() {
    let mtm = MainThreadMarker::new().expect("must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    windows::set_global_timeout();

    // `flick --list` prints what Flick would show, for debugging.
    if std::env::args().any(|a| a == "--list") {
        let t = std::time::Instant::now();
        let items = windows::list();
        let took = t.elapsed();
        for w in &items {
            println!(
                "{:>6} {:>7} {}{} — {}",
                w.pid,
                w.wid,
                if w.minimized { "[min] " } else { "" },
                w.app_name,
                w.title
            );
        }
        println!(
            "{} windows in {:.1} ms",
            items.len(),
            took.as_secs_f64() * 1000.0
        );
        return;
    }

    let _status = status_item(mtm);
    STATE.with_borrow_mut(|s| {
        *s = Some(State {
            ui: ui::Ui::new(mtm),
            items: Vec::new(),
            sel: 0,
            mru: Vec::new(),
        })
    });
    if !unsafe { AXIsProcessTrusted() } {
        prompt_for_trust();
    }
    install_tap_when_trusted();
    app.run();
}
