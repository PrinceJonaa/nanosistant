// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NanoClawKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "NanoClawKit", targets: ["NanoClawKit"]),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "NanoClawKit",
            dependencies: []
        ),
        .testTarget(
            name: "NanoClawKitTests",
            dependencies: ["NanoClawKit"]
        ),
    ]
)
