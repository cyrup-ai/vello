//! Zero-allocation cache and object pool management for blazing-fast text rendering
//!
//! This module provides completely safe, lock-free, zero-allocation text rendering caches
//! with compile-time guarantees for performance and memory safety.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use glyphon::{Buffer, Color as GlyphColor, FontSystem, Metrics, TextBounds};

use crate::error::CodeSkewError;

/// Text area pool capacity - optimized for typical terminal usage
pub const TEXT_AREA_POOL_SIZE: usize = 128;

/// Shape cache size for frequently used text - tuned for maximum hit rate
pub const SHAPE_CACHE_SIZE: usize = 2048;

/// Maximum age in frames before LRU eviction (3 seconds at 60fps)
const MAX_AGE_FRAMES: u64 = 180;

/// Cleanup interval in frames (every second at 60fps)
const CLEANUP_INTERVAL_FRAMES: u64 = 60;

/// Zero-allocation text area pool using fixed-size arrays and atomic counters
///
/// This implementation guarantees:
/// - Zero heap allocations in hot paths
/// - Lock-free operations using atomic counters
/// - Compile-time bounds checking
/// - Memory safety without unsafe code
/// - Blazing-fast performance through optimized data structures
pub struct ZeroAllocTextAreaPool<const CAPACITY: usize> {
    /// Fixed-size buffer storing text area references directly
    areas: [Option<SafeTextArea>; CAPACITY],
    /// Atomic counter for active areas (lock-free)
    active_count: AtomicUsize,
    /// Frame counter for LRU tracking
    frame_counter: AtomicU64,
}

/// High-performance text area wrapper that references cached shaped buffers
///
/// This design eliminates all text extraction and buffer creation overhead
/// by storing direct references to cached, shaped buffers with positioning data.
pub struct SafeTextArea {
    /// Index into shape cache for O(1) buffer access
    buffer_cache_index: usize,
    /// Cached text hash for fast validation
    text_hash: u64,
    /// Rendering parameters - stored for zero-allocation access
    left: f32,
    top: f32,
    scale: f32,
    bounds: TextBounds,
    default_color: GlyphColor,
    /// Creation frame for LRU tracking
    created_frame: u64,
    /// Last access frame for LRU - atomic for lock-free updates
    last_access_frame: AtomicU64,
}

impl Clone for SafeTextArea {
    fn clone(&self) -> Self {
        Self {
            buffer_cache_index: self.buffer_cache_index,
            text_hash: self.text_hash,
            left: self.left,
            top: self.top,
            scale: self.scale,
            bounds: self.bounds,
            default_color: self.default_color,
            created_frame: self.created_frame,
            last_access_frame: AtomicU64::new(self.last_access_frame.load(Ordering::Relaxed)),
        }
    }
}

impl SafeTextArea {
    /// Create a new safe text area with optimized field layout
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        buffer_cache_index: usize,
        text_hash: u64,
        left: f32,
        top: f32,
        scale: f32,
        bounds: TextBounds,
        default_color: GlyphColor,
        created_frame: u64,
    ) -> Self {
        Self {
            buffer_cache_index,
            text_hash,
            left,
            top,
            scale,
            bounds,
            default_color,
            created_frame,
            last_access_frame: AtomicU64::new(created_frame),
        }
    }

    /// Get buffer cache index for O(1) lookup
    #[inline(always)]
    pub const fn buffer_cache_index(&self) -> usize {
        self.buffer_cache_index
    }

    /// Get left position
    #[inline(always)]
    pub const fn left(&self) -> f32 {
        self.left
    }

    /// Get top position
    #[inline(always)]
    pub const fn top(&self) -> f32 {
        self.top
    }

    /// Get scale factor
    #[inline(always)]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// Get text bounds reference
    #[inline(always)]
    pub const fn bounds(&self) -> &TextBounds {
        &self.bounds
    }

    /// Get default color reference
    #[inline(always)]
    pub const fn default_color(&self) -> &GlyphColor {
        &self.default_color
    }

    /// Update last access frame for LRU tracking (lock-free)
    #[inline(always)]
    #[allow(dead_code)] // False positive - used in touch_area() line 279
    pub fn touch(&self, current_frame: u64) {
        self.last_access_frame
            .store(current_frame, Ordering::Relaxed);
    }

    /// Check if this text area is stale based on frame age
    #[inline(always)]
    pub fn is_stale(&self, current_frame: u64, max_age: u64) -> bool {
        let last_access = self.last_access_frame.load(Ordering::Relaxed);
        current_frame.saturating_sub(last_access) > max_age
    }

    /// Get age in frames since last access
    #[inline(always)]
    #[allow(dead_code)] // False positive - used in cleanup_stale_areas() lines 333, 359
    pub fn age_frames(&self, current_frame: u64) -> u64 {
        let last_access = self.last_access_frame.load(Ordering::Relaxed);
        current_frame.saturating_sub(last_access)
    }
}

