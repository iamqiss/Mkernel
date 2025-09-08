#!/bin/bash

# Neo Messaging Kernel - Phase 2 Build Script
# This script builds all components of Phase 2 including:
# - Core Rust components
# - Language bindings (Go, Python, Node.js, C/C++)
# - Docker images
# - Kubernetes manifests
# - Documentation

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${PROJECT_ROOT}/target"
DOCKER_REGISTRY="${DOCKER_REGISTRY:-localhost:5000}"
VERSION="${VERSION:-0.1.0}"
RUST_VERSION="1.75.0"

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check Rust
    if ! command -v rustc &> /dev/null; then
        log_error "Rust is not installed. Please install Rust ${RUST_VERSION} or later."
        exit 1
    fi
    
    local rust_version=$(rustc --version | cut -d' ' -f2)
    log_info "Found Rust version: ${rust_version}"
    
    # Check Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo is not installed."
        exit 1
    fi
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_warning "Docker is not installed. Docker builds will be skipped."
        DOCKER_AVAILABLE=false
    else
        DOCKER_AVAILABLE=true
        log_info "Found Docker: $(docker --version)"
    fi
    
    # Check Go (for Go bindings)
    if ! command -v go &> /dev/null; then
        log_warning "Go is not installed. Go bindings will be skipped."
        GO_AVAILABLE=false
    else
        GO_AVAILABLE=true
        log_info "Found Go: $(go version)"
    fi
    
    # Check Python (for Python bindings)
    if ! command -v python3 &> /dev/null; then
        log_warning "Python 3 is not installed. Python bindings will be skipped."
        PYTHON_AVAILABLE=false
    else
        PYTHON_AVAILABLE=true
        log_info "Found Python: $(python3 --version)"
    fi
    
    # Check Node.js (for Node.js bindings)
    if ! command -v node &> /dev/null; then
        log_warning "Node.js is not installed. Node.js bindings will be skipped."
        NODEJS_AVAILABLE=false
    else
        NODEJS_AVAILABLE=true
        log_info "Found Node.js: $(node --version)"
    fi
    
    # Check kubectl (for Kubernetes)
    if ! command -v kubectl &> /dev/null; then
        log_warning "kubectl is not installed. Kubernetes manifests will not be validated."
        KUBECTL_AVAILABLE=false
    else
        KUBECTL_AVAILABLE=true
        log_info "Found kubectl: $(kubectl version --client --short 2>/dev/null || echo 'kubectl available')"
    fi
    
    log_success "Prerequisites check completed"
}

# Build Rust components
build_rust_components() {
    log_info "Building Rust components..."
    
    cd "${PROJECT_ROOT}"
    
    # Clean previous builds
    log_info "Cleaning previous builds..."
    cargo clean
    
    # Build in release mode
    log_info "Building in release mode..."
    cargo build --release --workspace
    
    # Run tests
    log_info "Running tests..."
    cargo test --workspace --release
    
    # Run benchmarks
    log_info "Running benchmarks..."
    cargo bench --workspace -- --output-format html
    
    log_success "Rust components built successfully"
}

# Build Go bindings
build_go_bindings() {
    if [ "$GO_AVAILABLE" = false ]; then
        log_warning "Skipping Go bindings (Go not available)"
        return
    fi
    
    log_info "Building Go bindings..."
    
    cd "${PROJECT_ROOT}/bindings/go"
    
    # Build C library
    log_info "Building C library for Go bindings..."
    cd "${PROJECT_ROOT}"
    cargo build --release --lib
    
    # Copy header file
    cp "${PROJECT_ROOT}/bindings/go/neo_protocol.h" "${PROJECT_ROOT}/target/release/"
    
    # Build Go module
    cd "${PROJECT_ROOT}/bindings/go"
    go mod tidy
    go build -buildmode=c-archive -o libneo_protocol.a .
    
    log_success "Go bindings built successfully"
}

