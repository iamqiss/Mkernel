//! Security module for Neo Protocol

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use bytes::Bytes;

use crate::{Result, Error};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable authentication
    pub enable_auth: bool,
    /// Enable authorization
    pub enable_authorization: bool,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Authentication method
    pub auth_method: AuthMethod,
    /// Token expiration time
    pub token_expiration: Duration,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Encryption configuration
    pub encryption: EncryptionConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_auth: false,
            enable_authorization: false,
            enable_encryption: false,
            auth_method: AuthMethod::None,
            token_expiration: Duration::from_secs(3600), // 1 hour
            rate_limiting: RateLimitConfig::default(),
            encryption: EncryptionConfig::default(),
        }
    }
}

/// Authentication method
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// No authentication
    None,
    /// Token-based authentication
    Token,
    /// Certificate-based authentication
    Certificate,
    /// API key authentication
    ApiKey,
    /// Custom authentication
    Custom(String),
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per second limit
    pub requests_per_second: u32,
    /// Burst limit
    pub burst_limit: u32,
    /// Rate limit window
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: 1000,
            burst_limit: 100,
            window_size: Duration::from_secs(1),
        }
    }
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Key size in bits
    pub key_size: u32,
    /// Key rotation interval
    pub key_rotation_interval: Duration,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_size: 256,
            key_rotation_interval: Duration::from_secs(86400), // 24 hours
        }
    }
}

/// Encryption algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
    /// XChaCha20-Poly1305
    XChaCha20Poly1305,
}

/// Security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// User ID
    pub user_id: String,
    /// User roles
    pub roles: Vec<String>,
    /// User permissions
    pub permissions: Vec<String>,
    /// Authentication token
    pub token: Option<String>,
    /// Token expiration
    pub token_expires_at: Option<SystemTime>,
    /// Client certificate
    pub client_certificate: Option<Bytes>,
    /// Additional security metadata
    pub metadata: HashMap<String, String>,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            roles: Vec::new(),
            permissions: Vec::new(),
            token: None,
            token_expires_at: None,
            client_certificate: None,
            metadata: HashMap::new(),
        }
    }

    /// Check if the context has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if the context has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if the token is valid and not expired
    pub fn is_token_valid(&self) -> bool {
        if let Some(expires_at) = self.token_expires_at {
            SystemTime::now() < expires_at
        } else {
            false
        }
    }
}

/// Authentication provider trait
pub trait AuthProvider: Send + Sync {
    /// Authenticate a user with credentials
    fn authenticate(&self, credentials: &AuthCredentials) -> Result<SecurityContext>;
    
    /// Validate a token
    fn validate_token(&self, token: &str) -> Result<SecurityContext>;
    
    /// Refresh a token
    fn refresh_token(&self, token: &str) -> Result<String>;
    
    /// Revoke a token
    fn revoke_token(&self, token: &str) -> Result<()>;
}

/// Authorization provider trait
pub trait AuthzProvider: Send + Sync {
    /// Check if a user has permission to access a resource
    fn check_permission(&self, context: &SecurityContext, resource: &str, action: &str) -> Result<bool>;
    
    /// Get user permissions for a resource
    fn get_permissions(&self, context: &SecurityContext, resource: &str) -> Result<Vec<String>>;
}

/// Authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
    /// Credential type
    pub credential_type: CredentialType,
    /// Credential data
    pub data: Bytes,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Credential type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    /// Username and password
    UsernamePassword,
    /// API key
    ApiKey,
    /// Certificate
    Certificate,
    /// Token
    Token,
    /// Custom
    Custom(String),
}

/// Rate limiter
pub struct RateLimiter {
    /// Rate limit configuration
    config: RateLimitConfig,
    /// Request counters per client
    counters: HashMap<String, ClientCounter>,
}

