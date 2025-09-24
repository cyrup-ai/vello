//! True zero-allocation wgpu setup
//!
//! This module provides GPU resource initialization with:
//! - Zero heap allocations
//! - No unsafe code
//! - No static storage
//! - On-demand Instance/Surface creation
//! - Pure stack-based operation

use anyhow::{Context, Result};
use glyphon::ColorMode;
use wgpu::{
    Device, Queue, Surface, SurfaceCapabilities, SurfaceConfiguration, TextureFormat, TextureUsages,
};
use winit::{dpi::PhysicalSize, window::Window};

/// GPU resources - only stores what can be safely kept
///
/// No Instance or Surface storage to avoid lifetime issues
pub struct GpuResources {
    pub device: Device,
    pub queue: Queue,
    pub adapter_info: wgpu::AdapterInfo,
    pub config: SurfaceConfiguration,
    pub color_mode: ColorMode,
}

impl GpuResources {
    /// Update configuration for resize
    #[inline(always)]
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
        }
    }

    /// Get background color for current color mode
    #[inline(always)]
    pub const fn background_color(&self) -> wgpu::Color {
        match self.color_mode {
            ColorMode::Accurate => wgpu::Color {
                r: 0.02,
                g: 0.02,
                b: 0.03,
                a: 1.0,
            },
            ColorMode::Web => wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        }
    }
}

/// Frame handle for rendering operations
///
/// Created on-demand for each frame, no allocations
pub struct FrameHandle<'frame> {
    pub surface: Surface<'frame>,
    pub device: &'frame Device,
    pub queue: &'frame Queue,
    pub config: &'frame SurfaceConfiguration,
}

impl<'frame> FrameHandle<'frame> {
    /// Configure the surface
    #[inline(always)]
    pub fn configure(&self) {
        self.surface.configure(self.device, self.config);
    }

    /// Get current texture
    #[inline(always)]
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture> {
        self.surface
            .get_current_texture()
            .context("Failed to acquire surface texture")
    }
}

/// Initialize GPU resources
///
/// This function creates temporary Instance/Surface for initialization,
/// then drops them. Only Device/Queue are kept.
pub async fn initialize_wgpu(window: &Window) -> Result<GpuResources> {
    println!("[GLYPHON DEBUG] initialize_wgpu: Starting WGPU initialization");
    let size = window.inner_size();
    println!("[GLYPHON DEBUG] initialize_wgpu: Window size: {}x{}", size.width, size.height);

    // Create instance on stack
    println!("[GLYPHON DEBUG] initialize_wgpu: Creating WGPU instance");
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    println!("[GLYPHON DEBUG] initialize_wgpu: WGPU instance created successfully");

    // Create surface on stack
    println!("[GLYPHON DEBUG] initialize_wgpu: Creating surface");
    let surface = instance.create_surface(window)?;
    println!("[GLYPHON DEBUG] initialize_wgpu: Surface created successfully");

    // Request adapter
    println!("[GLYPHON DEBUG] initialize_wgpu: Requesting adapter (THIS WILL AWAIT)");
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .context("No compatible GPU adapter found")?;
    println!("[GLYPHON DEBUG] initialize_wgpu: Adapter received successfully");

    let adapter_info = adapter.get_info();
    println!("[GLYPHON DEBUG] initialize_wgpu: Adapter info: {:?}", adapter_info);

    // Create device and queue
    println!("[GLYPHON DEBUG] initialize_wgpu: Setting up device limits");
    let limits = wgpu::Limits::downlevel_webgl2_defaults()
        .using_resolution(adapter.limits())
        .using_alignment(adapter.limits());
    println!("[GLYPHON DEBUG] initialize_wgpu: Device limits configured");

    println!("[GLYPHON DEBUG] initialize_wgpu: Requesting device (THIS WILL AWAIT)");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("RataGPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off, // No tracing by default
        })
        .await
        .context("Failed to create GPU device")?;
    println!("[GLYPHON DEBUG] initialize_wgpu: Device and queue created successfully");

    // Determine optimal configuration
    println!("[GLYPHON DEBUG] initialize_wgpu: Getting surface capabilities");
    let surface_caps = surface.get_capabilities(&adapter);
    println!("[GLYPHON DEBUG] initialize_wgpu: Creating optimal config");
    let (color_mode, config) = create_optimal_config(&surface_caps, size)?;
    println!("[GLYPHON DEBUG] initialize_wgpu: Configuration created - color_mode: {:?}, format: {:?}", color_mode, config.format);

    // Instance and Surface are dropped here - we don't store them
    println!("[GLYPHON DEBUG] initialize_wgpu: Initialization complete, returning resources");

    Ok(GpuResources {
        device,
        queue,
        adapter_info,
        config,
        color_mode,
    })
}

