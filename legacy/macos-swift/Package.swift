// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "AazadVoice",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(url: "https://github.com/FluidInference/FluidAudio.git", from: "0.7.0"),
    ],
    targets: [
        .executableTarget(
            name: "AazadVoice",
            dependencies: [.product(name: "FluidAudio", package: "FluidAudio")],
            path: "Sources/AazadVoice"
        ),
    ]
)
