#!/bin/bash

# Neo Messaging Kernel - Phase 3 Build Script
# This script builds the complete Phase 3 enterprise implementation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="neo-messaging-kernel"
PHASE="Phase 3"
FEATURES="enterprise,security,multicluster,compliance"
BUILD_TYPE="release"
TARGET_DIR="target"
DOCKER_IMAGE="neo-messaging-kernel:phase3"

# Functions
print_header() {
    echo -e "${BLUE}"
    echo "=========================================="
    echo "  Neo Messaging Kernel - $PHASE Build"
    echo "=========================================="
    echo -e "${NC}"
}

print_step() {
    echo -e "${YELLOW}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    print_step "Checking prerequisites..."
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo is not installed. Please install Rust first."
        exit 1
    fi
    
    # Check Rust version
    RUST_VERSION=$(cargo --version | cut -d' ' -f2)
    print_info "Rust version: $RUST_VERSION"
    
    # Check if Docker is installed
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    # Check if Docker Compose is installed
    if ! command -v docker-compose &> /dev/null; then
        print_error "Docker Compose is not installed. Please install Docker Compose first."
        exit 1
    fi
    
    print_success "All prerequisites are met"
}

# Clean previous builds
clean_build() {
    print_step "Cleaning previous builds..."
    
    if [ -d "$TARGET_DIR" ]; then
        rm -rf "$TARGET_DIR"
        print_info "Removed $TARGET_DIR directory"
    fi
    
    # Clean Docker images
    if docker images | grep -q "$DOCKER_IMAGE"; then
        docker rmi "$DOCKER_IMAGE" 2>/dev/null || true
        print_info "Removed Docker image: $DOCKER_IMAGE"
    fi
    
    print_success "Clean completed"
}

# Update dependencies
update_dependencies() {
    print_step "Updating dependencies..."
    
    cargo update
    print_success "Dependencies updated"
}

# Run tests
run_tests() {
    print_step "Running tests..."
    
    # Run unit tests
    print_info "Running unit tests..."
    cargo test --workspace --features "$FEATURES"
    
    # Run integration tests
    print_info "Running integration tests..."
    cargo test --test integration_tests --features "$FEATURES"
    
    # Run enterprise tests
    print_info "Running enterprise tests..."
    cargo test --test enterprise_tests --features "$FEATURES"
    
    print_success "All tests passed"
}

# Run benchmarks
run_benchmarks() {
    print_step "Running benchmarks..."
    
    # Run protocol benchmarks
    print_info "Running protocol benchmarks..."
    cargo bench --bench protocol_benchmarks --features "$FEATURES"
    
    # Run broker benchmarks
    print_info "Running broker benchmarks..."
    cargo bench --bench broker_benchmarks --features "$FEATURES"
    
    # Run comprehensive benchmarks
    print_info "Running comprehensive benchmarks..."
    cargo bench --bench comprehensive_benchmarks --features "$FEATURES"
    
    print_success "All benchmarks completed"
}

# Build the project
build_project() {
    print_step "Building project..."
    
    # Build in release mode with enterprise features
    print_info "Building with features: $FEATURES"
    cargo build --release --features "$FEATURES"
    
    print_success "Project built successfully"
}

# Build Docker image
build_docker() {
    print_step "Building Docker image..."
    
    # Build Docker image
    docker build -f docker/Dockerfile -t "$DOCKER_IMAGE" .
    
    print_success "Docker image built: $DOCKER_IMAGE"
}

# Run linting
run_linting() {
    print_step "Running linting..."
    
    # Install clippy if not already installed
    if ! cargo clippy --version &> /dev/null; then
        print_info "Installing clippy..."
        rustup component add clippy
    fi
    
    # Run clippy
    cargo clippy --workspace --features "$FEATURES" -- -D warnings
    
    print_success "Linting completed"
}

# Run security audit
run_security_audit() {
    print_step "Running security audit..."
    
    # Install cargo-audit if not already installed
    if ! cargo audit --version &> /dev/null; then
        print_info "Installing cargo-audit..."
        cargo install cargo-audit
    fi
    
    # Run security audit
    cargo audit
    
    print_success "Security audit completed"
}

# Generate documentation
generate_docs() {
    print_step "Generating documentation..."
    
    # Generate API documentation
    cargo doc --workspace --features "$FEATURES" --no-deps
    
    print_success "Documentation generated"
}

