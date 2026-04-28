// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "Rack",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "Rack", targets: ["Rack"]),
        .executable(name: "rack", targets: ["rack"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.65.0"),
        .package(url: "https://github.com/swiftwasm/WasmKit.git", from: "0.1.0"),
    ],
    targets: [
        // Main menu bar app
        .executableTarget(
            name: "Rack",
            dependencies: [
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "WasmKit", package: "WasmKit"),
            ],
            path: "Sources/Rack",
            exclude: ["d.md"],
            resources: [
                .copy("Plugins"),
            ]
        ),
        // CLI: `rack dev`, `rack ls`, `rack start`, `rack stop`
        .executableTarget(
            name: "rack",
            path: "Sources/rack"
        ),
        // C bridge: unix socket <-> loopback TCP
        // Built as a standalone tool, bundled inside Rack.app
        .executableTarget(
            name: "rack-bridge",
            path: "Sources/rack-bridge"
        ),
    ]
)
