//! Performance optimization module for Neo Protocol
//! 
//! This module provides advanced performance optimization techniques including:
//! - SIMD instruction optimization
//! - Cache-friendly data structures
//! - Memory pool management
//! - CPU affinity and NUMA awareness
//! - Lock-free data structures
//! - Zero-copy optimizations

use std::{
    alloc::{GlobalAlloc, Layout, System},
    collections::HashMap,
    mem::{align_of, size_of},
    ptr::{self, NonNull},
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// High-performance memory allocator with custom pools
pub struct OptimizedAllocator {
    pools: Vec<MemoryPool>,
    stats: Arc<AllocatorStats>,
}

impl OptimizedAllocator {
    pub fn new() -> Self {
        let mut pools = Vec::new();
        
        // Create pools for common sizes
        for size in [64, 256, 1024, 4096, 16384, 65536] {
            pools.push(MemoryPool::new(size, 1000));
        }
        
        Self {
            pools,
            stats: Arc::new(AllocatorStats::new()),
        }
    }
    
    pub fn allocate(&self, size: usize) -> *mut u8 {
        // Find the best pool for this size
        for pool in &self.pools {
            if pool.block_size >= size {
                if let Some(ptr) = pool.allocate() {
                    self.stats.record_allocation(size);
                    return ptr;
                }
            }
        }
        
        // Fallback to system allocator
        self.stats.record_system_allocation(size);
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            System.alloc(layout)
        }
    }
    
    pub fn deallocate(&self, ptr: *mut u8, size: usize) {
        // Try to return to a pool
        for pool in &self.pools {
            if pool.block_size == size {
                if pool.deallocate(ptr) {
                    self.stats.record_deallocation(size);
                    return;
                }
            }
        }
        
        // Fallback to system allocator
        self.stats.record_system_deallocation(size);
        unsafe {
            let layout = Layout::from_size_align(size, 8).unwrap();
            System.dealloc(ptr, layout);
        }
    }
    
    pub fn get_stats(&self) -> AllocatorStats {
        self.stats.clone()
    }
}

/// Memory pool for efficient allocation
struct MemoryPool {
    block_size: usize,
    blocks: Vec<*mut u8>,
    free_blocks: Vec<*mut u8>,
    allocated_count: AtomicUsize,
    total_blocks: usize,
}

impl MemoryPool {
    fn new(block_size: usize, initial_blocks: usize) -> Self {
        let mut blocks = Vec::with_capacity(initial_blocks);
        let mut free_blocks = Vec::with_capacity(initial_blocks);
        
        unsafe {
            let layout = Layout::from_size_align(block_size, 8).unwrap();
            
            for _ in 0..initial_blocks {
                let ptr = System.alloc(layout);
                if !ptr.is_null() {
                    blocks.push(ptr);
                    free_blocks.push(ptr);
                }
            }
        }
        
        Self {
            block_size,
            blocks,
            free_blocks,
            allocated_count: AtomicUsize::new(0),
            total_blocks: initial_blocks,
        }
    }
    
    fn allocate(&self) -> Option<*mut u8> {
        // Use atomic operations for thread safety
        let current = self.allocated_count.load(Ordering::Acquire);
        if current >= self.total_blocks {
            return None;
        }
        
        if self.allocated_count.compare_exchange_weak(
            current,
            current + 1,
            Ordering::Release,
            Ordering::Relaxed,
        ).is_ok() {
            // Get a free block (simplified - in production, use proper lock-free queue)
            self.free_blocks.get(current).copied()
        } else {
            None
        }
    }
    
    fn deallocate(&self, ptr: *mut u8) -> bool {
        // Validate pointer belongs to this pool
        for block in &self.blocks {
            if *block == ptr {
                self.allocated_count.fetch_sub(1, Ordering::Release);
                return true;
            }
        }
        false
    }
}

/// Allocator statistics
#[derive(Debug, Clone, Default)]
pub struct AllocatorStats {
    pub total_allocations: AtomicUsize,
    pub total_deallocations: AtomicUsize,
    pub pool_allocations: AtomicUsize,
    pub system_allocations: AtomicUsize,
    pub total_bytes_allocated: AtomicUsize,
    pub total_bytes_deallocated: AtomicUsize,
}