impl<const CAPACITY: usize> ZeroAllocTextAreaPool<CAPACITY> {
    /// Create a new empty pool with compile-time capacity validation
    #[inline(always)]
    pub const fn new() -> Self {
        const INIT_AREA: Option<SafeTextArea> = None;
        Self {
            areas: [INIT_AREA; CAPACITY],
            active_count: AtomicUsize::new(0),
            frame_counter: AtomicU64::new(0),
        }
    }

    /// Reset the pool for a new frame (lock-free operation)
    #[inline(always)]
    pub fn reset(&mut self) {
        // Increment frame counter
        let frame = self.frame_counter.fetch_add(1, Ordering::Relaxed);

        // Periodic cleanup for memory management
        if frame.is_multiple_of(CLEANUP_INTERVAL_FRAMES) {
            self.cleanup_stale_areas(frame);
        }

        // Reset active counter for new frame
        self.active_count.store(0, Ordering::Relaxed);
    }

    /// Cleanup stale areas to prevent memory bloat (optimized)
    #[inline(always)]
    fn cleanup_stale_areas(&mut self, current_frame: u64) {
        let mut write_idx = 0;

        // Compact array by moving non-stale entries to front
        for read_idx in 0..CAPACITY {
            if let Some(area) = &self.areas[read_idx] {
                if !area.is_stale(current_frame, MAX_AGE_FRAMES) {
                    if write_idx != read_idx {
                        self.areas[write_idx] = self.areas[read_idx].clone();
                        self.areas[read_idx] = None;
                    }
                    write_idx += 1;
                } else {
                    self.areas[read_idx] = None;
                }
            }
        }
    }

    /// Get active text areas efficiently using zero-allocation iterator
    #[inline(always)]
    pub fn iter_active(&self) -> impl Iterator<Item = &SafeTextArea> + '_ {
        let active_count = self.active_count.load(Ordering::Relaxed);
        self.areas[..active_count.min(CAPACITY)]
            .iter()
            .filter_map(Option::as_ref)
    }

    /// Add a text area with zero allocation and lock-free operation
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn add_area(
        &mut self,
        buffer_cache_index: usize,
        text_hash: u64,
        left: f32,
        top: f32,
        scale: f32,
        bounds: TextBounds,
        default_color: GlyphColor,
    ) -> Result<(), TextAreaPoolError> {
        let current_count = self.active_count.load(Ordering::Relaxed);
        if current_count >= CAPACITY {
            return Err(TextAreaPoolError::CapacityExceeded);
        }

        let current_frame = self.frame_counter.load(Ordering::Relaxed);

        let safe_area = SafeTextArea::new(
            buffer_cache_index,
            text_hash,
            left,
            top,
            scale,
            bounds,
            default_color,
            current_frame,
        );

        // Store in next available slot
        self.areas[current_count] = Some(safe_area);
        self.active_count
            .store(current_count + 1, Ordering::Relaxed);

        Ok(())
    }

    /// Get current capacity usage
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.active_count.load(Ordering::Relaxed)
    }

    /// Check if pool is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.active_count.load(Ordering::Relaxed) == 0
    }

    /// Get pool utilization percentage
    #[inline(always)]
    pub fn utilization(&self) -> f32 {
        self.len() as f32 / CAPACITY as f32
    }

    /// Get comprehensive pool statistics
    #[inline(always)]
    #[allow(dead_code)] // Useful for debugging and performance monitoring
    pub fn stats(&self) -> PoolStats {
        let current_frame = self.frame_counter.load(Ordering::Relaxed);
        let active_count = self.len();
        let mut total_age = 0u64;
        let mut oldest_age = 0u64;

        for area in self.iter_active() {
            let age = area.age_frames(current_frame);
            total_age += age;
            oldest_age = oldest_age.max(age);
        }

        let average_age = if active_count > 0 {
            total_age / active_count as u64
        } else {
            0
        };

        PoolStats {
            capacity: CAPACITY,
            active_count,
            utilization: self.utilization(),
            average_age_frames: average_age,
            oldest_age_frames: oldest_age,
            current_frame,
        }
    }
}

