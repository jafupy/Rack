// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Rack",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "Rack", targets: ["Rack"]),
        .executable(name: "rack", targets: ["RackCLI"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.65.0"),
        .package(url: "https://github.com/apple/swift-nio-ssl.git", from: "2.27.0"),
    ],
    targets: [
        .executableTarget(
            name: "Rack",
            dependencies: [
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "NIOWebSocket", package: "swift-nio"),
                .product(name: "NIOSSL", package: "swift-nio-ssl"),
            ],
            path: "Sources/Rack",
            exclude: ["d.md"],
            resources: [
                .copy("Plugins"),
                .copy("PackageInfo.json"),
            ]
        ),
        .executableTarget(
            name: "RackCLI",
            path: "Sources/rack-cli"
        ),
    ]
)
