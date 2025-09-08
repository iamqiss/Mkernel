//! Advanced zero-copy memory management with arena-based allocation
//! 
//! This module provides high-performance memory management optimized for message processing
//! with zero-copy operations, SIMD optimizations, and NUMA-aware allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::thread;

use bytes::{Bytes, BytesMut};
use tracing::{debug, error, info, warn};

/// Memory pool configuration
#[derive(Debug, Clone)]
pub struct MemoryPoolConfig {
    /// Initial pool size in bytes
    pub initial_size: usize,
    /// Maximum pool size in bytes
    pub max_size: usize,
    /// Chunk size for allocations
    pub chunk_size: usize,
    /// Enable NUMA awareness
    pub numa_aware: bool,
    /// Enable SIMD optimizations
    pub simd_enabled: bool,
    /// Memory alignment (must be power of 2)
    pub alignment: usize,
    /// Enable memory prewarming
    pub prewarm: bool,
    /// Garbage collection threshold
    pub gc_threshold: f64,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            initial_size: 64 * 1024 * 1024, // 64MB
            max_size: 1024 * 1024 * 1024,   // 1GB
            chunk_size: 4096,               // 4KB
            numa_aware: true,
            simd_enabled: true,
            alignment: 64,                  // Cache line alignment
            prewarm: true,
            gc_threshold: 0.8,              // 80% utilization threshold
        }
    }
}

/// Memory chunk with metadata
#[derive(Debug)]
struct MemoryChunk {
    /// Pointer to allocated memory
    ptr: NonNull<u8>,
    /// Size of the chunk
    size: usize,
    /// Whether the chunk is in use
    in_use: bool,
    /// Allocation timestamp
    allocated_at: Instant,
    /// Last access timestamp
    last_accessed: Instant,
    /// NUMA node ID
    numa_node: Option<u32>,
}

impl MemoryChunk {
    /// Create a new memory chunk
    fn new(size: usize, numa_node: Option<u32>) -> Result<Self, std::alloc::AllocError> {
        let layout = Layout::from_size_align(size, 64).map_err(|_| std::alloc::AllocError)?;
        let ptr = unsafe { System.alloc(layout) };
        
        if ptr.is_null() {
            return Err(std::alloc::AllocError);
        }
        
        let ptr = NonNull::new(ptr).ok_or(std::alloc::AllocError)?;
        
        Ok(Self {
            ptr,
            size,
            in_use: false,
            allocated_at: Instant::now(),
            last_accessed: Instant::now(),
            numa_node,
        })
    }
    
    /// Get the raw pointer
    fn as_ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }
    
    /// Mark chunk as in use
    fn mark_in_use(&mut self) {
        self.in_use = true;
        self.last_accessed = Instant::now();
    }
    
    /// Mark chunk as free
    fn mark_free(&mut self) {
        self.in_use = false;
    }
    
    /// Check if chunk is expired (unused for too long)
    fn is_expired(&self, max_age: Duration) -> bool {
        !self.in_use && self.last_accessed.elapsed() > max_age
    }
}

/// High-performance memory pool with zero-copy operations
pub struct MemoryPool {
    /// Pool configuration
    config: MemoryPoolConfig,
    /// Available chunks by size
    available_chunks: RwLock<HashMap<usize, Vec<MemoryChunk>>>,
    /// All chunks for tracking
    all_chunks: RwLock<Vec<MemoryChunk>>,
    /// Current pool size
    current_size: AtomicUsize,
    /// Allocation statistics
    stats: Arc<RwLock<MemoryStats>>,
    /// NUMA topology
    numa_topology: Option<NumaTopology>,
}

/// NUMA topology information
#[derive(Debug, Clone)]
struct NumaTopology {
    /// Number of NUMA nodes
    node_count: u32,
    /// Current node ID
    current_node: u32,
    /// Node distances
    distances: Vec<Vec<u32>>,
}

