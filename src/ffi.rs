//! Raw bindings to the few C APIs we need (Accessibility, CGWindowList,
//! CGEventTap, CoreFoundation). Kept minimal on purpose.
#![allow(non_upper_case_globals, dead_code)]

use std::ffi::c_void;

pub type CFTypeRef = *const c_void;
pub type CGEventRef = *mut c_void;
pub type AXError = i32;

pub const kAXErrorSuccess: AXError = 0;

// CGWindowList
pub const kCGWindowListOptionOnScreenOnly: u32 = 1 << 0;
pub const kCGWindowListExcludeDesktopElements: u32 = 1 << 4;

// CGEventTap
pub const kCGSessionEventTap: u32 = 1;
pub const kCGHeadInsertEventTap: u32 = 0;
pub const kCGEventTapOptionDefault: u32 = 0;
pub const kCGEventKeyDown: u32 = 10;
pub const kCGEventKeyUp: u32 = 11;
pub const kCGEventFlagsChanged: u32 = 12;
pub const kCGEventLeftMouseDown: u32 = 1;
pub const kCGEventLeftMouseUp: u32 = 2;
pub const kCGEventRightMouseDown: u32 = 3;
pub const kCGEventRightMouseUp: u32 = 4;
pub const kCGEventMouseMoved: u32 = 5;
pub const kCGEventOtherMouseDown: u32 = 25;
pub const kCGEventOtherMouseUp: u32 = 26;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}
pub const kCGEventTapDisabledByTimeout: u32 = 0xFFFF_FFFE;
pub const kCGEventTapDisabledByUserInput: u32 = 0xFFFF_FFFF;
pub const kCGKeyboardEventKeycode: u32 = 9;

pub const kCGEventFlagMaskShift: u64 = 0x0002_0000;
pub const kCGEventFlagMaskControl: u64 = 0x0004_0000;
pub const kCGEventFlagMaskAlternate: u64 = 0x0008_0000;
pub const kCGEventFlagMaskCommand: u64 = 0x0010_0000;

pub const kCFNumberSInt64Type: i32 = 4;

pub type TapCallback =
    extern "C" fn(proxy: *mut c_void, ty: u32, event: CGEventRef, info: *mut c_void) -> CGEventRef;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn AXIsProcessTrusted() -> bool;
    pub fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
    pub static kAXTrustedCheckOptionPrompt: CFTypeRef;
    pub fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    pub fn AXUIElementCreateSystemWide() -> CFTypeRef;
    pub fn AXUIElementCopyAttributeValue(
        el: CFTypeRef,
        attr: CFTypeRef,
        out: *mut CFTypeRef,
    ) -> AXError;
    pub fn AXUIElementSetAttributeValue(
        el: CFTypeRef,
        attr: CFTypeRef,
        value: CFTypeRef,
    ) -> AXError;
    pub fn AXUIElementPerformAction(el: CFTypeRef, action: CFTypeRef) -> AXError;
    pub fn AXUIElementSetMessagingTimeout(el: CFTypeRef, seconds: f32) -> AXError;
    /// Private but stable for a decade; used by every window switcher.
    pub fn _AXUIElementGetWindow(el: CFTypeRef, wid: *mut u32) -> AXError;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
    pub static kCGWindowNumber: CFTypeRef;
    pub static kCGWindowOwnerPID: CFTypeRef;
    pub static kCGWindowLayer: CFTypeRef;
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: TapCallback,
        info: *mut c_void,
    ) -> CFTypeRef;
    pub fn CGEventTapEnable(tap: CFTypeRef, enable: bool);
    pub fn CGEventGetFlags(event: CGEventRef) -> u64;
    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    /// Global coordinates, origin at the top-left of the main display.
    pub fn CGEventGetLocation(event: CGEventRef) -> CGPoint;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    pub static kCFBooleanTrue: CFTypeRef;
    pub static kCFBooleanFalse: CFTypeRef;
    pub static kCFRunLoopCommonModes: CFTypeRef;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    pub fn CFGetTypeID(cf: CFTypeRef) -> usize;
    pub fn CFStringGetTypeID() -> usize;
    pub fn CFBooleanGetTypeID() -> usize;
    pub fn CFArrayGetCount(arr: CFTypeRef) -> isize;
    pub fn CFArrayGetValueAtIndex(arr: CFTypeRef, idx: isize) -> CFTypeRef;
    pub fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    pub fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        n: isize,
        key_cb: *const c_void,
        val_cb: *const c_void,
    ) -> CFTypeRef;
    pub static kCFTypeDictionaryKeyCallBacks: c_void;
    pub static kCFTypeDictionaryValueCallBacks: c_void;
    pub fn CFNumberGetValue(num: CFTypeRef, ty: i32, out: *mut c_void) -> bool;
    pub fn CFBooleanGetValue(b: CFTypeRef) -> bool;
    pub fn CFMachPortCreateRunLoopSource(
        alloc: CFTypeRef,
        port: CFTypeRef,
        order: isize,
    ) -> CFTypeRef;
    pub fn CFRunLoopGetMain() -> CFTypeRef;
    pub fn CFRunLoopAddSource(rl: CFTypeRef, source: CFTypeRef, mode: CFTypeRef);
}

