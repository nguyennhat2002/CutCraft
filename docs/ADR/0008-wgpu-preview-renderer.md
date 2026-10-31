# ADR-0008: Reuse Eframe's WGPU Device for a Renderer-Owned Preview Pipeline

- Status: Accepted for M4-A
- Date: 2026-10-31

## Context

M3-C/M3-D display decoded RGBA frames through an egui `TextureHandle` and an egui mesh. That upload is already GPU-backed by eframe, but it is owned and scheduled by the UI texture manager, not by `capcut-renderer`. M4-A needs a clearly owned GPU presentation path for one decoded video layer without regressing PTS/clip identity, aspect/orientation, stale-result suppression, AutoSave, or recovery.

The actual application backend must be established from the manifest and runtime, not assumed from architecture prose.

### Gate 0 audit evidence

- `eframe = "0.37.0"` is used with default features.
- eframe 0.37's `default` feature list includes `wgpu` (not `glow`).
- `Cargo.lock` resolves `wgpu 30.0.1` and `egui-wgpu 0.37.0`.
- `eframe::CreationContext` exposes `wgpu_render_state: Option<egui_wgpu::RenderState>` when the wgpu feature is enabled.
- `egui_wgpu::RenderState` exposes the existing `device`, `queue`, `adapter`, `target_format`, and egui renderer. It is cloneable.
- The existing Preview uploads `ColorImage::from_rgba_unmultiplied` through `Context::load_texture`/`TextureHandle::set` and paints an egui `Mesh`; this remains the fallback.
- The decoder emits owned normalized `Rgba8` frames with source/generation/clip/asset tags, actual PTS, dimensions, and requested target. Rotation is applied by the media adapter. There is one active visual layer in M4-A.

## Decision

### 1. Reuse the existing device and queue

The renderer does **not** create a second wgpu instance, adapter, device, queue, or surface. `capcut-ui` retrieves `CreationContext.wgpu_render_state` once at app creation and passes the existing `device`, `queue`, and `target_format` to `capcut_renderer::PreviewRenderer`.

- Device/queue ownership remains with eframe/egui-wgpu.
- `PreviewRenderer` stores cloneable wgpu handles and owns only Preview-specific GPU resources.
- If `wgpu_render_state` is unavailable, renderer initialization fails closed: the existing egui texture path remains active and a visible/logged fallback diagnostic is recorded.
- `CUTCRAFT_PREVIEW_RENDERER=egui` forces the fallback for QA; `gpu` requires the renderer and reports failure if it cannot initialize.

### 2. Ownership contract

| Layer | Responsibility |
|---|---|
| `capcut-media` | Probe, demux/decode, PTS/timebase, source identity, owned CPU RGBA frame |
| `capcut-engine` | Authoritative project, clip and asset references, validated commands |
| `capcut-renderer` | Preview GPU pipeline, texture allocation/reuse, upload validation, draw resources, renderer metrics, bounded resource lifetime |
| `capcut-ui` | Preview viewport/canvas geometry, choosing the current valid frame, invoking renderer upload, implementing the egui-wgpu callback bridge, fallback UI, user-visible diagnostics |
| `capcut-project` | Durable document/AutoSave/recovery only; never stores GPU handles, textures, or frames |

No independent Timeline clock, duplicate media registry, or project persistence changes are introduced. GPU resources are transient runtime resources.

### 3. Presentation design

- One renderer-owned `wgpu::Texture` with usage `TEXTURE_BINDING | COPY_DST` and a linear clamp sampler. The texture format follows the eframe target color space: `Rgba8Unorm` for linear targets and `Rgba8UnormSrgb` for sRGB targets, so the decoded sRGB bytes are sampled and written without a double gamma step.
- One fullscreen-triangle pipeline with no vertex buffer. Fragment shader samples the frame and writes it into the egui callback viewport. Alpha uses standard source-alpha blending; M4-A decoded sources are effectively opaque.
- The UI computes the existing aspect-fitted canvas (using frame/probe aspect), passes that rect as the egui-wgpu paint callback rect; egui-wgpu sets the viewport. The Editor layout is unchanged.
- Rotation is not re-applied in the renderer: the media adapter already normalizes orientation. The renderer consumes normalized RGBA8 only; YUV, BGRA, limited/full-range reinterpretation, and color-management transforms are explicitly unsupported in M4-A.
- Texture reuse: if width/height are unchanged, reuse the texture and bind group; upload only when the frame identity changes. A dimension change reallocates the texture and bind group and increments a reallocation counter.
- Frame identity includes source ID, generation, clip tag, asset tag, actual PTS/timebase, and dimensions. The renderer rejects older generations for the same source and exposes `has_frame_for(&DecodedFrame)` so the UI never paints a stale identity.
- Clearing the renderer releases the texture/view/bind group and is called on project switch/open/close/home. The GPU frame cannot cross project sessions.

### 4. Composition boundary

M4-A supports exactly one active video layer over a blank/background Preview:

