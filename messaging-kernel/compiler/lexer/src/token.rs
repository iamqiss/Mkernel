//! Token definitions for Neo protocol lexer

use logos::Logos;
use std::fmt;

/// Token kind enumeration for Neo protocol
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Keywords
    #[token("service")]
    Service,
    #[token("message")]
    Message,
    #[token("rpc")]
    Rpc,
    #[token("event")]
    Event,
    #[token("config")]
    Config,
    #[token("version")]
    Version,
    #[token("namespace")]
    Namespace,
    #[token("queue")]
    Queue,
    #[token("topic")]
    Topic,
    #[token("timeout")]
    Timeout,
    #[token("retry_policy")]
    RetryPolicy,
    #[token("partition_key")]
    PartitionKey,
    #[token("retention")]
    Retention,
    #[token("max_concurrent_requests")]
    MaxConcurrentRequests,
    #[token("queue_buffer_size")]
    QueueBufferSize,
    #[token("compression")]
    Compression,
    #[token("serialization")]
    Serialization,
    #[token("optional")]
    Optional,
    #[token("replicas")]
    Replicas,
    #[token("resources")]
    Resources,
    #[token("cpu")]
    Cpu,
    #[token("memory")]
    Memory,
    #[token("broker")]
    Broker,
    #[token("precursor")]
    Precursor,
    #[token("persistence")]
    Persistence,
    #[token("storage_path")]
    StoragePath,
    #[token("max_message_size")]
    MaxMessageSize,
    #[token("retention_policy")]
    RetentionPolicy,
    #[token("queues")]
    Queues,
    #[token("topics")]
    Topics,
    #[token("partition_count")]
    PartitionCount,
    #[token("replication_factor")]
    ReplicationFactor,
    #[token("network")]
    Network,
    #[token("port")]
    Port,
    #[token("tls")]
    Tls,
    #[token("enabled")]
    Enabled,
    #[token("cert_path")]
    CertPath,
    #[token("key_path")]
    KeyPath,
    #[token("monitoring")]
    Monitoring,
    #[token("metrics")]
    Metrics,
    #[token("endpoint")]
    Endpoint,
    #[token("tracing")]
    Tracing,
    #[token("sampling_rate")]
    SamplingRate,
    #[token("logging")]
    Logging,
    #[token("level")]
    Level,
    #[token("format")]
    Format,
    #[token("output")]
    Output,

    // Data types
    #[token("u8")]
    U8,
    #[token("u16")]
    U16,
    #[token("u32")]
    U32,
    #[token("u64")]
    U64,
    #[token("i8")]
    I8,
    #[token("i16")]
    I16,
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,
    #[token("bool")]
    Bool,
    #[token("string")]
    String,
    #[token("bytes")]
    Bytes,
    #[token("timestamp")]
    Timestamp,

    // Compression types
    #[token("lz4")]
    Lz4,
    #[token("gzip")]
    Gzip,
    #[token("zstd")]
    Zstd,

    // Serialization types
    #[token("qiss")]
    Qiss,
    #[token("protobuf")]
    Protobuf,
    #[token("json")]
    Json,

    // Retry policies
    #[token("exponential_backoff")]
    ExponentialBackoff,
    #[token("linear_backoff")]
    LinearBackoff,
    #[token("fixed_delay")]
    FixedDelay,

    // Log levels
    #[token("trace")]
    Trace,
    #[token("debug")]
    Debug,
    #[token("info")]
    Info,
    #[token("warn")]
    Warn,
    #[token("error")]
    ErrorKeyword,

    // Log formats
    #[token("json_format")]
    JsonFormat,
    #[token("text")]
    TextFormat,

    // Log outputs
    #[token("stdout")]
    Stdout,
    #[token("stderr")]
    Stderr,
    #[token("file")]
    File,

    // Runtime types
    #[token("rust")]
    Rust,
    #[token("go")]
    Go,
    #[token("python")]
    Python,
    #[token("node")]
    Node,
    #[token("java")]
    Java,

    // Literals
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,
    #[regex(r"'([^'\\]|\\.)*'")]
    CharLiteral,
    #[regex(r"[0-9]+[smhdwy]", priority = 3)]
    DurationLiteral,
    #[regex(r"[0-9]+[kmgKMG]?[bB]", priority = 3)]
    SizeLiteral,
    #[regex(r"[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    NumericLiteral,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    // Operators and punctuation
    #[token("=")]
    Equals,
    #[token("->")]
    Arrow,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("<")]
    LeftAngle,
    #[token(">")]
    RightAngle,

    // Comments
    #[regex(r"//[^\r\n]*", logos::skip)]
    LineComment,
    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    BlockComment,

    // Whitespace
    #[regex(r"[ \t\r\n]+", logos::skip)]
    Whitespace,

    // Error token for invalid input
    Error,
}

