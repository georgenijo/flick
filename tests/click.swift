// Click test: hold ⌥, open the switcher, click a tile (or outside the panel).
// Usage: click <tile-index | outside> <start-app-pid>
import AppKit

let args = CommandLine.arguments
let target = args[1]
let src = CGEventSource(stateID: .hidSystemState)

func wait(_ ms: Int) { usleep(useconds_t(ms * 1000)) }
func key(_ code: CGKeyCode, _ down: Bool, _ flags: CGEventFlags) {
    let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down)!
    e.flags = flags; e.post(tap: .cghidEventTap)
}
func click(_ p: CGPoint, _ flags: CGEventFlags) {
    CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: p, mouseButton: .left)!.post(tap: .cghidEventTap)
    wait(60)
    for t in [CGEventType.leftMouseDown, .leftMouseUp] {
        let e = CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: p, mouseButton: .left)!
        e.flags = flags; e.post(tap: .cghidEventTap); wait(30)
    }
}
func front() -> String {
    var a: CFTypeRef?
    AXUIElementCopyAttributeValue(AXUIElementCreateSystemWide(), "AXFocusedApplication" as CFString, &a)
    guard let a else { return "?" }
    var pid: pid_t = 0; AXUIElementGetPid(a as! AXUIElement, &pid)
    var w: CFTypeRef?; var t: CFTypeRef?
    AXUIElementCopyAttributeValue(a as! AXUIElement, "AXFocusedWindow" as CFString, &w)
    if let w { AXUIElementCopyAttributeValue(w as! AXUIElement, "AXTitle" as CFString, &t) }
    return "\(NSRunningApplication(processIdentifier: pid)?.localizedName ?? "?") / \((t as? String)?.prefix(40) ?? "")"
}
func panelBounds() -> CGRect? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
    for w in list where (w[kCGWindowOwnerName as String] as? String) == "Flick" && (w[kCGWindowLayer as String] as? Int) == 101 {
        let b = w[kCGWindowBounds as String] as! [String: CGFloat]
        return CGRect(x: b["X"]!, y: b["Y"]!, width: b["Width"]!, height: b["Height"]!)
    }
    return nil
}

NSRunningApplication(processIdentifier: Int32(args[2])!)?.activate()
wait(800)
let mouseStart = CGEvent(source: nil)!.location
print("before:", front())
key(58, true, .maskAlternate); wait(30)
key(48, true, .maskAlternate); key(48, false, .maskAlternate)
wait(400)
guard let b = panelBounds() else { print("FAIL: panel not visible"); key(58, false, []); exit(1) }
let p: CGPoint
if target == "outside" {
    p = CGPoint(x: b.minX - 80, y: b.midY)
} else {
    let i = CGFloat(Int(target)!)
    p = CGPoint(x: b.minX + 14 + i * 120 + 60, y: b.minY + 14 + 58) // tile centre (top row)
}
click(p, .maskAlternate)
wait(300)
print("panel after click:", panelBounds() == nil ? "closed" : "still open")
key(58, false, []); wait(500)
print("after:", front())
CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: mouseStart, mouseButton: .left)!.post(tap: .cghidEventTap)
