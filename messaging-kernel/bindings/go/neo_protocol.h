#ifndef NEO_PROTOCOL_H
#define NEO_PROTOCOL_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Error codes
#define NEO_SUCCESS 0
#define NEO_ERROR_INVALID_ARG -1
#define NEO_ERROR_OUT_OF_MEMORY -2
#define NEO_ERROR_NETWORK -3
#define NEO_ERROR_TIMEOUT -4
#define NEO_ERROR_AUTHENTICATION -5
#define NEO_ERROR_AUTHORIZATION -6
#define NEO_ERROR_SERVICE_NOT_FOUND -7
#define NEO_ERROR_METHOD_NOT_FOUND -8
#define NEO_ERROR_SERIALIZATION -9
#define NEO_ERROR_DESERIALIZATION -10
#define NEO_ERROR_INTERNAL -11

// Message types
typedef enum {
    NEO_MESSAGE_TYPE_RPC_REQUEST = 1,
    NEO_MESSAGE_TYPE_RPC_RESPONSE = 2,
    NEO_MESSAGE_TYPE_RPC_ERROR = 3,
    NEO_MESSAGE_TYPE_EVENT = 4,
    NEO_MESSAGE_TYPE_HEARTBEAT = 5,
    NEO_MESSAGE_TYPE_AUTH_REQUEST = 6,
    NEO_MESSAGE_TYPE_AUTH_RESPONSE = 7,
    NEO_MESSAGE_TYPE_CLOSE = 8
} neo_message_type_t;

// Client functions
typedef struct neo_client neo_client_t;

/**
 * Create a new Neo client
 * @param config JSON configuration string
 * @return Pointer to client instance or NULL on error
 */
neo_client_t* neo_client_new(const char* config);

/**
 * Connect to the server
 * @param client Client instance
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_client_connect(neo_client_t* client);

/**
 * Make an RPC call
 * @param client Client instance
 * @param service_name Service name
 * @param method_name Method name
 * @param payload Request payload
 * @param payload_size Payload size
 * @param correlation_id Correlation ID
 * @param timeout_ms Timeout in milliseconds
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_client_call(
    neo_client_t* client,
    const char* service_name,
    const char* method_name,
    const void* payload,
    size_t payload_size,
    uint64_t correlation_id,
    uint32_t timeout_ms
);

/**
 * Publish an event
 * @param client Client instance
 * @param topic Event topic
 * @param partition_key Partition key (can be NULL)
 * @param payload Event payload
 * @param payload_size Payload size
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_client_publish_event(
    neo_client_t* client,
    const char* topic,
    const char* partition_key,
    const void* payload,
    size_t payload_size
);

/**
 * Free client instance
 * @param client Client instance
 */
void neo_client_free(neo_client_t* client);

// Server functions
typedef struct neo_server neo_server_t;

/**
 * Create a new Neo server
 * @param config JSON configuration string
 * @return Pointer to server instance or NULL on error
 */
neo_server_t* neo_server_new(const char* config);

/**
 * Register a service
 * @param server Server instance
 * @param service_def JSON service definition
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_server_register_service(neo_server_t* server, const char* service_def);

/**
 * Start the server
 * @param server Server instance
 * @param address Server address
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_server_start(neo_server_t* server, const char* address);

/**
 * Stop the server
 * @param server Server instance
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_server_stop(neo_server_t* server);

/**
 * Free server instance
 * @param server Server instance
 */
void neo_server_free(neo_server_t* server);

// Metrics collector functions
typedef struct neo_metrics_collector neo_metrics_collector_t;

/**
 * Create a new metrics collector
 * @return Pointer to metrics collector instance or NULL on error
 */
neo_metrics_collector_t* neo_metrics_collector_new(void);

/**
 * Start the metrics collector
 * @param collector Metrics collector instance
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_metrics_collector_start(neo_metrics_collector_t* collector);

/**
 * Get metrics snapshot
 * @param collector Metrics collector instance
 * @return JSON string with metrics data (must be freed by caller)
 */
char* neo_metrics_collector_get_snapshot(neo_metrics_collector_t* collector);

/**
 * Get health status
 * @param collector Metrics collector instance
 * @return JSON string with health status (must be freed by caller)
 */
char* neo_metrics_collector_get_health_status(neo_metrics_collector_t* collector);

/**
 * Record RPC request
 * @param collector Metrics collector instance
 */
void neo_metrics_collector_record_rpc_request(neo_metrics_collector_t* collector);

/**
 * Record RPC response
 * @param collector Metrics collector instance
 * @param latency_us Latency in microseconds
 */
void neo_metrics_collector_record_rpc_response(neo_metrics_collector_t* collector, uint64_t latency_us);

/**
 * Record RPC error
 * @param collector Metrics collector instance
 * @param method_name Method name
 */
void neo_metrics_collector_record_rpc_error(neo_metrics_collector_t* collector, const char* method_name);

/**
 * Record bytes sent
 * @param collector Metrics collector instance
 * @param bytes Number of bytes sent
 */
void neo_metrics_collector_record_bytes_sent(neo_metrics_collector_t* collector, uint64_t bytes);

/**
 * Record bytes received
 * @param collector Metrics collector instance
 * @param bytes Number of bytes received
 */
void neo_metrics_collector_record_bytes_received(neo_metrics_collector_t* collector, uint64_t bytes);

/**
 * Increment active connections
 * @param collector Metrics collector instance
 */
void neo_metrics_collector_increment_connections(neo_metrics_collector_t* collector);

/**
 * Decrement active connections
 * @param collector Metrics collector instance
 */
void neo_metrics_collector_decrement_connections(neo_metrics_collector_t* collector);

/**
 * Record service registration
 * @param collector Metrics collector instance
 */
void neo_metrics_collector_record_service_registration(neo_metrics_collector_t* collector);

/**
 * Free metrics collector instance
 * @param collector Metrics collector instance
 */
void neo_metrics_collector_free(neo_metrics_collector_t* collector);

// Utility functions

/**
 * Get version string
 * @return Version string
 */
const char* neo_get_version(void);

/**
 * Get protocol version
 * @return Protocol version number
 */
uint8_t neo_get_protocol_version(void);

/**
 * Get maximum message size
 * @return Maximum message size in bytes
 */
size_t neo_get_max_message_size(void);

/**
 * Initialize the Neo library
 * @return NEO_SUCCESS on success, error code on failure
 */
int neo_init(void);

/**
 * Shutdown the Neo library
 */
void neo_shutdown(void);

#ifdef __cplusplus
}
#endif

#endif // NEO_PROTOCOL_H