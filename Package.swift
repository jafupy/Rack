// swift-tools-version: 6.0

import PackageDescription

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
        .unsafeFlags(["-L", ".build/rust/debug", "-lrack_services"])
      ]
    ),
    .executableTarget(
      name: "RackMac",
      dependencies: ["RackUI"],
      path: "packages/mac/src"
    ),
  ]
)
