# Neo Messaging Kernel

A high-performance, integrated messaging platform that combines protocol, serialization, message brokering, and deployment into a unified stack.

## Components

- **Neo Protocol** (.neo files) - Custom high-performance RPC protocol
- **Qiss Binary Format** - Optimized binary serialization format  
- **Precursor Broker** - Built-in message broker with zero-copy optimization
- **Neoship.lift** - Unified manifest and deployment system

## Quick Start

```bash
# Install the Neo CLI
cargo install neo-cli

# Create a new service
neo new my-service
cd my-service

# Define your service in service.neo
# Build and run
neo build
neo run --manifest neoship.lift
```

## Performance Goals

- Sub-microsecond serialization/deserialization
- Zero-copy message passing where possible
- Integrated stack optimizations across all components
- Minimal memory allocation and GC pressure

## Architecture

The Neo Messaging Kernel is designed as an integrated platform where all components are optimized to work together, eliminating the overhead typically found at component boundaries in traditional microservice stacks.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
