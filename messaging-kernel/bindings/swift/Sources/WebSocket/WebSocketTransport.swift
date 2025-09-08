import Foundation
import NIO
import NIOHTTP1
import NIOSSL
import NIOTransportServices
import Logging

/// WebSocket transport implementation for Neo protocol
public class WebSocketTransport: NeoTransport {
    private let config: NeoClientConfig
    private let eventLoopGroup: EventLoopGroup
    private let logger: Logger
    private var channel: Channel?
    private var webSocketClient: WebSocketClient?
    
    public init(config: NeoClientConfig) {
        self.config = config
        self.eventLoopGroup = NIOTSEventLoopGroup()
        self.logger = Logger(label: "com.neo.protocol.websocket")
    }
    
    deinit {
        try? eventLoopGroup.syncShutdownGracefully()
    }
    
    public func connect() async throws {
        logger.info("Connecting to WebSocket server: \(config.serverURL)")
        
        let bootstrap = ClientBootstrap(group: eventLoopGroup)
            .channelOption(ChannelOptions.socket(SocketOptionLevel(SOL_SOCKET), SO_REUSEADDR), value: 1)
            .channelInitializer { channel in
                if self.config.enableTLS {
                    return self.configureTLS(channel: channel)
                } else {
                    return self.configureWebSocket(channel: channel)
                }
            }
        
        do {
            let channel = try await bootstrap.connect(
                host: config.serverURL.host ?? "localhost",
                port: config.serverURL.port ?? (config.enableTLS ? 443 : 80)
            ).get()
            
            self.channel = channel
            self.webSocketClient = WebSocketClient(channel: channel, config: self.config)
            
            logger.info("Connected to WebSocket server")
        } catch {
            logger.error("Failed to connect to WebSocket server: \(error)")
            throw NeoError.connectionFailed(error.localizedDescription)
        }
    }
    
    public func disconnect() async throws {
        logger.info("Disconnecting from WebSocket server")
        
        if let channel = channel {
            try await channel.close()
            self.channel = nil
            self.webSocketClient = nil
        }
        
        logger.info("Disconnected from WebSocket server")
    }
    
    public func call<T: NeoRequest, R: NeoResponse>(
        service: String,
        method: String,
        request: T,
        responseType: R.Type
    ) async throws -> R {
        guard let webSocketClient = webSocketClient else {
            throw NeoError.connectionFailed("Not connected to server")
        }
        
        logger.debug("Making WebSocket RPC call: \(service).\(method)")
        
        do {
            let response = try await webSocketClient.call(
                service: service,
                method: method,
                request: request,
                responseType: responseType
            )
            
            logger.debug("WebSocket RPC call successful: \(service).\(method)")
            return response
        } catch {
            logger.error("WebSocket RPC call failed: \(service).\(method), error: \(error)")
            throw error
        }
    }
    
    public func subscribe<E: NeoEvent>(
        topic: String,
        eventType: E.Type
    ) -> AsyncThrowingStream<E, Error> {
        guard let webSocketClient = webSocketClient else {
            return AsyncThrowingStream { continuation in
                continuation.finish(throwing: NeoError.connectionFailed("Not connected to server"))
            }
        }
        
        logger.debug("Subscribing to WebSocket topic: \(topic)")
        return webSocketClient.subscribe(topic: topic, eventType: eventType)
    }
    
    private func configureTLS(channel: Channel) -> EventLoopFuture<Void> {
        let sslContext = try! NIOSSLContext(configuration: .makeClientDefault())
        let sslHandler = try! NIOSSLClientHandler(context: sslContext, serverHostname: config.serverURL.host)
        
        return channel.pipeline.addHandler(sslHandler).flatMap { _ in
            self.configureWebSocket(channel: channel)
        }
    }
    
    private func configureWebSocket(channel: Channel) -> EventLoopFuture<Void> {
        let httpHandler = HTTPClientHandler()
        let webSocketHandler = WebSocketHandler()
        
        return channel.pipeline.addHandler(httpHandler).flatMap { _ in
            channel.pipeline.addHandler(webSocketHandler)
        }
    }
}

/// WebSocket client implementation
private class WebSocketClient {
    private let channel: Channel
    private let config: NeoClientConfig
    private let codec: QissCodec
    private let logger: Logger
    private var pendingRequests: [String: CheckedContinuation<Data, Error>] = [:]
    private var eventSubscriptions: [String: AsyncThrowingStream<Data, Error>.Continuation] = [:]
    
