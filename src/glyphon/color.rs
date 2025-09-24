//! Color palette management for zero-allocation terminal rendering

use glyphon::Color as GlyphColor;

/// Color palette size (supports full 8-bit color space)
pub const COLOR_PALETTE_SIZE: usize = 256;

/// SIMD-optimized color palette with pre-computed conversions
#[repr(align(64))]
pub struct ColorPalette {
    /// RGBA colors in linear space for accurate blending
    colors_linear: [[f32; 4]; COLOR_PALETTE_SIZE],
    /// RGBA colors in sRGB space for display
    colors_srgb: [[u8; 4]; COLOR_PALETTE_SIZE],
    /// Glyphon colors pre-computed
    glyph_colors: [GlyphColor; COLOR_PALETTE_SIZE],
}

impl ColorPalette {
    /// Create a new color palette with default colors
    #[inline]
    pub const fn new() -> Self {
        let mut palette = Self {
            colors_linear: [[0.0; 4]; COLOR_PALETTE_SIZE],
            colors_srgb: [[0; 4]; COLOR_PALETTE_SIZE],
            glyph_colors: [GlyphColor::rgba(0, 0, 0, 255); COLOR_PALETTE_SIZE],
        };

        // Initialize standard 16 colors
        palette.colors_srgb[0] = [0, 0, 0, 255]; // Black
        palette.colors_srgb[1] = [128, 0, 0, 255]; // Dark Red
        palette.colors_srgb[2] = [0, 128, 0, 255]; // Dark Green
        palette.colors_srgb[3] = [128, 128, 0, 255]; // Dark Yellow
        palette.colors_srgb[4] = [0, 0, 128, 255]; // Dark Blue
        palette.colors_srgb[5] = [128, 0, 128, 255]; // Dark Magenta
        palette.colors_srgb[6] = [0, 128, 128, 255]; // Dark Cyan
        palette.colors_srgb[7] = [192, 192, 192, 255]; // Light Gray
        palette.colors_srgb[8] = [128, 128, 128, 255]; // Dark Gray
        palette.colors_srgb[9] = [255, 0, 0, 255]; // Red
        palette.colors_srgb[10] = [0, 255, 0, 255]; // Green
        palette.colors_srgb[11] = [255, 255, 0, 255]; // Yellow
        palette.colors_srgb[12] = [0, 0, 255, 255]; // Blue
        palette.colors_srgb[13] = [255, 0, 255, 255]; // Magenta
        palette.colors_srgb[14] = [0, 255, 255, 255]; // Cyan
        palette.colors_srgb[15] = [255, 255, 255, 255]; // White

        // Fill remaining colors with grayscale gradient
        let mut i = 16;
        while i < COLOR_PALETTE_SIZE {
            let intensity = ((i - 16) as f32 / (COLOR_PALETTE_SIZE - 16) as f32 * 255.0) as u8;
            palette.colors_srgb[i] = [intensity, intensity, intensity, 255];
            i += 1;
        }

        palette
    }

    /// Initialize runtime values (linear colors and glyphon colors)
    #[inline]
    pub fn initialize_runtime(&mut self) {
        for i in 0..COLOR_PALETTE_SIZE {
            let [r, g, b, a] = self.colors_srgb[i];

            // sRGB to linear conversion
            self.colors_linear[i] = [
                Self::srgb_to_linear(r as f32 / 255.0),
                Self::srgb_to_linear(g as f32 / 255.0),
                Self::srgb_to_linear(b as f32 / 255.0),
                a as f32 / 255.0,
            ];

            // Initialize glyphon color
            self.glyph_colors[i] = GlyphColor::rgba(r, g, b, a);
        }
    }