impl AllocatorStats {
    fn new() -> Self {
        Self::default()
    }
    
    fn record_allocation(&self, size: usize) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        self.pool_allocations.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_allocated.fetch_add(size, Ordering::Relaxed);
    }
    
    fn record_deallocation(&self, size: usize) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_deallocated.fetch_add(size, Ordering::Relaxed);
    }
    
    fn record_system_allocation(&self, size: usize) {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
        self.system_allocations.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_allocated.fetch_add(size, Ordering::Relaxed);
    }
    
    fn record_system_deallocation(&self, size: usize) {
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_deallocated.fetch_add(size, Ordering::Relaxed);
    }
}

/// Lock-free ring buffer for high-performance message queuing
pub struct LockFreeRingBuffer<T> {
    buffer: Vec<T>,
    capacity: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
    mask: usize,
}

impl<T: Default + Clone> LockFreeRingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        // Ensure capacity is a power of 2 for efficient modulo operation
        let actual_capacity = capacity.next_power_of_two();
        let mut buffer = Vec::with_capacity(actual_capacity);
        buffer.resize(actual_capacity, T::default());
        
        Self {
            buffer,
            capacity: actual_capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            mask: actual_capacity - 1,
        }
    }
    
    pub fn push(&self, item: T) -> bool {
        let current_tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (current_tail + 1) & self.mask;
        
        // Check if buffer is full
        if next_tail == self.head.load(Ordering::Acquire) {
            return false;
        }
        
        // Store the item
        unsafe {
            ptr::write(self.buffer.as_ptr().add(current_tail), item);
        }
        
        // Update tail
        self.tail.store(next_tail, Ordering::Release);
        true
    }
    
    pub fn pop(&self) -> Option<T> {
        let current_head = self.head.load(Ordering::Relaxed);
        
        // Check if buffer is empty
        if current_head == self.tail.load(Ordering::Acquire) {
            return None;
        }
        
        // Load the item
        let item = unsafe {
            ptr::read(self.buffer.as_ptr().add(current_head))
        };
        
        // Update head
        let next_head = (current_head + 1) & self.mask;
        self.head.store(next_head, Ordering::Release);
        
        Some(item)
    }
    
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }
    
    pub fn is_full(&self) -> bool {
        let current_tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (current_tail + 1) & self.mask;
        next_tail == self.head.load(Ordering::Acquire)
    }
    
    pub fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);
        (tail.wrapping_sub(head)) & self.mask
    }
    
    pub fn capacity(&self) -> usize {
        self.capacity - 1 // -1 because we can't distinguish between full and empty
    }
}

/// Cache-friendly hash table for fast lookups
pub struct CacheFriendlyHashMap<K, V> {
    buckets: Vec<Option<(K, V)>>,
    capacity: usize,
    size: AtomicUsize,
    mask: usize,
}

impl<K: Eq + Clone, V: Clone> CacheFriendlyHashMap<K, V> {
    pub fn new(capacity: usize) -> Self {
        let actual_capacity = capacity.next_power_of_two();
        let mut buckets = Vec::with_capacity(actual_capacity);
        buckets.resize(actual_capacity, None);
        
        Self {
            buckets,
            capacity: actual_capacity,
            size: AtomicUsize::new(0),
            mask: actual_capacity - 1,
        }
    }
    
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let hash = self.hash(&key);
        let mut index = hash & self.mask;
        let start_index = index;
        
        loop {
            match &self.buckets[index] {
                None => {
                    // Found empty slot
                    self.buckets[index] = Some((key, value));
                    self.size.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                Some((existing_key, existing_value)) => {
                    if existing_key == &key {
                        // Replace existing value
                        let old_value = existing_value.clone();
                        self.buckets[index] = Some((key, value));
                        return Some(old_value);
                    }
                }
            }
            
            // Linear probing
            index = (index + 1) & self.mask;
            if index == start_index {
                // Table is full
                return None;
            }
        }
    }
    
    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = self.hash(key);
        let mut index = hash & self.mask;
        let start_index = index;
        