/// Client request counter
struct ClientCounter {
    /// Current request count
    count: u32,
    /// Window start time
    window_start: SystemTime,
    /// Burst counter
    burst_count: u32,
    /// Last burst time
    last_burst: SystemTime,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            counters: HashMap::new(),
        }
    }

    /// Check if a request is allowed
    pub fn is_allowed(&mut self, client_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let now = SystemTime::now();
        let counter = self.counters.entry(client_id.to_string()).or_insert_with(|| {
            ClientCounter {
                count: 0,
                window_start: now,
                burst_count: 0,
                last_burst: now,
            }
        });

        // Reset window if needed
        if now.duration_since(counter.window_start).unwrap() >= self.config.window_size {
            counter.count = 0;
            counter.window_start = now;
        }

        // Check burst limit
        if now.duration_since(counter.last_burst).unwrap() >= Duration::from_millis(100) {
            counter.burst_count = 0;
            counter.last_burst = now;
        }

        if counter.burst_count >= self.config.burst_limit {
            return false;
        }

        // Check rate limit
        if counter.count >= self.config.requests_per_second {
            return false;
        }

        counter.count += 1;
        counter.burst_count += 1;
        true
    }

    /// Get current rate limit status for a client
    pub fn get_status(&self, client_id: &str) -> RateLimitStatus {
        if let Some(counter) = self.counters.get(client_id) {
            let now = SystemTime::now();
            let window_elapsed = now.duration_since(counter.window_start).unwrap_or_default();
            let remaining_window = self.config.window_size.saturating_sub(window_elapsed);
            
            RateLimitStatus {
                requests_remaining: self.config.requests_per_second.saturating_sub(counter.count),
                burst_remaining: self.config.burst_limit.saturating_sub(counter.burst_count),
                window_remaining: remaining_window,
                reset_time: counter.window_start + self.config.window_size,
            }
        } else {
            RateLimitStatus {
                requests_remaining: self.config.requests_per_second,
                burst_remaining: self.config.burst_limit,
                window_remaining: self.config.window_size,
                reset_time: SystemTime::now() + self.config.window_size,
            }
        }
    }
}

/// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    /// Remaining requests in current window
    pub requests_remaining: u32,
    /// Remaining burst requests
    pub burst_remaining: u32,
    /// Remaining time in current window
    pub window_remaining: Duration,
    /// Time when the window resets
    pub reset_time: SystemTime,
}

/// Encryption provider trait
pub trait EncryptionProvider: Send + Sync {
    /// Encrypt data
    fn encrypt(&self, data: &[u8], key: &[u8]) -> Result<Bytes>;
    
    /// Decrypt data
    fn decrypt(&self, encrypted_data: &[u8], key: &[u8]) -> Result<Bytes>;
    
    /// Generate a new encryption key
    fn generate_key(&self) -> Result<Bytes>;
    
    /// Derive a key from a password
    fn derive_key(&self, password: &[u8], salt: &[u8]) -> Result<Bytes>;
}