    init(channel: Channel, config: NeoClientConfig) {
        self.channel = channel
        self.config = config
        self.codec = QissCodec()
        self.logger = Logger(label: "com.neo.protocol.websocket.client")
    }
    
    func call<T: NeoRequest, R: NeoResponse>(
        service: String,
        method: String,
        request: T,
        responseType: R.Type
    ) async throws -> R {
        let requestData = try codec.encode(request)
        let requestId = NeoUtils.generateRequestID()
        
        let webSocketRequest = WebSocketRequest(
            id: requestId,
            type: .rpc,
            service: service,
            method: method,
            data: requestData
        )
        
        let responseData = try await withCheckedThrowingContinuation { continuation in
            pendingRequests[requestId] = continuation
            
            let message = try! JSONEncoder().encode(webSocketRequest)
            let frame = WebSocketFrame(fin: true, opcode: .binary, data: ByteBuffer(data: message))
            
            channel.writeAndFlush(frame, promise: nil)
        }
        
        let response = try codec.decode(responseData, as: responseType)
        return response
    }
    
    func subscribe<E: NeoEvent>(
        topic: String,
        eventType: E.Type
    ) -> AsyncThrowingStream<E, Error> {
        return AsyncThrowingStream { continuation in
            let subscriptionId = NeoUtils.generateRequestID()
            
            let subscribeRequest = WebSocketRequest(
                id: subscriptionId,
                type: .subscribe,
                topic: topic,
                data: Data()
            )
            
            let message = try! JSONEncoder().encode(subscribeRequest)
            let frame = WebSocketFrame(fin: true, opcode: .binary, data: ByteBuffer(data: message))
            
            channel.writeAndFlush(frame, promise: nil)
            
            // Store the continuation for this subscription
            self.eventSubscriptions[subscriptionId] = continuation
            
            // TODO: Implement proper event streaming
            continuation.finish(throwing: NeoError.unknown("WebSocket event streaming not fully implemented"))
        }
    }
    
    func handleMessage(_ data: Data) {
        do {
            let response = try JSONDecoder().decode(WebSocketResponse.self, from: data)
            
            if let requestId = response.requestId {
                // Handle RPC response
                if let continuation = pendingRequests.removeValue(forKey: requestId) {
                    if let responseData = response.data {
                        continuation.resume(returning: responseData)
                    } else if let error = response.error {
                        continuation.resume(throwing: NeoError.serverError(response.statusCode ?? 500, error))
                    }
                }
            } else if let topic = response.topic {
                // Handle event
                if let continuation = eventSubscriptions[topic] {
                    if let eventData = response.data {
                        continuation.yield(eventData)
                    }
                }
            }
        } catch {
            logger.error("Failed to decode WebSocket message: \(error)")
        }
    }
}

/// WebSocket request structure
private struct WebSocketRequest: Codable {
    let id: String
    let type: RequestType
    let service: String?
    let method: String?
    let topic: String?
    let data: Data
    
    enum RequestType: String, Codable {
        case rpc
        case subscribe
        case unsubscribe
    }
}

/// WebSocket response structure
private struct WebSocketResponse: Codable {
    let requestId: String?
    let topic: String?
    let statusCode: Int?
    let data: Data?
    let error: String?
}

/// WebSocket frame handler
private class WebSocketHandler: ChannelInboundHandler {
    typealias InboundIn = WebSocketFrame
    typealias OutboundOut = WebSocketFrame
    
    private var webSocketClient: WebSocketClient?
    
    func userInboundEventTriggered(context: ChannelHandlerContext, event: Any) {
        if let event = event as? WebSocketEvent {
            switch event {
            case .connected:
                // WebSocket connected
                break
            case .disconnected:
                // WebSocket disconnected
                break
            }
        }
    }
    
    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        let frame = unwrapInboundIn(data)
        
        switch frame.opcode {
        case .binary:
            if let data = frame.unmaskedData {
                webSocketClient?.handleMessage(Data(data.readableBytesView))
            }
        case .text:
            // Handle text frames if needed
            break
        case .close:
            // Handle close frames
            break
        default:
            break
        }
    }
}

/// WebSocket events
private enum WebSocketEvent {
    case connected
    case disconnected
}