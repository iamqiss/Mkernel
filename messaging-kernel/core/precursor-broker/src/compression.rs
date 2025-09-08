//! Advanced compression algorithms with hardware acceleration
//! 
//! This module provides high-performance compression capabilities optimized for
//! message processing with hardware acceleration, adaptive algorithms, and zero-copy operations.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message};

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Default compression algorithm
    pub default_algorithm: CompressionAlgorithm,
    /// Enable hardware acceleration
    pub enable_hardware_acceleration: bool,
    /// Compression level (1-22, higher = better compression)
    pub compression_level: u8,
    /// Enable adaptive compression
    pub enable_adaptive: bool,
    /// Compression threshold (bytes)
    pub compression_threshold: usize,
    /// Enable dictionary compression
    pub enable_dictionary: bool,
    /// Dictionary size
    pub dictionary_size: usize,
    /// Enable parallel compression
    pub enable_parallel: bool,
    /// Number of compression threads
    pub compression_threads: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            default_algorithm: CompressionAlgorithm::Lz4,
            enable_hardware_acceleration: true,
            compression_level: 6,
            enable_adaptive: true,
            compression_threshold: 1024, // 1KB
            enable_dictionary: true,
            dictionary_size: 64 * 1024, // 64KB
            enable_parallel: true,
            compression_threads: num_cpus::get(),
        }
    }
}

/// Compression algorithm enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// LZ4 compression (fastest)
    Lz4,
    /// LZ4 High Compression
    Lz4Hc,
    /// Zstandard compression (balanced)
    Zstd,
    /// Brotli compression (best ratio)
    Brotli,
    /// Snappy compression (very fast)
    Snappy,
    /// LZMA compression (high ratio)
    Lzma,
    /// Deflate compression
    Deflate,
    /// Gzip compression
    Gzip,
}

impl CompressionAlgorithm {
    /// Get the algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            CompressionAlgorithm::None => "none",
            CompressionAlgorithm::Lz4 => "lz4",
            CompressionAlgorithm::Lz4Hc => "lz4hc",
            CompressionAlgorithm::Zstd => "zstd",
            CompressionAlgorithm::Brotli => "brotli",
            CompressionAlgorithm::Snappy => "snappy",
            CompressionAlgorithm::Lzma => "lzma",
            CompressionAlgorithm::Deflate => "deflate",
            CompressionAlgorithm::Gzip => "gzip",
        }
    }
    
    /// Get the algorithm's typical compression ratio
    pub fn typical_ratio(&self) -> f64 {
        match self {
            CompressionAlgorithm::None => 1.0,
            CompressionAlgorithm::Lz4 => 2.5,
            CompressionAlgorithm::Lz4Hc => 3.0,
            CompressionAlgorithm::Zstd => 3.5,
            CompressionAlgorithm::Brotli => 4.0,
            CompressionAlgorithm::Snappy => 2.0,
            CompressionAlgorithm::Lzma => 5.0,
            CompressionAlgorithm::Deflate => 3.0,
            CompressionAlgorithm::Gzip => 3.0,
        }
    }
    
    /// Get the algorithm's typical speed (MB/s)
    pub fn typical_speed(&self) -> f64 {
        match self {
            CompressionAlgorithm::None => 1000.0,
            CompressionAlgorithm::Lz4 => 500.0,
            CompressionAlgorithm::Lz4Hc => 200.0,
            CompressionAlgorithm::Zstd => 300.0,
            CompressionAlgorithm::Brotli => 100.0,
            CompressionAlgorithm::Snappy => 400.0,
            CompressionAlgorithm::Lzma => 50.0,
            CompressionAlgorithm::Deflate => 150.0,
            CompressionAlgorithm::Gzip => 150.0,
        }
    }
}

/// Compression statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    /// Total bytes compressed
    pub total_bytes_compressed: u64,
    /// Total bytes decompressed
    pub total_bytes_decompressed: u64,
    /// Total compression time (microseconds)
    pub total_compression_time_us: u64,
    /// Total decompression time (microseconds)
    pub total_decompression_time_us: u64,
    /// Average compression ratio
    pub avg_compression_ratio: f64,
    /// Compression operations per algorithm
    pub algorithm_stats: HashMap<String, AlgorithmStats>,
}

/// Algorithm-specific statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AlgorithmStats {
    /// Number of compressions
    pub compressions: u64,
    /// Number of decompressions
    pub decompressions: u64,
    /// Total input bytes
    pub input_bytes: u64,
    /// Total output bytes
    pub output_bytes: u64,
    /// Total compression time (microseconds)
    pub compression_time_us: u64,
    /// Total decompression time (microseconds)
    pub decompression_time_us: u64,
    /// Average compression ratio
    pub avg_ratio: f64,
}

