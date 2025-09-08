// Package neo provides high-performance Go bindings for the Neo Messaging Kernel
//
// This package provides native Go bindings for the Neo Protocol, enabling
// high-performance RPC calls and message passing with minimal overhead.
//
// Example usage:
//
//	package main
//
//	import (
//		"context"
//		"fmt"
//		"log"
//		"time"
//
//		"github.com/neo-qiss/messaging-kernel/bindings/go"
//	)
//
//	func main() {
//		// Create a new Neo client
//		client, err := neo.NewClient(neo.ClientConfig{
//			ServerAddress: "127.0.0.1:8080",
//			Timeout:       30 * time.Second,
//		})
//		if err != nil {
//			log.Fatal(err)
//		}
//		defer client.Close()
//
//		// Make an RPC call
//		response, err := client.Call(context.Background(), "user_service", "get_user", []byte("user_id_123"))
//		if err != nil {
//			log.Fatal(err)
//		}
//
//		fmt.Printf("Response: %s\n", string(response))
//	}
package neo

/*
#cgo CFLAGS: -I${SRCDIR}/../../core/neo-protocol/include
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lneo_protocol
#include "neo_protocol.h"
#include <stdlib.h>
*/
import "C"
import (
	"context"
	"encoding/json"
	"fmt"
	"runtime"
	"sync"
	"time"
	"unsafe"
)

// Version information
const (
	Version     = "0.1.0"
	ProtocolVersion = 1
	MaxMessageSize  = 10 * 1024 * 1024 // 10MB
)

// Error types
type Error struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Details string `json:"details,omitempty"`
}

func (e Error) Error() string {
	return fmt.Sprintf("neo error %d: %s", e.Code, e.Message)
}

// Message types
type MessageType int

const (
	MessageTypeRpcRequest  MessageType = 1
	MessageTypeRpcResponse MessageType = 2
	MessageTypeRpcError    MessageType = 3
	MessageTypeEvent       MessageType = 4
	MessageTypeHeartbeat   MessageType = 5
	MessageTypeAuthRequest MessageType = 6
	MessageTypeAuthResponse MessageType = 7
	MessageTypeClose       MessageType = 8
)

// Client configuration
type ClientConfig struct {
	ServerAddress     string        `json:"server_address"`
	Timeout           time.Duration `json:"timeout"`
	MaxConcurrentReqs int           `json:"max_concurrent_requests"`
	EnableCompression bool          `json:"enable_compression"`
	EnableAuth        bool          `json:"enable_auth"`
	AuthToken         string        `json:"auth_token,omitempty"`
}

// Server configuration
type ServerConfig struct {
	Address           string        `json:"address"`
	MaxConcurrentReqs int           `json:"max_concurrent_requests"`
	EnableCompression bool          `json:"enable_compression"`
	EnableAuth        bool          `json:"enable_auth"`
	HeartbeatInterval time.Duration `json:"heartbeat_interval"`
}

// Service definition
type ServiceDefinition struct {
	Name      string                 `json:"name"`
	Version   string                 `json:"version"`
	Namespace string                 `json:"namespace"`
	Methods   map[string]MethodDef   `json:"methods"`
	Events    map[string]EventDef    `json:"events"`
}

type MethodDef struct {
	Name        string        `json:"name"`
	InputType   string        `json:"input_type"`
	OutputType  string        `json:"output_type"`
	Timeout     time.Duration `json:"timeout"`
	Queue       string        `json:"queue,omitempty"`
	Description string        `json:"description,omitempty"`
}

type EventDef struct {
	Name          string `json:"name"`
	Topic         string `json:"topic"`
	PartitionKey  string `json:"partition_key,omitempty"`
	Description   string `json:"description,omitempty"`
}

// RPC request
type RpcRequest struct {
	ServiceName   string    `json:"service_name"`
	MethodName    string    `json:"method_name"`
	CorrelationID uint64    `json:"correlation_id"`
	Payload       []byte    `json:"payload"`
	Timeout       time.Duration `json:"timeout,omitempty"`
	Timestamp     time.Time `json:"timestamp"`
}

// RPC response
type RpcResponse struct {
	CorrelationID uint64    `json:"correlation_id"`
	Payload       []byte    `json:"payload"`
	Success       bool      `json:"success"`
	Error         *Error    `json:"error,omitempty"`
	Timestamp     time.Time `json:"timestamp"`
}

// Event message
type EventMessage struct {
	Topic        string            `json:"topic"`
	PartitionKey string            `json:"partition_key,omitempty"`
	Payload      []byte            `json:"payload"`
	Headers      map[string]string `json:"headers,omitempty"`
	Timestamp    time.Time         `json:"timestamp"`
}

