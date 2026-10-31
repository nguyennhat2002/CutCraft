//! Device-backed Preview colour, stride and lifetime checks.
//!
//! These tests create their own headless wgpu device so they can render through
//! the real `PreviewRenderer` pipeline and read the *presented* pixels back.
//! The application never does this: it reuses eframe's device/queue, and that is
//! verified by `crates/renderer/src` containing no `Instance::new` call.
//!
//! Skipped (not failed) when no adapter is available, so a headless CI run does
//! not silently become colour proof.
//!
//! Reference policy being asserted here:
//! - On the non-sRGB target that QA actually uses (`Bgra8Unorm`), the pipeline is
//!   a byte-exact passthrough for opaque RGBA8 input: no transfer-function decode
//!   or encode happens.
//! - `Bgra8Unorm` memory order is B,G,R,A, so readback must swizzle before
//!   comparison; that layout is itself asserted below.
//! - Straight (non-premultiplied) alpha composites over the existing target.
//! - Tolerances: 0 for opaque passthrough, 1 where a float clear value crosses a
//!   rounding boundary, 3 for blended results, 2 for the sRGB round trip.

use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use capcut_media::{DecodedFrame, MediaTimebase, MediaTimestamp, PixelFormat};
use capcut_renderer::{PreviewRenderError, PreviewRenderer, PreviewUpload};
use wgpu::{
    BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor, Extent3d, LoadOp, Operations,
    Origin3d, PollType, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, StoreOp, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

/// Surface format observed on the Windows QA machine (eframe + Vulkan).
const QA_TARGET: TextureFormat = TextureFormat::Bgra8Unorm;
/// Alternate target so the sRGB branch is exercised on real hardware too.
const SRGB_TARGET: TextureFormat = TextureFormat::Bgra8UnormSrgb;

/// Widths used for multi-row readback must keep `width * 4` a multiple of 256.
const W: u32 = 64;
const W_DOUBLE: u32 = 128;
/// Blending happens in floating point, so allow a small band.
const BLEND_TOLERANCE: i32 = 3;

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
        std::thread::yield_now();
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: Queue,
    target: TextureFormat,
}