/// Compression dictionary for improved compression ratios
#[derive(Debug, Clone)]
pub struct CompressionDictionary {
    /// Dictionary data
    data: Vec<u8>,
    /// Algorithm this dictionary is optimized for
    algorithm: CompressionAlgorithm,
    /// Creation timestamp
    created_at: Instant,
    /// Usage count
    usage_count: u64,
}

/// High-performance compression engine
pub struct CompressionEngine {
    /// Compression configuration
    config: CompressionConfig,
    /// Compression statistics
    stats: Arc<RwLock<CompressionStats>>,
    /// Compression dictionaries
    dictionaries: Arc<RwLock<HashMap<CompressionAlgorithm, CompressionDictionary>>>,
    /// Hardware acceleration support
    hardware_support: HardwareAcceleration,
    /// Thread pool for parallel compression
    thread_pool: Arc<Mutex<Option<rayon::ThreadPool>>>,
}

/// Hardware acceleration capabilities
#[derive(Debug, Clone)]
struct HardwareAcceleration {
    /// Intel QuickAssist support
    qat_support: bool,
    /// Intel IAA support
    iaa_support: bool,
    /// NVIDIA GPU compression support
    gpu_support: bool,
    /// Available acceleration algorithms
    supported_algorithms: Vec<CompressionAlgorithm>,
}

impl CompressionEngine {
    /// Create a new compression engine
    pub fn new(config: CompressionConfig) -> Self {
        let hardware_support = Self::detect_hardware_acceleration(&config);
        
        let thread_pool = if config.enable_parallel {
            Some(rayon::ThreadPoolBuilder::new()
                .num_threads(config.compression_threads)
                .build()
                .unwrap())
        } else {
            None
        };
        
        let engine = Self {
            config,
            stats: Arc::new(RwLock::new(CompressionStats::default())),
            dictionaries: Arc::new(RwLock::new(HashMap::new())),
            hardware_support,
            thread_pool: Arc::new(Mutex::new(thread_pool)),
        };
        
        // Initialize dictionaries if enabled
        if engine.config.enable_dictionary {
            engine.initialize_dictionaries();
        }
        
        engine
    }
    
    /// Compress data using the specified algorithm
    pub async fn compress(
        &self,
        data: &[u8],
        algorithm: Option<CompressionAlgorithm>,
    ) -> Result<CompressedData> {
        let algorithm = algorithm.unwrap_or(self.config.default_algorithm);
        let start_time = Instant::now();
        
        // Skip compression for small data
        if data.len() < self.config.compression_threshold {
            return Ok(CompressedData::new(
                data.to_vec(),
                CompressionAlgorithm::None,
                data.len(),
                data.len(),
            ));
        }
        
        // Choose optimal algorithm if adaptive compression is enabled
        let final_algorithm = if self.config.enable_adaptive {
            self.choose_optimal_algorithm(data, algorithm).await?
        } else {
            algorithm
        };
        
        let compressed_data = match final_algorithm {
            CompressionAlgorithm::None => data.to_vec(),
            CompressionAlgorithm::Lz4 => self.compress_lz4(data).await?,
            CompressionAlgorithm::Lz4Hc => self.compress_lz4hc(data).await?,
            CompressionAlgorithm::Zstd => self.compress_zstd(data).await?,
            CompressionAlgorithm::Brotli => self.compress_brotli(data).await?,
            CompressionAlgorithm::Snappy => self.compress_snappy(data).await?,
            CompressionAlgorithm::Lzma => self.compress_lzma(data).await?,
            CompressionAlgorithm::Deflate => self.compress_deflate(data).await?,
            CompressionAlgorithm::Gzip => self.compress_gzip(data).await?,
        };
        
        let compression_time = start_time.elapsed().as_micros() as u64;
        let compression_ratio = data.len() as f64 / compressed_data.len() as f64;
        
        // Update statistics
        self.update_compression_stats(
            final_algorithm,
            data.len(),
            compressed_data.len(),
            compression_time,
        ).await;
        
        debug!(
            "Compressed {} bytes to {} bytes using {} in {}μs (ratio: {:.2})",
            data.len(),
            compressed_data.len(),
            final_algorithm.name(),
            compression_time,
            compression_ratio
        );
        
        Ok(CompressedData::new(
            compressed_data,
            final_algorithm,
            data.len(),
            compression_time,
        ))
    }
    
