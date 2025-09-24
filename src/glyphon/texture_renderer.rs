//! Texture-based renderer adapted from ratagpu's core renderer
//!
//! This renderer uses ratagpu's zero-allocation approach but renders to a texture
//! instead of directly to the surface, for use in compute shader pipelines.

use std::sync::Arc;

use anyhow::{Context, Result};
use glyphon::{
    Cache, ColorMode, FontSystem, Resolution, SwashCache, TextAtlas, TextRenderer, Viewport,
};
use wgpu::{Device, Queue, RenderPassColorAttachment, SurfaceConfiguration, TextureFormat};

use super::{
    cache::{LockFreeShapeCache, ZeroAllocTextAreaPool},
    cell::{Cell, CellGrid},
    color::ColorPalette,
    font_system::{FontMetrics, create_font_system_with_nerd_font},
    text_rendering::{TextRenderConfig, ZeroAllocTextRenderer},
};

/// Production-quality texture renderer using ratagpu's architecture
pub struct GlyphonTextureRenderer<const COLS: usize, const ROWS: usize> {
    // Core wgpu resources
    device: Arc<Device>,
    queue: Queue,
    #[allow(dead_code)] // Surface configuration for future rendering modes
    config: SurfaceConfiguration,
    #[allow(dead_code)] // Color mode configuration
    color_mode: ColorMode,

    // Glyphon components
    font_system: FontSystem,
    swash_cache: SwashCache,
    #[allow(dead_code)] // Glyphon cache for texture operations
    cache: Cache,
    viewport: Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,

    // Zero-allocation data structures (ratagpu style)
    cell_grid: CellGrid<COLS, ROWS>,
    color_palette: ColorPalette,
    shape_cache: LockFreeShapeCache<2048>, // SHAPE_CACHE_SIZE from ratagpu
    text_area_pool: ZeroAllocTextAreaPool<128>, // TEXT_AREA_POOL_SIZE from ratagpu

    // Configuration
    font_size: f32,
    line_height: f32,
    scale_factor: f32,
    frame_count: u64,
}

impl<const COLS: usize, const ROWS: usize> GlyphonTextureRenderer<COLS, ROWS> {
    /// Create new texture renderer using ratagpu's proven architecture
    pub async fn new(
        device: Arc<Device>,
        queue: Queue,
        config: SurfaceConfiguration,
        font_size: f32,
    ) -> Result<Self> {
        println!("[GLYPHON DEBUG] GlyphonTextureRenderer::new: Starting renderer creation");
        let color_mode = ColorMode::Accurate;
        let scale_factor = 1.0;

        // Initialize glyphon components (ratagpu style) with FiraCode Nerd Font Mono
        println!("[GLYPHON DEBUG] GlyphonTextureRenderer::new: About to create font system (THIS WILL AWAIT)");
        let font_system = create_font_system_with_nerd_font("FiraCode Nerd Font Mono").await?;
        println!("[GLYPHON DEBUG] GlyphonTextureRenderer::new: Font system created successfully");
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device.as_ref());
        let viewport = Viewport::new(device.as_ref(), &cache);
        let mut atlas = TextAtlas::with_color_mode(
            device.as_ref(),
            &queue,
            &cache,
            TextureFormat::Rgba8UnormSrgb,
            color_mode,
        );
        let text_renderer =
            TextRenderer::new(&mut atlas, device.as_ref(), Default::default(), None);

        // Initialize zero-allocation data structures
        let cell_grid = CellGrid::new();
        let mut color_palette = ColorPalette::new();
        color_palette.initialize_runtime();
        let shape_cache = LockFreeShapeCache::new();
        let text_area_pool = ZeroAllocTextAreaPool::new();

        // Calculate metrics
        let font_metrics = FontMetrics::new(font_size);
        let line_height = font_metrics.line_height;

