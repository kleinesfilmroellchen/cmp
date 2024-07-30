use std::error::Error;

use anyhow::anyhow;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
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
/// let mut app = App::new();
/// app.add_plugins(DefaultPlugins);
/// app.add_asset_loader(QOIAssetLoader);
/// // Initialize the rest of your game...
/// app.run();
/// ```
///
/// The asset loader hooks into Bevy's asset system like normal, meaning you can load QOI images like any other asset.
pub struct QOIAssetLoader;

impl AssetLoader for QOIAssetLoader {
	type Asset = Image;
	type Error = Box<dyn Error + Send + Sync + 'static>;
	type Settings = ();

	async fn load<'a>(
		&'a self,
		reader: &'a mut Reader<'_>,
		_: &'a Self::Settings,
		_: &'a mut LoadContext<'_>,
	) -> Result<Self::Asset, Self::Error> {
		let mut bytes = Vec::new();
		reader.read_to_end(&mut bytes).await?;
		let mut decoder = Decoder::new(&bytes)?.with_channels(qoi::Channels::Rgba);
		let decoded = decoder.decode_to_vec()?;
		let header = decoder.header();

		Ok(Image::new(
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
			RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
		))
	}

	fn extensions(&self) -> &[&str] {
		&["qoi"]
	}
}
