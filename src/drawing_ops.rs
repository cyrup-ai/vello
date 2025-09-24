//! Drawing operations for Scene extracted from scene.rs
//!
//! This module contains all drawing operations (fill, stroke, draw_image, etc.)

use peniko::{
    BrushRef, Color, Fill, Font, Image,
    kurbo::{Affine, Rect, Shape, Stroke, StrokeOpts},
};
#[cfg(feature = "bump_estimate")]
use vello_encoding::BumpAllocatorMemory;
use vello_encoding::Transform;

use crate::{glyph_builder::DrawGlyphs, scene_core::Scene};

impl Scene {
    /// Draw a rounded rectangle blurred with a gaussian filter.
    pub fn draw_blurred_rounded_rect(
        &mut self,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        // The impulse response of a gaussian filter is infinite.
        // For performance reason we cut off the filter at some extent where the response is close to zero.
        let kernel_size = 2.5 * std_dev;

        let shape: Rect = rect.inflate(kernel_size, kernel_size);
        self.draw_blurred_rounded_rect_in(&shape, transform, rect, brush, radius, std_dev);
    }

    /// Draw a rounded rectangle blurred with a gaussian filter in `shape`.
    ///
    /// For performance reasons, `shape` should not extend more than approximately 2.5 times
    /// `std_dev` away from the edges of `rect` (as any such points will not be perceptably painted to,
    /// but calculations will still be performed for them).
    ///
    /// This method effectively draws the blurred rounded rectangle clipped to the given shape.
    /// If just the blurred rounded rectangle is desired without clipping,
    /// use the simpler [`Self::draw_blurred_rounded_rect`].
    /// For many users, that method will be easier to use.
    pub fn draw_blurred_rounded_rect_in(
        &mut self,
        shape: &impl Shape,
        transform: Affine,
        rect: Rect,
        brush: Color,
        radius: f64,
        std_dev: f64,
    ) {
        let t = Transform::from_kurbo(&transform);
        self.encoding.encode_transform(t);

        self.encoding.encode_fill_style(Fill::NonZero);
        if self.encoding.encode_shape(&shape, true) {
            let brush_transform =
                Transform::from_kurbo(&transform.pre_translate(rect.center().to_vec2()));
            if self.encoding.encode_transform(brush_transform) {
                self.encoding.swap_last_path_tags();
            }
            self.encoding.encode_blurred_rounded_rect(
                brush,
                rect.width() as _,
                rect.height() as _,
                radius as _,
                std_dev as _,
            );
        }
    }

    /// Fills a shape using the specified style and brush.
    #[expect(
        single_use_lifetimes,
        reason = "False positive: https://github.com/rust-lang/rust/issues/129255"
    )]
    pub fn fill<'b>(
        &mut self,
        style: Fill,
        transform: Affine,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        let t = Transform::from_kurbo(&transform);
        self.encoding.encode_transform(t);
        self.encoding.encode_fill_style(style);
        if self.encoding.encode_shape(shape, true) {
            if let Some(brush_transform) = brush_transform {
                if self
                    .encoding
                    .encode_transform(Transform::from_kurbo(&(transform * brush_transform)))
                {
                    self.encoding.swap_last_path_tags();
                }
            }
            self.encoding.encode_brush(brush, 1.0);
            #[cfg(feature = "bump_estimate")]
            self.estimator
                .count_path(shape.path_elements(0.1), &t, None);
        }
    }

    /// Strokes a shape using the specified style and brush.
    #[expect(
        single_use_lifetimes,
        reason = "False positive: https://github.com/rust-lang/rust/issues/129255"
    )]
    pub fn stroke<'b>(
        &mut self,
        style: &Stroke,
        transform: Affine,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine>,
        shape: &impl Shape,
    ) {
        // The setting for tolerance are a compromise. For most applications,
        // shape tolerance doesn't matter, as the input is likely Bézier paths,
        // which is exact. Note that shape tolerance is hard-coded as 0.1 in
        // the encoding crate.
        //
        // Stroke tolerance is a different matter. Generally, the cost scales
        // with inverse O(n^6), so there is moderate rendering cost to setting
        // too fine a value. On the other hand, error scales with the transform
        // applied post-stroking, so may exceed visible threshold. When we do
        // GPU-side stroking, the transform will be known. In the meantime,
        // this is a compromise.
        const SHAPE_TOLERANCE: f64 = 0.01;
        const STROKE_TOLERANCE: f64 = SHAPE_TOLERANCE;

        const GPU_STROKES: bool = true; // Set this to `true` to enable GPU-side stroking
        if GPU_STROKES {
            if style.width == 0. {
                return;
            }

            let t = Transform::from_kurbo(&transform);
            self.encoding.encode_transform(t);
            let encoded_stroke = self.encoding.encode_stroke_style(style);
            debug_assert!(encoded_stroke, "Stroke width is non-zero");

            // We currently don't support dashing on the GPU. If the style has a dash pattern, then
            // we convert it into stroked paths on the CPU and encode those as individual draw
            // objects.
            let encode_result = if style.dash_pattern.is_empty() {
                #[cfg(feature = "bump_estimate")]
                self.estimator
                    .count_path(shape.path_elements(SHAPE_TOLERANCE), &t, Some(style));
                self.encoding.encode_shape(shape, false)
            } else {
                // TODO: We currently collect the output of the dash iterator because
                // `encode_path_elements` wants to consume the iterator. We want to avoid calling
                // `dash` twice when `bump_estimate` is enabled because it internally allocates.
                // Bump estimation will move to resolve time rather than scene construction time,
                // so we can revert this back to not collecting when that happens.
                let dashed = peniko::kurbo::dash(
                    shape.path_elements(SHAPE_TOLERANCE),
                    style.dash_offset,
                    &style.dash_pattern,
                )
                .collect::<Vec<_>>();
                #[cfg(feature = "bump_estimate")]
                self.estimator
                    .count_path(dashed.iter().copied(), &t, Some(style));
                self.encoding
                    .encode_path_elements(dashed.into_iter(), false)
            };
            if encode_result {
                if let Some(brush_transform) = brush_transform {
                    if self
                        .encoding
                        .encode_transform(Transform::from_kurbo(&(transform * brush_transform)))
                    {
                        self.encoding.swap_last_path_tags();
                    }
                }
                self.encoding.encode_brush(brush, 1.0);
            }
        } else {
            let stroked = peniko::kurbo::stroke(
                shape.path_elements(SHAPE_TOLERANCE),
                style,
                &StrokeOpts::default(),
                STROKE_TOLERANCE,
            );
            self.fill(Fill::NonZero, transform, brush, brush_transform, &stroked);
        }
    }

    /// Draws an image at its natural size with the given transform.
    #[inline]
    pub fn draw_image(&mut self, image: &Image, transform: Affine) {
        self.fill(
            Fill::NonZero,
            transform,
            image,
            None,
            &Rect::new(0.0, 0.0, image.width as f64, image.height as f64),
        );
    }

    /// Returns a builder for encoding a glyph run.
    #[inline]
    pub fn draw_glyphs(&mut self, font: &Font) -> DrawGlyphs<'_> {
        // TODO: Integrate `BumpEstimator` with the glyph cache.
        DrawGlyphs::new(self, font)
    }
}
