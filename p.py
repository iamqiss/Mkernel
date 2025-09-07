#!/usr/bin/env python3
"""
Messaging Kernel Project Scaffolder
Creates the complete directory structure and basic files for the Neo Messaging Kernel project.
"""

import os
from pathlib import Path
from datetime import datetime

def create_directory_structure():
    """Create the complete directory structure for Messaging Kernel."""
    
    # Define the project structure
    structure = {
        "messaging-kernel": {
            "core": {
                "neo-protocol": {
                    "src": {},
                    "tests": {},
                    "benches": {}
                },
                "qiss-format": {
                    "src": {},
                    "tests": {},
                    "benches": {}
                },
                "precursor-broker": {
                    "src": {},
                    "tests": {},
                    "benches": {}
                },
                "neoship-manifest": {
                    "src": {},
                    "tests": {},
                    "benches": {}
                }
            },
            "compiler": {
                "lexer": {
                    "src": {},
                    "tests": {}
                },
                "parser": {
                    "src": {},
                    "tests": {}
                },
                "codegen": {
                    "src": {},
                    "tests": {},
                    "templates": {
                        "rust": {},
                        "go": {},
                        "javascript": {},
                        "python": {}
                    }
                },
                "cli": {
                    "src": {},
                    "tests": {}
                }
            },
            "runtime": {
                "rust": {
                    "src": {},
                    "examples": {},
                    "tests": {}
                },
                "go": {
                    "pkg": {},
                    "examples": {},
                    "tests": {}
                },
                "javascript": {
                    "src": {},
                    "examples": {},
                    "tests": {}
                },
                "python": {
                    "messaging_kernel": {},
                    "examples": {},
                    "tests": {}
                }
            },
            "tools": {
                "neo-cli": {
                    "src": {},
                    "tests": {}
                },
                "debugger": {
                    "src": {},
                    "ui": {}
                },
                "benchmarks": {
                    "src": {},
                    "results": {},
                    "scripts": {}
                }
            },
            "examples": {
                "hello-world": {
                    "src": {}
                },
                "microservices": {
                    "user-service": {},
                    "order-service": {},
                    "notification-service": {}
                },
                "high-throughput": {
                    "producer": {},
                    "consumer": {},
                    "configs": {}
                }
            },
            "docs": {
                "spec": {},
                "guides": {},
                "api": {},
                "assets": {
                    "images": {},
                    "diagrams": {}
                }
            },
            "tests": {
                "integration": {
                    "src": {},
                    "fixtures": {}
                },
                "performance": {
                    "src": {},
                    "configs": {}
                },
                "compatibility": {
                    "src": {},
                    "test-cases": {}
                }
            }
        }
    }

    def create_dirs(base_path, structure):
        """Recursively create directory structure."""
        for name, subdirs in structure.items():
            current_path = base_path / name
            current_path.mkdir(parents=True, exist_ok=True)
            print(f"📁 Created: {current_path}")
            
            if subdirs:
                create_dirs(current_path, subdirs)
    
    # Create the directory structure
    base_path = Path.cwd()
    create_dirs(base_path, structure)
    
    return base_path / "messaging-kernel"

def create_root_files(project_root):
    """Create root-level configuration files."""
    
    # Create README.md
    readme_content = """# Neo Messaging Kernel

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
"""
    
    with open(project_root / "README.md", "w") as f:
        f.write(readme_content)
    print("📄 Created: README.md")
    
    # Create Cargo.toml workspace
    cargo_toml = """[workspace]
resolver = "2"
members = [
    "core/neo-protocol",
    "core/qiss-format", 
    "core/precursor-broker",
    "core/neoship-manifest",
    "compiler/lexer",
    "compiler/parser",
    "compiler/codegen",
    "compiler/cli",
    "runtime/rust",
    "tools/neo-cli",
    "tools/debugger",
    "tools/benchmarks"
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Neo Qiss"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/your-username/messaging-kernel"

[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
bytes = "1.0"
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
"""
    
    with open(project_root / "Cargo.toml", "w") as f:
        f.write(cargo_toml)
    print("📄 Created: Cargo.toml")
    
    # Create LICENSE
    license_content = """MIT License

Copyright (c) 2025 Neo Qiss

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""
    
    with open(project_root / "LICENSE", "w") as f:
        f.write(license_content)
    print("📄 Created: LICENSE")
    
    # Create CHANGELOG.md
    changelog_content = f"""# Changelog

All notable changes to the Neo Messaging Kernel will be documented in this file.

## [Unreleased] - {datetime.now().strftime('%Y-%m-%d')}

### Added
- Initial project structure
- Core components: Neo Protocol, Qiss Format, Precursor Broker, Neoship Manifest
- Compiler infrastructure for .neo files
- Multi-language runtime support (Rust, Go, JavaScript, Python)
- Developer tooling and CLI
- Comprehensive examples and documentation

### Changed
- N/A

### Fixed
- N/A

### Removed
- N/A
"""
    
    with open(project_root / "CHANGELOG.md", "w") as f:
        f.write(changelog_content)
    print("📄 Created: CHANGELOG.md")

def create_example_neo_file(project_root):
    """Create an example .neo service definition."""
    
    neo_example = """// Example Neo service definition
// This demonstrates the unified syntax for RPC, messaging, and configuration

