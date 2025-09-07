//! Parser for Neo protocol files
//! 
//! This crate provides a comprehensive parser for .neo files that generates
//! an Abstract Syntax Tree (AST) representation of the service definitions.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

use neo_lexer::{Lexer, Token, TokenKind, Span};

pub mod error;
pub mod ast;
pub mod parser;

pub use error::ParserError;
pub use ast::*;
pub use parser::Parser;

/// Result type for parser operations
pub type Result<T> = std::result::Result<T, ParserError>;

/// Parse a Neo protocol file from source code
pub fn parse(source: &str) -> Result<ServiceDefinition> {
    let owned_source = source.to_string();
    let lexer = Lexer::new(&owned_source);
    let tokens = lexer.tokenize()
        .map_err(|e| ParserError::LexerError(e))?;
    
    let mut parser = Parser::new(tokens);
    parser.parse_service_definition()
}

/// Parse multiple service definitions from source code
pub fn parse_multiple(source: &str) -> Result<Vec<ServiceDefinition>> {
    let owned_source = source.to_string();
    let lexer = Lexer::new(&owned_source);
    let tokens = lexer.tokenize()
        .map_err(|e| ParserError::LexerError(e))?;
    
    let mut parser = Parser::new(tokens);
    parser.parse_multiple_services()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_service() {
        let source = r#"
            service UserService {
                version = "1.0.0";
                namespace = "com.example.users";
                
                message User {
                    id: u64;
                    username: string;
                    email: string;
                }
                
                rpc GetUser(UserId) -> User {
                    queue = "users.get";
                    timeout = 5s;
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
        
        let service = result.unwrap();
        assert_eq!(service.name, "UserService");
        assert_eq!(service.version, "1.0.0");
        assert_eq!(service.namespace, "com.example.users");
        assert_eq!(service.messages.len(), 1);
        assert_eq!(service.rpcs.len(), 1);
    }

    #[test]
    fn test_parse_complex_service() {
        let source = r#"
            service UserService {
                version = "1.0.0";
                namespace = "com.example.users";
                
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
                
                rpc GetUser(UserId) -> User {
                    queue = "users.get";
                    timeout = 5s;
                    retry_policy = exponential_backoff(max_attempts: 3);
                }
                
                rpc CreateUser(User) -> UserId {
                    queue = "users.create";
                    timeout = 10s;
                }
                
                event UserCreated(UserCreatedEvent) {
                    topic = "users.events";
                    partition_key = user.id;
                    retention = 7d;
                }
                
                config {
                    max_concurrent_requests = 1000;
                    queue_buffer_size = 10000;
                    compression = lz4;
                    serialization = qiss;
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
        
        let service = result.unwrap();
        assert_eq!(service.name, "UserService");
        assert_eq!(service.messages.len(), 3);
        assert_eq!(service.rpcs.len(), 2);
        assert_eq!(service.events.len(), 1);
        assert!(service.config.is_some());
    }

    #[test]
    fn test_parse_error_handling() {
        let source = r#"
            service UserService {
                version = "1.0.0";
                // Missing closing brace
        "#;

        let result = parse(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let source = r#"
            service UserService {
                version = "1.0.0";
                invalid_syntax @#$;
            }
        "#;

        let result = parse(source);
        assert!(result.is_err());
    }
}