# Contributing to Neo Messaging Kernel

Welcome to the Neo Messaging Kernel project! We're excited to have you contribute to building the next generation of high-performance messaging infrastructure. This guide will help you get started and ensure your contributions align with the project's goals and standards.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Architecture Overview](#architecture-overview)
- [Contributing Guidelines](#contributing-guidelines)
- [Performance Standards](#performance-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation Standards](#documentation-standards)
- [Submitting Changes](#submitting-changes)
- [Community](#community)

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to [conduct@neo-messaging-kernel.dev](mailto:conduct@neo-messaging-kernel.dev).

### Our Standards

- **Be respectful** - Treat everyone with respect, regardless of experience level
- **Be collaborative** - We're building something amazing together
- **Be performance-minded** - Every change should consider performance impact
- **Be security-conscious** - Think about security implications of your code
- **Be thorough** - Test your changes and document your decisions

## Getting Started

### What Can You Contribute?

- **Core Implementation** - Rust code for protocols, serialization, and message broking
- **Language Bindings** - Go, Python, JavaScript, C++ runtime implementations
- **Tooling** - CLI tools, debuggers, profilers, and development utilities
- **Documentation** - API docs, guides, tutorials, and examples
- **Testing** - Unit tests, integration tests, benchmarks, and fuzzing
- **Performance Analysis** - Benchmarks, profiling, and optimization opportunities

### Good First Issues

Look for issues labeled:
- `good-first-issue` - Perfect for newcomers
- `help-wanted` - We need community help on these
- `documentation` - Documentation improvements
- `performance` - Performance optimization opportunities

## Development Setup

### Prerequisites

```bash
# Required
rustc 1.75+
git

# Optional (for language bindings)
go 1.21+
python 3.9+
node.js 18+
```

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/neo-qiss/messaging-kernel.git
cd messaging-kernel

# Build all Rust components
cargo build --workspace

# Run all tests
cargo test --workspace

# Run benchmarks (takes time!)
cargo bench --workspace

# Check formatting and lints
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

### Development Tools

We recommend these tools for contributing:

```bash
# Install development dependencies
cargo install cargo-watch      # Auto-rebuild on changes
cargo install cargo-expand     # Expand macros for debugging
cargo install cargo-asm        # Inspect generated assembly
cargo install criterion        # Benchmarking
cargo install cargo-fuzz       # Fuzzing support

# For performance analysis
cargo install perf-event       # CPU profiling
cargo install heaptrack       # Memory profiling
```

### Editor Setup

**VS Code:**
- Install `rust-analyzer` extension
- Install `Better TOML` extension
- Configure `rust-analyzer` for workspace mode

**Vim/Neovim:**
- Install `rust.vim` and configure LSP with rust-analyzer

**IntelliJ:**
- Install the official Rust plugin

## Architecture Overview

Understanding the architecture is crucial for meaningful contributions.

### Component Hierarchy

```
Neo Messaging Kernel
├── Core Layer (Rust)
│   ├── neo-protocol/          # Protocol implementation
│   ├── qiss-format/           # Binary serialization
│   ├── precursor-broker/      # Message broker
│   └── neoship-manifest/      # Manifest handling
├── Compiler Layer
│   ├── lexer/                 # .neo file lexer
│   ├── parser/                # Syntax analysis
│   └── codegen/               # Code generation
└── Runtime Layer
    ├── rust/                  # Native runtime
    ├── go/                    # Go bindings (CGO)
    ├── python/                # Python extensions
    └── javascript/            # WASM + native
```

### Key Design Principles

1. **Zero-Copy Philosophy** - Avoid memory copies wherever possible
2. **Integrated Optimization** - Components designed to work together
3. **Performance First** - Every decision evaluated for performance impact
4. **Memory Safety** - Leverage Rust's safety without sacrificing speed
5. **Async by Default** - Built on tokio for high concurrency

### Critical Performance Paths

These code paths are performance-critical and require extra scrutiny:

- **Message serialization/deserialization** (qiss-format)
- **Network I/O handling** (neo-protocol)
- **Message routing in broker** (precursor-broker)
- **Memory allocation patterns** (everywhere)
- **Lock-free data structures** (broker and protocol)

## Contributing Guidelines

### Code Style

We follow Rust standard conventions with some additions:

```rust
// Use explicit lifetimes when they aid understanding
pub fn process_message<'a>(buf: &'a [u8]) -> Result<Message<'a>, Error> {
    // Prefer zero-copy when possible
    Message::from_bytes(buf)  // Instead of parsing into owned data
}

// Document performance characteristics
/// Deserializes a message in O(1) time through zero-copy parsing.
/// 
/// # Performance
/// This function performs no allocations and completes in ~15ns
/// for typical message sizes.
pub fn deserialize_fast(bytes: &[u8]) -> Result<&Message, Error> {
    // Implementation
}
```

**Formatting:**
```bash
# Before committing
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): short description

Longer description if needed.

- Bullet points for details
- Performance impact notes
- Breaking change warnings

Closes #123
```

**Types:**
- `feat:` - New features
- `fix:` - Bug fixes
- `perf:` - Performance improvements
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `docs:` - Documentation changes
- `build:` - Build system changes

**Examples:**
```
feat(qiss): implement zero-copy string deserialization

Add support for zero-copy string parsing that avoids allocations
for UTF-8 validated byte slices. Improves deserialization performance
by 40% for string-heavy payloads.

- Add safe wrapper around raw byte slice validation
- Extend benchmark suite for string-heavy messages
- Update documentation with performance characteristics

Closes #156
```

### Branch Naming

- `feature/component-description` - New features
- `fix/issue-description` - Bug fixes
- `perf/optimization-description` - Performance improvements
- `docs/documentation-update` - Documentation changes

### Performance Standards

**Every contribution must consider performance impact:**

1. **Benchmark New Code**
   ```bash
   # Add benchmarks for new functionality
   cargo bench --bench your_component
   
   # Compare before/after performance
   git checkout main
   cargo bench --bench existing_bench > before.txt
   git checkout your-branch
   cargo bench --bench existing_bench > after.txt
   ```

2. **Profile Memory Usage**
   ```bash
   # Check for memory leaks and allocation patterns
   valgrind --tool=memcheck target/debug/your_binary
   
   # Or use heaptrack for detailed analysis
   heaptrack target/debug/your_binary
   ```

3. **Assembly Inspection for Hot Paths**
   ```bash
   # Inspect generated assembly for critical functions
   cargo asm your_crate::critical_function --rust
   ```

**Performance Regression Policy:**
- **>5% regression** requires explicit justification and maintainer approval
- **>10% regression** requires alternative implementation or redesign
- **Any regression** in critical paths (serialization, networking) needs careful review

### Memory Safety Standards

While we prioritize performance, safety is non-negotiable:

```rust
// Acceptable: Well-documented unsafe with clear invariants
/// # Safety
/// Caller must ensure `bytes` contains valid UTF-8 and lives for 'a
pub unsafe fn string_from_bytes_unchecked<'a>(bytes: &'a [u8]) -> &'a str {
    std::str::from_utf8_unchecked(bytes)
}

// Preferred: Safe wrapper around unsafe internals
pub fn string_from_bytes<'a>(bytes: &'a [u8]) -> Result<&'a str, Utf8Error> {
    std::str::from_utf8(bytes)
}
```

**Unsafe Code Requirements:**
1. Document safety invariants clearly
2. Provide safe wrappers when possible
3. Add comprehensive tests including edge cases
4. Consider using `miri` for unsafe code validation:
   ```bash
   cargo +nightly miri test
   ```

## Testing Requirements

### Test Categories

1. **Unit Tests** - Test individual components in isolation
2. **Integration Tests** - Test component interactions
3. **Property Tests** - Test invariants with random inputs (using `proptest`)
4. **Benchmark Tests** - Ensure performance characteristics
5. **Fuzz Tests** - Find edge cases with random inputs

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn basic_functionality() {
        // Basic happy path test
    }
    
    #[test] 
    fn error_conditions() {
        // Test error handling
    }
    
    #[test]
    fn edge_cases() {
        // Test boundary conditions
    }
}

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn serialization_roundtrip(data in any::<Vec<u8>>()) {
            let serialized = serialize(&data)?;
            let deserialized = deserialize(&serialized)?;
            prop_assert_eq!(data, deserialized);
        }
    }
}

#[cfg(test)]
mod benchmarks {
    use criterion::{black_box, Criterion};
    
    fn benchmark_serialization(c: &mut Criterion) {
        let data = generate_test_data();
        c.bench_function("serialize_1kb", |b| {
            b.iter(|| serialize(black_box(&data)))
        });
    }
}
```

### Required Tests for PRs

- **All new code** must have unit tests
- **Performance-critical code** must have benchmarks
- **Public APIs** must have integration tests
- **Unsafe code** must have property tests

### Running Tests

```bash
# Fast tests (during development)
cargo test --workspace --lib

# Full test suite (before PR)
cargo test --workspace --all-targets

# Performance regression testing
cargo bench --workspace

# Memory safety testing
cargo +nightly miri test

# Fuzz testing (run for extended periods)
cargo fuzz run fuzz_deserializer -- -max_total_time=300
```

## Documentation Standards

### Code Documentation

```rust
/// Brief one-line description of the function.
///
/// More detailed explanation of what the function does, its purpose,
/// and how it fits into the larger system.
///
/// # Arguments
/// 
/// * `input` - Description of the input parameter
/// * `config` - Configuration options for processing
///
/// # Returns
/// 
/// Returns a `Result` containing the processed output or an error.
///
/// # Performance
/// 
/// This function completes in O(n) time and allocates approximately
/// `input.len() * 2` bytes of memory.
///
/// # Examples
/// 
/// ```rust
/// use neo_protocol::process_message;
/// 
/// let input = b"hello world";
/// let result = process_message(input, &default_config())?;
/// assert_eq!(result.len(), 11);
/// ```
///
/// # Safety
/// 
/// (Only for unsafe functions) Explains the safety invariants.
pub fn process_message(input: &[u8], config: &Config) -> Result<Vec<u8>, Error> {
    // Implementation
}
```

### Architecture Documentation

For significant architectural changes, include:

1. **RFC Document** - Detailed design proposal
2. **Performance Analysis** - Expected impact on system performance  
3. **Migration Guide** - How existing code needs to change
4. **Alternative Approaches** - Why this approach was chosen

## Submitting Changes

### Before Submitting

**Pre-submission Checklist:**
- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Benchmarks show no significant regression
- [ ] Code is formatted (`cargo fmt`)
- [ ] Lints pass (`cargo clippy`)
- [ ] Documentation is updated
- [ ] Commit messages follow conventional format

### Pull Request Process

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following the guidelines above
3. **Add tests** for new functionality
4. **Update documentation** as needed
5. **Run the full test suite** locally
6. **Submit a pull request** with detailed description

### Pull Request Template

```markdown
## Description
Brief description of changes and motivation.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)  
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Performance improvement
- [ ] Documentation update

## Performance Impact
- [ ] No performance impact
- [ ] Performance improvement (include benchmark results)
- [ ] Potential performance regression (include justification)

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated  
- [ ] Benchmarks added/updated
- [ ] Manual testing performed

## Checklist
- [ ] My code follows the project's coding standards
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
```

### Review Process

1. **Automated Checks** - CI must pass (builds, tests, lints)
2. **Peer Review** - At least one approving review from a maintainer
3. **Performance Review** - For performance-critical changes
4. **Security Review** - For changes affecting security boundaries

**Review Timeline:**
- Simple fixes: 1-2 days
- New features: 3-7 days
- Architectural changes: 1-2 weeks

## Community

### Getting Help

- **Discord**: [Join our Discord server](https://discord.gg/neo-messaging-kernel) for real-time discussion
- **GitHub Discussions**: [Ask questions and discuss ideas](https://github.com/neo-qiss/messaging-kernel/discussions)
- **GitHub Issues**: Report bugs or request features

### Contributor Recognition

We recognize contributors through:
- **GitHub Contributors** section
- **Release Notes** mention for significant contributions
- **Contributor of the Month** recognition
- **Conference Speaking Opportunities** for major contributors

### Maintainer Responsibilities

Current maintainers:
- **@neo-qiss** - Project founder and lead maintainer
- *(More maintainers will be added as the project grows)*

Maintainers are responsible for:
- Code review and approval
- Release management
- Community moderation
- Architecture decisions
- Performance standards enforcement

---

Thank you for contributing to Neo Messaging Kernel! Together we're building the future of high-performance messaging infrastructure. 🚀

**Questions?** Reach out on [Discord](https://discord.gg/neo-messaging-kernel) or open a [Discussion](https://github.com/neo-qiss/messaging-kernel/discussions).
