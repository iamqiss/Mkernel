import Foundation
import NeoProtocolCore
import NeoProtocolHTTP2
import NeoProtocolWebSocket

/// Main Neo Protocol library
/// 
/// This is the main entry point for the Neo Protocol Swift SDK.
/// It provides a unified interface for making RPC calls and subscribing to events
/// across different transport protocols (HTTP/2, WebSocket).
///
/// ## Usage
///
/// ```swift
/// import NeoProtocol
///
/// // Create a client
/// let config = NeoClientConfig(
///     serverURL: URL(string: "https://api.example.com")!
/// )
/// let client = NeoClient(config: config)
///
/// // Connect to the server
/// try await client.connect()
///
/// // Make an RPC call
/// let user = try await client.call(
///     service: "UserService",
///     method: "GetUser",
///     request: UserId(id: 123),
///     responseType: User.self
/// )
///
/// // Subscribe to events
/// for try await event in client.subscribe(
///     topic: "users.events",
///     eventType: UserCreated.self
/// ) {
///     print("User created: \(event)")
/// }
/// ```
@available(iOS 13.0, macOS 10.15, watchOS 6.0, tvOS 13.0, *)
public class NeoProtocol {
    /// Create a new Neo client with the specified configuration
    /// - Parameter config: The client configuration
    /// - Returns: A configured Neo client
    public static func createClient(config: NeoClientConfig) -> NeoClient {
        return NeoClient(config: config)
    }
    
    /// Create a new Neo client with HTTP/2 transport
    /// - Parameter config: The client configuration
    /// - Returns: A configured Neo client with HTTP/2 transport
    public static func createHTTP2Client(config: NeoClientConfig) -> NeoClient {
        let transport = HTTP2Transport(config: config)
        return NeoClient(config: config, transport: transport)
    }
    
    /// Create a new Neo client with WebSocket transport
    /// - Parameter config: The client configuration
    /// - Returns: A configured Neo client with WebSocket transport
    public static func createWebSocketClient(config: NeoClientConfig) -> NeoClient {
        let transport = WebSocketTransport(config: config)
        return NeoClient(config: config, transport: transport)
    }
}

// MARK: - Re-exports

@_exported import NeoProtocolCore
@_exported import NeoProtocolHTTP2
@_exported import NeoProtocolWebSocket