impl TokenKind {
    /// Check if this token kind is a keyword
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            TokenKind::Service
                | TokenKind::Message
                | TokenKind::Rpc
                | TokenKind::Event
                | TokenKind::Config
                | TokenKind::Version
                | TokenKind::Namespace
                | TokenKind::Queue
                | TokenKind::Topic
                | TokenKind::Timeout
                | TokenKind::RetryPolicy
                | TokenKind::PartitionKey
                | TokenKind::Retention
                | TokenKind::MaxConcurrentRequests
                | TokenKind::QueueBufferSize
                | TokenKind::Compression
                | TokenKind::Serialization
                | TokenKind::Optional
                | TokenKind::Replicas
                | TokenKind::Resources
                | TokenKind::Cpu
                | TokenKind::Memory
                | TokenKind::Broker
                | TokenKind::Precursor
                | TokenKind::Persistence
                | TokenKind::StoragePath
                | TokenKind::MaxMessageSize
                | TokenKind::RetentionPolicy
                | TokenKind::Queues
                | TokenKind::Topics
                | TokenKind::PartitionCount
                | TokenKind::ReplicationFactor
                | TokenKind::Network
                | TokenKind::Port
                | TokenKind::Tls
                | TokenKind::Enabled
                | TokenKind::CertPath
                | TokenKind::KeyPath
                | TokenKind::Monitoring
                | TokenKind::Metrics
                | TokenKind::Endpoint
                | TokenKind::Tracing
                | TokenKind::SamplingRate
                | TokenKind::Logging
                | TokenKind::Level
                | TokenKind::Format
                | TokenKind::Output
        )
    }

    /// Check if this token kind is a data type
    pub fn is_data_type(self) -> bool {
        matches!(
            self,
            TokenKind::U8
                | TokenKind::U16
                | TokenKind::U32
                | TokenKind::U64
                | TokenKind::I8
                | TokenKind::I16
                | TokenKind::I32
                | TokenKind::I64
                | TokenKind::F32
                | TokenKind::F64
                | TokenKind::Bool
                | TokenKind::String
                | TokenKind::Bytes
                | TokenKind::Timestamp
        )
    }

    /// Check if this token kind is a literal
    pub fn is_literal(self) -> bool {
        matches!(
            self,
            TokenKind::StringLiteral
                | TokenKind::CharLiteral
                | TokenKind::NumericLiteral
                | TokenKind::DurationLiteral
                | TokenKind::SizeLiteral
        )
    }

    /// Check if this token kind is an operator
    pub fn is_operator(self) -> bool {
        matches!(
            self,
            TokenKind::Equals
                | TokenKind::Arrow
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::Comma
                | TokenKind::Dot
        )
    }

    /// Check if this token kind is a delimiter
    pub fn is_delimiter(self) -> bool {
        matches!(
            self,
            TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::LeftParen
                | TokenKind::RightParen
                | TokenKind::LeftBracket
                | TokenKind::RightBracket
                | TokenKind::LeftAngle
                | TokenKind::RightAngle
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Service => write!(f, "service"),
            TokenKind::Message => write!(f, "message"),
            TokenKind::Rpc => write!(f, "rpc"),
            TokenKind::Event => write!(f, "event"),
            TokenKind::Config => write!(f, "config"),
            TokenKind::Version => write!(f, "version"),
            TokenKind::Namespace => write!(f, "namespace"),
            TokenKind::Queue => write!(f, "queue"),
            TokenKind::Topic => write!(f, "topic"),
            TokenKind::Timeout => write!(f, "timeout"),
            TokenKind::RetryPolicy => write!(f, "retry_policy"),
            TokenKind::PartitionKey => write!(f, "partition_key"),
            TokenKind::Retention => write!(f, "retention"),
            TokenKind::MaxConcurrentRequests => write!(f, "max_concurrent_requests"),
            TokenKind::QueueBufferSize => write!(f, "queue_buffer_size"),
            TokenKind::Compression => write!(f, "compression"),
            TokenKind::Serialization => write!(f, "serialization"),
            TokenKind::Optional => write!(f, "optional"),
            TokenKind::Replicas => write!(f, "replicas"),
            TokenKind::Resources => write!(f, "resources"),
            TokenKind::Cpu => write!(f, "cpu"),
            TokenKind::Memory => write!(f, "memory"),
            TokenKind::Broker => write!(f, "broker"),
            TokenKind::Precursor => write!(f, "precursor"),
            TokenKind::Persistence => write!(f, "persistence"),
            TokenKind::StoragePath => write!(f, "storage_path"),
            TokenKind::MaxMessageSize => write!(f, "max_message_size"),
            TokenKind::RetentionPolicy => write!(f, "retention_policy"),
            TokenKind::Queues => write!(f, "queues"),
            TokenKind::Topics => write!(f, "topics"),
            TokenKind::PartitionCount => write!(f, "partition_count"),
            TokenKind::ReplicationFactor => write!(f, "replication_factor"),
            TokenKind::Network => write!(f, "network"),
            TokenKind::Port => write!(f, "port"),
            TokenKind::Tls => write!(f, "tls"),
            TokenKind::Enabled => write!(f, "enabled"),
            TokenKind::CertPath => write!(f, "cert_path"),
            TokenKind::KeyPath => write!(f, "key_path"),
            TokenKind::Monitoring => write!(f, "monitoring"),
            TokenKind::Metrics => write!(f, "metrics"),
            TokenKind::Endpoint => write!(f, "endpoint"),
            TokenKind::Tracing => write!(f, "tracing"),
            TokenKind::SamplingRate => write!(f, "sampling_rate"),
            TokenKind::Logging => write!(f, "logging"),
            TokenKind::Level => write!(f, "level"),
            TokenKind::Format => write!(f, "format"),
            TokenKind::Output => write!(f, "output"),
            TokenKind::U8 => write!(f, "u8"),
            TokenKind::U16 => write!(f, "u16"),
            TokenKind::U32 => write!(f, "u32"),
            TokenKind::U64 => write!(f, "u64"),
            TokenKind::I8 => write!(f, "i8"),
            TokenKind::I16 => write!(f, "i16"),
            TokenKind::I32 => write!(f, "i32"),
            TokenKind::I64 => write!(f, "i64"),
            TokenKind::F32 => write!(f, "f32"),
            TokenKind::F64 => write!(f, "f64"),
            TokenKind::Bool => write!(f, "bool"),
            TokenKind::String => write!(f, "string"),
            TokenKind::Bytes => write!(f, "bytes"),
            TokenKind::Timestamp => write!(f, "timestamp"),
            TokenKind::Lz4 => write!(f, "lz4"),
            TokenKind::Gzip => write!(f, "gzip"),
            TokenKind::Zstd => write!(f, "zstd"),
            TokenKind::Qiss => write!(f, "qiss"),
            TokenKind::Protobuf => write!(f, "protobuf"),
            TokenKind::Json => write!(f, "json"),
            TokenKind::ExponentialBackoff => write!(f, "exponential_backoff"),
            TokenKind::LinearBackoff => write!(f, "linear_backoff"),
            TokenKind::FixedDelay => write!(f, "fixed_delay"),
            TokenKind::Trace => write!(f, "trace"),
            TokenKind::Debug => write!(f, "debug"),
            TokenKind::Info => write!(f, "info"),
            TokenKind::Warn => write!(f, "warn"),
            TokenKind::JsonFormat => write!(f, "json_format"),
            TokenKind::TextFormat => write!(f, "text"),
            TokenKind::Stdout => write!(f, "stdout"),
            TokenKind::Stderr => write!(f, "stderr"),
            TokenKind::File => write!(f, "file"),
            TokenKind::Rust => write!(f, "rust"),
            TokenKind::Go => write!(f, "go"),
            TokenKind::Python => write!(f, "python"),
            TokenKind::Node => write!(f, "node"),
            TokenKind::Java => write!(f, "java"),
            TokenKind::StringLiteral => write!(f, "string literal"),
            TokenKind::CharLiteral => write!(f, "char literal"),
            TokenKind::NumericLiteral => write!(f, "numeric literal"),
            TokenKind::DurationLiteral => write!(f, "duration literal"),
            TokenKind::SizeLiteral => write!(f, "size literal"),
            TokenKind::Identifier => write!(f, "identifier"),
            TokenKind::Equals => write!(f, "="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::LeftAngle => write!(f, "<"),
            TokenKind::RightAngle => write!(f, ">"),
            TokenKind::LineComment => write!(f, "line comment"),
            TokenKind::BlockComment => write!(f, "block comment"),
            TokenKind::Whitespace => write!(f, "whitespace"),
            TokenKind::ErrorKeyword => write!(f, "error"),
            TokenKind::Error => write!(f, "error"),
        }
    }
}