/// Owned CF object, released on drop.
pub struct Cf(pub CFTypeRef);
// CF objects we hold (AX elements, arrays) are immutable and thread-safe to retain/release.
unsafe impl Send for Cf {}
impl Drop for Cf {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}

pub fn cf_i64(num: CFTypeRef) -> Option<i64> {
    if num.is_null() {
        return None;
    }
    let mut v: i64 = 0;
    unsafe { CFNumberGetValue(num, kCFNumberSInt64Type, &mut v as *mut i64 as *mut c_void) }
        .then_some(v)
}

// --- Window-server focus (what AltTab uses). Loaded at runtime from the
// private SkyLight framework; if anything is missing we fall back to AX only.

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct ProcessSerialNumber {
    pub high: u32,
    pub low: u32,
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    pub fn GetProcessForPID(pid: i32, psn: *mut ProcessSerialNumber) -> i32;
}

unsafe extern "C" {
    fn dlopen(path: *const std::ffi::c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const std::ffi::c_char) -> *mut c_void;
}

type SetFront = unsafe extern "C" fn(*const ProcessSerialNumber, u32, u32) -> i32;
type PostEvent = unsafe extern "C" fn(*const ProcessSerialNumber, *const u8) -> i32;

pub struct SkyLight {
    set_front: SetFront,
    post_event: PostEvent,
}

pub fn skylight() -> Option<&'static SkyLight> {
    static SL: std::sync::OnceLock<Option<SkyLight>> = std::sync::OnceLock::new();
    SL.get_or_init(|| unsafe {
        let h = dlopen(
            c"/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight".as_ptr(),
            1,
        );
        if h.is_null() {
            return None;
        }
        let a = dlsym(h, c"_SLPSSetFrontProcessWithOptions".as_ptr());
        let b = dlsym(h, c"SLPSPostEventRecordTo".as_ptr());
        if a.is_null() || b.is_null() {
            return None;
        }
        Some(SkyLight {
            set_front: std::mem::transmute::<*mut c_void, SetFront>(a),
            post_event: std::mem::transmute::<*mut c_void, PostEvent>(b),
        })
    })
    .as_ref()
}

impl SkyLight {
    /// Bring `pid` to the front with `wid` as its key window.
    pub fn focus(&self, pid: i32, wid: u32) -> bool {
        unsafe {
            let mut psn = ProcessSerialNumber::default();
            if GetProcessForPID(pid, &mut psn) != 0 {
                return false;
            }
            const USER_GENERATED: u32 = 0x200;
            if (self.set_front)(&psn, wid, USER_GENERATED) != 0 {
                return false;
            }
            // Synthetic "make key window" event records, as the Dock sends.
            let mut bytes = [0u8; 0xf8];
            bytes[0x04] = 0xf8;
            bytes[0x3a] = 0x10;
            bytes[0x3c..0x40].copy_from_slice(&wid.to_ne_bytes());
            bytes[0x20..0x30].fill(0xff);
            bytes[0x08] = 0x01;
            (self.post_event)(&psn, bytes.as_ptr());
            bytes[0x08] = 0x02;
            (self.post_event)(&psn, bytes.as_ptr());
            true
        }
    }
}
