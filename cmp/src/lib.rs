//! CMP core engine.
#![feature(duration_constants, let_chains)]
#![deny(clippy::all, missing_docs)]

use std::time::Duration;

use bevy::asset::ChangeWatcher;
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use bevy::winit::WinitWindows;
use config::{ConfigPlugin, GameSettings};
use input::GUIInputPlugin;
use ui::UIPlugin;
use winit::window::Icon;

pub(crate) mod config;
pub(crate) mod debug;
pub(crate) mod graphics;
pub(crate) mod input;
pub(crate) mod model;
pub(crate) mod ui;
pub mod util;

pub use graphics::GraphicsPlugin;

/// Base plugin for the entire core engine.
/// FIXME: Extract the rendering into its own plugin.
pub struct CmpPlugin;

impl Plugin for CmpPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			DefaultPlugins
				.build()
				.set(AssetPlugin {
					asset_folder:      "../assets".into(),
					watch_for_changes: Some(ChangeWatcher { delay: Duration::from_secs(3) }),
				})
				.set(ImagePlugin::default_nearest()).set(AnimationPlugin),
		)
		.add_plugins((GUIInputPlugin, UIPlugin, ConfigPlugin))
		.insert_resource(WindowIcon::default())
		.add_systems(Startup, (debug::create_stats, setup_window, model::spawn_test_tiles))
		// .add_systems(Update, tile::wave_tiles)
		.add_systems(Update, (set_window_icon, debug::print_stats, apply_window_settings));
	}
}

#[derive(Resource, Default)]
struct WindowIcon(Handle<Image>);

fn setup_window(
	asset_server: Res<AssetServer>,
	mut icon: ResMut<WindowIcon>,
	mut windows: Query<&mut bevy::prelude::Window, With<PrimaryWindow>>,
) {
	icon.0 = asset_server.load::<Image, _>("logo-overscaled.png");

	let mut window = windows.single_mut();
	window.title = "Camping Madness Project".to_string();
}

fn apply_window_settings(
	mut windows: Query<&mut bevy::prelude::Window, With<PrimaryWindow>>,
	settings: Res<GameSettings>,
) {
	let mut window = windows.single_mut();
	if settings.is_changed() {
		window.present_mode = if settings.use_vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };
	}
}

fn set_window_icon(
	winit_map: NonSend<WinitWindows>,
	mut windows: Query<(Entity, &mut bevy::prelude::Window), With<PrimaryWindow>>,
	mut ev_asset: EventReader<AssetEvent<Image>>,
	images: Res<Assets<Image>>,
	window_icon: Res<WindowIcon>,
) {
	for ev in ev_asset.iter() {
		if let AssetEvent::Created { handle } = ev {
			if *handle == window_icon.0 {
				for window in &mut windows {
					let winit_window =
						winit_map.windows.get(winit_map.entity_to_winit.get(&window.0).unwrap()).unwrap();

					let (icon_rgba, icon_width, icon_height) = {
						let image = images
							.get(&window_icon.0)
							.unwrap()
							.convert(bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb)
							.unwrap();
						let (width, height) = image.size().into();
						let rgba = image.data;
						(rgba, width, height)
					};

					let icon = Icon::from_rgba(icon_rgba, icon_width as u32, icon_height as u32).unwrap();

					winit_window.set_window_icon(Some(icon));
				}
			}
		}
	}
}