impl Gpu {
    /// `None` when this machine cannot provide a device for `target`, which is a
    /// skip rather than a failure so headless CI stays honest.
    fn open(target: TextureFormat) -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&RequestAdapterOptions::default()))
            .map_err(|error| eprintln!("SKIP preview_gpu: no adapter: {error}"))
            .ok()?;
        let features = adapter.get_texture_format_features(target);
        if !features
            .allowed_usages
            .contains(TextureUsages::RENDER_ATTACHMENT)
        {
            eprintln!("SKIP preview_gpu: target {target:?} is not renderable here");
            return None;
        }
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("cutcraft-preview-test-device"),
            ..Default::default()
        }))
        .map_err(|error| eprintln!("SKIP preview_gpu: no device: {error}"))
        .ok()?;
        Some(Self {
            device,
            queue,
            target,
        })
    }

    fn renderer(&self) -> PreviewRenderer {
        PreviewRenderer::new(&self.device, &self.queue, self.target)
            .expect("preview renderer on the test device")
    }

    fn make_target(
        &self,
        width: u32,
        height: u32,
        label: &'static str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.target,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    fn begin_pass<'encoder>(
        &self,
        encoder: &'encoder mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        label: &'static str,
        clear: [f32; 4],
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: f64::from(clear[0]),
                        g: f64::from(clear[1]),
                        b: f64::from(clear[2]),
                        a: f64::from(clear[3]),
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
    }

    /// Render the current GPU frame into a fresh target and return the raw bytes
    /// in target memory order.
    fn read_back_raw(
        &self,
        renderer: &mut PreviewRenderer,
        width: u32,
        height: u32,
        clear: [f32; 4],
    ) -> Vec<u8> {
        let stride = u64::from(width) * 4;
        assert_eq!(
            stride % 256,
            0,
            "multi-row readback needs a 256-byte aligned stride"
        );
        let (texture, view) = self.make_target(width, height, "preview-test-target");
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("preview-test-encoder"),
            });
        {
            let mut pass = self.begin_pass(&mut encoder, &view, "preview-test-pass", clear);
            assert!(renderer.paint(&mut pass), "renderer painted a frame");
        }
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("preview-test-readback"),
            size: stride * u64::from(height),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(u32::try_from(stride).expect("stride")),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let index = self.queue.submit(Some(encoder.finish()));
        read_buffer(&self.device, &buffer, index)
    }

    /// Render and return pixels normalised to RGBA regardless of target order.
    fn read_back(
        &self,
        renderer: &mut PreviewRenderer,
        width: u32,
        height: u32,
        clear: [f32; 4],
    ) -> Vec<u8> {
        swizzle_to_rgba(
            &self.read_back_raw(renderer, width, height, clear),
            usize::try_from(width * height).expect("pixels"),
            self.target,
        )
    }

    /// Read back a frame whose rows are not 256-byte aligned by copying one row
    /// at a time, which is how a real unaligned stride has to be resolved.
    fn read_back_unaligned_rows(
        &self,
        renderer: &mut PreviewRenderer,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let row_bytes = usize::try_from(width)
            .expect("width")
            .checked_mul(4)
            .expect("row bytes");
        let padded = 256usize;
        let (texture, view) = self.make_target(width, height, "preview-test-target-unaligned");
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("preview-test-encoder-unaligned"),
            });
        {
            let mut pass =
                self.begin_pass(&mut encoder, &view, "preview-test-pass-unaligned", [0.0; 4]);
            assert!(renderer.paint(&mut pass));
        }
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("preview-test-readback-unaligned"),
            size: u64::from(height) * padded as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        for row in 0..height {
            encoder.copy_texture_to_buffer(
                TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d { x: 0, y: row, z: 0 },
                    aspect: TextureAspect::All,
                },
                TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: TexelCopyBufferLayout {
                        offset: u64::from(row) * padded as u64,
                        bytes_per_row: None,
                        rows_per_image: None,
                    },
                },
                Extent3d {
                    width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
        let index = self.queue.submit(Some(encoder.finish()));
        let raw = read_buffer(&self.device, &buffer, index);
        let mut out = vec![0u8; row_bytes * usize::try_from(height).expect("height")];
        for row in 0..height as usize {
            let source = row * padded..row * padded + row_bytes;
            out[row * row_bytes..(row + 1) * row_bytes].copy_from_slice(&raw[source]);
        }
        swizzle_to_rgba(
            &out,
            usize::try_from(width * height).expect("pixels"),
            self.target,
        )
    }

    /// Read one normalised RGBA pixel out of a buffer produced by `read_back`.
    fn pixel(&self, data: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = (usize::try_from(y).expect("y") * usize::try_from(width).expect("width")
            + usize::try_from(x).expect("x"))
            * 4;
        [
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]
    }
}

fn read_buffer(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    index: wgpu::SubmissionIndex,
) -> Vec<u8> {
    let size = buffer.slice(..).size();
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, 0..size, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(PollType::Wait {
            submission_index: Some(index),
            timeout: Some(Duration::from_secs(20)),
        })
        .expect("test device poll");
    receiver
        .recv()
        .expect("map callback")
        .expect("buffer maps for reading");
    let data = buffer
        .get_mapped_range(..)
        .expect("mapped readback range")
        .to_vec();
    buffer.unmap();
    data
}

/// `Bgra8Unorm` keeps channels in memory order B,G,R,A, so a raw readback is not
/// directly comparable with the RGBA8 frames the renderer was given.
fn swizzle_to_rgba(raw: &[u8], pixels: usize, target: TextureFormat) -> Vec<u8> {
    let bgra = matches!(
        target,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
    );
    let mut out = vec![0u8; pixels * 4];
    for index in 0..pixels {
        let source = &raw[index * 4..index * 4 + 4];
        let rgba = if bgra {
            [source[2], source[1], source[0], source[3]]
        } else {
            [source[0], source[1], source[2], source[3]]
        };
        out[index * 4..index * 4 + 4].copy_from_slice(&rgba);
    }
    out
}

