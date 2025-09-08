import Foundation
import NIO
import NIOHTTP1
import NIOHTTP2
import NIOSSL
import NIOTransportServices
import Logging

/// HTTP/2 transport implementation for Neo protocol
public class HTTP2Transport: NeoTransport {
    private let config: NeoClientConfig
    private let eventLoopGroup: EventLoopGroup
    private let logger: Logger
    private var channel: Channel?
    private var http2Client: HTTP2Client?
    
    public init(config: NeoClientConfig) {
        self.config = config
        self.eventLoopGroup = NIOTSEventLoopGroup()
        self.logger = Logger(label: "com.neo.protocol.http2")
    }
    
    deinit {
        try? eventLoopGroup.syncShutdownGracefully()
    }
    
    public func connect() async throws {
        logger.info("Connecting to HTTP/2 server: \(config.serverURL)")
        
        let bootstrap = ClientBootstrap(group: eventLoopGroup)
            .channelOption(ChannelOptions.socket(SocketOptionLevel(SOL_SOCKET), SO_REUSEADDR), value: 1)
            .channelInitializer { channel in
                if self.config.enableTLS {
                    return self.configureTLS(channel: channel)
                } else {
                    return self.configureHTTP2(channel: channel)
                }
            }
        
        do {
            let channel = try await bootstrap.connect(
                host: config.serverURL.host ?? "localhost",
                port: config.serverURL.port ?? (config.enableTLS ? 443 : 80)
            ).get()
            
            self.channel = channel
            self.http2Client = HTTP2Client(channel: channel, config: self.config)
            
            logger.info("Connected to HTTP/2 server")
        } catch {
            logger.error("Failed to connect to HTTP/2 server: \(error)")
            throw NeoError.connectionFailed(error.localizedDescription)
        }
    }
    
    public func disconnect() async throws {
        logger.info("Disconnecting from HTTP/2 server")
        
        if let channel = channel {
            try await channel.close()
            self.channel = nil
            self.http2Client = nil
        }
        
        logger.info("Disconnected from HTTP/2 server")
    }
    
    public func call<T: NeoRequest, R: NeoResponse>(
        service: String,
        method: String,
        request: T,
        responseType: R.Type
    ) async throws -> R {
        guard let http2Client = http2Client else {
            throw NeoError.connectionFailed("Not connected to server")
        }
        
        logger.debug("Making HTTP/2 RPC call: \(service).\(method)")
        
        do {
            let response = try await http2Client.call(
                service: service,
                method: method,
                request: request,
                responseType: responseType
            )
            
            logger.debug("HTTP/2 RPC call successful: \(service).\(method)")
            return response
        } catch {
            logger.error("HTTP/2 RPC call failed: \(service).\(method), error: \(error)")
            throw error
        }
    }
    
    public func subscribe<E: NeoEvent>(
        topic: String,
        eventType: E.Type
    ) -> AsyncThrowingStream<E, Error> {
        guard let http2Client = http2Client else {
            return AsyncThrowingStream { continuation in
                continuation.finish(throwing: NeoError.connectionFailed("Not connected to server"))
            }
        }
        
        logger.debug("Subscribing to HTTP/2 topic: \(topic)")
        return http2Client.subscribe(topic: topic, eventType: eventType)
    }
    
    private func configureTLS(channel: Channel) -> EventLoopFuture<Void> {
        let sslContext = try! NIOSSLContext(configuration: .makeClientDefault())
        let sslHandler = try! NIOSSLClientHandler(context: sslContext, serverHostname: config.serverURL.host)
        
        return channel.pipeline.addHandler(sslHandler).flatMap { _ in
            self.configureHTTP2(channel: channel)
        }
    }
    
    private func configureHTTP2(channel: Channel) -> EventLoopFuture<Void> {
        let http2Handler = HTTP2ToHTTP1ServerCodec()
        let http2Multiplexer = HTTP2StreamMultiplexer(mode: .client, channel: channel)
        
        return channel.pipeline.addHandler(http2Handler).flatMap { _ in
            channel.pipeline.addHandler(http2Multiplexer)
        }
    }
}

/// HTTP/2 client implementation
private class HTTP2Client {
    private let channel: Channel
    private let config: NeoClientConfig
    private let codec: QissCodec
    private let logger: Logger
    
    init(channel: Channel, config: NeoClientConfig) {
        self.channel = channel
        self.config = config
        self.codec = QissCodec()
        self.logger = Logger(label: "com.neo.protocol.http2.client")
    }
    
    func call<T: NeoRequest, R: NeoResponse>(
        service: String,
        method: String,
        request: T,
        responseType: R.Type
    ) async throws -> R {
        let requestData = try codec.encode(request)
        let path = "/\(service)/\(method)"
        
        let httpRequest = HTTPRequestHead(
            version: .http2,
            method: .POST,
            uri: path,
            headers: [
                "content-type": "application/neo-binary",
                "content-length": "\(requestData.count)",
                "neo-service": service,
                "neo-method": method
            ]
        )
        
        let response = try await withCheckedThrowingContinuation { continuation in
            let streamPromise = channel.eventLoop.makePromise(of: Channel.self)
            
            // Create a new stream for this request
            channel.pipeline.handler(type: HTTP2StreamMultiplexer.self).whenSuccess { multiplexer in
                multiplexer.createStreamChannel(promise: streamPromise) { streamChannel in
                    let handler = HTTP2RPCResponseHandler<R>(
                        continuation: continuation,
                        codec: self.codec
                    )
                    return streamChannel.pipeline.addHandler(handler)
                }
            }
            
            streamPromise.futureResult.whenSuccess { streamChannel in
                // Send the request
                streamChannel.writeAndFlush(HTTP2Frame.FramePayload.headers(.init(headers: httpRequest.headers)), promise: nil)
                streamChannel.writeAndFlush(HTTP2Frame.FramePayload.data(.init(data: .byteBuffer(ByteBuffer(data: requestData)))), promise: nil)
            }
        }
        
        return response
    }
    
    func subscribe<E: NeoEvent>(
        topic: String,
        eventType: E.Type
    ) -> AsyncThrowingStream<E, Error> {
        return AsyncThrowingStream { continuation in
            // TODO: Implement HTTP/2 server-sent events for event streaming
            continuation.finish(throwing: NeoError.unknown("HTTP/2 event streaming not implemented"))
        }
    }
}

/// HTTP/2 RPC response handler
private class HTTP2RPCResponseHandler<T: NeoResponse>: ChannelInboundHandler {
    typealias InboundIn = HTTP2Frame.FramePayload
    typealias OutboundOut = HTTP2Frame.FramePayload
    
    private let continuation: CheckedContinuation<T, Error>
    private let codec: QissCodec
    private var responseData = Data()
    private var headers: HTTPHeaders?
    
    init(continuation: CheckedContinuation<T, Error>, codec: QissCodec) {
        self.continuation = continuation
        self.codec = codec
    }
    
    func channelRead(context: ChannelHandlerContext, data: NIOAny) {
        let frame = unwrapInboundIn(data)
        
        switch frame {
        case .headers(let headers):
            self.headers = headers.headers
        case .data(let data):
            if let buffer = data.data {
                responseData.append(contentsOf: buffer.readableBytesView)
            }
            
            if data.endStream {
                do {
                    let response = try codec.decode(responseData, as: T.self)
                    continuation.resume(returning: response)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        default:
            break
        }
    }
    
    func errorCaught(context: ChannelHandlerContext, error: Error) {
        continuation.resume(throwing: error)
    }
}