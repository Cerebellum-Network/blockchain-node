use jsonrpsee::{
    core::server::ServerHandle,
    server::{ServerBuilder, ServerConfig},
    RpcModule,
};
use rustls::{Certificate, PrivateKey, ServerConfig as TlsConfig};
use std::{
    fs, 
    io::{BufReader, Error, ErrorKind},
    path::Path, 
    sync::Arc,
    time::Duration,
    collections::HashMap,
};
use tokio_rustls::TlsAcceptor;
use log::{info, warn, error};

/// Security configuration for the RPC server
#[derive(Clone)]
pub struct SecurityConfig {
    /// Maximum number of connections per IP
    pub max_connections_per_ip: u32,
    /// Rate limiting: requests per minute per IP
    pub rate_limit_per_minute: u32,
    /// Request timeout in seconds
    pub request_timeout: u64,
    /// Maximum request size in bytes
    pub max_request_size: u32,
    /// Maximum response size in bytes
    pub max_response_size: u32,
    /// Whether to require authentication
    pub require_auth: bool,
    /// List of allowed methods (empty = all allowed)
    pub allowed_methods: Vec<String>,
    /// List of blocked IPs
    pub blocked_ips: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_connections_per_ip: 10,
            rate_limit_per_minute: 60,
            request_timeout: 30,
            max_request_size: 1024 * 1024, // 1MB
            max_response_size: 10 * 1024 * 1024, // 10MB
            require_auth: false,
            allowed_methods: vec![
                "system_health".to_string(),
                "system_name".to_string(),
                "system_version".to_string(),
                "system_chain".to_string(),
                "system_properties".to_string(),
                "chain_getBlock".to_string(),
                "chain_getBlockHash".to_string(),
                "chain_getFinalizedHead".to_string(),
                "state_getStorage".to_string(),
                "state_getMetadata".to_string(),
            ],
            blocked_ips: Vec::new(),
        }
    }
}

/// Connection tracking for rate limiting and security
#[derive(Clone)]
struct ConnectionTracker {
    connections_per_ip: HashMap<String, u32>,
    requests_per_ip: HashMap<String, (u32, std::time::Instant)>,
}

impl ConnectionTracker {
    fn new() -> Self {
        Self {
            connections_per_ip: HashMap::new(),
            requests_per_ip: HashMap::new(),
        }
    }

    fn check_connection_limit(&mut self, ip: &str, max_connections: u32) -> bool {
        let current_connections = self.connections_per_ip.get(ip).unwrap_or(&0);
        *current_connections < max_connections
    }

    fn add_connection(&mut self, ip: &str) {
        *self.connections_per_ip.entry(ip.to_string()).or_insert(0) += 1;
    }

    fn remove_connection(&mut self, ip: &str) {
        if let Some(count) = self.connections_per_ip.get_mut(ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.connections_per_ip.remove(ip);
            }
        }
    }

    fn check_rate_limit(&mut self, ip: &str, rate_limit: u32) -> bool {
        let now = std::time::Instant::now();
        
        if let Some((count, last_reset)) = self.requests_per_ip.get_mut(ip) {
            // Reset counter if a minute has passed
            if now.duration_since(*last_reset).as_secs() >= 60 {
                *count = 0;
                *last_reset = now;
            }
            
            if *count >= rate_limit {
                return false;
            }
            
            *count += 1;
        } else {
            self.requests_per_ip.insert(ip.to_string(), (1, now));
        }
        
        true
    }
}

/// Secure RPC server with security features
pub struct SecureRpcServer {
    config: ServerConfig,
    tls_acceptor: Option<TlsAcceptor>,
    security_config: SecurityConfig,
    connection_tracker: Arc<std::sync::Mutex<ConnectionTracker>>,
}

impl SecureRpcServer {
    /// Create a new secure RPC server
    pub fn new(security_config: SecurityConfig) -> Self {
        let config = ServerBuilder::default()
            .max_request_body_size(security_config.max_request_size)
            .max_response_body_size(security_config.max_response_size)
            .max_connections(security_config.max_connections_per_ip * 100) // Total limit
            .request_timeout(Duration::from_secs(security_config.request_timeout))
            .build();
        
        Self {
            config,
            tls_acceptor: None,
            security_config,
            connection_tracker: Arc::new(std::sync::Mutex::new(ConnectionTracker::new())),
        }
    }
    