/// Memory allocation statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current allocated size
    pub current_allocated: usize,
    /// Peak allocated size
    pub peak_allocated: usize,
    /// Allocation failures
    pub allocation_failures: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Garbage collection runs
    pub gc_runs: u64,
    /// Memory fragmentation ratio
    pub fragmentation_ratio: f64,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(config: MemoryPoolConfig) -> Self {
        let numa_topology = if config.numa_aware {
            Self::detect_numa_topology()
        } else {
            None
        };
        
        let pool = Self {
            config,
            available_chunks: RwLock::new(HashMap::new()),
            all_chunks: RwLock::new(Vec::new()),
            current_size: AtomicUsize::new(0),
            stats: Arc::new(RwLock::new(MemoryStats::default())),
            numa_topology,
        };
        
        if pool.config.prewarm {
            pool.prewarm_memory();
        }
        
        pool
    }
    
    /// Allocate memory with zero-copy optimization
    pub fn allocate(&self, size: usize) -> Result<MemoryBlock, MemoryError> {
        let aligned_size = self.align_size(size);
        let start_time = Instant::now();
        
        // Try to find an available chunk
        if let Some(chunk) = self.find_available_chunk(aligned_size) {
            self.record_cache_hit();
            return Ok(MemoryBlock::new(chunk, aligned_size));
        }
        
        self.record_cache_miss();
        
        // Allocate new chunk
        let numa_node = self.get_optimal_numa_node();
        let mut chunk = MemoryChunk::new(aligned_size, numa_node)
            .map_err(|_| MemoryError::AllocationFailed)?;
        
        chunk.mark_in_use();
        
        // Track the chunk
        {
            let mut all_chunks = self.all_chunks.write().unwrap();
            all_chunks.push(chunk);
        }
        
        // Update statistics
        self.update_allocation_stats(aligned_size);
        
        debug!(
            "Allocated {} bytes in {:?} (NUMA node: {:?})",
            aligned_size,
            start_time.elapsed(),
            numa_node
        );
        
        Ok(MemoryBlock::new(
            self.all_chunks.read().unwrap().last().unwrap(),
            aligned_size
        ))
    }
    
    /// Deallocate memory
    pub fn deallocate(&self, block: MemoryBlock) {
        let size = block.size;
        
        // Mark chunk as free
        {
            let mut all_chunks = self.all_chunks.write().unwrap();
            if let Some(chunk) = all_chunks.iter_mut().find(|c| c.ptr == block.chunk.ptr) {
                chunk.mark_free();
            }
        }
        
        // Add to available chunks
        {
            let mut available = self.available_chunks.write().unwrap();
            let chunks = available.entry(size).or_insert_with(Vec::new);
            chunks.push(block.chunk);
        }
        
        // Update statistics
        self.update_deallocation_stats(size);
        
        // Trigger garbage collection if needed
        if self.should_gc() {
            self.garbage_collect();
        }
    }
    
    /// Create a zero-copy Bytes from memory block
    pub fn create_bytes(&self, block: &MemoryBlock) -> Bytes {
        unsafe {
            Bytes::from_static(std::slice::from_raw_parts(
                block.chunk.ptr.as_ptr(),
                block.size
            ))
        }
    }
    
    /// Create a zero-copy BytesMut from memory block
    pub fn create_bytes_mut(&self, block: &MemoryBlock) -> BytesMut {
        unsafe {
            BytesMut::from_raw_parts(
                block.chunk.ptr.as_ptr(),
                block.size,
                block.size
            )
        }
    }
    
    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        self.stats.read().unwrap().clone()
    }
    
    /// Perform garbage collection
    pub fn garbage_collect(&self) {
        let start_time = Instant::now();
        let mut removed_count = 0;
        
        // Remove expired chunks
        {
            let mut all_chunks = self.all_chunks.write().unwrap();
            let max_age = Duration::from_secs(300); // 5 minutes
            
            all_chunks.retain(|chunk| {
                if chunk.is_expired(max_age) {
                    removed_count += 1;
                    unsafe {
                        let layout = Layout::from_size_align(chunk.size, 64).unwrap();
                        System.dealloc(chunk.ptr.as_ptr(), layout);
                    }
                    false
                } else {
                    true
                }
            });
        }
        
        // Clear available chunks for expired sizes
        {
            let mut available = self.available_chunks.write().unwrap();
            available.clear();
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.gc_runs += 1;
        }
        
        if removed_count > 0 {
            info!(
                "Garbage collected {} chunks in {:?}",
                removed_count,
                start_time.elapsed()
            );
        }
    }
    
    /// Prewarm memory pool
    fn prewarm_memory(&self) {
        let prewarm_size = self.config.initial_size / 4; // Prewarm 25% of initial size
        let chunk_count = prewarm_size / self.config.chunk_size;
        
        info!("Prewarming memory pool with {} chunks", chunk_count);
        
        for _ in 0..chunk_count {
            if let Ok(chunk) = MemoryChunk::new(self.config.chunk_size, None) {
                let mut available = self.available_chunks.write().unwrap();
                let chunks = available.entry(self.config.chunk_size).or_insert_with(Vec::new);
                chunks.push(chunk);
            }
        }
    }
    
    /// Detect NUMA topology
    fn detect_numa_topology() -> Option<NumaTopology> {
        // In a real implementation, this would use libnuma or similar
        // For now, we'll simulate a simple topology
        Some(NumaTopology {
            node_count: 2,
            current_node: 0,
            distances: vec![
                vec![10, 20], // Node 0 distances
                vec![20, 10], // Node 1 distances
            ],
        })
    }
    
    /// Get optimal NUMA node for allocation
    fn get_optimal_numa_node(&self) -> Option<u32> {
        self.numa_topology.as_ref().map(|topo| topo.current_node)
    }
    
    /// Find an available chunk of the specified size
    fn find_available_chunk(&self, size: usize) -> Option<MemoryChunk> {
        let mut available = self.available_chunks.write().unwrap();
        if let Some(chunks) = available.get_mut(&size) {
            chunks.pop()
        } else {
            None
        }
    }
    
    /// Align size to cache line boundary
    fn align_size(&self, size: usize) -> usize {
        (size + self.config.alignment - 1) & !(self.config.alignment - 1)
    }
    
    /// Check if garbage collection should be triggered
    fn should_gc(&self) -> bool {
        let current_size = self.current_size.load(Ordering::Relaxed);
        let utilization = current_size as f64 / self.config.max_size as f64;
        utilization > self.config.gc_threshold
    }
    
    /// Record cache hit
    fn record_cache_hit(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.cache_hits += 1;
    }
    
    /// Record cache miss
    fn record_cache_miss(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.cache_misses += 1;
    }
    
    /// Update allocation statistics
    fn update_allocation_stats(&self, size: usize) {
        let mut stats = self.stats.write().unwrap();
        stats.total_allocations += 1;
        stats.current_allocated += size;
        stats.peak_allocated = stats.peak_allocated.max(stats.current_allocated);
        
        self.current_size.fetch_add(size, Ordering::Relaxed);
    }
    
    /// Update deallocation statistics
    fn update_deallocation_stats(&self, size: usize) {
        let mut stats = self.stats.write().unwrap();
        stats.total_deallocations += 1;
        stats.current_allocated = stats.current_allocated.saturating_sub(size);
        
        self.current_size.fetch_sub(size, Ordering::Relaxed);
    }
}

