//! Code generation for Neo protocol services
//! 
//! This crate provides code generation capabilities for converting Neo protocol
//! service definitions into executable Rust code.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

use neo_parser::ServiceDefinition;

pub mod error;
pub mod rust_generator;

pub use error::CodegenError;
pub use rust_generator::RustGenerator;

/// Generate Rust code for a service definition
pub fn generate_rust_code(service: &ServiceDefinition) -> Result<String> {
    let generator = RustGenerator::new();
    generator.generate_service(service)
        .map_err(|e| CodegenError::GenerationFailed(e.to_string()))
}

/// Result type for code generation operations
pub type Result<T> = std::result::Result<T, CodegenError>;

#[cfg(test)]
mod tests {
    use super::*;
    use neo_parser::ast::*;
    // use neo_lexer::Span;

    #[test]
    fn test_generate_rust_code() {
        let service = ServiceDefinition {
            name: "UserService".to_string(),
            version: "1.0.0".to_string(),
            namespace: "com.example.users".to_string(),
            messages: vec![],
            rpcs: vec![],
            events: vec![],
            config: None,
            span: neo_lexer::Span::new(0, 0),
        };

        let result = generate_rust_code(&service);
        assert!(result.is_ok());
        
        let code = result.unwrap();
        assert!(code.contains("UserService"));
        assert!(code.contains("Generated code for #service_name"));
    }
}