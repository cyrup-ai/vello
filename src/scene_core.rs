//! Core Scene functionality extracted from scene.rs
//!
//! This module contains the Scene struct and its core methods.

use peniko::{
    BlendMode, Fill, Mix,
    kurbo::{Affine, Shape},
};
#[cfg(feature = "bump_estimate")]
use vello_encoding::BumpAllocatorMemory;
use vello_encoding::{Encoding, Transform};

/// The main datatype for rendering graphics.
///
/// A `Scene` stores a sequence of drawing commands, their context, and the
/// associated resources, which can later be rendered.
///
/// Most users will render this using [`Renderer::render_to_texture`][crate::Renderer::render_to_texture].
///
/// Rendering from a `Scene` will *not* clear it, which should be done in a separate step, by calling [`Scene::reset`].
///
/// If this is not done for a scene which is retained (to avoid allocations) between frames, this will likely
/// quickly increase the complexity of the render result, leading to crashes or potential host system instability.
#[derive(Clone, Default)]
pub struct Scene {
    pub(crate) encoding: Encoding,
    #[cfg(feature = "bump_estimate")]
    pub(crate) estimator: vello_encoding::BumpEstimator,
}
static_assertions::assert_impl_all!(Scene: Send, Sync);

impl Scene {
    /// Creates a new scene.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all content from the scene.
    #[inline]
    pub fn reset(&mut self) {
        self.encoding.reset();
        #[cfg(feature = "bump_estimate")]
        self.estimator.reset();
    }

    /// Tally up the bump allocator estimate for the current state of the encoding,
    /// taking into account an optional `transform` applied to the entire scene.
    #[cfg(feature = "bump_estimate")]
    #[inline]
    pub fn bump_estimate(&self, transform: Option<Affine>) -> BumpAllocatorMemory {
        self.estimator
            .tally(transform.as_ref().map(Transform::from_kurbo).as_ref())
    }

    /// Returns the underlying raw encoding.
    #[inline]
    pub fn encoding(&self) -> &Encoding {
        &self.encoding
    }

    /// Returns a mutable reference to the underlying raw encoding.
    ///
    /// This can be used to more easily create invalid scenes, and so should be used with care.
    #[inline]
    pub fn encoding_mut(&mut self) -> &mut Encoding {
        &mut self.encoding
    }

    /// Pushes a new layer clipped by the specified shape and composed with
    /// previous layers using the specified blend mode.
    ///
    /// Every drawing command after this call will be clipped by the shape
    /// until the layer is popped.
    ///
    /// **However, the transforms are *not* saved or modified by the layer stack.**
    ///
    /// Clip layers (`blend` = [`Mix::Clip`]) should have an alpha value of 1.0.
    /// For an opacity group with non-unity alpha, specify [`Mix::Normal`].
    pub fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine,
        clip: &impl Shape,
    ) {
        let blend = blend.into();
        if blend.mix == Mix::Clip && alpha != 1.0 {
            log::warn!("Clip mix mode used with semitransparent alpha");
        }
        let t = Transform::from_kurbo(&transform);
        self.encoding.encode_transform(t);
        self.encoding.encode_fill_style(Fill::NonZero);
        if !self.encoding.encode_shape(clip, true) {
            // If the layer shape is invalid, encode a valid empty path. This suppresses
            // all drawing until the layer is popped.
            self.encoding.encode_empty_shape();
            #[cfg(feature = "bump_estimate")]
            {
                use peniko::kurbo::{PathEl, Point};
                let path = [PathEl::MoveTo(Point::ZERO), PathEl::LineTo(Point::ZERO)];
                self.estimator.count_path(path.into_iter(), &t, None);
            }
        } else {
            #[cfg(feature = "bump_estimate")]
            self.estimator.count_path(clip.path_elements(0.1), &t, None);
        }
        self.encoding
            .encode_begin_clip(blend, alpha.clamp(0.0, 1.0));
    }

    /// Pops the current layer.
    #[inline]
    pub fn pop_layer(&mut self) {
        self.encoding.encode_end_clip();
    }

    /// Appends a child scene.
    ///
    /// The given transform is applied to every transform in the child.
    /// This is an O(N) operation.
    pub fn append(&mut self, other: &Self, transform: Option<Affine>) {
        let t = transform.as_ref().map(Transform::from_kurbo);
        self.encoding.append(&other.encoding, &t);
        #[cfg(feature = "bump_estimate")]
        self.estimator.append(&other.estimator, t.as_ref());
    }
}

impl From<Encoding> for Scene {
    fn from(encoding: Encoding) -> Self {
        // It's fine to create a default estimator here, and that field will be
        // removed at some point - see https://github.com/linebender/vello/issues/541
        Self {
            encoding,
            #[cfg(feature = "bump_estimate")]
            estimator: vello_encoding::BumpEstimator::default(),
        }
    }
}
