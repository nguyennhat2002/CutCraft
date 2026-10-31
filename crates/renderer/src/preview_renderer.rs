//! Renderer-owned single-layer video Preview pipeline.
//!
//! The renderer reuses the device/queue created by eframe's wgpu backend. It
//! owns only Preview textures and draw resources; decoded CPU frames stay in
//! `capcut-media`/UI and are never stored in project state.

use std::time::Instant;

use capcut_media::{DecodedFrame, PixelFormat};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, ColorTargetState, ColorWrites,
    Device, Extent3d, FilterMode, FragmentState, MipmapFilterMode, MultisampleState, Origin3d,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, TexelCopyBufferLayout,
    TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexState,
};

pub const MAX_PREVIEW_DIMENSION: u32 = 8192;
pub const MAX_PREVIEW_TEXTURE_BYTES: u64 = 256 * 1024 * 1024;

const SHADER: &str = include_str!("shaders/video_preview.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameIdentity {
    pub source_id: u64,
    pub generation: u64,
    pub clip_id: u64,
    pub asset_id: u64,
    pub pts_ticks: i64,
    pub pts_timebase_numerator: u32,
    pub pts_timebase_denominator: u32,
    pub width: u32,
    pub height: u32,
}

impl FrameIdentity {
    pub fn from_decoded(frame: &DecodedFrame) -> Self {
        Self {
            source_id: frame.source_id,
            generation: frame.generation,
            clip_id: frame.clip_tag,
            asset_id: frame.asset_tag,
            pts_ticks: frame.pts.ticks,
            pts_timebase_numerator: frame.pts.timebase.numerator,
            pts_timebase_denominator: frame.pts.timebase.denominator,
            width: frame.width,
            height: frame.height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewUpload {
    Uploaded,
    SkippedSameFrame,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewRenderError {
    InvalidFrame(&'static str),
    InvalidDimensions { width: u32, height: u32 },
    Oversized { bytes: u64, limit: u64 },
    UnsupportedFormat,
    TruncatedBuffer { expected: usize, actual: usize },
    StaleGeneration { current: u64, received: u64 },
    ResourceUnavailable,
}

impl std::fmt::Display for PreviewRenderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFrame(message) => write!(formatter, "invalid preview frame: {message}"),
            Self::InvalidDimensions { width, height } => {
                write!(
                    formatter,
                    "preview frame dimensions {width}x{height} are invalid"
                )
            }
            Self::Oversized { bytes, limit } => write!(
                formatter,
                "preview frame needs {bytes} bytes, exceeding the {limit}-byte renderer limit"
            ),
            Self::UnsupportedFormat => {
                formatter.write_str("preview renderer only accepts normalized RGBA8 frames")
            }
            Self::TruncatedBuffer { expected, actual } => write!(
                formatter,
                "preview frame buffer is {actual} bytes; expected {expected}"
            ),
            Self::StaleGeneration { current, received } => write!(
                formatter,
                "preview frame generation {received} is older than current generation {current}"
            ),
            Self::ResourceUnavailable => formatter.write_str("preview GPU resource is unavailable"),
        }
    }
}

impl std::error::Error for PreviewRenderError {}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PreviewRendererMetrics {
    pub frame_upload_cpu_ms: Option<f64>,
    pub texture_allocation_cpu_ms: Option<f64>,
    pub gpu_texture_bytes: u64,
    pub cpu_staging_bytes: u64,
    pub peak_gpu_texture_bytes: u64,
    pub peak_cpu_staging_bytes: u64,
    pub upload_count: u64,
    pub reallocation_count: u64,
    pub texture_reuse_count: u64,
    pub uploads_skipped: u64,
    pub upload_rejections: u64,
    pub renderer_submit_ms: Option<f64>,
    pub gpu_upload_completion_ms: Option<f64>,
    pub preview_present_ms: Option<f64>,
}

/// One texture, one sampler, one pipeline, one active video frame.
pub struct PreviewRenderer {
    device: Device,
    queue: Queue,
    target_format: TextureFormat,
    texture_format: TextureFormat,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
    sampler: Sampler,
    texture: Option<Texture>,
    _view: Option<TextureView>,
    bind_group: Option<BindGroup>,
    size: Option<(u32, u32)>,
    identity: Option<FrameIdentity>,
    metrics: PreviewRendererMetrics,
}

impl std::fmt::Debug for PreviewRenderer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreviewRenderer")
            .field("target_format", &self.target_format)
            .field("texture_format", &self.texture_format)
            .field("size", &self.size)
            .field("identity", &self.identity)
            .field("has_texture", &self.texture.is_some())
            .field("metrics", &self.metrics)
            .finish()
    }
}