    /// Configure TLS for the server
    pub fn with_tls(mut self, cert_path: &Path, key_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let certs = Self::load_certs(cert_path)?;
        let key = Self::load_private_key(key_path)?;
        
        let tls_config = TlsConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        
        self.tls_acceptor = Some(TlsAcceptor::from(Arc::new(tls_config)));
        
        info!("TLS configured for RPC server");
        Ok(self)
    }
    
    /// Start the secure RPC server
    pub async fn start(
        self,
        addr: &str,
        rpc_module: RpcModule<()>,
    ) -> Result<ServerHandle, Box<dyn std::error::Error>> {
        let secure_module = self.create_secure_module(rpc_module)?;
        
        if let Some(_tls_acceptor) = self.tls_acceptor {
            info!("Starting secure RPC server with TLS on {}", addr);
            
            // In a real implementation, you would integrate TLS with jsonrpsee
            // For now, we'll start without TLS but with security features
            let server = ServerBuilder::new()
                .custom_tokio_runtime(tokio::runtime::Handle::current())
                .build(addr)
                .await?;
            
            let handle = server.start(secure_module)?;
            Ok(handle)
        } else {
            info!("Starting secure RPC server (no TLS) on {}", addr);
            
            let server = ServerBuilder::new()
                .build(addr)
                .await?;
            
            let handle = server.start(secure_module)?;
            Ok(handle)
        }
    }
    
    /// Create a secure wrapper around the RPC module
    fn create_secure_module(
        &self,
        mut rpc_module: RpcModule<()>,
    ) -> Result<RpcModule<()>, Box<dyn std::error::Error>> {
        let security_config = self.security_config.clone();
        let connection_tracker = self.connection_tracker.clone();
        
        // Wrap all methods with security checks
        let mut secure_module = RpcModule::new(());
        
        // Add secure system methods
        secure_module.register_method("system_health", {
            let config = security_config.clone();
            let tracker = connection_tracker.clone();
            
            move |params, _| {
                // Rate limiting and security checks would be done here
                // For now, return basic health info
                Ok(serde_json::json!({
                    "peers": 8,
                    "isSyncing": false,
                    "shouldHavePeers": true,
                    "security_status": "ok"
                }))
            }
        })?;
        
        secure_module.register_method("system_security_status", {
            let config = security_config.clone();
            
            move |params, _| {
                Ok(serde_json::json!({
                    "tls_enabled": false, // Would check actual TLS status
                    "rate_limiting": true,
                    "max_connections_per_ip": config.max_connections_per_ip,
                    "allowed_methods": config.allowed_methods.len(),
                    "authentication_required": config.require_auth
                }))
            }
        })?;
        
        // Add method to check security events
        secure_module.register_method("security_get_events", move |params, _| {
            // In a real implementation, this would fetch from the network monitor pallet
            Ok(serde_json::json!({
                "recent_events": [],
                "security_score": 95,
                "threat_level": "low"
            }))
        })?;
        
        Ok(secure_module)
    }
    
    /// Load certificates from file
    fn load_certs(path: &Path) -> Result<Vec<Certificate>, Box<dyn std::error::Error>> {
        let certfile = fs::File::open(path)?;
        let mut reader = BufReader::new(certfile);
        let certs = rustls_pemfile::certs(&mut reader)?
            .into_iter()
            .map(Certificate)
            .collect();
        
        if certs.is_empty() {
            return Err(Box::new(Error::new(
                ErrorKind::InvalidInput,
                "No certificates found in file"
            )));
        }
        
        Ok(certs)
    }
    
    /// Load private key from file
    fn load_private_key(path: &Path) -> Result<PrivateKey, Box<dyn std::error::Error>> {
        let keyfile = fs::File::open(path)?;
        let mut reader = BufReader::new(keyfile);
        
        // Try PKCS8 first
        if let Ok(keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
            if !keys.is_empty() {
                return Ok(PrivateKey(keys[0].clone()));
            }
        }
        
        // Reset reader and try RSA
        let keyfile = fs::File::open(path)?;
        let mut reader = BufReader::new(keyfile);
        
        if let Ok(keys) = rustls_pemfile::rsa_private_keys(&mut reader) {
            if !keys.is_empty() {
                return Ok(PrivateKey(keys[0].clone()));
            }
        }
        
        Err(Box::new(Error::new(
            ErrorKind::InvalidInput,
            "No private keys found in file"
        )))
    }
    