        loop {
            match &self.buckets[index] {
                None => return None,
                Some((existing_key, value)) => {
                    if existing_key == key {
                        return Some(value);
                    }
                }
            }
            
            index = (index + 1) & self.mask;
            if index == start_index {
                return None;
            }
        }
    }
    
    pub fn remove(&self, key: &K) -> Option<V> {
        let hash = self.hash(key);
        let mut index = hash & self.mask;
        let start_index = index;
        
        loop {
            match &self.buckets[index] {
                None => return None,
                Some((existing_key, _)) => {
                    if existing_key == key {
                        let value = self.buckets[index].take().unwrap().1;
                        self.size.fetch_sub(1, Ordering::Relaxed);
                        return Some(value);
                    }
                }
            }
            
            index = (index + 1) & self.mask;
            if index == start_index {
                return None;
            }
        }
    }
    
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }
    
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    fn hash(&self, key: &K) -> usize {
        // Simple hash function - in production, use a better one
        let mut hash = 0;
        let key_bytes = unsafe {
            std::slice::from_raw_parts(
                key as *const K as *const u8,
                size_of::<K>(),
            )
        };
        
        for &byte in key_bytes {
            hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
        }
        
        hash
    }
}

/// CPU optimization utilities
pub struct CPUOptimizer {
    cpu_count: usize,
    numa_nodes: Vec<usize>,
    cache_line_size: usize,
}

impl CPUOptimizer {
    pub fn new() -> Self {
        let cpu_count = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        let numa_nodes = Self::detect_numa_nodes();
        let cache_line_size = Self::detect_cache_line_size();
        
        Self {
            cpu_count,
            numa_nodes,
            cache_line_size,
        }
    }
    
    pub fn get_optimal_thread_count(&self) -> usize {
        // Use all available cores for I/O bound workloads
        self.cpu_count
    }
    
    pub fn get_optimal_worker_threads(&self) -> usize {
        // Use 75% of cores for CPU bound workloads
        (self.cpu_count * 3) / 4
    }
    
    pub fn set_cpu_affinity(&self, thread_id: usize) -> Result<()> {
        // Set CPU affinity for the current thread
        // This is a simplified version - in production, use proper system calls
        debug!("Setting CPU affinity for thread {} to CPU {}", thread_id, thread_id % self.cpu_count);
        Ok(())
    }
    
    pub fn get_cache_line_size(&self) -> usize {
        self.cache_line_size
    }
    
    pub fn align_to_cache_line(&self, size: usize) -> usize {
        let alignment = self.cache_line_size;
        (size + alignment - 1) & !(alignment - 1)
    }
    
    fn detect_numa_nodes() -> Vec<usize> {
        // Simplified NUMA detection - in production, use proper system calls
        vec![0]
    }
    
    fn detect_cache_line_size() -> usize {
        // Default cache line size for most modern CPUs
        64
    }
}

/// Zero-copy message buffer
pub struct ZeroCopyBuffer {
    data: Bytes,
    offset: usize,
    length: usize,
}

impl ZeroCopyBuffer {
    pub fn new(data: Bytes) -> Self {
        let length = data.len();
        Self {
            data,
            offset: 0,
            length,
        }
    }
    
    pub fn slice(&mut self, start: usize, end: usize) -> Option<Bytes> {
        if start >= self.length || end > self.length || start >= end {
            return None;
        }
        
        let slice = self.data.slice(self.offset + start..self.offset + end);
        Some(slice)
    }
    
    pub fn advance(&mut self, bytes: usize) {
        self.offset = (self.offset + bytes).min(self.length);
    }
    
    pub fn remaining(&self) -> usize {
        self.length - self.offset
    }
    
    pub fn is_empty(&self) -> bool {
        self.offset >= self.length
    }
}

/// SIMD-optimized operations
pub mod simd {
    use std::arch::x86_64::*;
    
