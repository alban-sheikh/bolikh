// Renders the app icon at all required sizes → Resources/AppIcon.icns
// Usage: swift scripts/make-icon.swift
import AppKit

func render(size: CGFloat) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(size), pixelsHigh: Int(size),
                               bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                               colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    let s = size / 1024
    // Apple icon grid: 824pt body inside 1024 canvas.
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
    // Soft glow
    NSGraphicsContext.current?.saveGraphicsState()
    path.addClip()
    NSGradient(colors: [NSColor(red: 0.45, green: 0.55, blue: 1, alpha: 0.45), .clear])!
        .draw(fromCenter: NSPoint(x: 512 * s, y: 470 * s), radius: 0,
              toCenter: NSPoint(x: 512 * s, y: 470 * s), radius: 430 * s, options: [])
    NSGraphicsContext.current?.restoreGraphicsState()
    // Hairline border
    NSColor.white.withAlphaComponent(0.10).setStroke()
    path.lineWidth = 3 * s; path.stroke()

    // Waveform bars
    let heights: [CGFloat] = [120, 230, 380, 520, 300, 440, 260, 150]
    let barW: CGFloat = 46 * s, gap: CGFloat = 30 * s
    let total = CGFloat(heights.count) * barW + CGFloat(heights.count - 1) * gap
    var x = 512 * s - total / 2
    let bars = NSGradient(colors: [NSColor(red: 0.55, green: 0.78, blue: 1, alpha: 1),
                                   NSColor(red: 0.80, green: 0.60, blue: 1, alpha: 1)])!
    for h in heights {
        let r = NSRect(x: x, y: 512 * s - h * s / 2, width: barW, height: h * s)
        bars.draw(in: NSBezierPath(roundedRect: r, xRadius: barW / 2, yRadius: barW / 2), angle: 90)
        x += barW + gap
    }
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

let root = URL(fileURLWithPath: CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : ".")
let iconset = root.appendingPathComponent("build/AppIcon.iconset")
try? FileManager.default.removeItem(at: iconset)
try! FileManager.default.createDirectory(at: iconset, withIntermediateDirectories: true)
for base in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let px = CGFloat(base * scale)
        let name = scale == 1 ? "icon_\(base)x\(base).png" : "icon_\(base)x\(base)@2x.png"
        try! render(size: px).representation(using: .png, properties: [:])!
            .write(to: iconset.appendingPathComponent(name))
    }
}
print(iconset.path)