// Heartbeat message
type HeartbeatMessage struct {
	ClientID  string    `json:"client_id"`
	Timestamp time.Time `json:"timestamp"`
}

// Client for making RPC calls
type Client struct {
	config     ClientConfig
	clientPtr  unsafe.Pointer
	mu         sync.RWMutex
	closed     bool
	requestID  uint64
	responses  map[uint64]chan *RpcResponse
	responseMu sync.RWMutex
}

// Server for handling RPC calls
type Server struct {
	config       ServerConfig
	serverPtr    unsafe.Pointer
	services     map[string]*ServiceDefinition
	handlers     map[string]RPCHandler
	eventHandlers map[string]EventHandler
	mu           sync.RWMutex
	closed       bool
}

// RPC handler function type
type RPCHandler func(ctx context.Context, req *RpcRequest) (*RpcResponse, error)

// Event handler function type
type EventHandler func(ctx context.Context, event *EventMessage) error

// Metrics collector
type MetricsCollector struct {
	collectorPtr unsafe.Pointer
	mu           sync.RWMutex
}

// Metrics snapshot
type MetricsSnapshot struct {
	RpcRequestsTotal    uint64            `json:"rpc_requests_total"`
	RpcResponsesTotal   uint64            `json:"rpc_responses_total"`
	RpcErrorsTotal      uint64            `json:"rpc_errors_total"`
	ActiveConnections   uint64            `json:"active_connections"`
	BytesSentTotal      uint64            `json:"bytes_sent_total"`
	BytesReceivedTotal  uint64            `json:"bytes_received_total"`
	ServiceRegistrations uint64           `json:"service_registrations"`
	MemoryUsageBytes    uint64            `json:"memory_usage_bytes"`
	CPUUsagePercent     uint64            `json:"cpu_usage_percent"`
	UptimeSeconds       uint64            `json:"uptime_seconds"`
	MethodStats         map[string]MethodStats `json:"method_stats"`
	LatencyStats        *LatencyStats     `json:"latency_stats,omitempty"`
	Timestamp           uint64            `json:"timestamp"`
}

type MethodStats struct {
	Invocations uint64 `json:"invocations"`
	Errors      uint64 `json:"errors"`
}

type LatencyStats struct {
	Min  time.Duration `json:"min"`
	Max  time.Duration `json:"max"`
	P50  time.Duration `json:"p50"`
	P95  time.Duration `json:"p95"`
	P99  time.Duration `json:"p99"`
	Mean time.Duration `json:"mean"`
}

// Health status
type HealthStatus struct {
	Status    string                 `json:"status"`
	Timestamp uint64                 `json:"timestamp"`
	Checks    map[string]CheckResult `json:"checks"`
	Version   string                 `json:"version"`
	UptimeSeconds uint64             `json:"uptime_seconds"`
}

type CheckResult struct {
	Status      string        `json:"status"`
	Message     string        `json:"message"`
	DurationMs  uint64        `json:"duration_ms"`
	Timestamp   uint64        `json:"timestamp"`
}

// NewClient creates a new Neo client
func NewClient(config ClientConfig) (*Client, error) {
	configJSON, err := json.Marshal(config)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal config: %w", err)
	}

	configCStr := C.CString(string(configJSON))
	defer C.free(unsafe.Pointer(configCStr))

	clientPtr := C.neo_client_new(configCStr)
	if clientPtr == nil {
		return nil, fmt.Errorf("failed to create client")
	}

	client := &Client{
		config:    config,
		clientPtr: clientPtr,
		responses: make(map[uint64]chan *RpcResponse),
	}

	runtime.SetFinalizer(client, (*Client).finalize)
	return client, nil
}

// Connect establishes a connection to the server
func (c *Client) Connect(ctx context.Context) error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.closed {
		return fmt.Errorf("client is closed")
	}

	result := C.neo_client_connect(c.clientPtr)
	if result != 0 {
		return fmt.Errorf("failed to connect: error code %d", result)
	}

	return nil
}

