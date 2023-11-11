use anyhow::anyhow;
use bevy::asset::{AssetLoader, LoadedAsset};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use qoi::Decoder;

/// The asset loader that provides QOI loading capabilities.
///
/// Include this loader in your [App](`bevy::prelude::App`) like this:
///
/// ```rust
/// use bevy::prelude::*;
/// use bevy_qoi::QOIAssetLoader;
///
/// fn main() {
/// 	let mut app = App::new();
/// 	app.add_plugins(DefaultPlugins);
/// 	app.add_asset_loader(QOIAssetLoader);
/// 	// Initialize the rest of your game...
/// 	app.run();
/// }
/// ```
///
/// The asset loader hooks into Bevy's asset system like normal, meaning you can load QOI images like any other asset.
pub struct QOIAssetLoader;

impl AssetLoader for QOIAssetLoader {
	fn load<'a>(
		&'a self,
		bytes: &'a [u8],
		load_context: &'a mut bevy::asset::LoadContext,
	) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
		Box::pin(async move {
			let mut decoder = Decoder::new(&bytes)?.with_channels(qoi::Channels::Rgba);
			let decoded = decoder.decode_to_vec()?;
			let header = decoder.header();

			load_context.set_default_asset(LoadedAsset::new(Image::new(
				Extent3d { width: header.width, height: header.height, ..Default::default() },
				TextureDimension::D2,
				decoded,
				match header.channels {
					qoi::Channels::Rgb => Err(anyhow!("Rgb not supported.")),
					qoi::Channels::Rgba => Ok(match header.colorspace {
						qoi::ColorSpace::Srgb => TextureFormat::Rgba8UnormSrgb,
						qoi::ColorSpace::Linear => TextureFormat::Rgba8Unorm,
					}),
				}?,
			)));

			Ok(())
		})
	}

	fn extensions(&self) -> &[&str] {
		&["qoi"]
	}
}