# Create release package
create_release_package() {
    print_step "Creating release package..."
    
    RELEASE_DIR="release/neo-messaging-kernel-phase3"
    mkdir -p "$RELEASE_DIR"
    
    # Copy binaries
    cp target/release/neo-cli "$RELEASE_DIR/"
    cp target/release/neo-debugger "$RELEASE_DIR/"
    
    # Copy configuration files
    cp examples/enterprise-config.toml "$RELEASE_DIR/"
    cp docker/docker-compose.yml "$RELEASE_DIR/"
    cp k8s/*.yaml "$RELEASE_DIR/"
    
    # Copy documentation
    cp README.md "$RELEASE_DIR/"
    cp PHASE3_IMPLEMENTATION.md "$RELEASE_DIR/"
    cp LICENSE "$RELEASE_DIR/"
    
    # Create release notes
    cat > "$RELEASE_DIR/RELEASE_NOTES.md" << EOF
# Neo Messaging Kernel - Phase 3 Release Notes

## Version: Phase 3 Enterprise
## Build Date: $(date)

## New Features

### Advanced Security
- mTLS (Mutual TLS) authentication
- OAuth2/OIDC integration
- SAML authentication
- Advanced RBAC with hierarchical roles
- Comprehensive audit logging
- Advanced threat detection

### Multi-Cluster Support
- Cross-cluster replication
- Service mesh integration (Istio, Linkerd, Consul Connect)
- Cluster federation and discovery
- Load balancing across clusters
- Disaster recovery and failover

### Enterprise Integrations
- LDAP/Active Directory integration
- Enterprise SSO
- Compliance framework support
- SIEM integration
- Legacy system integration

### SLA and Compliance
- Service Level Agreement monitoring
- Compliance framework support (SOC2, GDPR, HIPAA, PCI DSS)
- Real-time alerting and notification
- Compliance reporting and audit trails
- Data governance and policy enforcement

## Performance Improvements
- Sub-microsecond serialization
- Zero-copy message passing
- High-performance multi-cluster replication
- Enterprise-grade security with minimal overhead

## Installation
1. Extract the release package
2. Configure using enterprise-config.toml
3. Deploy using Docker Compose or Kubernetes
4. Enable enterprise features as needed

## Documentation
- README.md: Basic usage and setup
- PHASE3_IMPLEMENTATION.md: Detailed implementation guide
- enterprise-config.toml: Complete configuration example
EOF

    # Create tarball
    tar -czf "neo-messaging-kernel-phase3.tar.gz" -C release neo-messaging-kernel-phase3
    
    print_success "Release package created: neo-messaging-kernel-phase3.tar.gz"
}

# Run integration tests with Docker
run_docker_tests() {
    print_step "Running Docker integration tests..."
    
    # Start services
    docker-compose -f docker/docker-compose.yml up -d
    
    # Wait for services to be ready
    sleep 30
    
    # Run integration tests
    cargo test --test docker_integration_tests --features "$FEATURES"
    
    # Stop services
    docker-compose -f docker/docker-compose.yml down
    
    print_success "Docker integration tests completed"
}

# Main build process
main() {
    print_header
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --clean)
                CLEAN_ONLY=true
                shift
                ;;
            --test-only)
                TEST_ONLY=true
                shift
                ;;
            --bench-only)
                BENCH_ONLY=true
                shift
                ;;
            --docker-only)
                DOCKER_ONLY=true
                shift
                ;;
            --no-docker)
                NO_DOCKER=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo "Options:"
                echo "  --clean       Clean build artifacts only"
                echo "  --test-only   Run tests only"
                echo "  --bench-only  Run benchmarks only"
                echo "  --docker-only Build Docker image only"
                echo "  --no-docker   Skip Docker operations"
                echo "  --help        Show this help message"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Check prerequisites
    check_prerequisites
    
    # Clean build if requested
    if [ "$CLEAN_ONLY" = true ]; then
        clean_build
        exit 0
    fi
    
    # Clean previous builds
    clean_build
    
    # Update dependencies
    update_dependencies
    
    # Run linting
    run_linting
    
    # Run security audit
    run_security_audit
    
    # Run tests if requested
    if [ "$TEST_ONLY" = true ]; then
        run_tests
        exit 0
    fi
    
    # Run benchmarks if requested
    if [ "$BENCH_ONLY" = true ]; then
        run_benchmarks
        exit 0
    fi
    
    # Build the project
    build_project
    
    # Run tests
    run_tests
    
    # Run benchmarks
    run_benchmarks
    
    # Build Docker image if not disabled
    if [ "$NO_DOCKER" != true ]; then
        build_docker
        
        # Run Docker tests
        run_docker_tests
    fi
    
    # Generate documentation
    generate_docs
    
    # Create release package
    create_release_package
    
    print_success "Phase 3 build completed successfully!"
    print_info "Release package: neo-messaging-kernel-phase3.tar.gz"
    print_info "Docker image: $DOCKER_IMAGE"
    print_info "Documentation: target/doc/index.html"
}

# Run main function
main "$@"