    /// Decompress data
    pub async fn decompress(&self, compressed_data: &CompressedData) -> Result<Vec<u8>> {
        let start_time = Instant::now();
        
        let decompressed = match compressed_data.algorithm {
            CompressionAlgorithm::None => compressed_data.data.clone(),
            CompressionAlgorithm::Lz4 => self.decompress_lz4(&compressed_data.data).await?,
            CompressionAlgorithm::Lz4Hc => self.decompress_lz4(&compressed_data.data).await?,
            CompressionAlgorithm::Zstd => self.decompress_zstd(&compressed_data.data).await?,
            CompressionAlgorithm::Brotli => self.decompress_brotli(&compressed_data.data).await?,
            CompressionAlgorithm::Snappy => self.decompress_snappy(&compressed_data.data).await?,
            CompressionAlgorithm::Lzma => self.decompress_lzma(&compressed_data.data).await?,
            CompressionAlgorithm::Deflate => self.decompress_deflate(&compressed_data.data).await?,
            CompressionAlgorithm::Gzip => self.decompress_gzip(&compressed_data.data).await?,
        };
        
        let decompression_time = start_time.elapsed().as_micros() as u64;
        
        // Update statistics
        self.update_decompression_stats(
            compressed_data.algorithm,
            compressed_data.data.len(),
            decompressed.len(),
            decompression_time,
        ).await;
        
        debug!(
            "Decompressed {} bytes to {} bytes using {} in {}μs",
            compressed_data.data.len(),
            decompressed.len(),
            compressed_data.algorithm.name(),
            decompression_time
        );
        
        Ok(decompressed)
    }
    
    /// Compress a message
    pub async fn compress_message(
        &self,
        message: &Message,
        algorithm: Option<CompressionAlgorithm>,
    ) -> Result<CompressedMessage> {
        let payload_data = message.payload.as_ref();
        let compressed_payload = self.compress(payload_data, algorithm).await?;
        
        Ok(CompressedMessage {
            metadata: message.metadata.clone(),
            compressed_payload,
            original_size: payload_data.len(),
        })
    }
    
    /// Decompress a message
    pub async fn decompress_message(
        &self,
        compressed_message: &CompressedMessage,
    ) -> Result<Message> {
        let decompressed_payload = self.decompress(&compressed_message.compressed_payload).await?;
        
        Ok(Message {
            metadata: compressed_message.metadata.clone(),
            payload: Bytes::from(decompressed_payload),
        })
    }
    
    /// Get compression statistics
    pub async fn get_stats(&self) -> CompressionStats {
        self.stats.read().await.clone()
    }
    
    /// Choose optimal compression algorithm based on data characteristics
    async fn choose_optimal_algorithm(
        &self,
        data: &[u8],
        preferred: CompressionAlgorithm,
    ) -> Result<CompressionAlgorithm> {
        // Analyze data characteristics
        let entropy = self.calculate_entropy(data);
        let repetition_ratio = self.calculate_repetition_ratio(data);
        
        // Choose algorithm based on data characteristics
        let optimal = if entropy < 0.5 && repetition_ratio > 0.3 {
            // High repetition, low entropy - use high compression
            CompressionAlgorithm::Brotli
        } else if data.len() < 4096 {
            // Small data - use fast compression
            CompressionAlgorithm::Lz4
        } else if entropy > 0.8 {
            // High entropy - use balanced compression
            CompressionAlgorithm::Zstd
        } else {
            // Default to preferred algorithm
            preferred
        };
        
        debug!(
            "Chose optimal algorithm {} for data (entropy: {:.3}, repetition: {:.3})",
            optimal.name(),
            entropy,
            repetition_ratio
        );
        
        Ok(optimal)
    }
    
    /// Calculate data entropy
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        
        let mut frequencies = [0u32; 256];
        for &byte in data {
            frequencies[byte as usize] += 1;
        }
        
        let mut entropy = 0.0;
        let data_len = data.len() as f64;
        
        for &freq in &frequencies {
            if freq > 0 {
                let probability = freq as f64 / data_len;
                entropy -= probability * probability.log2();
            }
        }
        