fn frame(
    source_id: u64,
    generation: u64,
    clip_tag: u64,
    asset_tag: u64,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> DecodedFrame {
    let timebase = MediaTimebase::new(1, 15_360).expect("timebase");
    DecodedFrame {
        source_id,
        generation,
        clip_tag,
        asset_tag,
        width,
        height,
        format: PixelFormat::Rgba8,
        rgba: rgba.to_vec(),
        pts: MediaTimestamp::new(1_536, timebase),
        requested_target: MediaTimestamp::new(1_536, timebase),
        decode_elapsed_ms: 1,
    }
}

fn solid(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(usize::try_from(width * height * 4).expect("pixels"));
    for _ in 0..width * height {
        pixels.extend_from_slice(&color);
    }
    pixels
}

fn expect_rgba(actual: [u8; 4], expected: [u8; 4], tolerance: i32, label: &str) {
    for channel in 0..3 {
        let delta = i32::from(actual[channel]) - i32::from(expected[channel]);
        assert!(
            delta.abs() <= tolerance,
            "{label}: channel {channel} is {}, expected {} within {tolerance} ({actual:?} vs {expected:?})",
            actual[channel],
            expected[channel]
        );
    }
}

/// Straight (non-premultiplied) alpha composites over the destination.
fn blend_over(src: [u8; 4], dst: [u8; 4]) -> [u8; 4] {
    let alpha = f64::from(src[3]) / 255.0;
    let channel = |index: usize| {
        let value = f64::from(src[index]) * alpha + f64::from(dst[index]) * (1.0 - alpha);
        u8::try_from(value.round() as i64).expect("blended channel")
    };
    [channel(0), channel(1), channel(2), 255]
}

/// The presentation contract depends on the target's memory order. Asserting it
/// here means a future surface-format change fails loudly instead of silently
/// inverting every colour test above.
#[test]
fn bgra_target_presents_rgba_input_in_bgra_memory_order() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP bgra_target_presents_rgba_input_in_bgra_memory_order: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let source = frame(9, 1, 1, 1, &solid(W, W, [255, 0, 0, 255]), W, W);
    renderer.upload_frame(&source).expect("upload red");
    let raw = gpu.read_back_raw(&mut renderer, W, W, [0.0; 4]);
    assert_eq!(
        gpu.pixel(&raw, W, 32, 32),
        [0, 0, 255, 255],
        "Bgra8Unorm stores blue first"
    );
    let swizzled = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    assert_eq!(
        gpu.pixel(&swizzled, W, 32, 32),
        [255, 0, 0, 255],
        "after swizzling, red is red"
    );
}

/// Opaque frames on the QA target must be byte-exact: nothing in this path
/// encodes or decodes a transfer function and alpha is 255.
#[test]
fn opaque_frames_are_byte_exact_on_the_qa_target() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP opaque_frames_are_byte_exact_on_the_qa_target: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    assert_eq!(renderer.texture_format(), TextureFormat::Rgba8Unorm);
    for (index, color) in [
        [255u8, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [128, 128, 128, 255],
        [214, 86, 32, 255],
        [1, 2, 3, 255],
        [0, 0, 0, 255],
        [255, 255, 255, 255],
    ]
    .into_iter()
    .enumerate()
    {
        let generation = u64::try_from(index).expect("index") + 1;
        let source = frame(10, generation, 1, 1, &solid(W, W, color), W, W);
        renderer.upload_frame(&source).expect("upload fixture");
        let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
        for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63), (32, 32)] {
            expect_rgba(
                gpu.pixel(&rendered, W, x, y),
                color,
                0,
                &format!("fixture {index} at {x},{y}"),
            );
        }
        assert_eq!(
            gpu.pixel(&rendered, W, 32, 32)[3],
            255,
            "opaque fixtures keep full alpha in the target"
        );
    }
}

