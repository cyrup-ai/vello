//! Font system management and metrics for the renderer

use std::path::Path;

use anyhow::{Context, Result};
use glyphon::{FontSystem, Metrics};

// Use super to navigate up from font_system -> glyphon -> crate root
use super::super::nerdfont::quic;

/// Default font size constraints
pub const MIN_FONT_SIZE: f32 = 8.0;
pub const MAX_FONT_SIZE: f32 = 48.0;
pub const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Monospace font character width ratio
pub const MONOSPACE_RATIO: f32 = 0.6;

/// Font metrics for terminal rendering
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    /// Font size in pixels
    pub font_size: f32,
    /// Character width in pixels
    pub char_width: f32,
    /// Line height in pixels
    pub line_height: f32,
}

impl FontMetrics {
    /// Create new font metrics from font size
    pub fn new(font_size: f32) -> Self {
        let metrics = Metrics::relative(font_size, 1.0);
        Self {
            font_size,
            char_width: font_size * MONOSPACE_RATIO,
            line_height: metrics.line_height,
        }
    }

    /// Update metrics with new font size
    pub fn update(&mut self, font_size: f32) {
        let metrics = Metrics::relative(font_size, 1.0);
        self.font_size = font_size;
        self.char_width = font_size * MONOSPACE_RATIO;
        self.line_height = metrics.line_height;
    }

    /// Check if font size change is significant enough to warrant update
    pub fn needs_update(&self, new_font_size: f32) -> bool {
        (new_font_size - self.font_size).abs() > 1.0
    }

    /// Calculate terminal dimensions from window size
    pub fn terminal_dimensions(&self, window_width: u32, window_height: u32) -> (usize, usize) {
        let cols = (window_width as f32 / self.char_width) as usize;
        let rows = (window_height as f32 / self.line_height) as usize;
        (cols, rows)
    }
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self::new(DEFAULT_FONT_SIZE)
    }
}

/// Create an optimized font system with fallback chain
///
/// Uses the QUIC font loader for intelligent font loading.
pub async fn create_optimized_font_system() -> Result<FontSystem> {
    let mut font_system = FontSystem::new();

    // Use the QUIC font loader asynchronously
    quic::load_into(&mut font_system)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load font with QUIC loader: {}", e))?;

    Ok(font_system)
}

/// Create font system with specific Nerd font
///
/// Downloads and loads a specific Nerd font from GitHub
pub async fn create_font_system_with_nerd_font(font_name: &str) -> Result<FontSystem> {
    println!("[GLYPHON DEBUG] create_font_system_with_nerd_font: Starting font creation for '{}'", font_name);
    let mut font_system = FontSystem::new();
    println!("[GLYPHON DEBUG] create_font_system_with_nerd_font: FontSystem created");

    // Use the QUIC font loader with specific font family asynchronously
    println!("[GLYPHON DEBUG] create_font_system_with_nerd_font: About to call QUIC loader (THIS WILL AWAIT)");
    quic::load_with_family(&mut font_system, Some(font_name))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load {} Nerd font: {}", font_name, e))?;
    println!("[GLYPHON DEBUG] create_font_system_with_nerd_font: QUIC loader completed successfully");

    println!("[GLYPHON DEBUG] create_font_system_with_nerd_font: Returning font system");
    Ok(font_system)
}

/// Create font system with basic system fonts (synchronous fallback)
///
/// This is a synchronous fallback that doesn't download fonts to avoid blocking
pub fn create_basic_font_system() -> Result<FontSystem> {
    println!("[GLYPHON DEBUG] create_basic_font_system: Creating basic font system (sync)");
    let font_system = FontSystem::new();
    println!("[GLYPHON DEBUG] create_basic_font_system: Basic font system created successfully");
    Ok(font_system)
}

/// Load a specific font file into the font system
pub fn load_font_file(font_system: &mut FontSystem, path: &Path) -> Result<()> {
    let font_data = std::fs::read(path)
        .with_context(|| format!("Failed to read font file: {}", path.display()))?;

    let source = glyphon::fontdb::Source::Binary(std::sync::Arc::new(font_data));
    font_system.db_mut().load_font_source(source);

    Ok(())
}

/// Calculate optimal font size based on window height
///
/// The font size is calculated to provide comfortable reading with
/// approximately 45 lines of text in the window.
#[inline]
pub const fn calculate_optimal_font_size(window_height: u32) -> f32 {
    let calculated = window_height as f32 / 45.0;

    // Clamp to reasonable bounds
    if calculated < MIN_FONT_SIZE {
        MIN_FONT_SIZE
    } else if calculated > MAX_FONT_SIZE {
        MAX_FONT_SIZE
    } else {
        calculated
    }
}

/// Calculate optimal font size for a target row count
#[inline]
pub fn calculate_font_size_for_rows(window_height: u32, target_rows: usize) -> f32 {
    let calculated = window_height as f32 / target_rows as f32;
    calculated.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
}

/// Font configuration for the renderer
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Custom font paths to try loading
    pub custom_fonts: Vec<String>,
    /// Whether to use system font fallbacks
    pub use_system_fallback: bool,
    /// Preferred font family
    pub family: glyphon::Family<'static>,
    /// Font weight
    pub weight: glyphon::Weight,
    /// Font style
    pub style: glyphon::Style,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            custom_fonts: vec!["assets/IosevkaTerm.ttf".to_string()],
            use_system_fallback: true,
            family: glyphon::Family::Monospace,
            weight: glyphon::Weight::NORMAL,
            style: glyphon::Style::Normal,
        }
    }
}