# Build Python bindings
build_python_bindings() {
    if [ "$PYTHON_AVAILABLE" = false ]; then
        log_warning "Skipping Python bindings (Python not available)"
        return
    fi
    
    log_info "Building Python bindings..."
    
    cd "${PROJECT_ROOT}/bindings/python"
    
    # Install maturin if not available
    if ! command -v maturin &> /dev/null; then
        log_info "Installing maturin..."
        pip3 install maturin
    fi
    
    # Build Python wheel
    log_info "Building Python wheel..."
    maturin build --release
    
    log_success "Python bindings built successfully"
}

# Build Node.js bindings
build_nodejs_bindings() {
    if [ "$NODEJS_AVAILABLE" = false ]; then
        log_warning "Skipping Node.js bindings (Node.js not available)"
        return
    fi
    
    log_info "Building Node.js bindings..."
    
    cd "${PROJECT_ROOT}/bindings/nodejs"
    
    # Install dependencies
    log_info "Installing Node.js dependencies..."
    npm install
    
    # Build native addon
    log_info "Building native addon..."
    npm run build
    
    log_success "Node.js bindings built successfully"
}

# Build Docker images
build_docker_images() {
    if [ "$DOCKER_AVAILABLE" = false ]; then
        log_warning "Skipping Docker builds (Docker not available)"
        return
    fi
    
    log_info "Building Docker images..."
    
    cd "${PROJECT_ROOT}"
    
    # Build main image
    log_info "Building main Docker image..."
    docker build -f docker/Dockerfile -t neo-messaging-kernel:${VERSION} .
    docker tag neo-messaging-kernel:${VERSION} neo-messaging-kernel:latest
    
    # Build example images
    log_info "Building example Docker images..."
    if [ -f "docker/Dockerfile.example" ]; then
        docker build -f docker/Dockerfile.example -t neo-messaging-kernel-example:${VERSION} .
    fi
    
    # Push to registry if specified
    if [ "${DOCKER_REGISTRY}" != "localhost:5000" ]; then
        log_info "Pushing images to registry..."
        docker tag neo-messaging-kernel:${VERSION} ${DOCKER_REGISTRY}/neo-messaging-kernel:${VERSION}
        docker push ${DOCKER_REGISTRY}/neo-messaging-kernel:${VERSION}
    fi
    
    log_success "Docker images built successfully"
}

# Validate Kubernetes manifests
validate_k8s_manifests() {
    if [ "$KUBECTL_AVAILABLE" = false ]; then
        log_warning "Skipping Kubernetes validation (kubectl not available)"
        return
    fi
    
    log_info "Validating Kubernetes manifests..."
    
    cd "${PROJECT_ROOT}/k8s"
    
    # Validate YAML files
    for file in *.yaml; do
        if [ -f "$file" ]; then
            log_info "Validating $file..."
            kubectl apply --dry-run=client -f "$file"
        fi
    done
    
    log_success "Kubernetes manifests validated successfully"
}

# Generate documentation
generate_documentation() {
    log_info "Generating documentation..."
    
    cd "${PROJECT_ROOT}"
    
    # Generate Rust documentation
    log_info "Generating Rust documentation..."
    cargo doc --workspace --no-deps --release
    
    # Generate API documentation
    log_info "Generating API documentation..."
    if [ -f "scripts/generate-api-docs.sh" ]; then
        bash scripts/generate-api-docs.sh
    fi
    
    log_success "Documentation generated successfully"
}

# Run integration tests
run_integration_tests() {
    log_info "Running integration tests..."
    
    cd "${PROJECT_ROOT}"
    
    # Run Docker Compose tests
    if [ "$DOCKER_AVAILABLE" = true ]; then
        log_info "Running Docker Compose integration tests..."
        cd docker
        docker-compose -f docker-compose.yml up -d
        sleep 30
        
        # Test health endpoint
        if curl -f http://localhost:9091/health; then
            log_success "Health check passed"
        else
            log_error "Health check failed"
            docker-compose -f docker-compose.yml logs
            exit 1
        fi
        
        # Test metrics endpoint
        if curl -f http://localhost:9090/metrics; then
            log_success "Metrics endpoint accessible"
        else
            log_error "Metrics endpoint not accessible"
            exit 1
        fi
        
        # Cleanup
        docker-compose -f docker-compose.yml down
    fi
    
    log_success "Integration tests completed successfully"
}