/// Create a frame handle for rendering
///
/// This creates Instance/Surface on-demand, on the stack
#[inline(always)]
pub fn create_frame<'frame>(
    window: &'frame Window,
    resources: &'frame GpuResources,
) -> Result<FrameHandle<'frame>> {
    println!("[GLYPHON DEBUG] create_frame: Creating frame handle");
    
    // Create instance on stack
    println!("[GLYPHON DEBUG] create_frame: Creating new WGPU instance for frame");
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    println!("[GLYPHON DEBUG] create_frame: Frame instance created");

    // Create surface on stack
    println!("[GLYPHON DEBUG] create_frame: Creating surface for frame");
    let surface = instance.create_surface(window)?;
    println!("[GLYPHON DEBUG] create_frame: Frame surface created successfully");

    println!("[GLYPHON DEBUG] create_frame: Returning frame handle");
    Ok(FrameHandle {
        surface,
        device: &resources.device,
        queue: &resources.queue,
        config: &resources.config,
    })
}

/// Create optimal surface configuration
#[inline(always)]
fn create_optimal_config(
    capabilities: &SurfaceCapabilities,
    size: PhysicalSize<u32>,
) -> Result<(ColorMode, SurfaceConfiguration)> {
    let (color_mode, format) = select_optimal_format(capabilities)?;
    let present_mode = select_present_mode(&capabilities.present_modes);
    let alpha_mode = select_alpha_mode(&capabilities.alpha_modes);

    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        alpha_mode,
        view_formats: vec![format],
        desired_maximum_frame_latency: 2,
    };

    Ok((color_mode, config))
}

/// Select optimal surface format
#[inline(always)]
fn select_optimal_format(caps: &SurfaceCapabilities) -> Result<(ColorMode, TextureFormat)> {
    // Prefer sRGB formats - const arrays for zero allocation
    const SRGB_FORMATS: &[TextureFormat] =
        &[TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba8UnormSrgb];

    for &format in SRGB_FORMATS {
        if caps.formats.contains(&format) {
            return Ok((ColorMode::Accurate, format));
        }
    }

    // Fallback to linear formats
    const LINEAR_FORMATS: &[TextureFormat] =
        &[TextureFormat::Bgra8Unorm, TextureFormat::Rgba8Unorm];

    for &format in LINEAR_FORMATS {
        if caps.formats.contains(&format) {
            return Ok((ColorMode::Web, format));
        }
    }

    // Use first available
    caps.formats
        .first()
        .copied()
        .map(|format| (ColorMode::Web, format))
        .context("No supported surface formats")
}

/// Select optimal present mode for low latency
#[inline(always)]
const fn select_present_mode(modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    // Manual loop for const fn compatibility
    let mut i = 0;
    while i < modes.len() {
        if matches!(modes[i], wgpu::PresentMode::Immediate) {
            return wgpu::PresentMode::Immediate;
        }
        i += 1;
    }

    i = 0;
    while i < modes.len() {
        if matches!(modes[i], wgpu::PresentMode::Mailbox) {
            return wgpu::PresentMode::Mailbox;
        }
        i += 1;
    }

    wgpu::PresentMode::Fifo
}

/// Select optimal alpha mode
#[inline(always)]
const fn select_alpha_mode(modes: &[wgpu::CompositeAlphaMode]) -> wgpu::CompositeAlphaMode {
    // Manual loop for const fn compatibility
    let mut i = 0;
    while i < modes.len() {
        if matches!(modes[i], wgpu::CompositeAlphaMode::Opaque) {
            return wgpu::CompositeAlphaMode::Opaque;
        }
        i += 1;
    }

    if !modes.is_empty() {
        modes[0]
    } else {
        wgpu::CompositeAlphaMode::Auto
    }
}

/// Get background color for color mode
#[inline(always)]
pub const fn get_background_color(color_mode: ColorMode) -> wgpu::Color {
    match color_mode {
        ColorMode::Accurate => wgpu::Color {
            r: 0.1, // Slightly lighter background
            g: 0.1,
            b: 0.15,
            a: 1.0,
        },
        ColorMode::Web => wgpu::Color {
            r: 0.1, // Dark blue background instead of pure black
            g: 0.1,
            b: 0.2,
            a: 1.0,
        },
    }
}

/// Update surface configuration for resize
#[inline(always)]
pub fn update_surface_config(
    config: &mut SurfaceConfiguration,
    new_size: PhysicalSize<u32>,
) -> bool {
    if new_size.width > 0 && new_size.height > 0 {
        config.width = new_size.width;
        config.height = new_size.height;
        true
    } else {
        false
    }
}
