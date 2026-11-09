// swift-tools-version:5.5
import PackageDescription

let package = Package(
    name: "tauri-plugin-native-chrome",
    platforms: [.iOS(.v15)],
    products: [
        .library(name: "tauri-plugin-native-chrome", type: .static, targets: ["NativeChrome"])
    ],
    dependencies: [.package(name: "Tauri", path: "../.tauri/tauri-api")],
    targets: [
        .target(name: "NativeChrome", dependencies: [.byName(name: "Tauri")], path: "Sources")
    ]
)