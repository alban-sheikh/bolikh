// Renders assets/icon-1024.png (app icon) and src-tauri/icons/tray.png (monochrome template).
// Usage (macOS): swift assets/make-icons.swift   then: npm run icon
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

let heights: [CGFloat] = [120, 230, 380, 520, 300, 440, 260, 150]

func bars(scale s: CGFloat, center: CGFloat, barW: CGFloat, gap: CGFloat, fill: (NSBezierPath) -> Void) {
    let total = CGFloat(heights.count) * barW + CGFloat(heights.count - 1) * gap
    var x = center - total / 2
    for h in heights {
        let r = NSRect(x: x, y: center - h * s / 2, width: barW, height: h * s)
        fill(NSBezierPath(roundedRect: r, xRadius: barW / 2, yRadius: barW / 2))
        x += barW + gap
    }
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
    NSGradient(colors: [NSColor(red: 0.04, green: 0.05, blue: 0.12, alpha: 1),
                        NSColor(red: 0.13, green: 0.10, blue: 0.32, alpha: 1)])!.draw(in: path, angle: 90)
    NSGraphicsContext.current?.saveGraphicsState()
    path.addClip()
    NSGradient(colors: [NSColor(red: 0.45, green: 0.55, blue: 1, alpha: 0.45), .clear])!
        .draw(fromCenter: NSPoint(x: 512 * s, y: 470 * s), radius: 0,
              toCenter: NSPoint(x: 512 * s, y: 470 * s), radius: 430 * s, options: [])
    NSGraphicsContext.current?.restoreGraphicsState()
    NSColor.white.withAlphaComponent(0.10).setStroke()
    path.lineWidth = 3 * s; path.stroke()
    let g = NSGradient(colors: [NSColor(red: 0.55, green: 0.78, blue: 1, alpha: 1),
                                NSColor(red: 0.80, green: 0.60, blue: 1, alpha: 1)])!
    bars(scale: s, center: 512 * s, barW: 46 * s, gap: 30 * s) { g.draw(in: $0, angle: 90) }
}

let tray = bitmap(64) { size in
    let s = size / 640
    NSColor.black.setFill()
    bars(scale: s, center: size / 2, barW: 4.2, gap: 2.6) { $0.fill() }
}

try! appIcon.write(to: URL(fileURLWithPath: "assets/icon-1024.png"))
try! tray.write(to: URL(fileURLWithPath: "src-tauri/icons/tray.png"))
print("ok")