// Call makes an RPC call to the specified service and method
func (c *Client) Call(ctx context.Context, serviceName, methodName string, payload []byte) ([]byte, error) {
	c.mu.RLock()
	if c.closed {
		c.mu.RUnlock()
		return nil, fmt.Errorf("client is closed")
	}
	c.mu.RUnlock()

	// Generate correlation ID
	c.mu.Lock()
	c.requestID++
	correlationID := c.requestID
	c.mu.Unlock()

	// Create response channel
	responseChan := make(chan *RpcResponse, 1)
	c.responseMu.Lock()
	c.responses[correlationID] = responseChan
	c.responseMu.Unlock()

	// Clean up response channel when done
	defer func() {
		c.responseMu.Lock()
		delete(c.responses, correlationID)
		c.responseMu.Unlock()
	}()

	// Prepare request
	serviceNameCStr := C.CString(serviceName)
	defer C.free(unsafe.Pointer(serviceNameCStr))

	methodNameCStr := C.CString(methodName)
	defer C.free(unsafe.Pointer(methodNameCStr))

	timeoutMs := uint32(c.config.Timeout.Milliseconds())

	// Make the call
	result := C.neo_client_call(
		c.clientPtr,
		serviceNameCStr,
		methodNameCStr,
		unsafe.Pointer(&payload[0]),
		C.size_t(len(payload)),
		C.uint64_t(correlationID),
		C.uint32_t(timeoutMs),
	)

	if result != 0 {
		return nil, fmt.Errorf("RPC call failed: error code %d", result)
	}

	// Wait for response
	select {
	case response := <-responseChan:
		if response.Error != nil {
			return nil, response.Error
		}
		return response.Payload, nil
	case <-ctx.Done():
		return nil, ctx.Err()
	}
}

// PublishEvent publishes an event to the specified topic
func (c *Client) PublishEvent(ctx context.Context, event *EventMessage) error {
	c.mu.RLock()
	if c.closed {
		c.mu.RUnlock()
		return fmt.Errorf("client is closed")
	}
	c.mu.RUnlock()

	topicCStr := C.CString(event.Topic)
	defer C.free(unsafe.Pointer(topicCStr))

	var partitionKeyCStr *C.char
	if event.PartitionKey != "" {
		partitionKeyCStr = C.CString(event.PartitionKey)
		defer C.free(unsafe.Pointer(partitionKeyCStr))
	}

	result := C.neo_client_publish_event(
		c.clientPtr,
		topicCStr,
		partitionKeyCStr,
		unsafe.Pointer(&event.Payload[0]),
		C.size_t(len(event.Payload)),
	)

	if result != 0 {
		return fmt.Errorf("failed to publish event: error code %d", result)
	}

	return nil
}

// Close closes the client connection
func (c *Client) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.closed {
		return nil
	}

	c.closed = true

	// Close all pending response channels
	c.responseMu.Lock()
	for _, ch := range c.responses {
		close(ch)
	}
	c.responses = make(map[uint64]chan *RpcResponse)
	c.responseMu.Unlock()

	if c.clientPtr != nil {
		C.neo_client_free(c.clientPtr)
		c.clientPtr = nil
	}

	return nil
}

// finalize is called by the garbage collector
func (c *Client) finalize() {
	c.Close()
}

// NewServer creates a new Neo server
func NewServer(config ServerConfig) (*Server, error) {
	configJSON, err := json.Marshal(config)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal config: %w", err)
	}

	configCStr := C.CString(string(configJSON))
	defer C.free(unsafe.Pointer(configCStr))

	serverPtr := C.neo_server_new(configCStr)
	if serverPtr == nil {
		return nil, fmt.Errorf("failed to create server")
	}

	server := &Server{
		config:        config,
		serverPtr:     serverPtr,
		services:      make(map[string]*ServiceDefinition),
		handlers:      make(map[string]RPCHandler),
		eventHandlers: make(map[string]EventHandler),
	}

	runtime.SetFinalizer(server, (*Server).finalize)
	return server, nil
}

// RegisterService registers a service with the server
func (s *Server) RegisterService(service *ServiceDefinition, rpcHandlers map[string]RPCHandler, eventHandlers map[string]EventHandler) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.closed {
		return fmt.Errorf("server is closed")
	}

	// Store service definition
	s.services[service.Name] = service

	// Store handlers
	for methodName, handler := range rpcHandlers {
		fullMethodName := service.Name + "." + methodName
		s.handlers[fullMethodName] = handler
	}

	for eventName, handler := range eventHandlers {
		fullEventName := service.Name + "." + eventName
		s.eventHandlers[fullEventName] = handler
	}

	// Register with C library
	serviceJSON, err := json.Marshal(service)
	if err != nil {
		return fmt.Errorf("failed to marshal service: %w", err)
	}

	serviceCStr := C.CString(string(serviceJSON))
	defer C.free(unsafe.Pointer(serviceCStr))

	result := C.neo_server_register_service(s.serverPtr, serviceCStr)
	if result != 0 {
		return fmt.Errorf("failed to register service: error code %d", result)
	}

	return nil
}