- The existing `MediaPreview` active-clip resolution and `UnsupportedComposition` state remain authoritative.
- The renderer API accepts one frame; it does not silently composite subtitles, text, effects, transitions, or overlapping video layers.
- Future contracts (not implemented): ordered layer list, text overlay pass, alpha/transform stack, effects/transition graphs, output color-space handling. These are explicit extension points, not permission to implement an effects engine.

### 5. Failure and fallback

- Invalid dimensions, wrong pixel format, truncated RGBA buffers, oversized textures, stale generations, or renderer initialization failure return typed errors; no panic.
- A GPU upload/paint failure falls back to the existing egui texture path for subsequent frames and records a diagnostic. It never mutates project state, AutoSave status, or recovery data.
- Tiny/zero canvas sizes skip drawing.
- Renderer shutdown drops Preview-specific resources; no explicit device destruction (eframe owns it).
- Bounded resources: max 8192 pixels per dimension and a 256 MiB RGBA texture budget, aligned with media decoder limits.

### 6. Metrics

`PreviewRendererMetrics` records:

- `frame_upload_cpu_ms` (CPU enqueue around `queue.write_texture`);
- `gpu_texture_bytes`, `cpu_staging_bytes`;
- `reallocation_count`, `texture_reuse_count`, `uploads_skipped`;
- `renderer_submit_ms` (CPU render-pass command recording);
- `gpu_upload_completion_ms` = N/A (not observed);
- `preview_present_ms` = N/A (not observed).

Decode/seek metrics remain in `MediaPreview`; no CPU queue submission is described as completed GPU presentation.

## Alternatives considered

- **Keep only the egui texture path**: safe and already GPU-backed, but does not establish a renderer-owned pipeline or resource/lifetime contract.
- **Create a second wgpu device and surface**: unnecessary duplicate ownership, extra initialization/failure modes, and possible cross-device texture problems.
- **Glow/OpenGL path**: not the default eframe backend; adds a parallel graphics stack with no benefit.
- **CPU compositor/readback**: would move pixels back to the CPU and undermine the GPU presentation objective.
- **Callback that just wraps `TextureHandle`**: still egui texture ownership; rejected as fake renaming.

## Consequences and limitations

- M4-A is single-frame GPU presentation, not continuous playback. There is no playback clock, frame queue, video/audio synchronization, or export path.
- Backend selection remains eframe's; the runtime adapter/backend is logged by the application. Device loss, surface loss, and GPU OOM behavior are not fully exercised in M4-A.
- GPU upload happens on the UI thread via `queue.write_texture`; the renderer records its CPU cost. There is no GPU completion/present timing, so those metrics remain N/A.
- No color-management pipeline, HDR, YUV shader, alpha-video semantics, multi-layer composition, text/effect rendering, or hardware decode.
- The egui texture fallback remains available when the renderer is unavailable or forced off.

## Validation (M4-A.1, no change to the accepted decision)

M4-A.1 measured the contract above on real hardware instead of asserting it. The
original decision is unchanged; this records what was verified and two semantics
that were previously only implied.

- **Colour on the QA target.** On the observed `Bgra8Unorm` surface, opaque
  RGBA8 fixtures are byte-exact from upload through sampling, blending and
  target write. Because that format is non-sRGB, no transfer function is applied
  anywhere in the path, so the decoded sRGB-encoded bytes are presented as-is.
- **Memory order.** `Bgra8Unorm` stores B,G,R,A, so readback must swizzle before
  comparison with the RGBA8 input. The test suite asserts that order explicitly,
  which makes a future surface-format change fail loudly instead of silently
  inverting every colour expectation.
- **sRGB branch.** On a real `Bgra8UnormSrgb` target the paired
  `Rgba8UnormSrgb` texture decodes on sampling and the target re-encodes, so the
  source byte values return within ±2. This confirms the format rule in
  section 3 rather than only the format table.
- **Alpha semantics.** The pipeline uses `BlendState::ALPHA_BLENDING`, i.e.
  straight (non-premultiplied) alpha composited over whatever the target already
  holds. Real video is opaque (the RGBA frames come from YUV, which carries no
  alpha), so this only matters for synthetic inputs; the blend result matches
  `src*a + dst*(1-a)` within ±3. Alpha is therefore not a transparency channel
  for compositing layers here, and no premultiplied-alpha input is accepted.
- **Identity is the cache key.** `FrameIdentity` (source, generation, clip,
  asset, PTS/timebase, dimensions) decides whether an upload is skipped. A frame
  with an identical identity is assumed to have identical pixels; a producer that
  changes pixel content must advance the generation. This is what makes
  "avoid reuploading unchanged frames" safe, and it is now asserted directly.
- **Bounds and rejection.** Zero/oversized dimensions, truncated buffers and
  over-limit sizes are rejected before any allocation, and the renderer stays
  usable for the next valid frame.
- **Strides.** Rows whose byte length is not a multiple of 256 (for example a
  360- or 60-pixel-wide frame) upload and present without row shift.