impl PreviewRenderer {
    /// Build Preview resources on the existing eframe device. This never
    /// creates a new device, queue, adapter or surface.
    pub fn new(
        device: &Device,
        queue: &Queue,
        target_format: TextureFormat,
    ) -> Result<Self, PreviewRenderError> {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("cutcraft-preview-shader"),
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("cutcraft-preview-bind-group-layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("cutcraft-preview-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("cutcraft-preview-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: target_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("cutcraft-preview-sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let texture_format = preview_texture_format(target_format);
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            target_format,
            texture_format,
            bind_group_layout,
            pipeline,
            sampler,
            texture: None,
            _view: None,
            bind_group: None,
            size: None,
            identity: None,
            metrics: PreviewRendererMetrics::default(),
        })
    }

    pub fn target_format(&self) -> TextureFormat {
        self.target_format
    }

    /// Texture format actually used for uploaded frames. It is derived from the
    /// render target color space, so `Bgra8Unorm` keeps bytes unnormalized and
    /// an sRGB target linearizes on sampling.
    pub fn texture_format(&self) -> TextureFormat {
        self.texture_format
    }

    pub fn metrics(&self) -> PreviewRendererMetrics {
        self.metrics
    }

    pub fn identity(&self) -> Option<FrameIdentity> {
        self.identity
    }

    pub fn has_frame_for(&self, frame: &DecodedFrame) -> bool {
        self.texture.is_some()
            && self.bind_group.is_some()
            && self.identity == Some(FrameIdentity::from_decoded(frame))
    }

    /// Upload the current decoded frame. Call only for the current accepted
    /// frame; stale generations are rejected defensively.
    pub fn upload_frame(
        &mut self,
        frame: &DecodedFrame,
    ) -> Result<PreviewUpload, PreviewRenderError> {
        let result = self.upload_frame_inner(frame);
        if result.is_err() {
            self.metrics.upload_rejections = self.metrics.upload_rejections.saturating_add(1);
        }
        result
    }