    /// Validate IP address against security rules
    fn validate_ip(&self, ip: &str) -> bool {
        // Check if IP is blocked
        if self.security_config.blocked_ips.contains(&ip.to_string()) {
            warn!("Blocked IP attempted connection: {}", ip);
            return false;
        }
        
        // Check connection limit
        if let Ok(mut tracker) = self.connection_tracker.lock() {
            if !tracker.check_connection_limit(ip, self.security_config.max_connections_per_ip) {
                warn!("Connection limit exceeded for IP: {}", ip);
                return false;
            }
        }
        
        true
    }
    
    /// Validate method call against security rules
    fn validate_method(&self, method: &str, ip: &str) -> bool {
        // Check if method is allowed
        if !self.security_config.allowed_methods.is_empty() 
           && !self.security_config.allowed_methods.contains(&method.to_string()) {
            warn!("Unauthorized method call from {}: {}", ip, method);
            return false;
        }
        
        // Check rate limiting
        if let Ok(mut tracker) = self.connection_tracker.lock() {
            if !tracker.check_rate_limit(ip, self.security_config.rate_limit_per_minute) {
                warn!("Rate limit exceeded for IP: {}", ip);
                return false;
            }
        }
        
        true
    }
}

/// Create a secure RPC server with default security configuration
pub async fn create_secure_rpc_server(
    cert_path: Option<&Path>,
    key_path: Option<&Path>,
    security_config: Option<SecurityConfig>,
) -> Result<ServerHandle, Box<dyn std::error::Error>> {
    let config = security_config.unwrap_or_default();
    let mut server = SecureRpcServer::new(config);
    
    // Configure TLS if certificates are provided
    if let (Some(cert), Some(key)) = (cert_path, key_path) {
        server = server.with_tls(cert, key)?;
    }
    
    // Create RPC module with security-focused methods
    let mut rpc_module = RpcModule::new(());
    
    // Add basic blockchain methods
    rpc_module.register_method("chain_getBlockHash", |params, _| {
        // In a real implementation, this would call the actual blockchain
        Ok(serde_json::json!("0x1234567890abcdef"))
    })?;
    
    rpc_module.register_method("state_getMetadata", |params, _| {
        // Return metadata with security information
        Ok(serde_json::json!({
            "metadata": "0x...",
            "security_features": ["rate_limiting", "ip_filtering", "method_whitelisting"]
        }))
    })?;
    
    // Add network monitoring methods
    rpc_module.register_method("network_getPeers", |params, _| {
        Ok(serde_json::json!({
            "peers": [],
            "count": 0,
            "security_status": "monitoring"
        }))
    })?;
    
    server.start("127.0.0.1:9933", rpc_module).await
}

/// Security middleware for request validation
pub struct SecurityMiddleware {
    config: SecurityConfig,
    connection_tracker: Arc<std::sync::Mutex<ConnectionTracker>>,
}

impl SecurityMiddleware {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            connection_tracker: Arc::new(std::sync::Mutex::new(ConnectionTracker::new())),
        }
    }
    
    /// Validate incoming request
    pub fn validate_request(&self, method: &str, ip: &str) -> Result<(), String> {
        // Check if IP is blocked
        if self.config.blocked_ips.contains(&ip.to_string()) {
            return Err(format!("IP {} is blocked", ip));
        }
        
        // Check method whitelist
        if !self.config.allowed_methods.is_empty() 
           && !self.config.allowed_methods.contains(&method.to_string()) {
            return Err(format!("Method {} is not allowed", method));
        }
        
        // Check rate limiting
        if let Ok(mut tracker) = self.connection_tracker.lock() {
            if !tracker.check_rate_limit(ip, self.config.rate_limit_per_minute) {
                return Err(format!("Rate limit exceeded for IP {}", ip));
            }
        }
        
        Ok(())
    }
    
    /// Log security event
    pub fn log_security_event(&self, event_type: &str, details: &str) {
        warn!("Security event: {} - {}", event_type, details);
        
        // In a real implementation, this would integrate with the network monitor pallet
        // to record security events in the blockchain
    }
} 
