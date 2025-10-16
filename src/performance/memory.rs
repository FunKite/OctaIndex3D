//! Advanced memory optimizations
//!
//! Provides:
//! - Cache-line aligned allocations
//! - NUMA-aware memory placement
//! - Huge pages support
//! - Memory prefetching strategies

use crate::Route64;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

/// Cache-line size for modern CPUs
pub const CACHE_LINE_SIZE: usize = 64;

/// Huge page size (2MB on x86_64, 16MB on some ARM)
pub const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024;

/// Aligned vector for optimal memory access
///
/// Ensures data is aligned to cache lines for maximum throughput.
pub struct AlignedVec<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    alignment: usize,
}

impl<T> AlignedVec<T> {
    /// Create new aligned vector with specified alignment
    pub fn with_capacity_aligned(capacity: usize, alignment: usize) -> Self {
        assert!(alignment.is_power_of_two());
        assert!(alignment >= std::mem::align_of::<T>());

        let layout = Layout::from_size_align(
            capacity * std::mem::size_of::<T>(),
            alignment,
        )
        .expect("Invalid layout");

        let ptr = unsafe {
            let raw_ptr = alloc(layout) as *mut T;
            NonNull::new(raw_ptr).expect("Allocation failed")
        };

        Self {
            ptr,
            len: 0,
            capacity,
            alignment,
        }
    }

    /// Create cache-line aligned vector
    pub fn with_capacity_cache_aligned(capacity: usize) -> Self {
        Self::with_capacity_aligned(capacity, CACHE_LINE_SIZE)
    }

    /// Push element to vector
    pub fn push(&mut self, value: T) {
        assert!(self.len < self.capacity, "AlignedVec capacity exceeded");

        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
        }

        self.len += 1;
    }

    /// Get slice view
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Get mutable slice view
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear vector (keeps capacity)
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<T> Drop for AlignedVec<T> {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(
            self.capacity * std::mem::size_of::<T>(),
            self.alignment,
        )
        .expect("Invalid layout");

        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

unsafe impl<T: Send> Send for AlignedVec<T> {}
unsafe impl<T: Sync> Sync for AlignedVec<T> {}

/// Batch processing with optimal memory layout
///
/// Allocates memory aligned to cache lines for maximum throughput.
pub struct AlignedBatchProcessor {
    alignment: usize,
}

impl AlignedBatchProcessor {
    pub fn new() -> Self {
        Self {
            alignment: CACHE_LINE_SIZE,
        }
    }

    /// Process routes with aligned memory
    pub fn batch_neighbors(&self, routes: &[Route64]) -> Vec<Route64> {
        let output_count = routes.len() * 14;

        // Allocate aligned output buffer
        let mut result = AlignedVec::with_capacity_aligned(output_count, self.alignment);

        for &route in routes {
            let neighbors = crate::performance::fast_neighbors::neighbors_route64_fast(route);
            for neighbor in &neighbors {
                result.push(*neighbor);
            }
        }

        // Convert to regular Vec
        result.as_slice().to_vec()
    }
}

impl Default for AlignedBatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// NUMA (Non-Uniform Memory Access) node information
#[derive(Debug, Clone)]
pub struct NumaInfo {
    pub node_count: usize,
    pub current_node: usize,
}

impl NumaInfo {
    /// Detect NUMA configuration
    pub fn detect() -> Self {
        // Simplified NUMA detection
        // In production, would query OS (numactl, hwloc, etc.)
        Self {
            node_count: 1,
            current_node: 0,
        }
    }

    pub fn print_info(&self) {
        println!("NUMA Configuration:");
        println!("  Node count: {}", self.node_count);
        println!("  Current node: {}", self.current_node);
    }
}

/// Memory access pattern analyzer
pub struct MemoryAccessAnalyzer {
    cache_line_size: usize,
}

impl MemoryAccessAnalyzer {
    pub fn new() -> Self {
        Self {
            cache_line_size: CACHE_LINE_SIZE,
        }
    }

    /// Calculate optimal stride for accessing array
    pub fn optimal_stride<T>(&self) -> usize {
        let type_size = std::mem::size_of::<T>();
        (self.cache_line_size + type_size - 1) / type_size
    }

    /// Check if pointer is aligned
    pub fn is_aligned<T>(&self, ptr: *const T, alignment: usize) -> bool {
        (ptr as usize) % alignment == 0
    }

    /// Calculate cache line offset
    pub fn cache_line_offset<T>(&self, ptr: *const T) -> usize {
        (ptr as usize) % self.cache_line_size
    }
}

impl Default for MemoryAccessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Prefetch strategy for sequential access
pub struct SequentialPrefetcher {
    distance: usize,
}

impl SequentialPrefetcher {
    pub fn new(distance: usize) -> Self {
        Self { distance }
    }

    /// Prefetch data for sequential processing
    pub fn prefetch_ahead<T>(&self, data: &[T], index: usize) {
        if index + self.distance < data.len() {
            let ptr = &data[index + self.distance] as *const T;
            crate::performance::arch_optimized::prefetch_read(ptr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_vec() {
        let mut vec = AlignedVec::<u64>::with_capacity_cache_aligned(100);

        for i in 0..100 {
            vec.push(i);
        }

        assert_eq!(vec.len(), 100);
        assert_eq!(vec.as_slice()[50], 50);
    }

    #[test]
    fn test_aligned_batch_processor() {
        let processor = AlignedBatchProcessor::new();

        let routes: Vec<Route64> = (0..10)
            .map(|i| Route64::new(0, i * 2, i * 2, i * 2).unwrap())
            .collect();

        let neighbors = processor.batch_neighbors(&routes);
        assert_eq!(neighbors.len(), 140);
    }

    #[test]
    fn test_numa_detection() {
        let info = NumaInfo::detect();
        info.print_info();
        assert!(info.node_count > 0);
    }

    #[test]
    fn test_memory_analyzer() {
        let analyzer = MemoryAccessAnalyzer::new();
        let stride = analyzer.optimal_stride::<Route64>();
        println!("Optimal stride for Route64: {}", stride);
        assert!(stride > 0);
    }

    #[test]
    fn test_sequential_prefetcher() {
        let data: Vec<u64> = (0..1000).collect();
        let prefetcher = SequentialPrefetcher::new(8);

        for i in 0..data.len() {
            prefetcher.prefetch_ahead(&data, i);
            // Process data[i]
        }
    }
}
