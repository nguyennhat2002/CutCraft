# Timeline Clip Rendering Specification (M1.10)

## Source and scope

- Visual reference: supplied FilmCraft timeline screenshot (external, not stored in this repository).
- Architectural reference: FilmCraft's MIT OR Apache-2.0 timeline renderer was studied at the level of title bands, thumbnail modes, visible-range culling, peak aggregation and bounded frame caches. No FilmCraft source or assets were copied.
- CutCraft changes are limited to Timeline presentation. Homepage, Preview, Inspector, Export, toolbar layout, track arrangement, empty-state behavior and engine/project data are unchanged.

## Current implementation status

- **Implemented:** rich clip painting, title bands, ellipsized labels, synthetic demo thumbnails, synthetic demo waveforms, text/effect styling, hover/selection outlines, thumbnail modes, display preferences and bounded presentation caches.
- **Not implemented:** real media thumbnail extraction, real audio decoding, background decoder jobs, real waveform peaks. Project clips therefore show an honest unavailable fallback; synthetic content is generated only in Visual Demo mode.

## Clip anatomy

- Outer clip radius: 3 px.
- Title band: 16 px, own background and text color, left-aligned label, ellipsis when narrow.
- Video body: thumbnail strip below the title band.
- Audio body: centered baseline plus min/max waveform.
- Text: warm rose styling with `Aa` identity.
- Effect: purple styling with Lucide sparkles and `fx`.
- Selected: 1 px black separation plus 2 px high-contrast inner outline.
- Hover: accent outline and pointing-hand cursor. Tooltips show the clip label.
- No trim handles are shown because trimming is not implemented.

## Thumbnail policy

- Modes: First Frame, First and Last, Continuous.
- Continuous tiles are anchored to the clip head so scrolling does not retime existing requests.
- Tile width derives from body height and 16:9 aspect; images are clipped to the media viewport without intentional stretching.
- Only tiles intersecting the visible body are requested.
- Maximum 24 tiles per clip and 48 new textures per frame.
- Clips below 12 px width or body height render a simplified shape without thumbnails.
- Cache key: media identity hash, media revision, source time (ms), requested width and height.
- Texture cache is bounded to 96 entries with FIFO eviction; peak cache is bounded to 24 media entries.
- Demo images are deterministic synthetic landscapes; project providers return Unavailable.

## Audio waveform policy

- Reusable min/max peaks, 2048 buckets over the source duration.
- Visible columns aggregate the corresponding source-time peak range.
- Source-in and speed mapping are applied before indexing peaks.
- One mesh is built per visible waveform.
- Silence draws no bars but retains the center baseline.
- Missing peaks show `Waveform unavailable`; no synthetic waveform is shown for project media.

## Time mapping

Presentation-only chain:

```text
timeline seconds
  → clip-local offset
  → source-in + offset × speed
  → clamp to presentation media duration
  → thumbnail / peak request
```

This is a display projection. Authoritative project timing remains an engine/core responsibility and must not use this float path as its edit timebase.

## Display preferences

Session-backed preferences never mutate media/project data:

- Show video thumbnails
- Thumbnail mode
- Show audio waveforms

The existing Timeline settings button opens these controls at full width. Compact density places the same controls in the More menu. Preferences survive Home/Resume and reset with a new temporary session.

## Demo isolation

`Visual Demo` expands to include:

- two adjacent video clips with synthetic thumbnail strips;
- two text clips with different widths;
- an effect clip;
- two audio clips with synthetic min/max waveforms;
- selection and unselected states driven by the real Timeline selection code.

The demo is a static borrowed fixture. It never enters session project data, and switching the display mode clears selection.

## Verification

Automated tests cover source mapping, trim/speed mapping, first/last sampling, zoom-dependent tile counts, visible culling, narrow clips, request limits, cache keys, project/demo separation, peak aggregation, silence and existing empty-timeline/selection behavior.

Manual evidence at 1600×900 (outer window 1616×939):

- `docs/ui/screenshots/m110-timeline-empty-1600x900.png`
- `docs/ui/screenshots/m110-timeline-demo-1600x900.png`
- `docs/ui/screenshots/m110-timeline-selected-1600x900.png`
- `docs/ui/screenshots/m110-timeline-zoom-in.png`
- `docs/ui/screenshots/m110-timeline-zoom-out.png`
- `docs/ui/screenshots/m110-timeline-display-menu.png`
- `docs/ui/screenshots/m110-clip-before-after.png`
- `docs/ui/screenshots/m110-video-clip-closeup.png`
- `docs/ui/screenshots/m110-audio-waveform-closeup.png`
- `docs/ui/screenshots/m110-selected-clip-closeup.png`
- `docs/ui/screenshots/m110-narrow-clip-closeup.png`

## Remaining integration dependencies

- A real media provider must decode frames off the UI thread and return ready/pending/unavailable states.
- The provider must invalidate on media replacement/revision and supply real source PTS mapping.
- Audio peaks must come from decoded audio, not the synthetic demo generator.
- Playback priority, worker limits, cancellation and cache byte budgets remain part of the future media integration milestone.
