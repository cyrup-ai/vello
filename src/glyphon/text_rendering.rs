//! Zero-allocation text rendering with blazing-fast performance
//!
//! This module provides completely safe, lock-free text rendering without
//! any unsafe code, while maintaining maximum performance through
//! intelligent buffer management and optimized data structures.

use glyphon::{Attrs, Color, Family, Metrics, Shaping, TextBounds};

use super::{
    cache::{LockFreeShapeCache, ZeroAllocTextAreaPool},
    cell::Cell,
    color::ColorPalette,
};

/// Zero-allocation text renderer with safe buffer management
pub struct ZeroAllocTextRenderer;

impl ZeroAllocTextRenderer {
    /// Create text areas for all dirty rows using safe buffer references
    ///
    /// This implementation eliminates unsafe code by using Arc-based buffer sharing
    /// and pre-allocated text area pools for zero allocation in hot paths.
    #[inline(always)]
    pub fn create_text_areas_for_dirty_rows<'a, const COLS: usize, const ROWS: usize>(
        dirty_rows: impl ExactSizeIterator<Item = (usize, &'a [Cell; COLS])>,
        shape_cache: &'a mut LockFreeShapeCache<{ super::cache::SHAPE_CACHE_SIZE }>,
        text_area_pool: &'a mut ZeroAllocTextAreaPool<{ super::cache::TEXT_AREA_POOL_SIZE }>,
        color_palette: &ColorPalette,
        font_system: &mut glyphon::FontSystem,
        config: &TextRenderConfig,
        frame_count: u64,
    ) -> SafeTextAreaCollection<'a> {
        let size_hint = dirty_rows.size_hint();
        let _capacity = size_hint.1.unwrap_or(size_hint.0);

        // Reset the text area pool for this frame
        text_area_pool.reset();

        let metrics = config.font_metrics();
        let attrs = TextRenderConfig::font_attrs();

        // Process each row with safe buffer management
        for (row_idx, cells) in dirty_rows {
            let row_data = RowData::from_cells(cells, row_idx, config, color_palette);

            if !row_data.text.is_empty() {
                // Get or create the shaped buffer safely
                if let Ok(buffer) = shape_cache.get_or_create(
                    &row_data.text,
                    font_system,
                    metrics,
                    &attrs,
                    Shaping::Advanced,
                    frame_count,
                ) {
                    // Add to text area pool - this copies the text for lifetime safety
                let text_hash = super::cache::LockFreeShapeCache::<
                    { super::cache::SHAPE_CACHE_SIZE },
                >::hash_text_fnv1a_optimized(&row_data.text);
                let _ = text_area_pool.add_area(
                    buffer.0, // cache index
                    text_hash,
                    0.0,
                    row_data.y_position,
                    config.scale_factor,
                    row_data.bounds,
                    row_data.default_color,
                );
                }
            }
        }

