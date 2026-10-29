# Reference measurements — M1.3 UI reconstruction

Measured from the reference screenshots supplied for this milestone. The reference
images remain outside the repository and are **not** copied into the product.
Only layout geometry, spacing, and sampled colors are used. CutCraft branding,
illustrations, icons, and copy remain original.

## Sources

| Screen | File | Pixel size |
|---|---|---|
| Homepage | `C:\Users\nbt04\OneDrive\Ảnh\Screenshots\Screenshot 2026-10-29 075635.png` | 1437 × 954 |
| Editor | `C:\Users\nbt04\OneDrive\Ảnh\Screenshots\Screenshot 2026-10-29 075727.png` | 1868 × 1077 |
| Export dialog | `C:\Users\nbt04\OneDrive\Ảnh\Screenshots\Screenshot 2026-10-29 075732.png` | 721 × 663 |

Method: native-scale screenshots, visual region reading, and point sampling with
Pillow (`Image.getpixel`). Coordinates use a top-left origin. Values are
approximate to the pixel and to ±2 RGB; they describe the reference, not a
contract that CutCraft must reproduce identically.

## Homepage

| Region | Reference geometry | Sampled color / note |
|---|---|---|
| Sidebar | x 0–193 (≈13.4% width) | `#00191D`–`#031416`, profile block at top |
| Main content origin | x ≈238, y ≈43 | 45 px gutter after sidebar |
| Create-project banner | x 238–1380, y 43–183 (1142 × 141) | Left `#65D7E3`, right `#19B0D1`; bright cyan gradient |
| Banner content | centered | Dark plus glyph in a dark rounded square, dark "Create project" text (~22 px) |
| Feature card row | y 203–344 (141 tall) | First card ≈379 wide; next three ≈228 wide; 20–28 px gaps |
| First feature card | teal surface | Search/prompt field, lighter teal panel |
| Other feature cards | `#161A1B`-ish surfaces | Icon + title + one-line subtitle |
| Creative-suite heading | y ≈412 | White ~14 px text |
| Creative-suite card row | y ≈441–625 | Four image-led cards, ~275 px wide |
| Projects heading | y ≈691 | Search, grid/list, trash, project-sync controls on the right |
| Empty state | y ≈783–858 | Folder outline icon, centered muted copy |
| Content background | — | `#031417` / `#051113`; sidebar darker teal |
| Card surface | — | `#122224` |
| Accent | — | Cyan/teal gradient, much brighter than the editor chrome |

Sidebar composition in the reference: brand, account/pro card, Home, Templates,
"Create with AI" group, Management (Library, Spaces), and invite/sale cards at the
bottom.

## Editor

| Region | Reference geometry | Sampled color / note |
|---|---|---|
| Application bar | y 0–34, full width | `#141414` |
| Category / column header row | y 34–≈76 | `#303030` |
| Upper workspace | y ≈76–586 | Dark content surfaces |
| Timeline region | y ≈592–1070 | Dark surfaces, divider above toolbar |
| Left column | x 0–626 | 33.5% of width |
| Center column | x 626–1158 | 28.5% of width |
| Right column | x 1166–1868 | 37.6% of width |
| Panel content | — | `#262626` |
| Header / tab strips | — | `#303030` |
| Secondary-nav selection | captions nav | `#3B3B3B` |
| Control/input surfaces | fields, toggles | `#343337` |
| Canvas | preview stage | `#000000` |
| Accent | selected tabs, primary controls | `#00C1CD` |

### Editor structure

- Category strip: Media, Audio, Text, Stickers, Effects, Transitions, Captions,
  Filters, Adjustment, Templates, AI avatar. Selected Captions uses cyan text and
  an underline.
- Left Captions panel: vertical items Auto captions, Templates, Auto lyrics, Add
  captions; Spoken language, Bilingual captions, Identify filler words; pinned
  footer with "Delete current captions" and a primary Generate action.
- Preview: column header reads "Player-Timeline 01"; 9:16 canvas ≈246 × 438 in the
  center of the preview stage; footer shows `00:00:00:00  00:00:18:23`, transport,
  fit/crop/fullscreen controls.
- Inspector: five top tabs (Captions, Text, Animation, Text to speech, AI avatars);
  secondary segmented Basic / Templates / Bubble / Effects; "Apply to all"; Font
  (System), Font size 10, Pattern B/U/I, Case TT/tt/Tt, Color, Character, Line,
  Alignment, Preset style with two rows of eight "Aa" tiles.
- Timeline: tool toolbar, ruler labelled at 10-second intervals, multiple media
  clips in orange/teal/purple/text-red, and narrow track headers with visibility
  and lock controls.

## Export dialog

| Region | Reference geometry | Note |
|---|---|---|
| Dialog | 721 × 663 | Dark `#262626`, rounded corners |
| Left cover | x 86–266, y 52–402 (180 × 350) | Placeholder/cover area, Edit cover below |
| Right form origin | x ≈345 | Vertically stacked groups |
| Footer | y ≈608–662 | Status text, Cancel, Export |
| Form groups | — | Export timeline, Name, Export to, Video settings, Audio |
| Video rows | — | Resolution, Bitrate, Codec, Format, Frame rate, Color space |

The right form uses left-aligned labels with right-aligned values/selectors. The
reference Export action is visually prominent; CutCraft keeps Export visually and
functionally disabled because no encoder exists.

## CutCraft mapping

| Reference observation | CutCraft implementation |
|---|---|
| 1600×900-class workspace with 33.5 / 28.5 / 37.6 columns | `config.rs` fractions 0.335 / center remainder / 0.376 |
| Dark content `#262626`, headers `#303030`, app bar `#141414` | `Palette::PANEL_BG`, `TOOLBAR_BG`, `MENU_BG` |
| Cyan accent `#00C1CD` | `Palette::ACCENT` and accent variants |
| Bright cyan home banner with centered create action | `home::create_banner` whole-surface button |
| Dense captions inspector | `widgets/inspector` caption controls bound to session demo state |
| 10-second ruler intervals, horizontal zoom/scroll | `TIMELINE_PIXELS_PER_SECOND = 10`, 180 s content, 4–80 px/s clamp |
| Narrow track headers and stacked tracks | 158 px headers, 6 tracks, adaptive 20–58 px rows |
| Export dialog with left cover and grouped settings | 760 × 620 modal, 9:16 cover placeholder, grouped form |

## Deliberate differences

- CutCraft uses its own name, logo mark, colors, icon set, and copy.
- The reference has five inspector tabs with assistant/avatar features. CutCraft
  shows six tabs for implemented shell areas and labels the panel `UI preview`.
- The reference preview shows a decoded video frame. CutCraft shows an honest
  empty preview because decoding is not implemented.
- Reference timeline media is replaced by three clearly labelled demo strips.
- The reference account/Pro/invite/sale sidebar content is replaced by a local
  workspace profile and a preview notice.