/// Per-pixel ramps must keep their row/column identity, which is what catches a
/// swapped or mis-strided upload.
#[test]
fn gradient_frames_keep_per_pixel_identity() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP gradient_frames_keep_per_pixel_identity: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let mut pixels = Vec::new();
    for y in 0..W {
        for x in 0..W {
            pixels.push(u8::try_from(x * 4).expect("ramp"));
            pixels.push(u8::try_from(y * 4).expect("ramp"));
            pixels.push(128);
            pixels.push(255);
        }
    }
    let source = frame(11, 1, 1, 1, &pixels, W, W);
    renderer.upload_frame(&source).expect("upload gradient");
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    for (x, y) in [(0, 0), (16, 40), (40, 16), (63, 63), (31, 32)] {
        expect_rgba(
            gpu.pixel(&rendered, W, x, y),
            [
                u8::try_from(x * 4).expect("ramp"),
                u8::try_from(y * 4).expect("ramp"),
                128,
                255,
            ],
            0,
            &format!("gradient at {x},{y}"),
        );
    }
}

/// Straight alpha composites over whatever the target already holds, which is
/// how the callback behaves inside egui's pass.
#[test]
fn straight_alpha_blends_over_the_existing_target() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP straight_alpha_blends_over_the_existing_target: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let mut pixels = Vec::new();
    for _ in 0..W {
        for x in 0..W {
            let alpha = if x < 16 {
                0
            } else if x < 32 {
                128
            } else if x < 48 {
                64
            } else {
                255
            };
            pixels.extend_from_slice(&[255, 0, 0, alpha]);
        }
    }
    let source = frame(12, 1, 1, 1, &pixels, W, W);
    renderer
        .upload_frame(&source)
        .expect("upload alpha fixture");
    let rendered = gpu.read_back(&mut renderer, W, W, [0.25, 0.5, 0.75, 1.0]);
    // The alpha-0 band proves what the clear actually wrote, so every blend
    // expectation uses the measured background rather than an assumed one.
    let background = gpu.pixel(&rendered, W, 8, 32);
    expect_rgba(
        background,
        [64, 128, 192, 255],
        1,
        "alpha 0 leaves the target as cleared",
    );
    for (x, alpha) in [(24u32, 128u8), (40u32, 64u8)] {
        expect_rgba(
            gpu.pixel(&rendered, W, x, 32),
            blend_over([255, 0, 0, alpha], background),
            BLEND_TOLERANCE,
            &format!("alpha {alpha} blends red over the background"),
        );
    }
    expect_rgba(
        gpu.pixel(&rendered, W, 56, 32),
        [255, 0, 0, 255],
        1,
        "alpha 255 is opaque",
    );
}

/// A row stride that is not a multiple of 256 must still land in the right
/// columns; this is the 360-wide portrait class that centre-pixel sampling
/// cannot detect.
#[test]
fn unaligned_row_stride_is_resolved_without_row_shift() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP unaligned_row_stride_is_resolved_without_row_shift: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let width = 60u32; // 240 bytes per row: deliberately unaligned.
    let height = 4u32;
    let mut pixels = Vec::new();
    for row in 0..height {
        let shade = u8::try_from(30 + row * 60).expect("shade");
        for _ in 0..width {
            pixels.extend_from_slice(&[shade, 255 - shade, shade, 255]);
        }
    }
    let source = frame(13, 1, 1, 1, &pixels, width, height);
    renderer
        .upload_frame(&source)
        .expect("upload unaligned frame");
    let rendered = gpu.read_back_unaligned_rows(&mut renderer, width, height);
    for row in 0..height {
        let shade = u8::try_from(30 + row * 60).expect("shade");
        for x in [0u32, 1, 29, 58, 59] {
            expect_rgba(
                gpu.pixel(&rendered, width, x, row),
                [shade, 255 - shade, shade, 255],
                1,
                &format!("unaligned row {row} column {x}"),
            );
        }
    }
}

