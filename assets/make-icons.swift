// Renders assets/icon-1024.png (app icon) and src-tauri/icons/tray.png (monochrome template).
// Usage (macOS): swift assets/make-icons.swift   then: npm run icon
//
// The Bolikh mark: a text cursor (I-beam) with two sound waves leaving it. Read together they form
// a lowercase "b": speech turning into typed text. Geometry matches website/logo.svg (100-unit box,
// y down): stem x=40.5 y 18..82, caps 33.5..47.5, waves centred (40.5,58) r=14 (±75°) and r=26 (±55°).
import AppKit

func bitmap(_ size: Int, _ draw: (CGFloat) -> Void) -> Data {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8,
                               samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
                               bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    draw(CGFloat(size))
    NSGraphicsContext.restoreGraphicsState()
    return rep.representation(using: .png, properties: [:])!
}

/// The mark as stroked paths inside `box` (a square), in AppKit's y-up space.
func mark(in box: NSRect, lineWidth: CGFloat) -> [NSBezierPath] {
    let u = box.width / 100
    func p(_ x: CGFloat, _ y: CGFloat) -> NSPoint { NSPoint(x: box.minX + x * u, y: box.maxY - y * u) }
    func line(_ a: NSPoint, _ b: NSPoint) -> NSBezierPath { let l = NSBezierPath(); l.move(to: a); l.line(to: b); return l }
    let stem = line(p(40.5, 18), p(40.5, 82))
    let top = line(p(33.5, 18), p(47.5, 18))
    let bottom = line(p(33.5, 82), p(47.5, 82))
    let c = p(40.5, 58)
    let inner = NSBezierPath(); inner.appendArc(withCenter: c, radius: 14 * u, startAngle: -75, endAngle: 75, clockwise: false)
    let outer = NSBezierPath(); outer.appendArc(withCenter: c, radius: 26 * u, startAngle: -55, endAngle: 55, clockwise: false)
    let all = [stem, top, bottom, inner, outer]
    for path in all { path.lineWidth = lineWidth * u; path.lineCapStyle = .round; path.lineJoinStyle = .round }
    return all
}

let appIcon = bitmap(1024) { size in
    let s = size / 1024
    let body = NSRect(x: 100 * s, y: 100 * s, width: 824 * s, height: 824 * s)
    let path = NSBezierPath(roundedRect: body, xRadius: 185 * s, yRadius: 185 * s)
    NSGraphicsContext.current?.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.35)
    shadow.shadowBlurRadius = 30 * s
    shadow.shadowOffset = NSSize(width: 0, height: -12 * s)
    shadow.set()
    NSColor.black.setFill(); path.fill()
    NSGraphicsContext.current?.restoreGraphicsState()
    // Warm charcoal body (Aazad "Golden Hour" palette) with a soft gold glow behind the mark.
    NSGradient(colors: [NSColor(red: 0.08, green: 0.07, blue: 0.06, alpha: 1),
                        NSColor(red: 0.17, green: 0.15, blue: 0.12, alpha: 1)])!.draw(in: path, angle: 90)
    NSGraphicsContext.current?.saveGraphicsState()
    path.addClip()
    NSGradient(colors: [NSColor(red: 0.95, green: 0.71, blue: 0.31, alpha: 0.22), .clear])!
        .draw(fromCenter: NSPoint(x: 560 * s, y: 480 * s), radius: 0,
              toCenter: NSPoint(x: 560 * s, y: 480 * s), radius: 420 * s, options: [])
    NSGraphicsContext.current?.restoreGraphicsState()
    NSColor(red: 0.93, green: 0.71, blue: 0.29, alpha: 0.18).setStroke()
    path.lineWidth = 3 * s; path.stroke()
    let gold = NSColor(red: 0.95, green: 0.74, blue: 0.33, alpha: 1)
    gold.setStroke()
    let inset: CGFloat = 200 * s
    for p in mark(in: body.insetBy(dx: inset - 100 * s, dy: inset - 100 * s), lineWidth: 7.5) { p.stroke() }
}

let tray = bitmap(64) { size in
    NSColor.black.setStroke()
    for p in mark(in: NSRect(x: -2, y: 0, width: size, height: size), lineWidth: 9) { p.stroke() }
}

try! appIcon.write(to: URL(fileURLWithPath: "assets/icon-1024.png"))
try! tray.write(to: URL(fileURLWithPath: "src-tauri/icons/tray.png"))
print("ok")