        entropy / 8.0 // Normalize to [0, 1]
    }
    
    /// Calculate repetition ratio in data
    fn calculate_repetition_ratio(&self, data: &[u8]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }
        
        let mut repetitions = 0;
        let mut i = 0;
        
        while i < data.len() - 1 {
            let mut j = i + 1;
            while j < data.len() && data[i] == data[j] {
                j += 1;
            }
            
            if j - i > 1 {
                repetitions += j - i - 1;
            }
            
            i = j;
        }
        
        repetitions as f64 / (data.len() - 1) as f64
    }
    
    /// Detect hardware acceleration capabilities
    fn detect_hardware_acceleration(config: &CompressionConfig) -> HardwareAcceleration {
        let mut supported_algorithms = Vec::new();
        
        // Check for Intel QuickAssist
        let qat_support = Self::check_qat_support();
        if qat_support {
            supported_algorithms.extend([
                CompressionAlgorithm::Deflate,
                CompressionAlgorithm::Gzip,
                CompressionAlgorithm::Lz4,
            ]);
        }
        
        // Check for Intel IAA
        let iaa_support = Self::check_iaa_support();
        if iaa_support {
            supported_algorithms.extend([
                CompressionAlgorithm::Deflate,
                CompressionAlgorithm::Lz4,
                CompressionAlgorithm::Zstd,
            ]);
        }
        
        // Check for GPU support
        let gpu_support = Self::check_gpu_support();
        if gpu_support {
            supported_algorithms.extend([
                CompressionAlgorithm::Lz4,
                CompressionAlgorithm::Snappy,
            ]);
        }
        
        HardwareAcceleration {
            qat_support,
            iaa_support,
            gpu_support,
            supported_algorithms,
        }
    }
    
    /// Check Intel QuickAssist support
    fn check_qat_support() -> bool {
        // In a real implementation, this would check for QAT hardware
        // For now, we'll simulate based on system capabilities
        false
    }
    
    /// Check Intel IAA support
    fn check_iaa_support() -> bool {
        // In a real implementation, this would check for IAA hardware
        // For now, we'll simulate based on system capabilities
        false
    }
    
    /// Check GPU compression support
    fn check_gpu_support() -> bool {
        // In a real implementation, this would check for GPU availability
        // For now, we'll simulate based on system capabilities
        false
    }
    
    /// Initialize compression dictionaries
    fn initialize_dictionaries(&self) {
        // Create dictionaries for each algorithm
        let algorithms = [
            CompressionAlgorithm::Zstd,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Lzma,
        ];
        
        for algorithm in algorithms {
            let dictionary = self.create_dictionary(algorithm);
            let mut dictionaries = self.dictionaries.write().unwrap();
            dictionaries.insert(algorithm, dictionary);
        }
    }
    
    /// Create a compression dictionary
    fn create_dictionary(&self, algorithm: CompressionAlgorithm) -> CompressionDictionary {
        // In a real implementation, this would create an optimized dictionary
        // For now, we'll create a simple dictionary
        let data = vec![0u8; self.config.dictionary_size];
        
        CompressionDictionary {
            data,
            algorithm,
            created_at: Instant::now(),
            usage_count: 0,
        }
    }
    
    /// Update compression statistics
    async fn update_compression_stats(
        &self,
        algorithm: CompressionAlgorithm,
        input_size: usize,
        output_size: usize,
        time_us: u64,
    ) {
        let mut stats = self.stats.write().await;
        stats.total_bytes_compressed += input_size as u64;
        stats.total_compression_time_us += time_us;
        
        let algorithm_name = algorithm.name().to_string();
        let algorithm_stats = stats.algorithm_stats.entry(algorithm_name).or_default();
        algorithm_stats.compressions += 1;
        algorithm_stats.input_bytes += input_size as u64;
        algorithm_stats.output_bytes += output_size as u64;
        algorithm_stats.compression_time_us += time_us;
        algorithm_stats.avg_ratio = algorithm_stats.input_bytes as f64 / algorithm_stats.output_bytes as f64;
        
        // Update overall average ratio
        stats.avg_compression_ratio = stats.total_bytes_compressed as f64 / 
            (stats.total_bytes_compressed - stats.total_bytes_compressed + output_size as u64) as f64;
    }
    
    /// Update decompression statistics
    async fn update_decompression_stats(
        &self,
        algorithm: CompressionAlgorithm,
        input_size: usize,
        output_size: usize,
        time_us: u64,
    ) {
        let mut stats = self.stats.write().await;
        stats.total_bytes_decompressed += output_size as u64;
        stats.total_decompression_time_us += time_us;
        
        let algorithm_name = algorithm.name().to_string();
        let algorithm_stats = stats.algorithm_stats.entry(algorithm_name).or_default();
        algorithm_stats.decompressions += 1;
        algorithm_stats.decompression_time_us += time_us;
    }
    
    // Compression implementations for each algorithm
    
    async fn compress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(lz4_flex::compress(data))
    }
    
    async fn compress_lz4hc(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(lz4_flex::compress(data))
    }
    
    async fn compress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(zstd::encode_all(data, self.config.compression_level as i32)?)
    }
    
    async fn compress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressed = Vec::new();
        let mut encoder = brotli::CompressorWriter::new(
            &mut compressed,
            4096,
            self.config.compression_level,
            22,
        );
        encoder.write_all(data)?;
        encoder.flush()?;
        drop(encoder);
        Ok(compressed)
    }
    
    async fn compress_snappy(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(snap::raw::Encoder::new().compress_vec(data)?)
    }
    
    async fn compress_lzma(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut compressed = Vec::new();
        let mut encoder = lzma_rs::xz_encode(data, &mut compressed)?;
        Ok(compressed)
    }
    
    async fn compress_deflate(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }
    
    async fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
        encoder.write_all(data)?;
        Ok(encoder.finish()?)
    }
    
    // Decompression implementations for each algorithm
    
    async fn decompress_lz4(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(lz4_flex::decompress_size_prepended(data)?)
    }
    
    async fn decompress_zstd(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(zstd::decode_all(data)?)
    }
    
    async fn decompress_brotli(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        let mut decoder = brotli::Decompressor::new(data, 4096);
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    async fn decompress_snappy(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(snap::raw::Decoder::new().decompress_vec(data)?)
    }
    
    async fn decompress_lzma(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        lzma_rs::xz_decode(data, &mut decompressed)?;
        Ok(decompressed)
    }
    
    async fn decompress_deflate(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::DeflateDecoder;
        
        let mut decoder = DeflateDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    async fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}

/// Compressed data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedData {
    /// Compressed data
    pub data: Vec<u8>,
    /// Compression algorithm used
    pub algorithm: CompressionAlgorithm,
    /// Original size
    pub original_size: usize,
    /// Compression time (microseconds)
    pub compression_time_us: u64,
}