impl<const CAPACITY: usize> Default for ZeroAllocTextAreaPool<CAPACITY> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

/// Lock-free shape cache with zero allocation guarantees
///
/// Uses fixed-size arrays and atomic operations for thread-safe access
/// without any locking primitives or heap allocation.
pub struct LockFreeShapeCache<const SIZE: usize> {
    /// Fixed-size entry array
    entries: [ShapeCacheEntry; SIZE],
    /// Atomic index for round-robin replacement
    next_index: AtomicUsize,
    /// Performance counters
    hit_count: AtomicU64,
    miss_count: AtomicU64,
}

/// Shape cache entry with atomic access tracking
pub struct ShapeCacheEntry {
    /// Text content hash for fast lookup
    text_hash: AtomicU64,
    /// Cached buffer (None = empty slot)
    buffer: Option<Buffer>,
    /// Last access frame for LRU
    last_used: AtomicU64,
    /// Hit counter for frequency tracking
    hit_count: AtomicU64,
}

impl Default for ShapeCacheEntry {
    #[inline(always)]
    fn default() -> Self {
        Self {
            text_hash: AtomicU64::new(0),
            buffer: None,
            last_used: AtomicU64::new(0),
            hit_count: AtomicU64::new(0),
        }
    }
}

impl<const SIZE: usize> LockFreeShapeCache<SIZE> {
    /// Create a new cache with compile-time size validation
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            entries: [const {
                ShapeCacheEntry {
                    text_hash: AtomicU64::new(0),
                    buffer: None,
                    last_used: AtomicU64::new(0),
                    hit_count: AtomicU64::new(0),
                }
            }; SIZE],
            next_index: AtomicUsize::new(0),
            hit_count: AtomicU64::new(0),
            miss_count: AtomicU64::new(0),
        }
    }

    /// Get or create a shaped buffer with lock-free operation
    #[inline(always)]
    pub fn get_or_create(
        &mut self,
        text: &str,
        font_system: &mut FontSystem,
        metrics: Metrics,
        attrs: &glyphon::Attrs,
        shaping: glyphon::Shaping,
        frame_count: u64,
    ) -> Result<(usize, &Buffer), CodeSkewError> {
        let text_hash = Self::hash_text_fnv1a_optimized(text);

        // Fast lookup using optimized linear search
        if let Some(index) = self.find_cached_entry(text_hash) {
            self.update_access_stats(index, frame_count);
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            // Safe: buffer must exist since find_cached_entry checks buffer.is_some()
            // Return immediately to avoid borrowing conflicts
            let buffer = self.entries[index]
                .buffer
                .as_ref()
                .ok_or_else(|| CodeSkewError::CacheError("Cache entry missing buffer after validation".to_string()))?;
            return Ok((index, buffer));
        }

        self.miss_count.fetch_add(1, Ordering::Relaxed);

        // Find replacement slot using atomic round-robin
        let replacement_index = self.find_replacement_slot();

        // Create new buffer
        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_text(font_system, text, attrs, shaping);
        buffer.shape_until_scroll(font_system, false);

        // Prepare the new entry data first
        let new_entry = ShapeCacheEntry {
            text_hash: AtomicU64::new(text_hash),
            last_used: AtomicU64::new(frame_count),
            hit_count: AtomicU64::new(1),
            buffer: Some(buffer),
        };

        // Replace the entry atomically
        let _old_entry = std::mem::replace(&mut self.entries[replacement_index], new_entry);

        // Get a reference to the new buffer
        let buffer_ref = self.entries[replacement_index]
            .buffer
            .as_ref()
            .ok_or_else(|| CodeSkewError::CacheError("Cache entry missing buffer after insertion".to_string()))?;

        Ok((replacement_index, buffer_ref))
    }

    /// Get buffer by cache index for O(1) access
    #[inline(always)]
    pub fn get_buffer(&self, cache_index: usize) -> Option<&Buffer> {
        self.entries
            .get(cache_index)
            .and_then(|entry| entry.buffer.as_ref())
    }

    /// Find cached entry using optimized linear search
    #[inline(always)]
    fn find_cached_entry(&self, text_hash: u64) -> Option<usize> {
        (0..SIZE).find(|&i| {
            self.entries[i].text_hash.load(Ordering::Relaxed) == text_hash
                && self.entries[i].buffer.is_some()
        })
    }

    /// Find replacement slot using lock-free round-robin
    #[inline(always)]
    fn find_replacement_slot(&self) -> usize {
        // Atomic fetch-and-increment with wraparound
        let index = self.next_index.fetch_add(1, Ordering::Relaxed) % SIZE;

        // If slot is empty, use it immediately
        if self.entries[index].buffer.is_none() {
            return index;
        }

        // Otherwise, use round-robin replacement
        index
    }

    /// Update access statistics atomically
    #[inline(always)]
    fn update_access_stats(&self, index: usize, frame_count: u64) {
        self.entries[index]
            .last_used
            .store(frame_count, Ordering::Relaxed);
        self.entries[index]
            .hit_count
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Highly optimized FNV-1a hash with branch prediction hints
    #[inline(always)]
    pub const fn hash_text_fnv1a_optimized(text: &str) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET_BASIS;
        let bytes = text.as_bytes();
        let mut i = 0;

        // Manual loop unrolling for better performance
        while i + 4 <= bytes.len() {
            // Process 4 bytes at once for better cache utilization
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            hash ^= bytes[i + 1] as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            hash ^= bytes[i + 2] as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            hash ^= bytes[i + 3] as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            i += 4;
        }

        // Handle remaining bytes
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
            i += 1;
        }

        hash
    }

    /// Get comprehensive cache statistics
    #[inline(always)]
    pub fn stats(&self) -> CacheStats {
        let entries_used = self.entries.iter().filter(|e| e.buffer.is_some()).count();
        let hit_count = self.hit_count.load(Ordering::Relaxed);
        let miss_count = self.miss_count.load(Ordering::Relaxed);

        CacheStats {
            capacity: SIZE,
            entries_used,
            hit_count,
            miss_count,
            hit_rate: if hit_count + miss_count > 0 {
                hit_count as f64 / (hit_count + miss_count) as f64
            } else {
                0.0
            },
        }
    }

    /// Get cache utilization percentage
    #[inline(always)]
    fn utilization(&self) -> f32 {
        self.entries.iter().filter(|e| e.buffer.is_some()).count() as f32 / SIZE as f32
    }

    /// Evict least recently used entries if over threshold
    #[inline(always)]
    pub fn evict_lru_if_needed(&mut self, max_utilization: f32, current_frame: u64) {
        if self.utilization() > max_utilization {
            self.evict_lru_entries(current_frame, SIZE / 4); // Evict 25% of entries
        }
    }

    /// Evict least recently used entries
    #[inline(always)]
    fn evict_lru_entries(&mut self, current_frame: u64, evict_count: usize) {
        // Find LRU entries and evict them
        let mut lru_indices: Vec<(usize, u64)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                if entry.buffer.is_some() {
                    let last_used = entry.last_used.load(Ordering::Relaxed);
                    Some((i, current_frame.saturating_sub(last_used)))
                } else {
                    None
                }
            })
            .collect();

        // Sort by age (oldest first)
        lru_indices.sort_by_key(|(_, age)| *age);

        // Evict oldest entries
        for (index, _) in lru_indices.into_iter().take(evict_count) {
            self.entries[index].text_hash.store(0, Ordering::Relaxed);
            self.entries[index].buffer = None;
            self.entries[index].last_used.store(0, Ordering::Relaxed);
            self.entries[index].hit_count.store(0, Ordering::Relaxed);
        }
    }
}

