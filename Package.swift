// swift-tools-version: 6.0
import PackageDescription

#if arch(arm64)
let rustCoreSearchPath = ".build/rust/aarch64-apple-darwin/release"
#else
let rustCoreSearchPath = ".build/rust/x86_64-apple-darwin/release"
#endif

let package = Package(
    name: "Rack",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "Rack", targets: ["Rack"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.65.0"),
        .package(url: "https://github.com/apple/swift-nio-ssl.git", from: "2.27.0"),
    ],
    targets: [
        .executableTarget(
            name: "Rack",
            dependencies: [
                "RackCoreFFI",
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "NIOWebSocket", package: "swift-nio"),
                .product(name: "NIOSSL", package: "swift-nio-ssl"),
            ],
            path: "Sources/Rack",
            exclude: ["d.md"],
            resources: [
                .copy("PackageInfo.json"),
            ],
            linkerSettings: [
                .unsafeFlags([
                    "-L", rustCoreSearchPath,
                    "-lrack_core",
                ])
            ]
        ),
        .target(
            name: "RackCoreFFI",
            path: "Sources/RackCoreFFI",
            publicHeadersPath: "include"
        ),
    ]
)
