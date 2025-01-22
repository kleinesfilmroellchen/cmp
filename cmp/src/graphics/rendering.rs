//! Pixel-perfect rendering code partially adapted from https://github.com/bevyengine/bevy/blob/main/examples/2d/pixel_grid_snap.rs

use bevy::core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::*;
use bevy::render::view::RenderLayers;
use bevy::window::WindowResized;

/// In-game resolution width.
pub const RES_WIDTH: u32 = 160 * 2;

/// In-game resolution height.
pub const RES_HEIGHT: u32 = 90 * 2;

/// Default render layers for pixel-perfect rendering.
/// You can skip adding this component, as this is the default.
const PIXEL_PERFECT_LAYERS: RenderLayers = RenderLayers::layer(0);

/// Render layers for high-resolution rendering.
pub const HIGH_RES_LAYERS: RenderLayers = RenderLayers::layer(1);

/// Low-resolution texture that contains the pixel-perfect world.
/// Canvas itself is rendered to the high-resolution world.
#[derive(Component)]
pub struct Canvas;

/// Camera that renders the pixel-perfect world to the [`Canvas`].
#[derive(Component)]
pub struct InGameCamera;

/// Camera that renders the [`Canvas`] (and other graphics on [`HIGH_RES_LAYERS`]) to the screen.
#[derive(Component)]
pub struct OuterCamera;

pub fn initialize_rendering(
	mut commands: Commands,
	_asset_server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
) {
	let canvas_size = Extent3d { width: RES_WIDTH, height: RES_HEIGHT, ..default() };

	// this Image serves as a canvas representing the low-resolution game screen
	let mut canvas = Image {
		texture_descriptor: TextureDescriptor {
			label:           None,
			size:            canvas_size,
			dimension:       TextureDimension::D2,
			format:          TextureFormat::Bgra8UnormSrgb,
			mip_level_count: 1,
			sample_count:    1,
			usage:           TextureUsages::TEXTURE_BINDING
				| TextureUsages::COPY_DST
				| TextureUsages::RENDER_ATTACHMENT,
			view_formats:    &[],
		},
		sampler: ImageSampler::nearest(),
		..default()
	};

	// fill image.data with zeroes
	canvas.resize(canvas_size);

	let image_handle = images.add(canvas);

	// this camera renders whatever is on `PIXEL_PERFECT_LAYERS` to the canvas
	commands.spawn((
		Camera2d,
		Camera {
			// render before the "main pass" camera
			order: -1,
			hdr: true,
			target: RenderTarget::Image(image_handle.clone().into()),
			..default()
		},
		DebandDither::Enabled,
		ContrastAdaptiveSharpening { enabled: false, sharpening_strength: 0.3, denoise: false },
		Msaa::Off,
		InGameCamera,
		PIXEL_PERFECT_LAYERS,
	));

	// spawn the canvas
	commands.spawn((Sprite::from_image(image_handle), Canvas, HIGH_RES_LAYERS));

	// the "outer" camera renders whatever is on `HIGH_RES_LAYERS` to the screen.
	// here, the canvas and one of the sample sprites will be rendered by this camera
	let projection = OrthographicProjection { scale: 1. / 4., near: -100000., ..OrthographicProjection::default_2d() };
	commands.spawn((
		projection,
		Camera2d,
		Camera { hdr: true, ..Default::default() },
		Msaa::Off,
		OuterCamera,
		HIGH_RES_LAYERS,
	));
}

/// Scales camera projection to fit the window (integer multiples only).
pub fn fit_canvas(
	mut resize_events: EventReader<WindowResized>,
	mut projection: Query<&mut OrthographicProjection, With<OuterCamera>>,
) {
	let Ok(mut projection) = projection.get_single_mut() else {
		return;
	};
	for event in resize_events.read() {
		let h_scale = event.width / RES_WIDTH as f32;
		let v_scale = event.height / RES_HEIGHT as f32;
		projection.scale = 1. / h_scale.max(v_scale).ceil();
	}
}
