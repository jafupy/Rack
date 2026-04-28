// swift-tools-version: 6.0
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
    ],
    targets: [
        .executableTarget(
            name: "Rack",
            dependencies: [
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
            ],
            path: "Sources/Rack",
            exclude: ["d.md"],
            resources: [
                .copy("Plugins"),
            ]
        ),
        .executableTarget(
            name: "rack",
            dependencies: [
                .product(name: "NIOCore", package: "swift-nio"),
                .product(name: "NIOPosix", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
            ],
            path: "Sources/rack"
        ),
        // rack-bridge is a Rust binary built via Cargo, not SPM.
        // See Sources/rack-bridge/Cargo.toml and .github/workflows/ci.yml.
    ]
)