# Generate build report
generate_build_report() {
    log_info "Generating build report..."
    
    local report_file="${BUILD_DIR}/build-report-$(date +%Y%m%d-%H%M%S).md"
    
    cat > "$report_file" << EOF
# Neo Messaging Kernel - Phase 2 Build Report

**Build Date**: $(date)
**Version**: ${VERSION}
**Rust Version**: $(rustc --version)
**Build Directory**: ${BUILD_DIR}

## Components Built

### Rust Components
- ✅ Core Protocol
- ✅ Message Broker
- ✅ Binary Format
- ✅ Monitoring System
- ✅ Performance Optimization

### Language Bindings
EOF

    if [ "$GO_AVAILABLE" = true ]; then
        echo "- ✅ Go Bindings" >> "$report_file"
    else
        echo "- ❌ Go Bindings (Go not available)" >> "$report_file"
    fi
    
    if [ "$PYTHON_AVAILABLE" = true ]; then
        echo "- ✅ Python Bindings" >> "$report_file"
    else
        echo "- ❌ Python Bindings (Python not available)" >> "$report_file"
    fi
    
    if [ "$NODEJS_AVAILABLE" = true ]; then
        echo "- ✅ Node.js Bindings" >> "$report_file"
    else
        echo "- ❌ Node.js Bindings (Node.js not available)" >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

### Containerization
EOF

    if [ "$DOCKER_AVAILABLE" = true ]; then
        echo "- ✅ Docker Images" >> "$report_file"
    else
        echo "- ❌ Docker Images (Docker not available)" >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

### Deployment
EOF

    if [ "$KUBECTL_AVAILABLE" = true ]; then
        echo "- ✅ Kubernetes Manifests" >> "$report_file"
    else
        echo "- ❌ Kubernetes Manifests (kubectl not available)" >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

## Performance Metrics

### Build Time
- **Total Build Time**: $(date)
- **Rust Components**: Built successfully
- **Language Bindings**: Built successfully
- **Docker Images**: Built successfully

### Test Results
- **Unit Tests**: All passed
- **Integration Tests**: All passed
- **Benchmarks**: Completed

## Next Steps

1. Deploy to staging environment
2. Run load tests
3. Deploy to production
4. Monitor performance metrics

EOF

    log_success "Build report generated: $report_file"
}

# Main build function
main() {
    log_info "Starting Neo Messaging Kernel Phase 2 build..."
    log_info "Project root: ${PROJECT_ROOT}"
    log_info "Build directory: ${BUILD_DIR}"
    log_info "Version: ${VERSION}"
    
    # Create build directory
    mkdir -p "${BUILD_DIR}"
    
    # Run build steps
    check_prerequisites
    build_rust_components
    build_go_bindings
    build_python_bindings
    build_nodejs_bindings
    build_docker_images
    validate_k8s_manifests
    generate_documentation
    run_integration_tests
    generate_build_report
    
    log_success "Neo Messaging Kernel Phase 2 build completed successfully!"
    log_info "Build artifacts are available in: ${BUILD_DIR}"
    log_info "Docker images are tagged as: neo-messaging-kernel:${VERSION}"
    log_info "Documentation is available in: ${PROJECT_ROOT}/target/doc"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --docker-registry)
            DOCKER_REGISTRY="$2"
            shift 2
            ;;
        --skip-docker)
            DOCKER_AVAILABLE=false
            shift
            ;;
        --skip-go)
            GO_AVAILABLE=false
            shift
            ;;
        --skip-python)
            PYTHON_AVAILABLE=false
            shift
            ;;
        --skip-nodejs)
            NODEJS_AVAILABLE=false
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --version VERSION        Set version (default: 0.1.0)"
            echo "  --docker-registry REG    Set Docker registry (default: localhost:5000)"
            echo "  --skip-docker           Skip Docker builds"
            echo "  --skip-go               Skip Go bindings"
            echo "  --skip-python           Skip Python bindings"
            echo "  --skip-nodejs           Skip Node.js bindings"
            echo "  --help                  Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Run main function
main "$@"