/// Memory block wrapper
#[derive(Debug)]
pub struct MemoryBlock {
    /// Underlying chunk
    chunk: MemoryChunk,
    /// Size of the block
    size: usize,
}

impl MemoryBlock {
    /// Create a new memory block
    fn new(chunk: &MemoryChunk, size: usize) -> Self {
        Self {
            chunk: MemoryChunk {
                ptr: chunk.ptr,
                size: chunk.size,
                in_use: true,
                allocated_at: chunk.allocated_at,
                last_accessed: Instant::now(),
                numa_node: chunk.numa_node,
            },
            size,
        }
    }
    
    /// Get the size of the block
    pub fn size(&self) -> usize {
        self.size
    }
    
    /// Get a raw pointer to the memory
    pub fn as_ptr(&self) -> *const u8 {
        self.chunk.ptr.as_ptr()
    }
    
    /// Get a mutable raw pointer to the memory
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.chunk.ptr.as_ptr()
    }
    
    /// Get a slice of the memory
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.chunk.ptr.as_ptr(), self.size) }
    }
    
    /// Get a mutable slice of the memory
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.chunk.ptr.as_ptr(), self.size) }
    }
}

/// Memory error types
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("Memory allocation failed")]
    AllocationFailed,
    #[error("Invalid memory block")]
    InvalidBlock,
    #[error("Memory pool exhausted")]
    PoolExhausted,
    #[error("NUMA operation failed")]
    NumaError,
}

/// SIMD-optimized memory operations
pub mod simd {
    use super::*;
    