        Ok(Self {
            device,
            queue,
            config,
            color_mode,
            font_system,
            swash_cache,
            cache,
            viewport,
            atlas,
            text_renderer,
            cell_grid,
            color_palette,
            shape_cache,
            text_area_pool,
            font_size,
            line_height,
            scale_factor,
            frame_count: 0,
        })
    }

    /// Set cell content (ratagpu compatible interface)
    pub fn set_cell(&mut self, row: usize, col: usize, cell: Cell) {
        if row < ROWS && col < COLS {
            self.cell_grid.set_cell(row, col, cell);
        }
    }

    /// Load layout data into cell grid
    pub fn load_layout(&mut self, layout: &[crate::layout::PositionedLine]) {
        // Clear existing content
        self.cell_grid.clear();

        for (row_idx, line) in layout.iter().enumerate().take(ROWS) {
            for (col_idx, styled_char) in line.chars.iter().enumerate().take(COLS) {
                let cell = Cell {
                    character: styled_char.char,
                    foreground: 15, // White - we'll handle colors differently
                    background: 0,  // Black
                    style_flags: 0,
                };
                self.set_cell(row_idx, col_idx, cell);
            }
        }
    }

    /// Render text to texture using ratagpu's proven pipeline
    pub fn render_to_texture(&mut self, width: u32, height: u32) -> Result<wgpu::Texture> {
        self.frame_count += 1;

        // Update viewport (ratagpu style)
        self.viewport
            .update(&self.queue, Resolution { width, height });

        // Create texture for rendering
        let texture = self
            .device
            .as_ref()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Glyphon Text Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Process dirty rows and create text areas (ratagpu's exact approach)
        let text_area_vec = {
            let config = TextRenderConfig::new(
                self.font_size,
                self.line_height,
                self.scale_factor,
                width,
                height,
            );

            // Use ratagpu's zero-allocation text renderer
            let text_areas =
                ZeroAllocTextRenderer::create_text_areas_for_dirty_rows_fallback::<COLS, ROWS>(
                    self.cell_grid.dirty_rows_iter(),
                    &mut self.shape_cache,
                    &mut self.text_area_pool,
                    &self.color_palette,
                    &mut self.font_system,
                    &config,
                    self.frame_count,
                );

            // Convert to glyphon text areas (ratagpu's exact method)
            let buffer_indices: Vec<_> = text_areas
                .iter()
                .map(|area| {
                    (
                        area.buffer_cache_index(),
                        area.left(),
                        area.top(),
                        area.scale(),
                        *area.bounds(),
                        *area.default_color(),
                    )
                })
                .collect();

            let mut glyphon_areas = Vec::with_capacity(buffer_indices.len());
            for (cache_index, left, top, scale, bounds, default_color) in buffer_indices {
                if let Some(buffer) = self.shape_cache.get_buffer(cache_index) {
                    glyphon_areas.push(glyphon::TextArea {
                        buffer,
                        left,
                        top,
                        scale,
                        bounds,
                        default_color,
                        custom_glyphs: &[],
                    });
                }
            }

            glyphon_areas
        };

        // Prepare rendering (ratagpu style)
        let has_text_areas = !text_area_vec.is_empty();
        if has_text_areas {
            self.text_renderer
                .prepare(
                    &*self.device,
                    &self.queue,
                    &mut self.font_system,
                    &mut self.atlas,
                    &self.viewport,
                    text_area_vec,
                    &mut self.swash_cache,
                )
                .context("Failed to prepare text rendering")?;
        }

        // Record commands (ratagpu style)
        let mut encoder =
            self.device
                .as_ref()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Glyphon Texture Render"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Glyphon Text Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if has_text_areas {
                self.text_renderer
                    .render(&self.atlas, &self.viewport, &mut render_pass)
                    .context("Failed to render text")?;
            }
        }

        // Submit commands
        self.queue.submit(Some(encoder.finish()));

        // Maintenance (ratagpu style)
        if self.frame_count % 60 == 0 {
            self.atlas.trim();
        }
        if self.frame_count % 300 == 0 {
            self.shape_cache.evict_lru_if_needed(0.8, self.frame_count);
        }

        // Clear dirty flags
        self.cell_grid.clear_dirty_flags();

        Ok(texture)
    }
}