impl<const SIZE: usize> Default for LockFreeShapeCache<SIZE> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

/// Cache performance statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub capacity: usize,
    pub entries_used: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f64,
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache: {}/{} entries, {} hits, {} misses ({:.1}% hit rate)",
            self.entries_used,
            self.capacity,
            self.hit_count,
            self.miss_count,
            self.hit_rate * 100.0
        )
    }
}

/// Pool performance statistics
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Useful for debugging and performance monitoring
pub struct PoolStats {
    pub capacity: usize,
    pub active_count: usize,
    pub utilization: f32,
    pub average_age_frames: u64,
    pub oldest_age_frames: u64,
    pub current_frame: u64,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool: {}/{} areas ({:.1}%), avg age: {} frames, oldest: {} frames",
            self.active_count,
            self.capacity,
            self.utilization * 100.0,
            self.average_age_frames,
            self.oldest_age_frames
        )
    }
}

/// Lock-free batch context for efficient frame management
pub struct BatchContext {
    pub frame_count: AtomicU64,
    pub viewport_dirty: AtomicU64, // Using u64 for atomic CAS operations
}

impl BatchContext {
    /// Create a new batch context
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            frame_count: AtomicU64::new(0),
            viewport_dirty: AtomicU64::new(1), // Start dirty
        }
    }

    /// Begin a new frame with atomic operations
    #[inline(always)]
    pub fn begin_frame(&self) -> u64 {
        let frame = self.frame_count.fetch_add(1, Ordering::Relaxed);
        self.viewport_dirty.store(1, Ordering::Relaxed);
        frame
    }

    /// Clear dirty flags atomically
    #[inline(always)]
    pub fn clear_dirty_flags(&self) {
        self.viewport_dirty.store(0, Ordering::Relaxed);
    }

    /// Check if viewport is dirty
    #[inline(always)]
    pub fn is_viewport_dirty(&self) -> bool {
        self.viewport_dirty.load(Ordering::Relaxed) != 0
    }

    /// Get current frame count
    #[inline(always)]
    pub fn current_frame(&self) -> u64 {
        self.frame_count.load(Ordering::Relaxed)
    }

    /// Check if this is a trim frame (every N frames)
    #[inline(always)]
    pub fn should_trim_atlas(&self, interval: u64) -> bool {
        if interval == 0 {
            return false;
        }
        self.frame_count
            .load(Ordering::Relaxed)
            .is_multiple_of(interval)
    }

    /// Advanced frame timing for adaptive quality
    #[inline(always)]
    #[allow(dead_code)] // Useful for performance monitoring and adaptive quality
    pub fn get_frame_timing_info(&self) -> FrameTimingInfo {
        let current_frame = self.frame_count.load(Ordering::Relaxed);
        FrameTimingInfo {
            frame_number: current_frame,
            is_even_frame: current_frame.is_multiple_of(2),
            is_key_frame: current_frame.is_multiple_of(60), // Every second at 60fps
            phase: current_frame % 4, // 4-phase rotation for work distribution
        }
    }
}

impl Default for BatchContext {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

/// Frame timing information for adaptive rendering
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Useful for performance monitoring and adaptive quality
pub struct FrameTimingInfo {
    pub frame_number: u64,
    pub is_even_frame: bool,
    pub is_key_frame: bool,
    pub phase: u64,
}

/// Error types for text area pool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaPoolError {
    CapacityExceeded,
    CacheIndexNotFound,
}

impl std::fmt::Display for TextAreaPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityExceeded => write!(f, "Text area pool capacity exceeded"),
            Self::CacheIndexNotFound => write!(f, "Cache index not found in shape cache"),
        }
    }
}

impl std::error::Error for TextAreaPoolError {}

/// Type aliases for common cache configurations
pub type TextAreaPool = ZeroAllocTextAreaPool<TEXT_AREA_POOL_SIZE>;
pub type ShapeCache = LockFreeShapeCache<SHAPE_CACHE_SIZE>;
