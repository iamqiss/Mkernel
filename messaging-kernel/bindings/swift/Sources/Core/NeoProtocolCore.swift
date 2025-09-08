import Foundation
import Logging
import NIO
import NIOFoundationCompat
import Crypto
import Collections
import Algorithms

// MARK: - Core Types

/// Represents a Neo protocol message
public protocol NeoMessage: Codable, Equatable {
    static var messageType: String { get }
}

/// Represents a Neo protocol request
public protocol NeoRequest: NeoMessage {
    associatedtype Response: NeoMessage
}

/// Represents a Neo protocol response
public protocol NeoResponse: NeoMessage {
    associatedtype Request: NeoMessage
}

/// Represents a Neo protocol event
public protocol NeoEvent: NeoMessage {
    static var eventType: String { get }
}

// MARK: - Configuration

/// Configuration for Neo client
public struct NeoClientConfig {
    public let serverURL: URL
    public let timeout: TimeInterval
    public let retryAttempts: Int
    public let retryDelay: TimeInterval
    public let enableCompression: Bool
    public let enableTLS: Bool
    public let certificatePinning: CertificatePinning?
    public let headers: [String: String]
    
    public init(
        serverURL: URL,
        timeout: TimeInterval = 30.0,
        retryAttempts: Int = 3,
        retryDelay: TimeInterval = 1.0,
        enableCompression: Bool = true,
        enableTLS: Bool = true,
        certificatePinning: CertificatePinning? = nil,
        headers: [String: String] = [:]
    ) {
        self.serverURL = serverURL
        self.timeout = timeout
        self.retryAttempts = retryAttempts
        self.retryDelay = retryDelay
        self.enableCompression = enableCompression
        self.enableTLS = enableTLS
        self.certificatePinning = certificatePinning
        self.headers = headers
    }
    
    public static let `default` = NeoClientConfig(
        serverURL: URL(string: "https://localhost:8080")!
    )
}

/// Certificate pinning configuration
public struct CertificatePinning {
    public let certificates: [Data]
    public let validationMode: ValidationMode
    
    public enum ValidationMode {
        case pinCertificate
        case pinPublicKey
        case pinSubjectPublicKeyInfo
    }
    
    public init(certificates: [Data], validationMode: ValidationMode) {
        self.certificates = certificates
        self.validationMode = validationMode
    }
}

// MARK: - Qiss Codec

/// High-performance binary serialization codec
public class QissCodec {
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()
    
    public init() {
        encoder.dataEncodingStrategy = .base64
        decoder.dataDecodingStrategy = .base64
    }
    
    /// Encode a message to binary data
    public func encode<T: NeoMessage>(_ message: T) throws -> Data {
        let jsonData = try encoder.encode(message)
        return try compress(jsonData)
    }
    
    /// Decode binary data to a message
    public func decode<T: NeoMessage>(_ data: Data, as type: T.Type) throws -> T {
        let jsonData = try decompress(data)
        return try decoder.decode(type, from: jsonData)
    }
    
    private func compress(_ data: Data) throws -> Data {
        // TODO: Implement Qiss binary compression
        return data
    }
    
    private func decompress(_ data: Data) throws -> Data {
        // TODO: Implement Qiss binary decompression
        return data
    }
}

// MARK: - Transport Protocol

/// Base protocol for all transport implementations
public protocol NeoTransport {
    func call<T: NeoRequest, R: NeoResponse>(
        service: String,
        method: String,
        request: T,
        responseType: R.Type
    ) async throws -> R
    
    func subscribe<E: NeoEvent>(
        topic: String,
        eventType: E.Type
    ) -> AsyncThrowingStream<E, Error>
    
    func connect() async throws
    func disconnect() async throws
}

// MARK: - Client

/// Main Neo client for making RPC calls and subscribing to events
public class NeoClient {
    private let transport: NeoTransport
    private let codec: QissCodec
    private let logger: Logger
    