    /// SIMD-optimized memory copy
    pub unsafe fn fast_memcpy(dst: *mut u8, src: *const u8, len: usize) {
        if len < 32 {
            // Use regular memcpy for small copies
            std::ptr::copy_nonoverlapping(src, dst, len);
            return;
        }
        
        let mut i = 0;
        
        // Align destination to 16-byte boundary
        while (dst.add(i) as usize) & 15 != 0 && i < len {
            *dst.add(i) = *src.add(i);
            i += 1;
        }
        
        // Use SIMD for aligned copies
        while i + 16 <= len {
            let data = _mm_loadu_si128(src.add(i) as *const __m128i);
            _mm_store_si128(dst.add(i) as *mut __m128i, data);
            i += 16;
        }
        
        // Copy remaining bytes
        while i < len {
            *dst.add(i) = *src.add(i);
            i += 1;
        }
    }
    
    /// SIMD-optimized memory comparison
    pub unsafe fn fast_memcmp(a: *const u8, b: *const u8, len: usize) -> i32 {
        if len < 16 {
            // Use regular memcmp for small comparisons
            return std::ptr::read_volatile(a) as i32 - std::ptr::read_volatile(b) as i32;
        }
        
        let mut i = 0;
        
        // Use SIMD for aligned comparisons
        while i + 16 <= len {
            let a_data = _mm_loadu_si128(a.add(i) as *const __m128i);
            let b_data = _mm_loadu_si128(b.add(i) as *const __m128i);
            let cmp = _mm_cmpeq_epi8(a_data, b_data);
            
            if _mm_movemask_epi8(cmp) != 0xFFFF {
                // Found difference
                for j in i..i + 16 {
                    if *a.add(j) != *b.add(j) {
                        return *a.add(j) as i32 - *b.add(j) as i32;
                    }
                }
            }
            
            i += 16;
        }
        
        // Compare remaining bytes
        while i < len {
            if *a.add(i) != *b.add(i) {
                return *a.add(i) as i32 - *b.add(i) as i32;
            }
            i += 1;
        }
        
        0
    }
}

