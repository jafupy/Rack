// swift-tools-version: 6.1
import PackageDescription

let package = Package(
    name: "Rack",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(name: "Rack", targets: ["Rack"])
    ],
    targets: [
        .executableTarget(
            name: "Rack",
            path: "Sources"
        )
    ]
)
