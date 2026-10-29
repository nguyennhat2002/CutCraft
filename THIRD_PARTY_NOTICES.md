# Third-party notices

## Lucide icons

CutCraft bundles Lucide icon artwork through the `egui-lucide` crate
(version 0.2.0), generated from Lucide **1.61.0**. Icons are rendered locally
through egui's cached SVG loader; no icons are downloaded at runtime.

- Lucide: <https://lucide.dev>
- License: ISC, with Feather-derived icons under MIT.
- Full notice: [docs/licenses/LUCIDE_LICENSE.txt](docs/licenses/LUCIDE_LICENSE.txt)

## egui-lucide

- Crate: `egui-lucide` 0.2.0, MIT.
- Full notice: [docs/licenses/EGUI_LUCIDE_LICENSE.txt](docs/licenses/EGUI_LUCIDE_LICENSE.txt)

## SVG loading

SVG rasterization is provided by `egui_extras` 0.37.0 with its `svg` feature
and the `resvg`/`usvg` dependency chain (MIT/Apache-2.0 family). No icon files
are fetched at application runtime.

## Shortcut configuration serialization

User shortcut profiles are stored as local JSON through `serde` 1.0.229 and
`serde_json` 1.0.154 (both MIT OR Apache-2.0), with the `itoa` and `ryu` JSON
formatting dependencies. No configuration is sent over the network.

The CutCraft brand mark and illustrative banner/demo artwork remain original
CutCraft assets. No CapCut icon or illustration assets are included.