/// Portrait and landscape frames must not transpose: markers sit in a specific
/// row and column.
#[test]
fn portrait_and_landscape_keep_orientation_of_uploaded_pixels() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP portrait_and_landscape_keep_orientation_of_uploaded_pixels: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    for (label, width, height) in [
        ("portrait", W, W_DOUBLE),
        ("landscape", W_DOUBLE, W),
        ("square", W, W),
    ] {
        let mut pixels = solid(width, height, [0, 0, 0, 255]);
        let bottom_left =
            (usize::try_from(height - 2).expect("row") * usize::try_from(width).expect("w") + 2)
                * 4;
        pixels[bottom_left..bottom_left + 4].copy_from_slice(&[0, 255, 0, 255]);
        let top_right = usize::try_from(width - 3).expect("column") * 4;
        pixels[top_right..top_right + 4].copy_from_slice(&[255, 255, 0, 255]);
        let source = frame(14, 1, 1, 1, &pixels, width, height);
        renderer
            .upload_frame(&source)
            .expect("upload oriented frame");
        let rendered = gpu.read_back(&mut renderer, width, height, [0.0; 4]);
        expect_rgba(
            gpu.pixel(&rendered, width, 2, height - 2),
            [0, 255, 0, 255],
            0,
            &format!("{label} bottom-left marker"),
        );
        expect_rgba(
            gpu.pixel(&rendered, width, width - 3, 0),
            [255, 255, 0, 255],
            0,
            &format!("{label} top-right marker"),
        );
        expect_rgba(
            gpu.pixel(&rendered, width, width / 2, height / 2),
            [0, 0, 0, 255],
            0,
            &format!("{label} background"),
        );
    }
}

/// A resolution change reallocates, and the new frame must fully replace the old
/// one — no stale pixels anywhere in the target.
#[test]
fn resolution_change_reallocates_and_replaces_every_pixel() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP resolution_change_reallocates_and_replaces_every_pixel: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let small = frame(15, 1, 1, 1, &solid(W, W, [255, 0, 0, 255]), W, W);
    renderer.upload_frame(&small).expect("upload 64x64");
    let large = frame(
        15,
        2,
        1,
        1,
        &solid(W_DOUBLE, W, [0, 0, 255, 255]),
        W_DOUBLE,
        W,
    );
    renderer.upload_frame(&large).expect("upload 128x64");
    let rendered = gpu.read_back(&mut renderer, W_DOUBLE, W, [0.0; 4]);
    for (x, y) in [(0, 0), (127, 0), (0, 63), (127, 63), (64, 32)] {
        expect_rgba(
            gpu.pixel(&rendered, W_DOUBLE, x, y),
            [0, 0, 255, 255],
            0,
            &format!("reallocation left no red at {x},{y}"),
        );
    }
    assert_eq!(
        renderer.metrics().reallocation_count,
        2,
        "both sizes allocated"
    );

    // Same size again exercises the reuse path.
    let again = frame(
        15,
        3,
        1,
        1,
        &solid(W_DOUBLE, W, [0, 255, 0, 255]),
        W_DOUBLE,
        W,
    );
    renderer.upload_frame(&again).expect("reuse upload");
    let metrics = renderer.metrics();
    assert_eq!(metrics.texture_reuse_count, 1, "same size reuses");
    assert_eq!(metrics.reallocation_count, 2, "reuse does not allocate");
    assert_eq!(metrics.upload_count, 3, "three accepted uploads");
    assert_eq!(
        metrics.peak_gpu_texture_bytes,
        u64::from(W_DOUBLE * W * 4),
        "peak tracks the largest live texture"
    );
    assert_eq!(
        metrics.peak_cpu_staging_bytes,
        u64::from(W_DOUBLE * W * 4),
        "peak staging tracks the largest upload"
    );

    // A solid colour survives both up- and down-scaling of the viewport.
    for (label, width, height) in [("downscale", W, W), ("upscale", 128u32, 128u32)] {
        let scaled = gpu.read_back(&mut renderer, width, height, [0.0; 4]);
        expect_rgba(
            gpu.pixel(&scaled, width, width / 2, height / 2),
            [0, 255, 0, 255],
            0,
            &format!("solid colour survives {label}"),
        );
    }
}