service UserService {
    version = "1.0.0";
    namespace = "com.example.users";
    
    // Define data structures
    message User {
        id: u64;
        username: string;
        email: string;
        created_at: timestamp;
        optional profile_image: string;
    }
    
    message UserId {
        id: u64;
    }
    
    message UserCreatedEvent {
        user: User;
        created_by: string;
    }
    
    // RPC methods with optional queue configuration
    rpc GetUser(UserId) -> User {
        queue = "users.get";
        timeout = 5s;
        retry_policy = exponential_backoff(max_attempts: 3);
    }
    
    rpc CreateUser(User) -> UserId {
        queue = "users.create";
        timeout = 10s;
    }
    
    // Event publishing
    event UserCreated(UserCreatedEvent) {
        topic = "users.events";
        partition_key = user.id;
        retention = 7d;
    }
    
    // Service configuration
    config {
        max_concurrent_requests = 1000;
        queue_buffer_size = 10000;
        compression = lz4;
        serialization = qiss;
    }
}
"""
    
    example_dir = project_root / "examples" / "hello-world"
    with open(example_dir / "user-service.neo", "w") as f:
        f.write(neo_example)
    print("📄 Created: examples/hello-world/user-service.neo")

def create_neoship_example(project_root):
    """Create an example neoship.lift manifest."""
    
    neoship_content = """# Neoship.lift - Neo Messaging Kernel Deployment Manifest

manifest:
  name: "user-microservice"
  version: "1.0.0"
  description: "Example user management microservice"

services:
  user-service:
    source: "./user-service.neo"
    runtime: "rust"
    replicas: 3
    resources:
      cpu: "500m"
      memory: "512Mi"
    
  notification-service:
    source: "./notification-service.neo" 
    runtime: "go"
    replicas: 2
    resources:
      cpu: "250m"
      memory: "256Mi"

broker:
  precursor:
    persistence: true
    storage_path: "/data/precursor"
    max_message_size: "10MB"
    retention_policy: "7d"
    
queues:
  - name: "users.get"
    partition_count: 4
    replication_factor: 2
    
  - name: "users.create"
    partition_count: 8
    replication_factor: 3

topics:
  - name: "users.events"
    partition_count: 16
    replication_factor: 2
    retention: "30d"

network:
  port: 8080
  tls:
    enabled: true
    cert_path: "/etc/certs/server.crt"
    key_path: "/etc/certs/server.key"

monitoring:
  metrics:
    enabled: true
    endpoint: "/metrics"
  
  tracing:
    enabled: true
    sampling_rate: 0.1
    
logging:
  level: "info"
  format: "json"
  output: "stdout"
"""
    
    example_dir = project_root / "examples" / "hello-world"
    with open(example_dir / "neoship.lift", "w") as f:
        f.write(neoship_content)
    print("📄 Created: examples/hello-world/neoship.lift")

def create_core_cargo_files(project_root):
    """Create Cargo.toml files for core components."""
    
    core_components = [
        ("neo-protocol", "High-performance RPC protocol implementation"),
        ("qiss-format", "Binary serialization format optimized for speed"),
        ("precursor-broker", "Built-in message broker with zero-copy optimization"),
        ("neoship-manifest", "Deployment manifest parser and validator")
    ]
    
    for component, description in core_components:
        cargo_content = f"""[package]
name = "{component}"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "{description}"

[dependencies]
tokio.workspace = true
serde.workspace = true
bytes.workspace = true
thiserror.workspace = true
anyhow.workspace = true
tracing.workspace = true

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"

[[bench]]
name = "benchmarks"
harness = false
"""
        
        cargo_path = project_root / "core" / component / "Cargo.toml"
        with open(cargo_path, "w") as f:
            f.write(cargo_content)
        print(f"📄 Created: core/{component}/Cargo.toml")
        
        # Create basic lib.rs
        lib_content = f"""//! {description}
//! 
//! This crate is part of the Neo Messaging Kernel, providing {description.lower()}.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod error;

pub use error::Error;

/// Result type for this crate
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn basic_functionality() {{
        // TODO: Add basic functionality tests
        assert!(true);
    }}
}}
"""
        
        lib_path = project_root / "core" / component / "src" / "lib.rs"
        with open(lib_path, "w") as f:
            f.write(lib_content)
        print(f"📄 Created: core/{component}/src/lib.rs")
        
        # Create error.rs
        error_content = f"""//! Error types for {component}

use thiserror::Error;

/// Error types for {component}
#[derive(Error, Debug)]
pub enum Error {{
    /// IO error
    #[error("IO error: {{0}}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {{0}}")]
    Serialization(String),
    
    /// Protocol error
    #[error("Protocol error: {{0}}")]
    Protocol(String),
    
    /// Configuration error
    #[error("Configuration error: {{0}}")]
    Config(String),
}}
"""
        
        error_path = project_root / "core" / component / "src" / "error.rs"
        with open(error_path, "w") as f:
            f.write(error_content)
        print(f"📄 Created: core/{component}/src/error.rs")

def main():
    """Main scaffolding function."""
    print("🚀 Creating Neo Messaging Kernel project structure...\n")
    
    try:
        # Create directory structure
        project_root = create_directory_structure()
        
        print("\n📄 Creating configuration files...")
        create_root_files(project_root)
        
        print("\n📄 Creating example files...")
        create_example_neo_file(project_root)
        create_neoship_example(project_root)
        
        print("\n📄 Creating core component files...")
        create_core_cargo_files(project_root)
        
        print(f"\n✅ Successfully created Neo Messaging Kernel project structure!")
        print(f"📁 Project location: {project_root}")
        print("\n🎯 Next steps:")
        print("1. cd messaging-kernel")
        print("2. cargo check  # Verify the workspace builds")
        print("3. Start implementing the core components!")
        print("4. Check out examples/hello-world/ for sample .neo and neoship.lift files")
        
    except Exception as e:
        print(f"❌ Error creating project structure: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main())