// Start starts the server
func (s *Server) Start(ctx context.Context) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.closed {
		return fmt.Errorf("server is closed")
	}

	addressCStr := C.CString(s.config.Address)
	defer C.free(unsafe.Pointer(addressCStr))

	result := C.neo_server_start(s.serverPtr, addressCStr)
	if result != 0 {
		return fmt.Errorf("failed to start server: error code %d", result)
	}

	return nil
}

// Stop stops the server
func (s *Server) Stop() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if s.closed {
		return nil
	}

	s.closed = true

	if s.serverPtr != nil {
		C.neo_server_free(s.serverPtr)
		s.serverPtr = nil
	}

	return nil
}

// finalize is called by the garbage collector
func (s *Server) finalize() {
	s.Stop()
}

// NewMetricsCollector creates a new metrics collector
func NewMetricsCollector() (*MetricsCollector, error) {
	collectorPtr := C.neo_metrics_collector_new()
	if collectorPtr == nil {
		return nil, fmt.Errorf("failed to create metrics collector")
	}

	collector := &MetricsCollector{
		collectorPtr: collectorPtr,
	}

	runtime.SetFinalizer(collector, (*MetricsCollector).finalize)
	return collector, nil
}

// Start starts the metrics collector
func (mc *MetricsCollector) Start() error {
	mc.mu.Lock()
	defer mc.mu.Unlock()

	result := C.neo_metrics_collector_start(mc.collectorPtr)
	if result != 0 {
		return fmt.Errorf("failed to start metrics collector: error code %d", result)
	}

	return nil
}

// GetSnapshot gets a metrics snapshot
func (mc *MetricsCollector) GetSnapshot() (*MetricsSnapshot, error) {
	mc.mu.RLock()
	defer mc.mu.RUnlock()

	jsonPtr := C.neo_metrics_collector_get_snapshot(mc.collectorPtr)
	if jsonPtr == nil {
		return nil, fmt.Errorf("failed to get metrics snapshot")
	}
	defer C.free(unsafe.Pointer(jsonPtr))

	jsonStr := C.GoString(jsonPtr)
	var snapshot MetricsSnapshot
	if err := json.Unmarshal([]byte(jsonStr), &snapshot); err != nil {
		return nil, fmt.Errorf("failed to unmarshal metrics snapshot: %w", err)
	}

	return &snapshot, nil
}

// GetHealthStatus gets the health status
func (mc *MetricsCollector) GetHealthStatus() (*HealthStatus, error) {
	mc.mu.RLock()
	defer mc.mu.RUnlock()

	jsonPtr := C.neo_metrics_collector_get_health_status(mc.collectorPtr)
	if jsonPtr == nil {
		return nil, fmt.Errorf("failed to get health status")
	}
	defer C.free(unsafe.Pointer(jsonPtr))

	jsonStr := C.GoString(jsonPtr)
	var status HealthStatus
	if err := json.Unmarshal([]byte(jsonStr), &status); err != nil {
		return nil, fmt.Errorf("failed to unmarshal health status: %w", err)
	}

	return &status, nil
}

// finalize is called by the garbage collector
func (mc *MetricsCollector) finalize() {
	if mc.collectorPtr != nil {
		C.neo_metrics_collector_free(mc.collectorPtr)
		mc.collectorPtr = nil
	}
}

// Utility functions

// DefaultClientConfig returns a default client configuration
func DefaultClientConfig() ClientConfig {
	return ClientConfig{
		ServerAddress:     "127.0.0.1:8080",
		Timeout:           30 * time.Second,
		MaxConcurrentReqs: 1000,
		EnableCompression: false,
		EnableAuth:        false,
	}
}

// DefaultServerConfig returns a default server configuration
func DefaultServerConfig() ServerConfig {
	return ServerConfig{
		Address:           "127.0.0.1:8080",
		MaxConcurrentReqs: 1000,
		EnableCompression: false,
		EnableAuth:        false,
		HeartbeatInterval: 30 * time.Second,
	}
}

// IsHealthy checks if the health status indicates a healthy system
func (hs *HealthStatus) IsHealthy() bool {
	return hs.Status == "healthy"
}

// IsDegraded checks if the health status indicates a degraded system
func (hs *HealthStatus) IsDegraded() bool {
	return hs.Status == "degraded"
}

// IsUnhealthy checks if the health status indicates an unhealthy system
func (hs *HealthStatus) IsUnhealthy() bool {
	return hs.Status == "unhealthy"
}