/// Identity/lifetime: a newer generation wins, an older one is rejected, and a
/// different source with the same numeric PTS is treated as a new frame.
#[test]
fn generation_source_and_clip_identity_controls_replacement() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP generation_source_and_clip_identity_controls_replacement: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let red = frame(20, 5, 1, 1, &solid(W, W, [255, 0, 0, 255]), W, W);
    let blue = frame(20, 6, 1, 1, &solid(W, W, [0, 0, 255, 255]), W, W);
    let stale = frame(20, 4, 1, 1, &solid(W, W, [0, 255, 0, 255]), W, W);
    renderer.upload_frame(&red).expect("generation 5 accepted");
    renderer.upload_frame(&blue).expect("generation 6 accepted");
    assert!(
        matches!(
            renderer.upload_frame(&stale),
            Err(PreviewRenderError::StaleGeneration {
                current: 6,
                received: 4
            })
        ),
        "an older generation for the same source is rejected"
    );
    assert_eq!(renderer.metrics().upload_rejections, 1);
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    expect_rgba(
        gpu.pixel(&rendered, W, 32, 32),
        [0, 0, 255, 255],
        0,
        "the rejected frame never reached the target",
    );

    // Same PTS, new source id: must upload rather than look "already present".
    let other_source = frame(21, 1, 1, 1, &solid(W, W, [255, 255, 0, 255]), W, W);
    assert!(!renderer.has_frame_for(&other_source));
    renderer
        .upload_frame(&other_source)
        .expect("new source accepted");
    assert!(renderer.has_frame_for(&other_source));
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    expect_rgba(
        gpu.pixel(&rendered, W, 32, 32),
        [255, 255, 0, 255],
        0,
        "new source replaced the old frame",
    );

    // Same source/generation but a different clip tag is a different identity.
    let other_clip = frame(21, 2, 9, 1, &solid(W, W, [10, 200, 30, 255]), W, W);
    renderer
        .upload_frame(&other_clip)
        .expect("new clip identity accepted");
    assert_eq!(renderer.identity().expect("identity").clip_id, 9);

    // A second clip sharing the same asset tag is still distinct.
    let same_asset_other_clip = frame(21, 3, 10, 1, &solid(W, W, [99, 99, 99, 255]), W, W);
    renderer
        .upload_frame(&same_asset_other_clip)
        .expect("second clip of the same asset");

    // Repeating the identical frame is a skip, not an upload.
    assert_eq!(
        renderer.upload_frame(&same_asset_other_clip),
        Ok(PreviewUpload::SkippedSameFrame)
    );
    assert_eq!(renderer.metrics().uploads_skipped, 1);
    assert_eq!(renderer.metrics().upload_count, 5);
    assert_eq!(
        renderer.metrics().reallocation_count,
        1,
        "one size throughout"
    );
}

/// Cache contract, stated explicitly: identity is the key, so a repeat of the
/// same source/generation/clip/asset/PTS/size is not re-uploaded. Decoded pixels
/// for a given PTS are deterministic, which is what makes this safe; a caller
/// that changes pixel content must change the generation.
#[test]
fn identical_identity_skips_the_upload_and_keeps_the_presented_frame() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP identical_identity_skips_the_upload_and_keeps_the_presented_frame: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let first = frame(55, 1, 1, 1, &solid(W, W, [12, 34, 56, 255]), W, W);
    assert_eq!(
        renderer.upload_frame(&first),
        Ok(PreviewUpload::Uploaded),
        "the first upload reaches the GPU"
    );
    let upload_ms = renderer.metrics().frame_upload_cpu_ms;
    let mut same_identity_other_pixels = first.clone();
    same_identity_other_pixels.rgba = solid(W, W, [200, 200, 200, 255]);
    assert_eq!(
        renderer.upload_frame(&same_identity_other_pixels),
        Ok(PreviewUpload::SkippedSameFrame),
        "the same identity is a skip"
    );
    assert_eq!(
        renderer.metrics().frame_upload_cpu_ms,
        upload_ms,
        "a skip does not re-stage bytes"
    );
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    expect_rgba(
        gpu.pixel(&rendered, W, 32, 32),
        [12, 34, 56, 255],
        0,
        "the skipped payload is deliberately not presented",
    );
    assert_eq!(renderer.metrics().upload_count, 1);
    assert_eq!(renderer.metrics().uploads_skipped, 1);

    // Bumping the generation is how a changed payload gets through.
    let mut newer = same_identity_other_pixels.clone();
    newer.generation = 2;
    renderer
        .upload_frame(&newer)
        .expect("new generation uploads");
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    expect_rgba(
        gpu.pixel(&rendered, W, 32, 32),
        [200, 200, 200, 255],
        0,
        "a newer generation replaces the texture content",
    );
}