/// Security manager
pub struct SecurityManager {
    /// Security configuration
    pub config: SecurityConfig,
    /// Authentication provider
    auth_provider: Option<Box<dyn AuthProvider>>,
    /// Authorization provider
    authz_provider: Option<Box<dyn AuthzProvider>>,
    /// Encryption provider
    encryption_provider: Option<Box<dyn EncryptionProvider>>,
    /// Rate limiter
    rate_limiter: RateLimiter,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            rate_limiter: RateLimiter::new(config.rate_limiting.clone()),
            config,
            auth_provider: None,
            authz_provider: None,
            encryption_provider: None,
        }
    }

    /// Set the authentication provider
    pub fn set_auth_provider(&mut self, provider: Box<dyn AuthProvider>) {
        self.auth_provider = Some(provider);
    }

    /// Set the authorization provider
    pub fn set_authz_provider(&mut self, provider: Box<dyn AuthzProvider>) {
        self.authz_provider = Some(provider);
    }

    /// Set the encryption provider
    pub fn set_encryption_provider(&mut self, provider: Box<dyn EncryptionProvider>) {
        self.encryption_provider = Some(provider);
    }

    /// Authenticate a user
    pub fn authenticate(&self, credentials: &AuthCredentials) -> Result<SecurityContext> {
        if !self.config.enable_auth {
            return Ok(SecurityContext::new("anonymous".to_string()));
        }

        if let Some(provider) = &self.auth_provider {
            provider.authenticate(credentials)
        } else {
            Err(Error::Security("Authentication provider not configured".to_string()))
        }
    }

    /// Validate a token
    pub fn validate_token(&self, token: &str) -> Result<SecurityContext> {
        if !self.config.enable_auth {
            return Ok(SecurityContext::new("anonymous".to_string()));
        }

        if let Some(provider) = &self.auth_provider {
            provider.validate_token(token)
        } else {
            Err(Error::Security("Authentication provider not configured".to_string()))
        }
    }

    /// Check authorization
    pub fn check_authorization(&self, context: &SecurityContext, resource: &str, action: &str) -> Result<bool> {
        if !self.config.enable_authorization {
            return Ok(true);
        }

        if let Some(provider) = &self.authz_provider {
            provider.check_permission(context, resource, action)
        } else {
            Err(Error::Security("Authorization provider not configured".to_string()))
        }
    }

    /// Check rate limit
    pub fn check_rate_limit(&mut self, client_id: &str) -> Result<bool> {
        if self.rate_limiter.is_allowed(client_id) {
            Ok(true)
        } else {
            Err(Error::RateLimitExceeded(format!("Rate limit exceeded for client: {}", client_id)))
        }
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8], key: &[u8]) -> Result<Bytes> {
        if !self.config.enable_encryption {
            return Ok(Bytes::copy_from_slice(data));
        }

        if let Some(provider) = &self.encryption_provider {
            provider.encrypt(data, key)
        } else {
            Err(Error::Security("Encryption provider not configured".to_string()))
        }
    }

    /// Decrypt data
    pub fn decrypt(&self, encrypted_data: &[u8], key: &[u8]) -> Result<Bytes> {
        if !self.config.enable_encryption {
            return Ok(Bytes::copy_from_slice(encrypted_data));
        }

        if let Some(provider) = &self.encryption_provider {
            provider.decrypt(encrypted_data, key)
        } else {
            Err(Error::Security("Encryption provider not configured".to_string()))
        }
    }

    /// Get rate limit status
    pub fn get_rate_limit_status(&self, client_id: &str) -> RateLimitStatus {
        self.rate_limiter.get_status(client_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(!config.enable_auth);
        assert!(!config.enable_authorization);
        assert!(!config.enable_encryption);
        assert_eq!(config.auth_method, AuthMethod::None);
    }

    #[test]
    fn test_security_context() {
        let mut context = SecurityContext::new("user123".to_string());
        context.roles.push("admin".to_string());
        context.permissions.push("read".to_string());
        
        assert!(context.has_role("admin"));
        assert!(!context.has_role("user"));
        assert!(context.has_permission("read"));
        assert!(!context.has_permission("write"));
    }

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig {
            enabled: true,
            requests_per_second: 10,
            burst_limit: 5,
            window_size: Duration::from_secs(1),
        };
        
        let mut limiter = RateLimiter::new(config);
        
        // Should allow requests within limits
        for _ in 0..5 {
            assert!(limiter.is_allowed("client1"));
        }
        
        // Should still allow burst requests
        for _ in 0..5 {
            assert!(limiter.is_allowed("client1"));
        }
        
        // Should reject after burst limit
        assert!(!limiter.is_allowed("client1"));
    }

    #[test]
    fn test_security_manager() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        
        // Should work with default config (no auth)
        let credentials = AuthCredentials {
            credential_type: CredentialType::UsernamePassword,
            data: Bytes::new(),
            metadata: HashMap::new(),
        };
        
        let context = manager.authenticate(&credentials).unwrap();
        assert_eq!(context.user_id, "anonymous");
    }
}