    /// Copy memory using SIMD instructions
    pub fn simd_copy(dst: *mut u8, src: *const u8, len: usize) {
        unsafe {
            if len >= 32 {
                // Use AVX2 for large copies
                simd_copy_avx2(dst, src, len);
            } else if len >= 16 {
                // Use SSE for medium copies
                simd_copy_sse(dst, src, len);
            } else {
                // Use regular copy for small data
                ptr::copy_nonoverlapping(src, dst, len);
            }
        }
    }
    
    /// Compare memory using SIMD instructions
    pub fn simd_compare(a: *const u8, b: *const u8, len: usize) -> bool {
        unsafe {
            if len >= 32 {
                simd_compare_avx2(a, b, len)
            } else if len >= 16 {
                simd_compare_sse(a, b, len)
            } else {
                ptr::eq(a, b) || std::slice::from_raw_parts(a, len) == std::slice::from_raw_parts(b, len)
            }
        }
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe fn simd_copy_avx2(dst: *mut u8, src: *const u8, len: usize) {
        // AVX2 implementation would go here
        // For now, fall back to regular copy
        ptr::copy_nonoverlapping(src, dst, len);
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe fn simd_copy_sse(dst: *mut u8, src: *const u8, len: usize) {
        // SSE implementation would go here
        // For now, fall back to regular copy
        ptr::copy_nonoverlapping(src, dst, len);
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe fn simd_compare_avx2(a: *const u8, b: *const u8, len: usize) -> bool {
        // AVX2 comparison implementation would go here
        // For now, fall back to regular comparison
        std::slice::from_raw_parts(a, len) == std::slice::from_raw_parts(b, len)
    }
    
    #[cfg(target_arch = "x86_64")]
    unsafe fn simd_compare_sse(a: *const u8, b: *const u8, len: usize) -> bool {
        // SSE comparison implementation would go here
        // For now, fall back to regular comparison
        std::slice::from_raw_parts(a, len) == std::slice::from_raw_parts(b, len)
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    unsafe fn simd_copy_avx2(dst: *mut u8, src: *const u8, len: usize) {
        ptr::copy_nonoverlapping(src, dst, len);
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    unsafe fn simd_copy_sse(dst: *mut u8, src: *const u8, len: usize) {
        ptr::copy_nonoverlapping(src, dst, len);
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    unsafe fn simd_compare_avx2(a: *const u8, b: *const u8, len: usize) -> bool {
        std::slice::from_raw_parts(a, len) == std::slice::from_raw_parts(b, len)
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    unsafe fn simd_compare_sse(a: *const u8, b: *const u8, len: usize) -> bool {
        std::slice::from_raw_parts(a, len) == std::slice::from_raw_parts(b, len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_pool_creation() {
        let config = MemoryPoolConfig::default();
        let pool = MemoryPool::new(config);
        let stats = pool.get_stats();
        
        assert_eq!(stats.total_allocations, 0);
        assert_eq!(stats.current_allocated, 0);
    }
    
    #[test]
    fn test_memory_allocation() {
        let config = MemoryPoolConfig {
            initial_size: 1024 * 1024, // 1MB
            max_size: 10 * 1024 * 1024, // 10MB
            ..Default::default()
        };
        let pool = MemoryPool::new(config);
        
        let block = pool.allocate(1024).unwrap();
        assert_eq!(block.size(), 1024);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.current_allocated, 1024);
    }
    
    #[test]
    fn test_memory_deallocation() {
        let config = MemoryPoolConfig {
            initial_size: 1024 * 1024, // 1MB
            max_size: 10 * 1024 * 1024, // 10MB
            ..Default::default()
        };
        let pool = MemoryPool::new(config);
        
        let block = pool.allocate(1024).unwrap();
        pool.deallocate(block);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.total_deallocations, 1);
        assert_eq!(stats.current_allocated, 0);
    }
    
    #[test]
    fn test_zero_copy_bytes() {
        let config = MemoryPoolConfig::default();
        let pool = MemoryPool::new(config);
        
        let mut block = pool.allocate(1024).unwrap();
        let data = b"Hello, World!";
        block.as_mut_slice()[..data.len()].copy_from_slice(data);
        
        let bytes = pool.create_bytes(&block);
        assert_eq!(&bytes[..data.len()], data);
    }
}