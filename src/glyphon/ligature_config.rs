//! Simple ligature configuration for code rendering
//!
//! This module provides basic control over ligature behavior in the existing
//! glyphon/cosmic-text rendering pipeline without reimplementing ligature logic.

use serde::{Deserialize, Serialize};

/// Simple ligature configuration for code rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LigatureConfig {
    /// Enable/disable ligatures globally
    pub enabled: bool,
    /// Font-specific overrides (font name -> enabled)
    pub font_overrides: std::collections::HashMap<String, bool>,
}

impl Default for LigatureConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Enable by default for programming fonts
            font_overrides: std::collections::HashMap::new(),
        }
    }
}

impl LigatureConfig {
    /// Create a new ligature configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration with ligatures enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            font_overrides: std::collections::HashMap::new(),
        }
    }

    /// Create configuration with ligatures disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            font_overrides: std::collections::HashMap::new(),
        }
    }

    /// Check if ligatures are enabled for a specific font
    pub fn is_enabled_for_font(&self, font_name: &str) -> bool {
        // Check font-specific override first
        if let Some(&override_enabled) = self.font_overrides.get(font_name) {
            return override_enabled;
        }

        // Fall back to global setting
        self.enabled
    }

    /// Add a font-specific override
    pub fn set_font_override(&mut self, font_name: String, enabled: bool) {
        self.font_overrides.insert(font_name, enabled);
    }

    /// Remove a font-specific override
    pub fn remove_font_override(&mut self, font_name: &str) {
        self.font_overrides.remove(font_name);
    }

    /// Get the global ligature setting
    pub fn is_globally_enabled(&self) -> bool {
        self.enabled
    }

    /// Set the global ligature setting
    pub fn set_globally_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Load configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("Failed to parse YAML: {}", e))
    }

    /// Save configuration to YAML string
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|e| format!("Failed to serialize YAML: {}", e))
    }

    /// Create default configuration for popular programming fonts
    pub fn with_programming_defaults() -> Self {
        let mut config = Self::enabled();

        // These fonts have excellent ligature support
        let excellent_fonts = [
            "FiraCode",
            "Fira Code",
            "FiraCode Nerd Font",
            "FiraCode Nerd Font Mono",
            "JetBrains Mono",
            "JetBrainsMono",
            "JetBrains Mono Nerd Font",
            "Cascadia Code",
            "CascadiaCode",
            "Cascadia Code Nerd Font",
            "Iosevka",
            "Iosevka Term",
            "Iosevka Nerd Font",
            "Victor Mono",
            "VictorMono",
        ];

        for font in excellent_fonts {
            config.set_font_override(font.to_string(), true);
        }

        // These fonts have limited or no ligature support
        let limited_fonts = [
            "Monaco",
            "Consolas",
            "Menlo",
            "Courier",
            "Courier New",
            "Ubuntu Mono",
            "Liberation Mono",
        ];

        for font in limited_fonts {
            config.set_font_override(font.to_string(), false);
        }

        config
    }

    /// Check if a font name suggests good ligature support
    pub fn font_has_good_ligature_support(font_name: &str) -> bool {
        let good_ligature_indicators = [
            "firacode",
            "fira code",
            "jetbrains",
            "jetbrainsmono",
            "cascadia",
            "cascadiacode",
            "iosevka",
            "victor",
            "victormono",
            "hack",
            "source code pro",
            "sourcecodepro",
        ];

        let font_lower = font_name.to_lowercase();
        good_ligature_indicators
            .iter()
            .any(|&indicator| font_lower.contains(indicator))
    }
}

/// Builder for ligature configuration
#[derive(Debug)]
pub struct LigatureConfigBuilder {
    config: LigatureConfig,
}

impl LigatureConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: LigatureConfig::default(),
        }
    }

    /// Enable or disable ligatures globally
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Add a font-specific override
    pub fn font_override(mut self, font_name: String, enabled: bool) -> Self {
        self.config.font_overrides.insert(font_name, enabled);
        self
    }

    /// Build the configuration
    pub fn build(self) -> LigatureConfig {
        self.config
    }
}

impl Default for LigatureConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