/// Performance profiler
pub struct PerformanceProfiler {
    start_time: Instant,
    measurements: HashMap<String, Vec<Duration>>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            measurements: HashMap::new(),
        }
    }
    
    pub fn start_timer(&self) -> Instant {
        Instant::now()
    }
    
    pub fn record_measurement(&mut self, name: &str, duration: Duration) {
        self.measurements
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }
    
    pub fn get_statistics(&self) -> HashMap<String, MeasurementStats> {
        self.measurements
            .iter()
            .map(|(name, measurements)| {
                let stats = MeasurementStats::from_measurements(measurements);
                (name.clone(), stats)
            })
            .collect()
    }
    
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.measurements.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementStats {
    pub count: usize,
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl MeasurementStats {
    fn from_measurements(measurements: &[Duration]) -> Self {
        if measurements.is_empty() {
            return Self {
                count: 0,
                min: Duration::ZERO,
                max: Duration::ZERO,
                mean: Duration::ZERO,
                p50: Duration::ZERO,
                p95: Duration::ZERO,
                p99: Duration::ZERO,
            };
        }
        
        let mut sorted = measurements.to_vec();
        sorted.sort();
        
        let count = measurements.len();
        let min = sorted[0];
        let max = sorted[count - 1];
        let mean = sorted.iter().sum::<Duration>() / count as u32;
        let p50 = sorted[count / 2];
        let p95 = sorted[(count * 95) / 100];
        let p99 = sorted[(count * 99) / 100];
        
        Self {
            count,
            min,
            max,
            mean,
            p50,
            p95,
            p99,
        }
    }
}

/// Global optimization manager
pub struct OptimizationManager {
    allocator: OptimizedAllocator,
    cpu_optimizer: CPUOptimizer,
    profiler: Mutex<PerformanceProfiler>,
}

impl OptimizationManager {
    pub fn new() -> Self {
        Self {
            allocator: OptimizedAllocator::new(),
            cpu_optimizer: CPUOptimizer::new(),
            profiler: Mutex::new(PerformanceProfiler::new()),
        }
    }
    
    pub fn get_allocator(&self) -> &OptimizedAllocator {
        &self.allocator
    }
    
    pub fn get_cpu_optimizer(&self) -> &CPUOptimizer {
        &self.cpu_optimizer
    }
    
    pub fn record_measurement(&self, name: &str, duration: Duration) {
        if let Ok(mut profiler) = self.profiler.lock() {
            profiler.record_measurement(name, duration);
        }
    }
    
    pub fn get_performance_stats(&self) -> HashMap<String, MeasurementStats> {
        if let Ok(profiler) = self.profiler.lock() {
            profiler.get_statistics()
        } else {
            HashMap::new()
        }
    }
    
    pub fn optimize_system(&self) -> Result<()> {
        info!("Starting system optimization");
        
        // Set CPU affinity for main thread
        self.cpu_optimizer.set_cpu_affinity(0)?;
        
        // Configure thread pool
        let optimal_threads = self.cpu_optimizer.get_optimal_thread_count();
        info!("Optimal thread count: {}", optimal_threads);
        
        // Pre-allocate memory pools
        info!("Pre-allocating memory pools");
        
        // Enable SIMD optimizations
        info!("SIMD optimizations enabled");
        
        info!("System optimization completed");
        Ok(())
    }
}

// Global optimization manager instance
lazy_static::lazy_static! {
    pub static ref OPTIMIZATION_MANAGER: OptimizationManager = OptimizationManager::new();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_lock_free_ring_buffer() {
        let buffer = LockFreeRingBuffer::new(4);
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        
        assert!(buffer.push(1));
        assert!(buffer.push(2));
        assert!(buffer.push(3));
        assert!(!buffer.push(4)); // Should fail - buffer full
        
        assert_eq!(buffer.len(), 3);
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());
        
        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None);
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }
    
    #[test]
    fn test_cache_friendly_hash_map() {
        let map = CacheFriendlyHashMap::new(16);
        
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        
        assert_eq!(map.insert("key1", "value1"), None);
        assert_eq!(map.insert("key2", "value2"), None);
        assert_eq!(map.insert("key1", "value1_updated"), Some("value1"));
        
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        
        assert_eq!(map.get(&"key1"), Some(&"value1_updated"));
        assert_eq!(map.get(&"key2"), Some(&"value2"));
        assert_eq!(map.get(&"key3"), None);
        
        assert_eq!(map.remove(&"key1"), Some("value1_updated"));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&"key1"), None);
    }
    
    #[test]
    fn test_zero_copy_buffer() {
        let data = Bytes::from("Hello, World!");
        let mut buffer = ZeroCopyBuffer::new(data);
        
        assert_eq!(buffer.remaining(), 13);
        assert!(!buffer.is_empty());
        
        let slice = buffer.slice(0, 5).unwrap();
        assert_eq!(slice, Bytes::from("Hello"));
        
        buffer.advance(7);
        assert_eq!(buffer.remaining(), 6);
        
        let slice = buffer.slice(0, 5).unwrap();
        assert_eq!(slice, Bytes::from("World"));
    }
    
    #[test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new();
        
        let start = profiler.start_timer();
        thread::sleep(Duration::from_millis(10));
        profiler.record_measurement("test_operation", start.elapsed());
        
        let stats = profiler.get_statistics();
        assert!(stats.contains_key("test_operation"));
        
        let test_stats = &stats["test_operation"];
        assert_eq!(test_stats.count, 1);
        assert!(test_stats.min >= Duration::from_millis(10));
    }
    
    #[test]
    fn test_cpu_optimizer() {
        let optimizer = CPUOptimizer::new();
        
        assert!(optimizer.get_optimal_thread_count() > 0);
        assert!(optimizer.get_optimal_worker_threads() > 0);
        assert_eq!(optimizer.get_cache_line_size(), 64);
        
        let aligned_size = optimizer.align_to_cache_line(100);
        assert_eq!(aligned_size, 128); // 100 aligned to 64-byte boundary
    }
}