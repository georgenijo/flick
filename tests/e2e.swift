// End-to-end check: presses real ⌥Tab key events and verifies the switch.
// Usage: e2e <flick-pid> <screenshot-path> <start-app-pid>
import AppKit

let args = CommandLine.arguments
let pid = args[1]
let shot = args[2]
let src = CGEventSource(stateID: .hidSystemState)
let optKey: CGKeyCode = 58, tab: CGKeyCode = 48, esc: CGKeyCode = 53

func key(_ code: CGKeyCode, _ down: Bool, _ flags: CGEventFlags) {
    let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down)!
    e.flags = flags
    e.post(tap: .cghidEventTap)
}
func wait(_ ms: Int) { usleep(useconds_t(ms * 1000)) }
func front() -> String {
    // Ask Accessibility live; NSWorkspace caches without a run loop.
    var a: CFTypeRef?
    AXUIElementCopyAttributeValue(AXUIElementCreateSystemWide(), "AXFocusedApplication" as CFString, &a)
    guard let a else { return "?" }
    let ax = a as! AXUIElement
    var pid: pid_t = 0; AXUIElementGetPid(ax, &pid)
    let app = NSRunningApplication(processIdentifier: pid)!
    var w: CFTypeRef?; var t: CFTypeRef?
    AXUIElementCopyAttributeValue(ax, "AXFocusedWindow" as CFString, &w)
    if let w { AXUIElementCopyAttributeValue(w as! AXUIElement, "AXTitle" as CFString, &t) }
    return "\(app.localizedName ?? "?") / \((t as? String)?.prefix(40) ?? "")"
}
func footprint() -> String {
    let p = Process(); p.executableURL = URL(fileURLWithPath: "/usr/bin/footprint"); p.arguments = [pid]
    let out = Pipe(); p.standardOutput = out; try! p.run(); p.waitUntilExit()
    let s = String(data: out.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
    return s.split(separator: "\n").first(where: { $0.contains("Footprint") }).map(String.init) ?? s
}
func open(holdMs: Int) { key(optKey, true, .maskAlternate); wait(30); key(tab, true, .maskAlternate); key(tab, false, .maskAlternate); wait(holdMs) }
func release() { key(optKey, false, []); wait(400) }

NSRunningApplication(processIdentifier: Int32(args[3])!)?.activate()
wait(800)
print("idle:", footprint())
let before = front()

// 1. Hold ⌥Tab: panel shows, then release switches to the next window's app.
open(holdMs: 400)
let ss = Process(); ss.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture"); ss.arguments = ["-x", shot]
try! ss.run(); ss.waitUntilExit()
print("panel open:", footprint())
release()
let after = front()
print("front: \(before) -> \(after)")

// 2. ⌥Tab again should come straight back (MRU order).
open(holdMs: 60); release()
print("front after quick ⌥Tab: \(front()) (expect \(before))")

// 3. Esc cancels: front app unchanged.
let pre = front()
open(holdMs: 300); key(esc, true, .maskAlternate); key(esc, false, .maskAlternate); wait(100); release()
print("after Esc: \(front()) (expect \(pre))")

// 4. Leak check: 30 open/cancel cycles.
for _ in 0..<30 { open(holdMs: 150); key(esc, true, .maskAlternate); key(esc, false, .maskAlternate); wait(30); release() }
wait(500)
print("after 30 cycles:", footprint())
