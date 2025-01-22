# QOI image loading for Bevy

Provides support for the [QOI image format](https://qoiformat.org) in Bevy apps. QOI is a new lossless image format that was specifically designed for use in games:
- immensely fast to decode (usually 2-3x as fast as PNG), making texture streaming much easier
- lossless and not much larger than PNG (usually 20% larger)
- bare-bones, but feature-complete format (supports 8-bit RGB or RGBA, both sRGB and linear color spaces, no metadata)

This asset loader is heavily based on <https://github.com/digitaljokerman/bevy_qoi>, licensed under MIT, and utilizes the best-in-class [`qoi`](https://crates.io/crates/qoi) decoding backend. Supports Bevy 0.15 and requires the latest nightly compiler.

## Usage

bevy_qoi provides the simple asset loader struct `QOIAssetLoader`. Register it with `App::register_asset_loader` as usual:

```rust
use bevy::prelude::*;
use bevy_qoi::QOIAssetLoader;

let mut app = App::new();
app.add_plugins(DefaultPlugins);
app.register_asset_loader(QOIAssetLoader);
// Initialize the rest of your game...
app.run();
```

Assets with the `.qoi` extension will automatically be loaded by the QOI asset loader.
