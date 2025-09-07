//! Parser implementation for Neo protocol files

use neo_lexer::{Token, TokenKind, Span};
use crate::{Result, ParserError};
use crate::ast::*;

/// A parser for Neo protocol files
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse a service definition
    pub fn parse_service_definition(&mut self) -> Result<ServiceDefinition> {
        self.expect_token(TokenKind::Service)?;
        let name = self.expect_identifier()?;
        self.expect_token(TokenKind::LeftBrace)?;

        let mut version = String::new();
        let mut namespace = String::new();
        let mut messages = Vec::new();
        let mut rpcs = Vec::new();
        let mut events = Vec::new();
        let mut config = None;

        while !self.is_at_end() && !self.check(TokenKind::RightBrace) {
            match self.peek()?.kind {
                TokenKind::Version => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    version = self.expect_string_literal()?;
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::Namespace => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    namespace = self.expect_string_literal()?;
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::Message => {
                    messages.push(self.parse_message_definition()?);
                }
                TokenKind::Rpc => {
                    rpcs.push(self.parse_rpc_definition()?);
                }
                TokenKind::Event => {
                    events.push(self.parse_event_definition()?);
                }
                TokenKind::Config => {
                    config = Some(self.parse_service_config()?);
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "version, namespace, message, rpc, event, or config".to_string(),
                        found: self.peek()?.kind,
                        span: self.peek()?.span,
                    });
                }
            }
        }

        self.expect_token(TokenKind::RightBrace)?;

        Ok(ServiceDefinition {
            name,
            version,
            namespace,
            messages,
            rpcs,
            events,
            config,
            span: Span::new(0, self.position),
        })
    }

    /// Parse multiple service definitions
    pub fn parse_multiple_services(&mut self) -> Result<Vec<ServiceDefinition>> {
        let mut services = Vec::new();
        
        while !self.is_at_end() {
            services.push(self.parse_service_definition()?);
        }
        
        Ok(services)
    }

    /// Parse a message definition
    fn parse_message_definition(&mut self) -> Result<MessageDefinition> {
        let span = self.peek()?.span;
        self.expect_token(TokenKind::Message)?;
        let name = self.expect_identifier()?;
        self.expect_token(TokenKind::LeftBrace)?;

        let mut fields = Vec::new();
        while !self.is_at_end() && !self.check(TokenKind::RightBrace) {
            fields.push(self.parse_field_definition()?);
        }

        self.expect_token(TokenKind::RightBrace)?;

        Ok(MessageDefinition {
            name,
            fields,
            span,
        })
    }

    /// Parse a field definition
    fn parse_field_definition(&mut self) -> Result<FieldDefinition> {
        let span = self.peek()?.span;
        let optional = self.check(TokenKind::Optional);
        if optional {
            self.advance();
        }

        let name = self.expect_identifier()?;
        self.expect_token(TokenKind::Colon)?;
        let field_type = self.parse_field_type()?;
        self.expect_token(TokenKind::Semicolon)?;

        Ok(FieldDefinition {
            name,
            field_type,
            optional,
            span,
        })
    }

    /// Parse a field type
    fn parse_field_type(&mut self) -> Result<FieldType> {
        match self.peek()?.kind {
            TokenKind::U8 => {
                self.advance();
                Ok(FieldType::U8)
            }
            TokenKind::U16 => {
                self.advance();
                Ok(FieldType::U16)
            }
            TokenKind::U32 => {
                self.advance();
                Ok(FieldType::U32)
            }
            TokenKind::U64 => {
                self.advance();
                Ok(FieldType::U64)
            }
            TokenKind::I8 => {
                self.advance();
                Ok(FieldType::I8)
            }
            TokenKind::I16 => {
                self.advance();
                Ok(FieldType::I16)
            }
            TokenKind::I32 => {
                self.advance();
                Ok(FieldType::I32)
            }
            TokenKind::I64 => {
                self.advance();
                Ok(FieldType::I64)
            }
            TokenKind::F32 => {
                self.advance();
                Ok(FieldType::F32)
            }
            TokenKind::F64 => {
                self.advance();
                Ok(FieldType::F64)
            }
            TokenKind::Bool => {
                self.advance();
                Ok(FieldType::Bool)
            }
            TokenKind::String => {
                self.advance();
                Ok(FieldType::String)
            }
            TokenKind::Bytes => {
                self.advance();
                Ok(FieldType::Bytes)
            }
            TokenKind::Timestamp => {
                self.advance();
                Ok(FieldType::Timestamp)
            }
            TokenKind::Identifier => {
                let name = self.expect_identifier()?;
                Ok(FieldType::Message(name))
            }
            _ => {
                Err(ParserError::UnexpectedToken {
                    expected: "field type".to_string(),
                    found: self.peek()?.kind,
                    span: self.peek()?.span,
                })
            }
        }
    }

    /// Parse an RPC definition
    fn parse_rpc_definition(&mut self) -> Result<RpcDefinition> {
        let span = self.peek()?.span;
        self.expect_token(TokenKind::Rpc)?;
        let name = self.expect_identifier()?;
        self.expect_token(TokenKind::LeftParen)?;
        let input_type = self.expect_identifier()?;
        self.expect_token(TokenKind::RightParen)?;
        self.expect_token(TokenKind::Arrow)?;
        let output_type = self.expect_identifier()?;
        self.expect_token(TokenKind::LeftBrace)?;

        let mut queue = None;
        let mut timeout = None;
        let mut retry_policy = None;

        while !self.is_at_end() && !self.check(TokenKind::RightBrace) {
            match self.peek()?.kind {
                TokenKind::Queue => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    queue = Some(self.expect_string_literal()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::Timeout => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    timeout = Some(self.parse_duration()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::RetryPolicy => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    retry_policy = Some(self.parse_retry_policy()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "queue, timeout, or retry_policy".to_string(),
                        found: self.peek()?.kind,
                        span: self.peek()?.span,
                    });
                }
            }
        }

        self.expect_token(TokenKind::RightBrace)?;

        Ok(RpcDefinition {
            name,
            input_type,
            output_type,
            queue,
            timeout,
            retry_policy,
            span,
        })
    }

    /// Parse an event definition
    fn parse_event_definition(&mut self) -> Result<EventDefinition> {
        let span = self.peek()?.span;
        self.expect_token(TokenKind::Event)?;
        let name = self.expect_identifier()?;
        self.expect_token(TokenKind::LeftParen)?;
        let message_type = self.expect_identifier()?;
        self.expect_token(TokenKind::RightParen)?;
        self.expect_token(TokenKind::LeftBrace)?;

        let mut topic = None;
        let mut partition_key = None;
        let mut retention = None;

        while !self.is_at_end() && !self.check(TokenKind::RightBrace) {
            match self.peek()?.kind {
                TokenKind::Topic => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    topic = Some(self.expect_string_literal()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::PartitionKey => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    partition_key = Some(self.expect_identifier()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::Retention => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    retention = Some(self.parse_duration()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "topic, partition_key, or retention".to_string(),
                        found: self.peek()?.kind,
                        span: self.peek()?.span,
                    });
                }
            }
        }

        self.expect_token(TokenKind::RightBrace)?;

        Ok(EventDefinition {
            name,
            message_type,
            topic,
            partition_key,
            retention,
            span,
        })
    }

    /// Parse a service configuration
    fn parse_service_config(&mut self) -> Result<ServiceConfig> {
        self.expect_token(TokenKind::Config)?;
        self.expect_token(TokenKind::LeftBrace)?;

        let mut max_concurrent_requests = None;
        let mut queue_buffer_size = None;
        let mut compression = None;
        let mut serialization = None;

        while !self.is_at_end() && !self.check(TokenKind::RightBrace) {
            match self.peek()?.kind {
                TokenKind::MaxConcurrentRequests => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    max_concurrent_requests = Some(self.expect_numeric_literal()? as u32);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::QueueBufferSize => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    queue_buffer_size = Some(self.expect_numeric_literal()? as u32);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::Compression => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    compression = Some(self.parse_compression_type()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                TokenKind::Serialization => {
                    self.advance();
                    self.expect_token(TokenKind::Equals)?;
                    serialization = Some(self.parse_serialization_type()?);
                    self.expect_token(TokenKind::Semicolon)?;
                }
                _ => {
                    return Err(ParserError::UnexpectedToken {
                        expected: "max_concurrent_requests, queue_buffer_size, compression, or serialization".to_string(),
                        found: self.peek()?.kind,
                        span: self.peek()?.span,
                    });
                }
            }
        }

        self.expect_token(TokenKind::RightBrace)?;

        Ok(ServiceConfig {
            max_concurrent_requests,
            queue_buffer_size,
            compression,
            serialization,
        })
    }

    /// Parse a duration
    fn parse_duration(&mut self) -> Result<Duration> {
        let value = self.expect_numeric_literal()?;
        let unit_str = self.expect_identifier()?;
        let unit = DurationUnit::from_str(&unit_str)
            .ok_or_else(|| ParserError::InvalidDurationUnit {
                unit: unit_str,
                span: self.previous().unwrap().span,
            })?;

        Ok(Duration::new(value, unit))
    }

    /// Parse a retry policy
    fn parse_retry_policy(&mut self) -> Result<RetryPolicy> {
        let policy_type_str = self.expect_identifier()?;
        let policy_type = RetryPolicyType::from_str(&policy_type_str)
            .ok_or_else(|| ParserError::InvalidRetryPolicyType {
                policy_type: policy_type_str,
                span: self.previous().unwrap().span,
            })?;

        self.expect_token(TokenKind::LeftParen)?;
        
        let mut max_attempts = None;
        let mut initial_delay = None;
        let mut max_delay = None;
        let mut multiplier = None;

        while !self.is_at_end() && !self.check(TokenKind::RightParen) {
            let param_name = self.expect_identifier()?;
            self.expect_token(TokenKind::Colon)?;
            
            match param_name.as_str() {
                "max_attempts" => {
                    max_attempts = Some(self.expect_numeric_literal()? as u32);
                }
                "initial_delay" => {
                    initial_delay = Some(self.parse_duration()?);
                }
                "max_delay" => {
                    max_delay = Some(self.parse_duration()?);
                }
                "multiplier" => {
                    multiplier = Some(self.expect_numeric_literal()? as f64);
                }
                _ => {
                    return Err(ParserError::InvalidRetryPolicyParameter {
                        parameter: param_name,
                        span: self.previous()?.span,
                    });
                }
            }

            if !self.check(TokenKind::RightParen) {
                self.expect_token(TokenKind::Comma)?;
            }
        }

        self.expect_token(TokenKind::RightParen)?;

        Ok(RetryPolicy {
            policy_type,
            max_attempts,
            initial_delay,
            max_delay,
            multiplier,
        })
    }

    /// Parse a compression type
    fn parse_compression_type(&mut self) -> Result<CompressionType> {
        let type_str = self.expect_identifier()?;
        CompressionType::from_str(&type_str)
            .ok_or_else(|| ParserError::InvalidCompressionType {
                compression_type: type_str,
                span: self.previous().unwrap().span,
            })
    }

    /// Parse a serialization type
    fn parse_serialization_type(&mut self) -> Result<SerializationType> {
        let type_str = self.expect_identifier()?;
        SerializationType::from_str(&type_str)
            .ok_or_else(|| ParserError::InvalidSerializationType {
                serialization_type: type_str,
                span: self.previous().unwrap().span,
            })
    }

    // Helper methods

    /// Check if we're at the end of the token stream
    fn is_at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// Peek at the current token
    fn peek(&self) -> Result<&Token> {
        if self.is_at_end() {
            Err(ParserError::UnexpectedEof {
                span: Span::new(0, 0),
            })
        } else {
            Ok(&self.tokens[self.position])
        }
    }

    /// Get the previous token
    fn previous(&self) -> Result<&Token> {
        if self.position == 0 {
            Err(ParserError::UnexpectedEof {
                span: Span::new(0, 0),
            })
        } else {
            Ok(&self.tokens[self.position - 1])
        }
    }

    /// Advance to the next token
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.position += 1;
        }
        self.previous().unwrap()
    }

    /// Check if the current token matches the expected kind
    fn check(&self, kind: TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.tokens[self.position].kind == kind
        }
    }

    /// Expect a specific token kind
    fn expect_token(&mut self, kind: TokenKind) -> Result<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(ParserError::UnexpectedToken {
                expected: format!("{:?}", kind),
                found: self.peek()?.kind,
                span: self.peek()?.span,
            })
        }
    }

    /// Expect an identifier token
    fn expect_identifier(&mut self) -> Result<String> {
        if self.check(TokenKind::Identifier) {
            let token = self.advance();
            Ok(token.text.to_string())
        } else {
            Err(ParserError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self.peek()?.kind,
                span: self.peek()?.span,
            })
        }
    }

    /// Expect a string literal token
    fn expect_string_literal(&mut self) -> Result<String> {
        if self.check(TokenKind::StringLiteral) {
            let token = self.advance();
            Ok(token.unquoted_text().to_string())
        } else {
            Err(ParserError::UnexpectedToken {
                expected: "string literal".to_string(),
                found: self.peek()?.kind,
                span: self.peek()?.span,
            })
        }
    }

    /// Expect a numeric literal token
    fn expect_numeric_literal(&mut self) -> Result<u64> {
        if self.check(TokenKind::NumericLiteral) {
            let token = self.advance();
            token.text.parse::<u64>()
                .map_err(|_| ParserError::InvalidNumericLiteral {
                    text: token.text.to_string(),
                    span: token.span,
                })
        } else {
            Err(ParserError::UnexpectedToken {
                expected: "numeric literal".to_string(),
                found: self.peek()?.kind,
                span: self.peek()?.span,
            })
        }
    }
}