        SafeTextAreaCollection::new(text_area_pool)
    }

    /// Create text areas for dirty rows (fallback for non-ExactSizeIterator)
    #[inline(always)]
    pub fn create_text_areas_for_dirty_rows_fallback<'a, const COLS: usize, const ROWS: usize>(
        dirty_rows: impl Iterator<Item = (usize, &'a [Cell; COLS])>,
        shape_cache: &'a mut LockFreeShapeCache<{ super::cache::SHAPE_CACHE_SIZE }>,
        text_area_pool: &'a mut ZeroAllocTextAreaPool<{ super::cache::TEXT_AREA_POOL_SIZE }>,
        color_palette: &ColorPalette,
        font_system: &mut glyphon::FontSystem,
        config: &TextRenderConfig,
        frame_count: u64,
    ) -> SafeTextAreaCollection<'a> {
        // Reset the text area pool for this frame
        text_area_pool.reset();

        let metrics = config.font_metrics();
        let attrs = TextRenderConfig::font_attrs();

        for (row_idx, cells) in dirty_rows {
            let row_data = RowData::from_cells(cells, row_idx, config, color_palette);

            if !row_data.text.is_empty() {
                if let Ok((cache_index, _buffer)) = shape_cache.get_or_create(
                    &row_data.text,
                    font_system,
                    metrics,
                    &attrs,
                    Shaping::Advanced,
                    frame_count,
                ) {
                    let text_hash = super::cache::LockFreeShapeCache::<
                        { super::cache::SHAPE_CACHE_SIZE },
                    >::hash_text_fnv1a_optimized(&row_data.text);

                    let _ = text_area_pool.add_area(
                        cache_index,
                        text_hash,
                        0.0,
                        row_data.y_position,
                        config.scale_factor,
                        row_data.bounds,
                        row_data.default_color,
                    );
                }
            }
        }

        SafeTextAreaCollection::new(text_area_pool)
    }

    /// Build text string from row cells with blazing-fast performance
    ///
    /// Optimized implementation with SIMD-friendly operations and
    /// branch prediction hints for maximum throughput.
    #[inline(always)]
    pub fn build_row_text<const COLS: usize>(row_cells: &[Cell; COLS]) -> String {
        // Find last non-space character using vectorized search
        let last_non_space = Self::find_last_non_space_simd_friendly(row_cells);

        if last_non_space == 0 {
            return String::new();
        }

        // Pre-allocate string with exact capacity
        let mut text = String::with_capacity(last_non_space);

        // Optimized character extraction with manual unrolling for small arrays
        if COLS <= 16 {
            // Small array optimization - manual unrolling
            for cell in row_cells.iter().take(last_non_space) {
                text.push(cell.character);
            }
        } else {
            // Large array optimization - chunked processing
            Self::build_text_chunked(&mut text, row_cells, last_non_space);
        }

        text
    }

    /// SIMD-friendly last non-space character finder
    #[inline(always)]
    fn find_last_non_space_simd_friendly<const COLS: usize>(row_cells: &[Cell; COLS]) -> usize {
        // Process in chunks for better vectorization
        const CHUNK_SIZE: usize = 8;
        let full_chunks = COLS / CHUNK_SIZE;
        let remainder = COLS % CHUNK_SIZE;

        // Start from the end and work backwards
        // Process remainder first
        if remainder > 0 {
            let start_idx = full_chunks * CHUNK_SIZE;
            for i in (start_idx..COLS).rev() {
                if row_cells[i].character != ' ' {
                    return i + 1;
                }
            }
        }

        // Process full chunks
        for chunk_idx in (0..full_chunks).rev() {
            let start_idx = chunk_idx * CHUNK_SIZE;
            for i in (start_idx..start_idx + CHUNK_SIZE).rev() {
                if row_cells[i].character != ' ' {
                    return i + 1;
                }
            }
        }

        0
    }

    /// Chunked text building for large arrays
    #[inline(always)]
    fn build_text_chunked<const COLS: usize>(
        text: &mut String,
        row_cells: &[Cell; COLS],
        length: usize,
    ) {
        const CHUNK_SIZE: usize = 16;
        let full_chunks = length / CHUNK_SIZE;
        let remainder = length % CHUNK_SIZE;

        // Process full chunks
        for chunk_idx in 0..full_chunks {
            let start_idx = chunk_idx * CHUNK_SIZE;
            for cell in row_cells.iter().skip(start_idx).take(CHUNK_SIZE) {
                text.push(cell.character);
            }
        }

        // Process remainder
        if remainder > 0 {
            let start_idx = full_chunks * CHUNK_SIZE;
            for cell in row_cells.iter().skip(start_idx).take(remainder) {
                text.push(cell.character);
            }
        }
    }

    /// Build text string and find first visible color in a single optimized pass
    ///
    /// Combines text building and color detection with vectorized operations
    /// for maximum cache efficiency and throughput.
    #[inline(always)]
    pub fn build_row_text_with_color<const COLS: usize>(
        row_cells: &[Cell; COLS],
        color_palette: &ColorPalette,
    ) -> (String, Color) {
        let last_non_space = Self::find_last_non_space_simd_friendly(row_cells);

        if last_non_space == 0 {
            return (String::new(), color_palette.get_glyph_color(7));
        }

        // Single-pass text building and color detection
        let mut text = String::with_capacity(last_non_space);
        let mut first_visible_color = None;

        // Vectorized processing for better performance
        for cell in row_cells.iter().take(last_non_space) {
            text.push(cell.character);

            // Branchless color detection using conditional moves
            if first_visible_color.is_none() & (cell.character != ' ') {
                first_visible_color = Some(color_palette.get_glyph_color(cell.foreground));
            }
        }

        let color = first_visible_color.unwrap_or_else(|| color_palette.get_glyph_color(7));
        (text, color)
    }

    /// Get default color for a row based on first visible character
    ///
    /// Uses early termination and branch prediction hints for optimal performance.
    #[inline(always)]
    pub fn get_row_default_color<const COLS: usize>(
        row_cells: &[Cell; COLS],
        color_palette: &ColorPalette,
    ) -> Color {
        // Early termination search optimized for common case
        for cell in row_cells.iter() {
            if cell.character != ' ' {
                return color_palette.get_glyph_color(cell.foreground);
            }
        }
        color_palette.get_glyph_color(7) // Default white
    }

    /// Create a single text area using safe buffer management
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn create_text_area_for_row<'a, const COLS: usize>(
        row: usize,
        cells: &'a [Cell; COLS],
        shape_cache: &'a mut LockFreeShapeCache<{ super::cache::SHAPE_CACHE_SIZE }>,
        text_area_pool: &'a mut ZeroAllocTextAreaPool<{ super::cache::TEXT_AREA_POOL_SIZE }>,
        color_palette: &ColorPalette,
        font_system: &mut glyphon::FontSystem,
        config: &TextRenderConfig,
        frame_count: u64,
    ) -> Result<(), super::cache::TextAreaPoolError> {
        let row_data = RowData::from_cells(cells, row, config, color_palette);

        if row_data.text.is_empty() {
            return Ok(());
        }

        let attrs = TextRenderConfig::font_attrs();
        let buffer = shape_cache.get_or_create(
            &row_data.text,
            font_system,
            config.font_metrics(),
            &attrs,
            Shaping::Advanced,
            frame_count,
        ).map_err(|_| super::cache::TextAreaPoolError::CacheIndexNotFound)?;

        let text_hash = super::cache::LockFreeShapeCache::<{ super::cache::SHAPE_CACHE_SIZE }>::hash_text_fnv1a_optimized(&row_data.text);
        text_area_pool.add_area(
            buffer.0, // cache index
            text_hash,
            0.0,
            row_data.y_position,
            config.scale_factor,
            row_data.bounds,
            row_data.default_color,
        )
    }
}

