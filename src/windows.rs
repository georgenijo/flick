//! Window discovery and focusing via the Accessibility API.
//! No screenshots: we never touch Screen Recording.

use crate::ffi::*;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplicationActivationOptions, NSApplicationActivationPolicy, NSRunningApplication,
    NSWorkspace,
};
use objc2_foundation::NSString;
use std::collections::HashMap;

pub struct Win {
    pub pid: i32,
    pub wid: u32,
    pub title: String,
    pub app_name: String,
    pub minimized: bool,
    ax: Cf,
}

fn s(x: &str) -> Retained<NSString> {
    NSString::from_str(x)
}

fn attr(el: CFTypeRef, name: &str) -> Option<Cf> {
    let key = s(name);
    let mut out: CFTypeRef = std::ptr::null();
    let err = unsafe { AXUIElementCopyAttributeValue(el, Retained::as_ptr(&key).cast(), &mut out) };
    (err == kAXErrorSuccess && !out.is_null()).then(|| Cf(out))
}

fn attr_string(el: CFTypeRef, name: &str) -> Option<String> {
    let v = attr(el, name)?;
    unsafe {
        if CFGetTypeID(v.0) != CFStringGetTypeID() {
            return None;
        }
        Some((*(v.0 as *const NSString)).to_string())
    }
}

fn attr_bool(el: CFTypeRef, name: &str) -> bool {
    match attr(el, name) {
        Some(v) => unsafe { CFGetTypeID(v.0) == CFBooleanGetTypeID() && CFBooleanGetValue(v.0) },
        None => false,
    }
}

fn set_bool(el: CFTypeRef, name: &str, value: bool) -> AXError {
    let key = s(name);
    unsafe {
        AXUIElementSetAttributeValue(
            el,
            Retained::as_ptr(&key).cast(),
            if value {
                kCFBooleanTrue
            } else {
                kCFBooleanFalse
            },
        )
    }
}

fn perform(el: CFTypeRef, action: &str) -> AXError {
    let a = s(action);
    unsafe { AXUIElementPerformAction(el, Retained::as_ptr(&a).cast()) }
}

/// Front-to-back order of on-screen windows: wid -> z rank.
fn z_order() -> HashMap<u32, usize> {
    let mut map = HashMap::new();
    unsafe {
        let list = Cf(CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
            0,
        ));
        if list.0.is_null() {
            return map;
        }
        for i in 0..CFArrayGetCount(list.0) {
            let d = CFArrayGetValueAtIndex(list.0, i);
            if cf_i64(CFDictionaryGetValue(d, kCGWindowLayer)) != Some(0) {
                continue;
            }
            if let Some(n) = cf_i64(CFDictionaryGetValue(d, kCGWindowNumber)) {
                let rank = map.len();
                map.entry(n as u32).or_insert(rank);
            }
        }
    }
    map
}

pub fn set_global_timeout() {
    unsafe {
        let sys = Cf(AXUIElementCreateSystemWide());
        // Never let one hung app stall the switcher.
        AXUIElementSetMessagingTimeout(sys.0, 0.25);
    }
}

/// Windows of one app. Runs on a worker thread; AX calls are thread-safe.
fn app_windows(pid: i32, app_name: String) -> Vec<Win> {
    let mut out = Vec::new();
    let ax_app = Cf(unsafe { AXUIElementCreateApplication(pid) });
    unsafe { AXUIElementSetMessagingTimeout(ax_app.0, 0.25) };
    let Some(wins) = attr(ax_app.0, "AXWindows") else {
        return out;
    };
    for i in 0..unsafe { CFArrayGetCount(wins.0) } {
        let w = unsafe { CFArrayGetValueAtIndex(wins.0, i) };
        if attr_string(w, "AXSubrole").as_deref() != Some("AXStandardWindow") {
            continue;
        }
        let mut wid = 0u32;
        unsafe { _AXUIElementGetWindow(w, &mut wid) };
        out.push(Win {
            pid,
            wid,
            title: attr_string(w, "AXTitle").unwrap_or_default(),
            app_name: app_name.clone(),
            minimized: attr_bool(w, "AXMinimized"),
            ax: Cf(unsafe { CFRetain(w) }),
        });
    }
    out
}

/// All switchable windows, most recently used first.
/// Apps are queried in parallel so one slow app doesn't add to the others.
pub fn list() -> Vec<Win> {
    let me = std::process::id() as i32;
    let apps: Vec<(i32, String)> = NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter(|a| a.activationPolicy() == NSApplicationActivationPolicy::Regular)
        .map(|a| {
            (
                a.processIdentifier(),
                a.localizedName().map(|n| n.to_string()).unwrap_or_default(),
            )
        })
        .filter(|(pid, _)| *pid != me)
        .collect();
    let z = z_order();
    let mut out: Vec<Win> = std::thread::scope(|s| {
        let handles: Vec<_> = apps
            .into_iter()
            .map(|(pid, name)| s.spawn(move || app_windows(pid, name)))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    // On-screen windows in z order, then the rest (minimized / hidden apps).
    out.sort_by_key(|w| (z.get(&w.wid).copied().unwrap_or(usize::MAX), w.minimized));
    out
}

pub fn focus(w: &Win) {
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(w.pid) else {
        return;
    };
    if app.isHidden() {
        app.unhide();
    }
    if w.minimized {
        set_bool(w.ax.0, "AXMinimized", false);
    }
    let via_ws = w.wid != 0
        && std::env::var_os("FLICK_NO_WS").is_none()
        && skylight().is_some_and(|sl| sl.focus(w.pid, w.wid));
    if !via_ws {
        let ax_app = Cf(unsafe { AXUIElementCreateApplication(w.pid) });
        set_bool(ax_app.0, "AXFrontmost", true);
        #[allow(deprecated)]
        app.activateWithOptions(NSApplicationActivationOptions::ActivateIgnoringOtherApps);
    }
    set_bool(w.ax.0, "AXMain", true);
    let raised = perform(w.ax.0, "AXRaise");
    if std::env::var_os("FLICK_DEBUG").is_some() {
        eprintln!("focus {} window-server={via_ws} raise={raised}", w.app_name);
    }
}

pub fn icon(pid: i32) -> Option<Retained<objc2_app_kit::NSImage>> {
    NSRunningApplication::runningApplicationWithProcessIdentifier(pid)?.icon()
}
