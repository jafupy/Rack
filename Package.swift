// swift-tools-version: 6.0

import PackageDescription

let rustProfileDir = Context.environment["RACK_RUST_PROFILE_DIR"] ?? "debug"

let package = Package(
  name: "Rack",
  platforms: [
    .macOS(.v14)
  ],
  products: [
    .library(name: "RackUI", targets: ["RackUI"]),
    .executable(name: "Rack", targets: ["RackMac"]),
  ],
  targets: [
    .target(
      name: "RackUI",
      path: "packages/ui/src",
      linkerSettings: [
        .unsafeFlags(["-L", ".build/rust/\(rustProfileDir)/deps", "-lrack_services"])
      ]
    ),
    .executableTarget(
      name: "RackMac",
      dependencies: ["RackUI"],
      path: "packages/mac/src"
    ),
  ]
)
