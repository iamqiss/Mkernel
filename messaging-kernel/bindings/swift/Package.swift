// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "NeoProtocol",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15),
        .watchOS(.v6),
        .tvOS(.v13)
    ],
    products: [
        .library(
            name: "NeoProtocol",
            targets: ["NeoProtocol"]
        ),
        .library(
            name: "NeoProtocolCore",
            targets: ["NeoProtocolCore"]
        ),
        .library(
            name: "NeoProtocolHTTP2",
            targets: ["NeoProtocolHTTP2"]
        ),
        .library(
            name: "NeoProtocolWebSocket",
            targets: ["NeoProtocolWebSocket"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-log.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-nio.git", from: "2.0.0"),
        .package(url: "https://github.com/apple/swift-nio-http2.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-nio-ssl.git", from: "2.0.0"),
        .package(url: "https://github.com/apple/swift-nio-transport-services.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-crypto.git", from: "2.0.0"),
        .package(url: "https://github.com/apple/swift-collections.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-algorithms.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-syntax.git", from: "509.0.0"),
        .package(url: "https://github.com/apple/swift-format.git", from: "509.0.0")
    ],
    targets: [
        .target(
            name: "NeoProtocolCore",
            dependencies: [
                .product(name: "Logging", package: "swift-log"),
                .product(name: "NIO", package: "swift-nio"),
                .product(name: "NIOFoundationCompat", package: "swift-nio"),
                .product(name: "Crypto", package: "swift-crypto"),
                .product(name: "Collections", package: "swift-collections"),
                .product(name: "Algorithms", package: "swift-algorithms")
            ],
            path: "Sources/Core"
        ),
        .target(
            name: "NeoProtocolHTTP2",
            dependencies: [
                "NeoProtocolCore",
                .product(name: "NIO", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "NIOHTTP2", package: "swift-nio-http2"),
                .product(name: "NIOSSL", package: "swift-nio-ssl"),
                .product(name: "NIOTransportServices", package: "swift-nio-transport-services")
            ],
            path: "Sources/HTTP2"
        ),
        .target(
            name: "NeoProtocolWebSocket",
            dependencies: [
                "NeoProtocolCore",
                .product(name: "NIO", package: "swift-nio"),
                .product(name: "NIOHTTP1", package: "swift-nio"),
                .product(name: "NIOSSL", package: "swift-nio-ssl"),
                .product(name: "NIOTransportServices", package: "swift-nio-transport-services")
            ],
            path: "Sources/WebSocket"
        ),
        .target(
            name: "NeoProtocol",
            dependencies: [
                "NeoProtocolCore",
                "NeoProtocolHTTP2",
                "NeoProtocolWebSocket"
            ],
            path: "Sources/NeoProtocol"
        ),
        .testTarget(
            name: "NeoProtocolTests",
            dependencies: [
                "NeoProtocol",
                "NeoProtocolCore",
                "NeoProtocolHTTP2",
                "NeoProtocolWebSocket"
            ],
            path: "Tests"
        ),
        .executableTarget(
            name: "NeoCodegen",
            dependencies: [
                "NeoProtocolCore",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
                .product(name: "SwiftSyntax", package: "swift-syntax"),
                .product(name: "SwiftFormat", package: "swift-format")
            ],
            path: "Sources/Codegen"
        )
    ]
)