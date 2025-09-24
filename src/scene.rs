//! Scene module - re-exports and coordination between scene modules
//!
//! This module coordinates the scene_core, drawing_ops, and glyph_builder modules.

// Re-export the core Scene struct
// Import drawing operations to add methods to Scene
// This ensures all drawing methods are available on Scene instances
#[allow(unused_imports)]
use crate::drawing_ops::*;
// Re-export DrawGlyphs from glyph_builder
