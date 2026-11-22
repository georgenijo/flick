// Draws Flick's app icon and builds assets/Flick.icns.
// Run from the repo root:  swift assets/make-icon.swift
import AppKit

let S: CGFloat = 1024

func color(_ hex: UInt32, _ a: CGFloat = 1) -> CGColor {
    CGColor(srgbRed: CGFloat((hex >> 16) & 0xff) / 255, green: CGFloat((hex >> 8) & 0xff) / 255,
            blue: CGFloat(hex & 0xff) / 255, alpha: a)
}

func draw(_ ctx: CGContext) {
    let space = CGColorSpace(name: CGColorSpace.sRGB)!

    // macOS icon grid: 824pt rounded square centred in 1024, with a soft drop shadow.
    let body = CGRect(x: 100, y: 100, width: 824, height: 824)
    let bodyPath = CGPath(roundedRect: body, cornerWidth: 185, cornerHeight: 185, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -12), blur: 28, color: color(0x000000, 0.35))
    ctx.addPath(bodyPath); ctx.setFillColor(color(0x1a1033)); ctx.fillPath()
    ctx.restoreGState()

    ctx.saveGState()
    ctx.addPath(bodyPath); ctx.clip()
    let bg = CGGradient(colorsSpace: space, colors: [color(0x2b1a6b), color(0x0f0a24)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(bg, start: CGPoint(x: 200, y: 924), end: CGPoint(x: 824, y: 100), options: [])
    // Soft violet glow behind the cards.
    let glow = CGGradient(colorsSpace: space, colors: [color(0x8b5cf6, 0.55), color(0x8b5cf6, 0)] as CFArray, locations: [0, 1])!
    ctx.drawRadialGradient(glow, startCenter: CGPoint(x: 520, y: 500), startRadius: 0,
                           endCenter: CGPoint(x: 520, y: 500), endRadius: 420, options: [])

    // Motion streaks: the flick.
    ctx.setLineCap(.round)
    for (i, y) in [CGFloat(600), 512, 424].enumerated() {
        let len: CGFloat = [95, 130, 95][i]
        let g = CGGradient(colorsSpace: space, colors: [color(0xc4b5fd, 0), color(0xc4b5fd, 0.7)] as CFArray, locations: [0, 1])!
        ctx.saveGState()
        let r = CGRect(x: 145 + (130 - len), y: y - 9, width: len, height: 18)
        ctx.addPath(CGPath(roundedRect: r, cornerWidth: 9, cornerHeight: 9, transform: nil)); ctx.clip()
        ctx.drawLinearGradient(g, start: CGPoint(x: r.minX, y: y), end: CGPoint(x: r.maxX, y: y), options: [])
        ctx.restoreGState()
    }

    // Three window cards, back to front, sliding right.
    func card(_ r: CGRect, fill: CGColor, bar: CGColor, alpha: CGFloat, shadow: CGFloat) {
        ctx.saveGState()
        ctx.setAlpha(alpha)
        let p = CGPath(roundedRect: r, cornerWidth: 34, cornerHeight: 34, transform: nil)
        ctx.setShadow(offset: CGSize(width: 0, height: -14), blur: 36, color: color(0x000000, shadow))
        ctx.addPath(p); ctx.setFillColor(fill); ctx.fillPath()
        ctx.setShadow(offset: .zero, blur: 0, color: nil)
        // Title bar with traffic lights.
        ctx.saveGState(); ctx.addPath(p); ctx.clip()
        ctx.setFillColor(bar); ctx.fill(CGRect(x: r.minX, y: r.maxY - 62, width: r.width, height: 62))
        ctx.restoreGState()
        for (i, c) in [0xff5f57, 0xfebc2e, 0x28c840].enumerated() {
            ctx.setFillColor(color(UInt32(c)))
            ctx.fillEllipse(in: CGRect(x: r.minX + 30 + CGFloat(i) * 34, y: r.maxY - 42, width: 22, height: 22))
        }
        ctx.restoreGState()
    }
    card(CGRect(x: 290, y: 330, width: 380, height: 330), fill: color(0x6d5bd0), bar: color(0x5a48b8), alpha: 0.55, shadow: 0.2)
    card(CGRect(x: 352, y: 300, width: 400, height: 350), fill: color(0xa594f9), bar: color(0x8f7cf2), alpha: 0.8, shadow: 0.3)
    card(CGRect(x: 418, y: 268, width: 420, height: 370), fill: color(0xfafaff), bar: color(0xe9e5ff), alpha: 1, shadow: 0.45)

    // Content lines on the front card.
    ctx.setFillColor(color(0xc9c2f5))
    for (i, w) in [CGFloat(300), 240, 270].enumerated() {
        ctx.addPath(CGPath(roundedRect: CGRect(x: 463, y: 500 - CGFloat(i) * 56, width: w, height: 22),
                           cornerWidth: 11, cornerHeight: 11, transform: nil))
        ctx.fillPath()
    }

    // Subtle top highlight on the body.
    let hl = CGGradient(colorsSpace: space, colors: [color(0xffffff, 0.12), color(0xffffff, 0)] as CFArray, locations: [0, 1])!
    ctx.drawLinearGradient(hl, start: CGPoint(x: 512, y: 924), end: CGPoint(x: 512, y: 700), options: [])
    ctx.restoreGState()

    ctx.addPath(bodyPath); ctx.setStrokeColor(color(0xffffff, 0.08)); ctx.setLineWidth(3); ctx.strokePath()
}

func png(size: Int, to url: URL) {
    let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpace(name: CGColorSpace.sRGB)!,
                        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .high
    ctx.scaleBy(x: CGFloat(size) / S, y: CGFloat(size) / S)
    draw(ctx)
    let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
    try! rep.representation(using: .png, properties: [:])!.write(to: url)
}

let assets = URL(fileURLWithPath: "assets")
png(size: 1024, to: assets.appendingPathComponent("icon.png"))
let set = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("Flick.iconset")
try? FileManager.default.removeItem(at: set)
try! FileManager.default.createDirectory(at: set, withIntermediateDirectories: true)
for s in [16, 32, 128, 256, 512] {
    png(size: s, to: set.appendingPathComponent("icon_\(s)x\(s).png"))
    png(size: s * 2, to: set.appendingPathComponent("icon_\(s)x\(s)@2x.png"))
}
let p = Process()
p.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
p.arguments = ["-c", "icns", set.path, "-o", "assets/Flick.icns"]
try! p.run(); p.waitUntilExit()
print("wrote assets/icon.png and assets/Flick.icns")
