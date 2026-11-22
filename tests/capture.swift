// Captures the open panel as a PNG (window only, with its shadow) for docs.
// Usage: capture <owner-name> <tile-to-hover> <out.png>
import AppKit

let args = CommandLine.arguments
let owner = args[1], tile = CGFloat(Int(args[2])!), out = args[3]
let src = CGEventSource(stateID: .hidSystemState)
func wait(_ ms: Int) { usleep(useconds_t(ms * 1000)) }
func key(_ code: CGKeyCode, _ down: Bool, _ flags: CGEventFlags) {
    let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down)!
    e.flags = flags; e.post(tap: .cghidEventTap)
}
func panel() -> (CGRect, Int)? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
    for w in list where (w[kCGWindowOwnerName as String] as? String) == owner && (w[kCGWindowLayer as String] as? Int) == 101 {
        let b = w[kCGWindowBounds as String] as! [String: CGFloat]
        return (CGRect(x: b["X"]!, y: b["Y"]!, width: b["Width"]!, height: b["Height"]!), w[kCGWindowNumber as String] as! Int)
    }
    return nil
}
let start = CGEvent(source: nil)!.location
key(58, true, .maskAlternate); wait(30); key(48, true, .maskAlternate); key(48, false, .maskAlternate); wait(400)
if let (b, id) = panel() {
    let p = CGPoint(x: b.minX + 14 + tile * 120 + 60, y: b.minY + 14 + 58)
    CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: p, mouseButton: .left)!.post(tap: .cghidEventTap)
    wait(250)
    // Park the cursor off the panel so it isn't in the shot.
    CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: CGPoint(x: b.minX - 200, y: b.minY - 200), mouseButton: .left)!.post(tap: .cghidEventTap)
    wait(150)
    let s = Process(); s.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture"); _ = id; s.arguments = ["-x", "-R\(Int(b.minX)),\(Int(b.minY)),\(Int(b.width)),\(Int(b.height))", out]
    try! s.run(); s.waitUntilExit()
    print("captured \(out)")
} else { print("panel not found") }
key(53, true, .maskAlternate); key(53, false, .maskAlternate); wait(50); key(58, false, [])
CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: start, mouseButton: .left)!.post(tap: .cghidEventTap)