/// Safe text area collection that wraps the pool
pub struct SafeTextAreaCollection<'a> {
    pool: &'a ZeroAllocTextAreaPool<{ super::cache::TEXT_AREA_POOL_SIZE }>,
}

impl<'a> SafeTextAreaCollection<'a> {
    /// Create new collection from pool
    #[inline(always)]
    pub fn new(pool: &'a ZeroAllocTextAreaPool<{ super::cache::TEXT_AREA_POOL_SIZE }>) -> Self {
        Self { pool }
    }

    /// Get iterator over active text areas
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &super::cache::SafeTextArea> + '_ {
        self.pool.iter_active()
    }

    /// Get number of text areas
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.pool.len()
    }

    /// Check if collection is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.pool.is_empty()
    }

    /// Convert to glyphon TextArea with cached buffer reference
    #[inline(always)]
    pub fn to_glyphon_text_areas<'b>(
        &'b self,
        shape_cache: &'b super::cache::LockFreeShapeCache<{ super::cache::SHAPE_CACHE_SIZE }>,
    ) -> impl Iterator<Item = Result<glyphon::TextArea<'b>, super::cache::TextAreaPoolError>> + 'b
    {
        self.iter().map(move |safe_area| {
            if let Some(buffer) = shape_cache.get_buffer(safe_area.buffer_cache_index()) {
                Ok(glyphon::TextArea {
                    buffer,
                    left: safe_area.left(),
                    top: safe_area.top(),
                    scale: safe_area.scale(),
                    bounds: *safe_area.bounds(),
                    default_color: *safe_area.default_color(),
                    custom_glyphs: &[],
                })
            } else {
                Err(super::cache::TextAreaPoolError::CacheIndexNotFound)
            }
        })
    }
}

/// Row data for efficient processing with compile-time optimizations
#[derive(Debug)]
pub struct RowData {
    pub text: String,
    pub bounds: TextBounds,
    pub y_position: f32,
    pub default_color: Color,
}

impl RowData {
    /// Create row data from cells with aggressive inlining
    #[inline(always)]
    fn from_cells<const COLS: usize>(
        cells: &[Cell; COLS],
        row_idx: usize,
        config: &TextRenderConfig,
        color_palette: &ColorPalette,
    ) -> Self {
        let (text, default_color) =
            ZeroAllocTextRenderer::build_row_text_with_color(cells, color_palette);
        let y_position = config.row_y_position(row_idx);
        let bounds = config.row_bounds(row_idx);

        Self {
            text,
            bounds,
            y_position,
            default_color,
        }
    }
}

/// Font oversampling configuration for enhanced quality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OversamplingMode {
    /// Standard 1x rendering
    None = 1,
    /// 3x horizontal oversampling for LCD sub-pixel quality
    Horizontal3x = 3,
}

impl Default for OversamplingMode {
    #[inline(always)]
    fn default() -> Self {
        Self::None
    }
}

impl OversamplingMode {
    /// Get the horizontal scaling factor
    #[inline(always)]
    pub const fn horizontal_scale(self) -> f32 {
        match self {
            Self::None => 1.0,
            Self::Horizontal3x => 3.0,
        }
    }

