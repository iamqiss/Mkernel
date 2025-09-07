//! Error types for the Neo protocol lexer

use thiserror::Error;
use crate::{Span, TokenKind};

/// Errors that can occur during lexing
#[derive(Error, Debug)]
pub enum LexerError {
    /// Invalid token encountered
    #[error("Invalid token '{text}' at {span}")]
    InvalidToken {
        /// The invalid token text
        text: String,
        /// The span of the invalid token
        span: Span,
    },

    /// Unexpected end of input
    #[error("Unexpected end of input at {span}")]
    UnexpectedEof {
        /// The span where EOF was encountered
        span: Span,
    },

    /// Invalid string literal
    #[error("Invalid string literal '{text}' at {span}")]
    InvalidStringLiteral {
        /// The invalid string text
        text: String,
        /// The span of the invalid string
        span: Span,
    },

    /// Invalid numeric literal
    #[error("Invalid numeric literal '{text}' at {span}")]
    InvalidNumericLiteral {
        /// The invalid numeric text
        text: String,
        /// The span of the invalid numeric
        span: Span,
    },

    /// Invalid character literal
    #[error("Invalid character literal '{text}' at {span}")]
    InvalidCharLiteral {
        /// The invalid character text
        text: String,
        /// The span of the invalid character
        span: Span,
    },

    /// Invalid duration literal
    #[error("Invalid duration literal '{text}' at {span}")]
    InvalidDurationLiteral {
        /// The invalid duration text
        text: String,
        /// The span of the invalid duration
        span: Span,
    },

    /// Invalid size literal
    #[error("Invalid size literal '{text}' at {span}")]
    InvalidSizeLiteral {
        /// The invalid size text
        text: String,
        /// The span of the invalid size
        span: Span,
    },

    /// Invalid identifier
    #[error("Invalid identifier '{text}' at {span}")]
    InvalidIdentifier {
        /// The invalid identifier text
        text: String,
        /// The span of the invalid identifier
        span: Span,
    },

    /// Unterminated string literal
    #[error("Unterminated string literal at {span}")]
    UnterminatedString {
        /// The span where the string started
        span: Span,
    },

    /// Unterminated character literal
    #[error("Unterminated character literal at {span}")]
    UnterminatedChar {
        /// The span where the character started
        span: Span,
    },

    /// Unterminated block comment
    #[error("Unterminated block comment at {span}")]
    UnterminatedComment {
        /// The span where the comment started
        span: Span,
    },

    /// Invalid escape sequence
    #[error("Invalid escape sequence '{sequence}' at {span}")]
    InvalidEscapeSequence {
        /// The invalid escape sequence
        sequence: String,
        /// The span of the escape sequence
        span: Span,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error
    #[error("Custom error: {0}")]
    Custom(String),
}

impl LexerError {
    /// Create an invalid token error
    pub fn invalid_token(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidToken {
            text: text.into(),
            span,
        }
    }

    /// Create an unexpected EOF error
    pub fn unexpected_eof(span: Span) -> Self {
        Self::UnexpectedEof { span }
    }

    /// Create an invalid string literal error
    pub fn invalid_string_literal(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidStringLiteral {
            text: text.into(),
            span,
        }
    }

    /// Create an invalid numeric literal error
    pub fn invalid_numeric_literal(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidNumericLiteral {
            text: text.into(),
            span,
        }
    }

    /// Create an invalid character literal error
    pub fn invalid_char_literal(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidCharLiteral {
            text: text.into(),
            span,
        }
    }

    /// Create an invalid duration literal error
    pub fn invalid_duration_literal(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidDurationLiteral {
            text: text.into(),
            span,
        }
    }

    /// Create an invalid size literal error
    pub fn invalid_size_literal(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidSizeLiteral {
            text: text.into(),
            span,
        }
    }

    /// Create an invalid identifier error
    pub fn invalid_identifier(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidIdentifier {
            text: text.into(),
            span,
        }
    }

    /// Create an unterminated string error
    pub fn unterminated_string(span: Span) -> Self {
        Self::UnterminatedString { span }
    }

    /// Create an unterminated character error
    pub fn unterminated_char(span: Span) -> Self {
        Self::UnterminatedChar { span }
    }

    /// Create an unterminated comment error
    pub fn unterminated_comment(span: Span) -> Self {
        Self::UnterminatedComment { span }
    }

    /// Create an invalid escape sequence error
    pub fn invalid_escape_sequence(sequence: impl Into<String>, span: Span) -> Self {
        Self::InvalidEscapeSequence {
            sequence: sequence.into(),
            span,
        }
    }

    /// Create a custom error
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

/// Result type for lexer operations
pub type Result<T> = std::result::Result<T, LexerError>;