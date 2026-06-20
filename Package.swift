// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "Rack",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(name: "RackMac", targets: ["RackMac"]),
        .executable(name: "Rack", targets: ["RackUI"])
    ],
    targets: [
        .target(
            name: "RackMac",
            path: "packages/mac/src"
        ),
        .executableTarget(
            name: "RackUI",
            dependencies: ["RackMac"],
            path: "packages/ui/src"
        )
    ]
)