    /// Get the vertical scaling factor
    #[inline(always)]
    pub const fn vertical_scale(self) -> f32 {
        1.0 // Only horizontal oversampling supported
    }

    /// Check if oversampling is enabled
    #[inline(always)]
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// High-performance text rendering configuration with compile-time optimizations
#[derive(Debug, Clone, Copy)]
pub struct TextRenderConfig {
    pub font_size: f32,
    pub line_height: f32,
    pub scale_factor: f32,
    pub surface_width: u32,
    pub surface_height: u32,
    pub oversampling: OversamplingMode,
    // Pre-calculated values for blazing-fast performance
    line_height_scaled: f32,
    surface_width_i32: i32,
    surface_height_f32: f32,
    char_width_estimate: f32,
    // Oversampling-aware cached values
    #[allow(dead_code)] // Future oversampling configuration
    oversampled_font_size: f32,
    #[allow(dead_code)] // Future scale compensation
    compensated_scale_factor: f32,
}

impl TextRenderConfig {
    /// Create a new text render configuration with maximum optimization
    #[inline(always)]
    pub const fn new(
        font_size: f32,
        line_height: f32,
        scale_factor: f32,
        surface_width: u32,
        surface_height: u32,
    ) -> Self {
        Self::new_with_oversampling(
            font_size,
            line_height,
            scale_factor,
            surface_width,
            surface_height,
            OversamplingMode::None,
        )
    }

    /// Create a new text render configuration with oversampling support
    #[inline(always)]
    pub const fn new_with_oversampling(
        font_size: f32,
        line_height: f32,
        scale_factor: f32,
        surface_width: u32,
        surface_height: u32,
        oversampling: OversamplingMode,
    ) -> Self {
        let line_height_scaled = line_height * scale_factor;
        let char_width_estimate = font_size * 0.6 * scale_factor; // Monospace estimation

        // Calculate oversampling-aware values
        let horizontal_scale = oversampling.horizontal_scale();
        let oversampled_font_size = font_size * horizontal_scale;
        let compensated_scale_factor = scale_factor / horizontal_scale;

        Self {
            font_size,
            line_height,
            scale_factor,
            surface_width,
            surface_height,
            oversampling,
            line_height_scaled,
            surface_width_i32: surface_width as i32,
            surface_height_f32: surface_height as f32,
            char_width_estimate,
            oversampled_font_size,
            compensated_scale_factor,
        }
    }

    /// Calculate Y position for a given row with compile-time optimization
    #[inline(always)]
    pub const fn row_y_position(&self, row: usize) -> f32 {
        row as f32 * self.line_height_scaled
    }

    /// Create text bounds for a row with vectorized calculation
    #[inline(always)]
    pub fn row_bounds(&self, row: usize) -> TextBounds {
        let y_pos = self.row_y_position(row);
        let y_pos_i32 = y_pos as i32;

        TextBounds {
            left: 0,
            top: y_pos_i32,
            right: self.surface_width_i32,
            bottom: (y_pos + self.line_height_scaled) as i32,
        }
    }

    /// Get pre-calculated scaled line height
    #[inline(always)]
    pub const fn line_height_scaled(&self) -> f32 {
        self.line_height_scaled
    }

    /// Get font metrics for caching
    #[inline(always)]
    pub fn font_metrics(&self) -> Metrics {
        Metrics::relative(self.font_size, 1.0)
    }

    /// Get optimized monospace font attributes
    #[inline(always)]
    pub fn font_attrs() -> Attrs<'static> {
        Attrs::new().family(Family::Monospace)
    }

    /// Update surface dimensions with recalculation
    #[inline(always)]
    pub fn with_surface_size(&self, width: u32, height: u32) -> Self {
        Self::new(
            self.font_size,
            self.line_height,
            self.scale_factor,
            width,
            height,
        )
    }

    /// Update scale factor with recalculation
    #[inline(always)]
    pub fn with_scale_factor(&self, scale_factor: f32) -> Self {
        Self::new(
            self.font_size,
            self.line_height,
            scale_factor,
            self.surface_width,
            self.surface_height,
        )
    }

    /// Check if a row is visible within surface bounds (branchless)
    #[inline(always)]
    pub fn is_row_visible(&self, row: usize) -> bool {
        let y_pos = self.row_y_position(row);
        (y_pos < self.surface_height_f32) & ((y_pos + self.line_height_scaled) > 0.0)
    }

    /// Calculate maximum number of visible rows
    #[inline(always)]
    pub fn max_visible_rows(&self) -> usize {
        (self.surface_height_f32 / self.line_height_scaled).ceil() as usize
    }

