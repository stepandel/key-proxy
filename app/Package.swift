// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "KeyProxy",
    platforms: [
        .macOS(.v14)
    ],
    targets: [
        .executableTarget(
            name: "KeyProxy",
            path: "Sources/KeyProxy",
            swiftSettings: [
                .unsafeFlags(["-parse-as-library"])
            ]
        )
    ]
)