    public init(config: NeoClientConfig, transport: NeoTransport? = nil) {
        self.transport = transport ?? HTTP2Transport(config: config)
        self.codec = QissCodec()
        self.logger = Logger(label: "com.neo.protocol.client")
    }
    
    /// Make an RPC call
    public func call<T: NeoRequest, R: NeoResponse>(
        service: String,
        method: String,
        request: T,
        responseType: R.Type
    ) async throws -> R {
        logger.info("Making RPC call: \(service).\(method)")
        
        do {
            let response = try await transport.call(
                service: service,
                method: method,
                request: request,
                responseType: responseType
            )
            
            logger.info("RPC call successful: \(service).\(method)")
            return response
        } catch {
            logger.error("RPC call failed: \(service).\(method), error: \(error)")
            throw error
        }
    }
    
    /// Subscribe to events
    public func subscribe<E: NeoEvent>(
        topic: String,
        eventType: E.Type
    ) -> AsyncThrowingStream<E, Error> {
        logger.info("Subscribing to topic: \(topic)")
        return transport.subscribe(topic: topic, eventType: eventType)
    }
    
    /// Connect to the server
    public func connect() async throws {
        logger.info("Connecting to server")
        try await transport.connect()
        logger.info("Connected to server")
    }
    
    /// Disconnect from the server
    public func disconnect() async throws {
        logger.info("Disconnecting from server")
        try await transport.disconnect()
        logger.info("Disconnected from server")
    }
}

// MARK: - Service Client Base

/// Base class for generated service clients
open class NeoServiceClient {
    protected let client: NeoClient
    protected let serviceName: String
    
    public init(client: NeoClient, serviceName: String) {
        self.client = client
        self.serviceName = serviceName
    }
}

// MARK: - Errors

/// Neo protocol errors
public enum NeoError: Error, LocalizedError {
    case connectionFailed(String)
    case timeout
    case serializationFailed(String)
    case deserializationFailed(String)
    case serviceNotFound(String)
    case methodNotFound(String)
    case invalidRequest
    case invalidResponse
    case networkError(Error)
    case serverError(Int, String)
    case unknown(String)
    
    public var errorDescription: String? {
        switch self {
        case .connectionFailed(let message):
            return "Connection failed: \(message)"
        case .timeout:
            return "Request timeout"
        case .serializationFailed(let message):
            return "Serialization failed: \(message)"
        case .deserializationFailed(let message):
            return "Deserialization failed: \(message)"
        case .serviceNotFound(let service):
            return "Service not found: \(service)"
        case .methodNotFound(let method):
            return "Method not found: \(method)"
        case .invalidRequest:
            return "Invalid request"
        case .invalidResponse:
            return "Invalid response"
        case .networkError(let error):
            return "Network error: \(error.localizedDescription)"
        case .serverError(let code, let message):
            return "Server error \(code): \(message)"
        case .unknown(let message):
            return "Unknown error: \(message)"
        }
    }
}

// MARK: - Utilities

/// Utility functions for Neo protocol
public enum NeoUtils {
    /// Generate a unique request ID
    public static func generateRequestID() -> String {
        return UUID().uuidString
    }
    
    /// Validate a service name
    public static func validateServiceName(_ name: String) -> Bool {
        let pattern = "^[a-zA-Z][a-zA-Z0-9]*$"
        let regex = try? NSRegularExpression(pattern: pattern)
        let range = NSRange(location: 0, length: name.utf16.count)
        return regex?.firstMatch(in: name, options: [], range: range) != nil
    }
    
    /// Validate a method name
    public static func validateMethodName(_ name: String) -> Bool {
        let pattern = "^[a-zA-Z][a-zA-Z0-9]*$"
        let regex = try? NSRegularExpression(pattern: pattern)
        let range = NSRange(location: 0, length: name.utf16.count)
        return regex?.firstMatch(in: name, options: [], range: range) != nil
    }
}