//! Production-quality Glyphon text rendering
//!
//! This module contains ratagpu's battle-tested, zero-allocation Glyphon
//! text rendering system adapted for hybrid render-to-texture usage.

pub mod cache;
pub mod cell;
pub mod color;

pub mod font_system;
pub mod ligature;
pub mod ligature_config;
pub mod text_rendering;
pub mod texture_renderer;
pub mod wgpu_setup;

// Re-export key types
pub use cache::{LockFreeShapeCache, SafeTextArea, ZeroAllocTextAreaPool};
pub use cell::{Cell, CellGrid};
pub use color::ColorPalette;
pub use font_system::{FontMetrics, create_optimized_font_system};
pub use ligature::{LigatureHelper, font_supports_ligatures, recommended_shaping, utils};
pub use ligature_config::{LigatureConfig, LigatureConfigBuilder};
pub use text_rendering::{TextRenderConfig, ZeroAllocTextRenderer};
pub use texture_renderer::GlyphonTextureRenderer;