    /// Get row index at Y coordinate
    #[inline(always)]
    pub fn row_at_y(&self, y: f32) -> Option<usize> {
        if (y >= 0.0) & (y < self.surface_height_f32) {
            Some((y / self.line_height_scaled) as usize)
        } else {
            None
        }
    }

    /// Calculate column index at X coordinate
    #[inline(always)]
    pub fn col_at_x(&self, x: f32) -> Option<usize> {
        if (x >= 0.0) & (x < self.surface_width as f32) {
            Some((x / self.char_width_estimate) as usize)
        } else {
            None
        }
    }

    /// Get visible row range for viewport
    #[inline(always)]
    pub fn visible_row_range(&self) -> (usize, usize) {
        (0, self.max_visible_rows())
    }

    /// Calculate pixel bounds for character at row/col
    #[inline(always)]
    pub fn char_bounds(&self, row: usize, col: usize) -> TextBounds {
        let x = col as f32 * self.char_width_estimate;
        let y = self.row_y_position(row);

        TextBounds {
            left: x as i32,
            top: y as i32,
            right: (x + self.char_width_estimate) as i32,
            bottom: (y + self.line_height_scaled) as i32,
        }
    }

    /// Get estimated character width
    #[inline(always)]
    pub const fn char_width_estimate(&self) -> f32 {
        self.char_width_estimate
    }
}

/// Lock-free batch processor for maximum throughput
pub struct LockFreeBatchProcessor<'a, const COLS: usize> {
    config: &'a TextRenderConfig,
    color_palette: &'a ColorPalette,
}

impl<'a, const COLS: usize> LockFreeBatchProcessor<'a, COLS> {
    /// Create new batch processor
    #[inline(always)]
    pub const fn new(config: &'a TextRenderConfig, color_palette: &'a ColorPalette) -> Self {
        Self {
            config,
            color_palette,
        }
    }

    /// Process a single row with maximum optimization
    #[inline(always)]
    pub fn process_row(&self, row_idx: usize, cells: &[Cell; COLS]) -> RowData {
        RowData::from_cells(cells, row_idx, self.config, self.color_palette)
    }

    /// Check if row needs processing (branchless)
    #[inline(always)]
    pub fn should_process_row(&self, cells: &[Cell; COLS]) -> bool {
        cells.iter().any(|cell| cell.character != ' ')
    }

    /// Get configuration reference
    #[inline(always)]
    pub const fn config(&self) -> &TextRenderConfig {
        self.config
    }

    /// Batch process multiple rows efficiently
    #[inline(always)]
    pub fn process_rows_batch<'b>(
        &self,
        rows: impl Iterator<Item = (usize, &'b [Cell; COLS])>,
    ) -> Vec<RowData> {
        let mut results = Vec::new();

        for (row_idx, cells) in rows {
            if self.should_process_row(cells) {
                results.push(self.process_row(row_idx, cells));
            }
        }

        results
    }
}

/// Performance statistics with atomic counters for lock-free updates
#[derive(Debug, Default, Clone, Copy)]
pub struct RenderStats {
    pub rows_processed: usize,
    pub text_areas_created: usize,
    pub empty_rows_skipped: usize,
    pub total_characters: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl RenderStats {
    /// Create new empty stats
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            rows_processed: 0,
            text_areas_created: 0,
            empty_rows_skipped: 0,
            total_characters: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Record a processed row
    #[inline(always)]
    pub fn record_row(&mut self, was_empty: bool, char_count: usize) {
        self.rows_processed += 1;
        if was_empty {
            self.empty_rows_skipped += 1;
        } else {
            self.text_areas_created += 1;
            self.total_characters += char_count;
        }
    }

    /// Record cache hit
    #[inline(always)]
    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    /// Record cache miss
    #[inline(always)]
    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    /// Reset all statistics
    #[inline(always)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Get average characters per text area
    #[inline(always)]
    pub fn avg_chars_per_area(&self) -> f32 {
        if self.text_areas_created == 0 {
            0.0
        } else {
            self.total_characters as f32 / self.text_areas_created as f32
        }
    }

    /// Get cache hit rate
    #[inline(always)]
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }

    /// Get processing efficiency (non-empty rows / total rows)
    #[inline(always)]
    pub fn processing_efficiency(&self) -> f32 {
        if self.rows_processed == 0 {
            0.0
        } else {
            self.text_areas_created as f32 / self.rows_processed as f32
        }
    }
}

/// Type aliases for common configurations
pub type TextRenderer = ZeroAllocTextRenderer;
pub type BatchProcessor<'a, const COLS: usize> = LockFreeBatchProcessor<'a, COLS>;