/// Two projects can reuse numeric clip/asset ids; the source id keeps their
/// frames from being confused, and clearing removes everything.
#[test]
fn overlapping_clip_and_asset_ids_across_projects_stay_distinct() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP overlapping_clip_and_asset_ids_across_projects_stay_distinct: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let project_a = frame(30, 1, 1, 1, &solid(W, W, [200, 10, 10, 255]), W, W);
    let project_b = frame(31, 1, 1, 1, &solid(W, W, [10, 10, 200, 255]), W, W);
    renderer
        .upload_frame(&project_a)
        .expect("project A uploads");
    renderer
        .upload_frame(&project_b)
        .expect("project B uploads with identical tags");
    let identity = renderer.identity().expect("identity");
    assert_eq!(
        (identity.source_id, identity.clip_id, identity.asset_id),
        (31, 1, 1)
    );
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    expect_rgba(
        gpu.pixel(&rendered, W, 32, 32),
        [10, 10, 200, 255],
        0,
        "project B frame is presented",
    );

    // Leaving the Editor clears the renderer, so nothing from either project
    // survives, and a repaint is refused.
    renderer.clear();
    assert_eq!(renderer.identity(), None);
    let mut encoder = gpu
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("preview-test-empty"),
        });
    let (_, view) = gpu.make_target(W, W, "preview-test-target-cleared");
    {
        let mut pass = gpu.begin_pass(&mut encoder, &view, "preview-test-cleared-pass", [0.0; 4]);
        assert!(
            !renderer.paint(&mut pass),
            "a cleared renderer draws nothing"
        );
    }
    drop(encoder);
    assert_eq!(
        renderer.metrics().gpu_texture_bytes,
        0,
        "clear releases bytes"
    );
    assert_eq!(
        renderer.metrics().peak_gpu_texture_bytes,
        u64::from(W * W * 4),
        "clear keeps the historical peak for reporting"
    );

    // Reopening a project accepts its frame again even though the generation is
    // not newer than the one just dropped.
    renderer
        .upload_frame(&project_a)
        .expect("first project reopens after clear");
    assert_eq!(renderer.identity().expect("identity").source_id, 30);
}

/// Rejections happen before any GPU work, and the renderer stays usable.
#[test]
fn invalid_frames_are_rejected_without_allocating() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP invalid_frames_are_rejected_without_allocating: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let truncated = frame(40, 1, 1, 1, &[0u8; 64], W, W);
    assert!(matches!(
        renderer.upload_frame(&truncated),
        Err(PreviewRenderError::TruncatedBuffer {
            expected: 16_384,
            actual: 64
        })
    ));
    let mut zero = frame(40, 1, 1, 1, &solid(1, 1, [0, 0, 0, 255]), 1, 1);
    zero.width = 0;
    zero.height = 0;
    zero.rgba.clear();
    assert!(matches!(
        renderer.upload_frame(&zero),
        Err(PreviewRenderError::InvalidDimensions { .. })
    ));
    // NOTE: `capcut_media::PixelFormat` currently has a single `Rgba8` variant,
    // so `UnsupportedFormat` cannot be constructed from the media type. It stays
    // a defensive branch until the media layer produces another format.
    let oversized = frame(
        40,
        1,
        1,
        1,
        &Vec::new(),
        capcut_renderer::MAX_PREVIEW_DIMENSION + 1,
        capcut_renderer::MAX_PREVIEW_DIMENSION + 1,
    );
    assert!(matches!(
        renderer.upload_frame(&oversized),
        Err(PreviewRenderError::InvalidDimensions { .. })
    ));
    assert_eq!(renderer.identity(), None, "nothing was accepted");
    assert_eq!(
        renderer.metrics().gpu_texture_bytes,
        0,
        "nothing was allocated"
    );
    assert_eq!(renderer.metrics().upload_rejections, 3);

    // A later valid frame still works, which is the recovery path the UI relies
    // on after it clears the renderer following an error.
    let good = frame(40, 2, 1, 1, &solid(W, W, [9, 9, 9, 255]), W, W);
    renderer
        .upload_frame(&good)
        .expect("valid frame after rejections");
    let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    expect_rgba(
        gpu.pixel(&rendered, W, 32, 32),
        [9, 9, 9, 255],
        0,
        "recovered",
    );
}