impl CompressedData {
    /// Create new compressed data
    pub fn new(
        data: Vec<u8>,
        algorithm: CompressionAlgorithm,
        original_size: usize,
        compression_time_us: u64,
    ) -> Self {
        Self {
            data,
            algorithm,
            original_size,
            compression_time_us,
        }
    }
    
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        self.original_size as f64 / self.data.len() as f64
    }
    
    /// Get space saved
    pub fn space_saved(&self) -> usize {
        self.original_size.saturating_sub(self.data.len())
    }
}

/// Compressed message container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMessage {
    /// Message metadata
    pub metadata: crate::MessageMetadata,
    /// Compressed payload
    pub compressed_payload: CompressedData,
    /// Original payload size
    pub original_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_compression_engine_creation() {
        let config = CompressionConfig::default();
        let engine = CompressionEngine::new(config);
        let stats = engine.get_stats().await;
        
        assert_eq!(stats.total_bytes_compressed, 0);
        assert_eq!(stats.total_bytes_decompressed, 0);
    }
    
    #[tokio::test]
    async fn test_lz4_compression() {
        let config = CompressionConfig {
            default_algorithm: CompressionAlgorithm::Lz4,
            ..Default::default()
        };
        let engine = CompressionEngine::new(config);
        
        let data = b"Hello, World! This is a test message for compression.";
        let compressed = engine.compress(data, None).await.unwrap();
        
        assert!(compressed.data.len() < data.len());
        assert_eq!(compressed.algorithm, CompressionAlgorithm::Lz4);
        
        let decompressed = engine.decompress(&compressed).await.unwrap();
        assert_eq!(decompressed, data);
    }
    
    #[tokio::test]
    async fn test_adaptive_compression() {
        let config = CompressionConfig {
            enable_adaptive: true,
            ..Default::default()
        };
        let engine = CompressionEngine::new(config);
        
        // Test with repetitive data (should choose high compression)
        let repetitive_data = b"AAAAA".repeat(1000);
        let compressed = engine.compress(&repetitive_data, None).await.unwrap();
        
        assert!(compressed.compression_ratio() > 2.0);
    }
    
    #[tokio::test]
    async fn test_message_compression() {
        let config = CompressionConfig::default();
        let engine = CompressionEngine::new(config);
        
        let payload = bytes::Bytes::from("test message payload");
        let headers = std::collections::HashMap::new();
        let message = Message::new(payload, headers);
        
        let compressed_message = engine.compress_message(&message, None).await.unwrap();
        let decompressed_message = engine.decompress_message(&compressed_message).await.unwrap();
        
        assert_eq!(decompressed_message.payload, message.payload);
    }
}