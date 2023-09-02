//! CMP core engine.
#![feature(duration_constants)]
#![deny(clippy::all, missing_docs)]

use std::time::Duration;

use bevy::asset::ChangeWatcher;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WinitWindows;
use input::GUIInputPlugin;
use winit::window::Icon;

use crate::geometry::{ActorPosition, GridPosition};

pub(crate) mod debug;
pub(crate) mod geometry;
pub(crate) mod graphics;
pub(crate) mod input;
pub(crate) mod tile;

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
				.set(ImagePlugin::default_nearest()),
		)
		.add_plugins(GUIInputPlugin::default())
		.insert_resource(WindowIcon::default())
		.add_systems(
			Startup,
			(graphics::initialize_graphics, debug::create_stats, setup_window, tile::spawn_test_tiles),
		)
		.add_systems(Update, (tile::wave_tiles, set_window_icon, debug::print_stats))
		.add_systems(
			PostUpdate,
			(graphics::position_objects::<ActorPosition>, graphics::position_objects::<GridPosition>),
		);
	}
}

#[derive(Resource, Default)]
struct WindowIcon(Handle<Image>);

fn setup_window(
	asset_server: Res<AssetServer>,
	mut icon: ResMut<WindowIcon>,
	mut windows: Query<&mut bevy::prelude::Window, With<PrimaryWindow>>,
) {
	icon.0 = asset_server.load::<Image, _>("grass.png");

	for mut window in &mut windows {
		window.title = "Camping Madness Project".to_string();
		// Uncomment this to test maximum performance; it’s more efficient to keep VSync on.
		// window.present_mode = bevy::window::PresentMode::AutoNoVsync;
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

					// here we use the `image` crate to load our icon data from a png file
					// this is not a very bevy-native solution, but it will do
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