/// A token with its associated metadata
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The span of the token in the source
    pub span: crate::Span,
    /// The text content of the token
    pub text: String,
}

impl Token {
    /// Create a new token
    pub fn new(kind: TokenKind, span: crate::Span, text: &str) -> Self {
        Self { kind, span, text: text.to_string() }
    }

    /// Get the text content without quotes for string literals
    pub fn unquoted_text(&self) -> &str {
        match self.kind {
            TokenKind::StringLiteral => {
                if self.text.len() >= 2 {
                    &self.text[1..self.text.len() - 1]
                } else {
                    &self.text
                }
            }
            TokenKind::CharLiteral => {
                if self.text.len() >= 2 {
                    &self.text[1..self.text.len() - 1]
                } else {
                    &self.text
                }
            }
            _ => &self.text,
        }
    }

    /// Check if this token is a specific keyword
    pub fn is_keyword(&self, keyword: TokenKind) -> bool {
        self.kind == keyword
    }

    /// Check if this token is a data type
    pub fn is_data_type(&self) -> bool {
        self.kind.is_data_type()
    }

    /// Check if this token is a literal
    pub fn is_literal(&self) -> bool {
        self.kind.is_literal()
    }

    /// Check if this token is an identifier
    pub fn is_identifier(&self) -> bool {
        self.kind == TokenKind::Identifier
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.text)
    }
}