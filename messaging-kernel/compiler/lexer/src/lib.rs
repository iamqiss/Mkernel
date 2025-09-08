//! Lexer for Neo protocol files
//! 
//! This crate provides a high-performance lexer for parsing .neo files into tokens.
//! It uses the logos crate for efficient tokenization with zero-copy string handling.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

use logos::Logos;
use std::fmt;

pub mod error;
pub mod token;

pub use error::LexerError;
pub use token::{Token, TokenKind};

/// Result type for lexer operations
pub type Result<T> = std::result::Result<T, LexerError>;

/// High-performance lexer for Neo protocol files
pub struct Lexer<'source> {
    inner: logos::Lexer<'source, TokenKind>,
    source: &'source str,
}

impl<'source> Lexer<'source> {
    /// Create a new lexer for the given source code
    pub fn new(source: &'source str) -> Self {
        Self {
            inner: TokenKind::lexer(source),
            source,
        }
    }

    /// Get the next token from the source
    pub fn next_token(&mut self) -> Option<Token> {
        match self.inner.next() {
            Some(Ok(kind)) => {
                let span = self.inner.span();
                let text = &self.source[span.clone()];
                
                Some(Token {
                    kind,
                    span: crate::Span::new(span.start, span.end),
                    text: text.to_string(),
                })
            }
            Some(Err(_)) => {
                let span = self.inner.span();
                let text = &self.source[span.clone()];
                
                Some(Token {
                    kind: TokenKind::Error,
                    span: crate::Span::new(span.start, span.end),
                    text: text.to_string(),
                })
            }
            None => None,
        }
    }

    /// Get all tokens from the source
    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while let Some(token) = self.next_token() {
            if token.kind == TokenKind::Error {
                return Err(LexerError::InvalidToken {
                    text: token.text.to_string(),
                    span: token.span,
                });
            }
            tokens.push(token);
        }
        
        Ok(tokens)
    }

    /// Get the current position in the source
    pub fn position(&self) -> usize {
        self.inner.span().start
    }

    /// Get the remaining source text
    pub fn remaining(&self) -> &'source str {
        &self.source[self.position()..]
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

/// Token span information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start position in the source
    pub start: usize,
    /// End position in the source
    pub end: usize,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of the span
    pub fn len(self) -> usize {
        self.end - self.start
    }

    /// Check if the span is empty
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Merge two spans
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let source = r#"
            service UserService {
                version = "1.0.0";
                namespace = "com.example.users";
                
                message User {
                    id: u64;
                    username: string;
                }
            }
        "#;

        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        // Verify we get the expected tokens
        assert!(!tokens.is_empty());
        
        // Check for specific tokens
        let token_texts: Vec<String> = tokens.iter().map(|t| t.text.to_string()).collect();
        assert!(token_texts.contains(&"service".to_string()));
        assert!(token_texts.contains(&"UserService".to_string()));
        assert!(token_texts.contains(&"version".to_string()));
        assert!(token_texts.contains(&"1.0.0".to_string()));
    }

    #[test]
    fn test_string_literals() {
        let source = r#"version = "1.0.0"; namespace = "com.example.users";"#;
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        let string_tokens: Vec<&Token> = tokens.iter()
            .filter(|t| t.kind == TokenKind::StringLiteral)
            .collect();
        
        assert_eq!(string_tokens.len(), 2);
        assert_eq!(string_tokens[0].text, "\"1.0.0\"");
        assert_eq!(string_tokens[1].text, "\"com.example.users\"");
    }

    #[test]
    fn test_numeric_literals() {
        let source = "id: u64; timeout = 5s; max_attempts: 3;";
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        let numeric_tokens: Vec<&Token> = tokens.iter()
            .filter(|t| t.kind == TokenKind::NumericLiteral)
            .collect();
        
        assert_eq!(numeric_tokens.len(), 2);
        assert_eq!(numeric_tokens[0].text, "5");
        assert_eq!(numeric_tokens[1].text, "3");
    }

    #[test]
    fn test_identifiers() {
        let source = "service UserService { message User { id: u64; } }";
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        let identifier_tokens: Vec<&Token> = tokens.iter()
            .filter(|t| t.kind == TokenKind::Identifier)
            .collect();
        
        assert!(identifier_tokens.len() >= 4);
        assert_eq!(identifier_tokens[0].text, "UserService");
        assert_eq!(identifier_tokens[1].text, "User");
        assert_eq!(identifier_tokens[2].text, "id");
        assert_eq!(identifier_tokens[3].text, "u64");
    }

    #[test]
    fn test_comments() {
        let source = r#"
            // This is a comment
            service UserService {
                /* Multi-line
                   comment */
                version = "1.0.0";
            }
        "#;

        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        
        // Comments should be filtered out
        let comment_tokens: Vec<&Token> = tokens.iter()
            .filter(|t| matches!(t.kind, TokenKind::LineComment | TokenKind::BlockComment))
            .collect();
        
        assert_eq!(comment_tokens.len(), 2);
    }

    #[test]
    fn test_error_handling() {
        let source = "service UserService { invalid_token @#$ }";
        let lexer = Lexer::new(source);
        let result = lexer.tokenize();
        
        assert!(result.is_err());
        if let Err(LexerError::InvalidToken { text, .. }) = result {
            assert_eq!(text, "@#$");
        }
    }
}