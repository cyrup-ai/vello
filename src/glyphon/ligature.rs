//! Simple ligature helper for glyphon text rendering
//!
//! This module provides utilities to configure ligature behavior in the existing
//! cosmic-text/glyphon rendering pipeline without reimplementing ligature logic.

use glyphon::{Attrs, Buffer, FontSystem, Metrics, Shaping};

use crate::glyphon::ligature_config::LigatureConfig;

/// Simple ligature helper that works with existing glyphon system
pub struct LigatureHelper {
    config: LigatureConfig,
}

impl LigatureHelper {
    /// Create a new ligature helper with the given configuration
    pub fn new(config: LigatureConfig) -> Self {
        Self { config }
    }

    /// Create a ligature helper with default programming font configuration
    pub fn with_programming_defaults() -> Self {
        Self::new(LigatureConfig::with_programming_defaults())
    }

    /// Create a ligature helper with ligatures enabled
    pub fn enabled() -> Self {
        Self::new(LigatureConfig::enabled())
    }

    /// Create a ligature helper with ligatures disabled
    pub fn disabled() -> Self {
        Self::new(LigatureConfig::disabled())
    }

    /// Get the current configuration
    pub fn config(&self) -> &LigatureConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: LigatureConfig) {
        self.config = config;
    }

    /// Determine the appropriate shaping mode for a font
    pub fn shaping_for_font(&self, font_name: &str) -> Shaping {
        if self.config.is_enabled_for_font(font_name) {
            // Use Advanced shaping which enables ligatures in cosmic-text
            Shaping::Advanced
        } else {
            // Use Basic shaping which disables ligatures
            Shaping::Basic
        }
    }

    /// Create a shaped buffer with appropriate ligature settings
    pub fn create_buffer(
        &self,
        text: &str,
        font_system: &mut FontSystem,
        metrics: Metrics,
        attrs: &Attrs,
        font_name: &str,
    ) -> Buffer {
        let shaping = self.shaping_for_font(font_name);

        let mut buffer = Buffer::new(font_system, metrics);
        buffer.set_text(font_system, text, attrs, shaping);
        buffer.shape_until_scroll(font_system, false);

        buffer
    }

    /// Check if ligatures should be enabled for a specific font
    pub fn should_enable_ligatures(&self, font_name: &str) -> bool {
        self.config.is_enabled_for_font(font_name)
    }

    /// Configure font-specific ligature override
    pub fn set_font_override(&mut self, font_name: String, enabled: bool) {
        self.config.set_font_override(font_name, enabled);
    }

    /// Enable or disable ligatures globally
    pub fn set_globally_enabled(&mut self, enabled: bool) {
        self.config.set_globally_enabled(enabled);
    }

    /// Check if ligatures are globally enabled
    pub fn is_globally_enabled(&self) -> bool {
        self.config.is_globally_enabled()
    }
}

impl Default for LigatureHelper {
    fn default() -> Self {
        Self::with_programming_defaults()
    }
}

/// Utility functions for ligature configuration
pub mod utils {
    use super::*;

    /// Check if a font name suggests good ligature support
    pub fn font_supports_ligatures(font_name: &str) -> bool {
        crate::glyphon::ligature_config::LigatureConfig::font_has_good_ligature_support(font_name)
    }

    /// Get recommended shaping mode for a font name
    pub fn recommended_shaping(font_name: &str) -> Shaping {
        if font_supports_ligatures(font_name) {
            Shaping::Advanced
        } else {
            Shaping::Basic
        }
    }

    /// Create font attributes optimized for code rendering
    pub fn code_font_attrs() -> Attrs<'static> {
        Attrs::new()
            .family(glyphon::Family::Monospace)
            .weight(glyphon::Weight::NORMAL)
            .style(glyphon::Style::Normal)
    }

    /// Create font attributes for a specific font family
    pub fn font_attrs_for_family(family: glyphon::Family<'static>) -> Attrs<'static> {
        Attrs::new()
            .family(family)
            .weight(glyphon::Weight::NORMAL)
            .style(glyphon::Style::Normal)
    }
}

// Re-export for convenience
pub use utils::*;
