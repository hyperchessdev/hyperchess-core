// SPDX-License-Identifier: GPL-3.0-or-later
// HyperChess Core — hyperchess-wasm
// File: crates/hyperchess-wasm/src/gpu.rs
// Version: 1.0.0
// Copyright (c) 2026 HyperChess Developer Team

//! Bare wgpu context: instance/adapter/device/surface acquisition. Pipeline,
//! meshes, and the actual draw calls live in `scene.rs` — this module only knows
//! how to stand up a GPU device against a canvas and hand back a configured frame.

use web_sys::HtmlCanvasElement;

/// Owns the wgpu handles for one canvas, for the lifetime of the renderer.
pub struct GpuContext {
    /// Presentation surface backed by the canvas. `'static` because the canvas
    /// element is moved into the surface rather than borrowed.
    pub surface: wgpu::Surface<'static>,
    /// Logical device used to create every pipeline and buffer in `scene.rs`.
    pub device: wgpu::Device,
    /// Command queue for submissions and buffer writes.
    pub queue: wgpu::Queue,
    /// Current surface configuration; kept so [`GpuContext::resize`] can
    /// reconfigure without re-querying adapter capabilities.
    pub config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    /// Acquire adapter, device, and a configured surface for `canvas`.
    ///
    /// Every failure is returned as a `String` rather than panicking: on the
    /// web these are all ordinary "this browser cannot do it" outcomes, and the
    /// JS caller needs to fall back to a 2D board rather than see a wasm trap.
    ///
    /// Two deliberate choices in the setup: limits are negotiated down to
    /// `downlevel_webgl2_defaults` so the GL fallback path works on hardware
    /// that cannot meet WebGPU defaults, and an sRGB surface format is
    /// preferred so shader output does not need manual gamma correction.
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        // See the comment on `new_instance_with_webgpu_detection` below: Chromium can
        // expose `navigator.gpu` with no real adapter behind it (notably headless/
        // Linux), which crashes deep in wgpu's webgpu backend if we don't probe first.
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        })
        .await;

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| format!("create_surface failed: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| format!("no suitable GPU adapter: {e}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("hyperchess-3d-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("request_device failed: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
        })
    }

    /// Reconfigure the surface for a new canvas size.
    ///
    /// Zero in either dimension is ignored rather than clamped — a hidden or
    /// collapsed canvas reports 0 and configuring a zero-sized surface is a
    /// validation error, so the old configuration is simply kept until the
    /// canvas is visible again.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Current width/height ratio for the projection matrix.
    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
    }
}