    fn upload_frame_inner(
        &mut self,
        frame: &DecodedFrame,
    ) -> Result<PreviewUpload, PreviewRenderError> {
        validate_frame(frame)?;
        let identity = FrameIdentity::from_decoded(frame);
        if self.identity == Some(identity) && self.texture.is_some() {
            self.metrics.uploads_skipped = self.metrics.uploads_skipped.saturating_add(1);
            return Ok(PreviewUpload::SkippedSameFrame);
        }
        if let Some(current) = self.identity {
            if current.source_id == identity.source_id && current.generation > identity.generation {
                return Err(PreviewRenderError::StaleGeneration {
                    current: current.generation,
                    received: identity.generation,
                });
            }
        }

        let size = (frame.width, frame.height);
        let needs_allocation = self.size != Some(size) || self.texture.is_none();
        if needs_allocation {
            self.allocate_texture(size.0, size.1)?;
            self.metrics.reallocation_count = self.metrics.reallocation_count.saturating_add(1);
            self.metrics.gpu_texture_bytes = u64::from(frame.width)
                .saturating_mul(u64::from(frame.height))
                .saturating_mul(4);
            self.identity = None;
        } else {
            self.metrics.texture_reuse_count = self.metrics.texture_reuse_count.saturating_add(1);
        }

        let Some(texture) = self.texture.as_ref() else {
            return Err(PreviewRenderError::ResourceUnavailable);
        };
        let started = Instant::now();
        self.queue.write_texture(
            TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &frame.rgba,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(frame.width.saturating_mul(4)),
                rows_per_image: Some(frame.height),
            },
            Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );
        self.metrics.frame_upload_cpu_ms = Some(started.elapsed().as_secs_f64() * 1_000.0);
        self.metrics.cpu_staging_bytes = frame.rgba.len() as u64;
        self.metrics.peak_cpu_staging_bytes = self
            .metrics
            .peak_cpu_staging_bytes
            .max(self.metrics.cpu_staging_bytes);
        self.metrics.peak_gpu_texture_bytes = self
            .metrics
            .peak_gpu_texture_bytes
            .max(self.metrics.gpu_texture_bytes);
        self.metrics.upload_count = self.metrics.upload_count.saturating_add(1);
        self.metrics.gpu_upload_completion_ms = None;
        self.metrics.preview_present_ms = None;
        self.identity = Some(identity);
        Ok(PreviewUpload::Uploaded)
    }

    /// Draw the current frame into a caller-owned render pass. In the app this
    /// is the pass egui-wgpu already owns; tests may pass an offscreen pass.
    /// Recording commands is CPU work only, so no GPU completion is awaited.
    pub fn paint(&mut self, render_pass: &mut RenderPass<'_>) -> bool {
        let (Some(bind_group), Some(_)) = (self.bind_group.as_ref(), self.identity) else {
            return false;
        };
        let started = Instant::now();
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        self.metrics.renderer_submit_ms = Some(started.elapsed().as_secs_f64() * 1_000.0);
        self.metrics.gpu_upload_completion_ms = None;
        self.metrics.preview_present_ms = None;
        true
    }

    pub fn clear(&mut self) {
        self.bind_group = None;
        self._view = None;
        self.texture = None;
        self.size = None;
        self.identity = None;
        self.metrics.gpu_texture_bytes = 0;
        self.metrics.cpu_staging_bytes = 0;
    }

    fn allocate_texture(&mut self, width: u32, height: u32) -> Result<(), PreviewRenderError> {
        let started = Instant::now();
        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("cutcraft-preview-frame"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.texture_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("cutcraft-preview-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.texture = Some(texture);
        self._view = Some(view);
        self.bind_group = Some(bind_group);
        self.size = Some((width, height));
        self.metrics.texture_allocation_cpu_ms = Some(started.elapsed().as_secs_f64() * 1_000.0);
        Ok(())
    }
}

fn preview_texture_format(target_format: TextureFormat) -> TextureFormat {
    if target_format.is_srgb() {
        TextureFormat::Rgba8UnormSrgb
    } else {
        TextureFormat::Rgba8Unorm
    }
}

/// Validate a decoded frame before any texture allocation or upload.
pub fn validate_frame(frame: &DecodedFrame) -> Result<(), PreviewRenderError> {
    if frame.format != PixelFormat::Rgba8 {
        return Err(PreviewRenderError::UnsupportedFormat);
    }
    if frame.width == 0
        || frame.height == 0
        || frame.width > MAX_PREVIEW_DIMENSION
        || frame.height > MAX_PREVIEW_DIMENSION
    {
        return Err(PreviewRenderError::InvalidDimensions {
            width: frame.width,
            height: frame.height,
        });
    }
    let expected = u64::from(frame.width)
        .checked_mul(u64::from(frame.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(PreviewRenderError::Oversized {
            bytes: u64::MAX,
            limit: MAX_PREVIEW_TEXTURE_BYTES,
        })?;
    if expected > MAX_PREVIEW_TEXTURE_BYTES {
        return Err(PreviewRenderError::Oversized {
            bytes: expected,
            limit: MAX_PREVIEW_TEXTURE_BYTES,
        });
    }
    let expected = usize::try_from(expected).map_err(|_| PreviewRenderError::Oversized {
        bytes: expected,
        limit: MAX_PREVIEW_TEXTURE_BYTES,
    })?;
    if frame.rgba.len() != expected {
        return Err(PreviewRenderError::TruncatedBuffer {
            expected,
            actual: frame.rgba.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use capcut_media::{MediaTimebase, MediaTimestamp};

    use super::*;

    fn frame(generation: u64) -> DecodedFrame {
        DecodedFrame {
            source_id: 7,
            generation,
            clip_tag: 3,
            asset_tag: 2,
            width: 2,
            height: 2,
            format: PixelFormat::Rgba8,
            rgba: vec![255; 16],
            pts: MediaTimestamp::new(42, MediaTimebase::new(1, 30).expect("timebase")),
            requested_target: MediaTimestamp::new(42, MediaTimebase::new(1, 30).expect("timebase")),
            decode_elapsed_ms: 1,
        }
    }

    #[test]
    fn frame_identity_includes_pts_dimensions_and_clip_asset_tags() {
        let identity = FrameIdentity::from_decoded(&frame(9));
        assert_eq!(identity.source_id, 7);
        assert_eq!(identity.generation, 9);
        assert_eq!(identity.clip_id, 3);
        assert_eq!(identity.asset_id, 2);
        assert_eq!(identity.pts_ticks, 42);
        assert_eq!(identity.pts_timebase_numerator, 1);
        assert_eq!(identity.pts_timebase_denominator, 30);
        assert_eq!((identity.width, identity.height), (2, 2));
    }

    #[test]
    fn validation_rejects_bad_dimensions_format_oversize_and_truncated_buffers() {
        let mut bad = frame(1);
        bad.width = 0;
        assert!(matches!(
            validate_frame(&bad),
            Err(PreviewRenderError::InvalidDimensions { .. })
        ));
        let mut bad = frame(1);
        bad.rgba.truncate(4);
        assert!(matches!(
            validate_frame(&bad),
            Err(PreviewRenderError::TruncatedBuffer { .. })
        ));
        let mut bad = frame(1);
        bad.width = MAX_PREVIEW_DIMENSION + 1;
        assert!(matches!(
            validate_frame(&bad),
            Err(PreviewRenderError::InvalidDimensions { .. })
        ));
        assert!(validate_frame(&frame(1)).is_ok());
    }

    #[test]
    fn texture_format_tracks_the_target_color_space() {
        assert_eq!(
            preview_texture_format(TextureFormat::Bgra8Unorm),
            TextureFormat::Rgba8Unorm
        );
        assert_eq!(
            preview_texture_format(TextureFormat::Bgra8UnormSrgb),
            TextureFormat::Rgba8UnormSrgb
        );
    }

    #[test]
    fn upload_metrics_leave_gpu_completion_and_present_unmeasured() {
        let metrics = PreviewRendererMetrics::default();
        assert!(metrics.gpu_upload_completion_ms.is_none());
        assert!(metrics.preview_present_ms.is_none());
    }
}