    /// Convert sRGB component to linear color space
    #[inline]
    const fn srgb_to_linear(srgb: f32) -> f32 {
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            // Approximation of pow(2.4) for const fn
            let base = (srgb + 0.055) / 1.055;
            base * base * base
        }
    }

    /// Convert linear component to sRGB color space
    #[inline]
    const fn linear_to_srgb(linear: f32) -> f32 {
        if linear <= 0.0031308 {
            linear * 12.92
        } else {
            // Approximation of pow(1/2.4) for const fn
            1.055 * linear - 0.055
        }
    }

    /// Get glyphon color for rendering
    #[inline]
    pub fn get_glyph_color(&self, index: u8) -> GlyphColor {
        self.glyph_colors[index as usize]
    }

    /// Get linear color for blending
    #[inline]
    pub fn get_linear_color(&self, index: u8) -> [f32; 4] {
        self.colors_linear[index as usize]
    }

    /// Get sRGB color
    #[inline]
    pub fn get_srgb_color(&self, index: u8) -> [u8; 4] {
        self.colors_srgb[index as usize]
    }

    /// Set a color in the palette
    pub fn set_color(&mut self, index: u8, r: u8, g: u8, b: u8, a: u8) {
        let idx = index as usize;
        if idx < COLOR_PALETTE_SIZE {
            self.colors_srgb[idx] = [r, g, b, a];

            // Update linear color
            self.colors_linear[idx] = [
                Self::srgb_to_linear(r as f32 / 255.0),
                Self::srgb_to_linear(g as f32 / 255.0),
                Self::srgb_to_linear(b as f32 / 255.0),
                a as f32 / 255.0,
            ];

            // Update glyphon color
            self.glyph_colors[idx] = GlyphColor::rgba(r, g, b, a);
        }
    }

    /// Set a color from RGB values (alpha defaults to 255)
    #[inline]
    pub fn set_rgb(&mut self, index: u8, r: u8, g: u8, b: u8) {
        self.set_color(index, r, g, b, 255);
    }

    /// Apply a style modifier to a color (returns new color index or same if no change)
    #[inline]
    pub fn apply_modifier(&self, color_index: u8, modifier: ColorModifier) -> u8 {
        match modifier {
            ColorModifier::None => color_index,
            ColorModifier::Bright => {
                // Map to bright variant if in basic colors
                if color_index < 8 {
                    color_index + 8
                } else {
                    color_index
                }
            }
            ColorModifier::Dim => {
                // Map to dim variant if available
                if (8..16).contains(&color_index) {
                    color_index - 8
                } else {
                    color_index
                }
            }
        }
    }

    /// Blend two colors together
    pub fn blend_colors(&self, fg_index: u8, bg_index: u8, alpha: f32) -> [u8; 4] {
        let fg = self.get_linear_color(fg_index);
        let bg = self.get_linear_color(bg_index);

        let blended = [
            fg[0] * alpha + bg[0] * (1.0 - alpha),
            fg[1] * alpha + bg[1] * (1.0 - alpha),
            fg[2] * alpha + bg[2] * (1.0 - alpha),
            1.0, // Always fully opaque
        ];

        [
            (Self::linear_to_srgb(blended[0]) * 255.0) as u8,
            (Self::linear_to_srgb(blended[1]) * 255.0) as u8,
            (Self::linear_to_srgb(blended[2]) * 255.0) as u8,
            255,
        ]
    }

    /// Get the closest palette index for an RGB color
    pub fn find_closest_color(&self, r: u8, g: u8, b: u8) -> u8 {
        let mut best_index = 0u8;
        let mut best_distance = u32::MAX;

        for i in 0..COLOR_PALETTE_SIZE {
            let [pr, pg, pb, _] = self.colors_srgb[i];
            let dr = (r as i32 - pr as i32).unsigned_abs();
            let dg = (g as i32 - pg as i32).unsigned_abs();
            let db = (b as i32 - pb as i32).unsigned_abs();
            let distance = dr * dr + dg * dg + db * db;

            if distance < best_distance {
                best_distance = distance;
                best_index = i as u8;
                if distance == 0 {
                    break;
                }
            }
        }

        best_index
    }

    /// Reset palette to default colors
    pub fn reset(&mut self) {
        *self = Self::new();
        self.initialize_runtime();
    }

    /// Load a custom palette from RGB values
    pub fn load_palette(&mut self, colors: &[[u8; 3]]) {
        for (i, &[r, g, b]) in colors.iter().enumerate() {
            if i >= COLOR_PALETTE_SIZE {
                break;
            }
            self.set_rgb(i as u8, r, g, b);
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut palette = Self::new();
        palette.initialize_runtime();
        palette
    }
}

/// Color modifiers for styling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Public API for library users
pub enum ColorModifier {
    None,
    Bright,
    Dim,
}

/// Standard color indices
#[allow(dead_code)] // Public API for library users
pub mod colors {
    pub const BLACK: u8 = 0;
    pub const DARK_RED: u8 = 1;
    pub const DARK_GREEN: u8 = 2;
    pub const DARK_YELLOW: u8 = 3;
    pub const DARK_BLUE: u8 = 4;
    pub const DARK_MAGENTA: u8 = 5;
    pub const DARK_CYAN: u8 = 6;
    pub const LIGHT_GRAY: u8 = 7;
    pub const DARK_GRAY: u8 = 8;
    pub const RED: u8 = 9;
    pub const GREEN: u8 = 10;
    pub const YELLOW: u8 = 11;
    pub const BLUE: u8 = 12;
    pub const MAGENTA: u8 = 13;
    pub const CYAN: u8 = 14;
    pub const WHITE: u8 = 15;
}