/// A frame that has never been uploaded draws nothing.
#[test]
fn paint_without_a_frame_is_a_no_op() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP paint_without_a_frame_is_a_no_op: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let mut encoder = gpu
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("preview-test-no-frame"),
        });
    let (_, view) = gpu.make_target(W, W, "preview-test-target-no-frame");
    {
        let mut pass = gpu.begin_pass(&mut encoder, &view, "preview-test-no-frame-pass", [0.0; 4]);
        assert!(
            !renderer.paint(&mut pass),
            "an empty renderer records no draw"
        );
    }
    assert!(
        renderer.metrics().renderer_submit_ms.is_none(),
        "a refused paint records no submit time"
    );
}

/// The sRGB branch is only reachable when eframe picks an sRGB target, so run it
/// on real hardware rather than asserting the format table alone.
#[test]
fn srgb_target_round_trips_opaque_colours() {
    let Some(gpu) = Gpu::open(SRGB_TARGET) else {
        eprintln!("SKIP srgb_target_round_trips_opaque_colours: no sRGB target here");
        return;
    };
    let mut renderer = gpu.renderer();
    assert_eq!(
        renderer.texture_format(),
        TextureFormat::Rgba8UnormSrgb,
        "the sRGB target selects an sRGB texture"
    );
    for (index, color) in [
        [255u8, 0, 0, 255],
        [0, 0, 255, 255],
        [128, 128, 128, 255],
        [17, 200, 99, 255],
        [1, 1, 1, 255],
    ]
    .into_iter()
    .enumerate()
    {
        // Each fixture needs its own identity: the renderer deliberately skips a
        // repeat of the same source/generation/clip/asset/PTS/size frame.
        let source = frame(
            50,
            u64::try_from(index).expect("index") + 1,
            1,
            1,
            &solid(W, W, color),
            W,
            W,
        );
        renderer.upload_frame(&source).expect("upload srgb fixture");
        let rendered = gpu.read_back(&mut renderer, W, W, [0.0; 4]);
        expect_rgba(
            gpu.pixel(&rendered, W, 32, 32),
            color,
            2,
            "sRGB decode + encode returns the source byte values",
        );
    }
}

/// Timing honesty: recorded numbers are CPU-side only.
#[test]
fn gpu_completion_and_present_stay_unmeasured() {
    let Some(gpu) = Gpu::open(QA_TARGET) else {
        eprintln!("SKIP gpu_completion_and_present_stay_unmeasured: no GPU");
        return;
    };
    let mut renderer = gpu.renderer();
    let source = frame(60, 1, 1, 1, &solid(W, W, [3, 4, 5, 255]), W, W);
    renderer.upload_frame(&source).expect("upload");
    gpu.read_back(&mut renderer, W, W, [0.0; 4]);
    let metrics = renderer.metrics();
    assert!(
        metrics.frame_upload_cpu_ms.is_some(),
        "CPU enqueue time is measured"
    );
    assert!(
        metrics.texture_allocation_cpu_ms.is_some(),
        "allocation latency is measured"
    );
    assert!(
        metrics.renderer_submit_ms.is_some(),
        "render-pass recording time is measured"
    );
    assert_eq!(
        metrics.gpu_upload_completion_ms, None,
        "GPU upload completion is not observable here"
    );
    assert_eq!(
        metrics.preview_present_ms, None,
        "present time is not observable here"
    );
}
