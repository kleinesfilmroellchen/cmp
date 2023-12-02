//! CMP core engine.
#![feature(
	test,
	duration_constants,
	let_chains,
	gen_blocks,
	iter_from_coroutine,
	coroutine_trait,
	coroutines,
	try_blocks,
	iter_intersperse,
	extract_if,
	adt_const_params,
	trivial_bounds,
	const_fn_floating_point_arithmetic,
	round_ties_even,
	trait_alias
)]
#![deny(clippy::all, missing_docs)]
#![allow(clippy::type_complexity, incomplete_features, clippy::too_many_arguments)]

extern crate test;

use std::sync::Arc;
#[allow(unused)]
use std::time::Duration;

use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use bevy::winit::WinitWindows;
use config::{CommandLineArguments, ConfigPlugin, GameSettings};
use input::GUIInputPlugin;
use model::area::AreaManagement;
use model::nav::NavManagement;
use model::{AccommodationManagement, TileManagement};
use plugins::ExternalPlugins;
use ui::UIPlugin;
use winit::window::Icon;

pub(crate) mod config;
pub(crate) mod debug;
pub(crate) mod graphics;
pub(crate) mod input;
pub(crate) mod model;
pub mod plugins;
pub(crate) mod ui;
pub mod util;

pub use graphics::GraphicsPlugin;

/// Concurrent set implementation with the fast AHash algorithm.
pub type DashSet<K> = dashmap::DashSet<K, std::hash::BuildHasherDefault<bevy::utils::AHasher>>;

const VERSION: &str =
	env!("CARGO_PKG_VERSION", "CMP must be built under Cargo, or set the CARGO_PKG_VERSION variable manually.");

/// Base plugin for the entire core engine.
pub struct CmpPlugin;

impl Plugin for CmpPlugin {
	fn build(&self, app: &mut App) {
		let args: Arc<CommandLineArguments> = Arc::new(argh::from_env());

		if args.version {
			println!("{}", program_info());
			std::process::exit(0);
		}

		let settings = Arc::new(GameSettings::from_arg_path(&args));
		let log_level = if settings.show_debug { Level::TRACE } else { Level::INFO };

		app.add_plugins(
			DefaultPlugins
				.build()
				.set(AssetPlugin {
					file_path:       "../assets".into(),
					processed_file_path: "../processed-assets".into(),
					#[cfg(debug_assertions)]
        			watch_for_changes_override: Some(true),
					#[cfg(not(debug_assertions))]
					watch_for_changes_override: Some(false),
        			mode: AssetMode::Unprocessed,
				})
				.set(ImagePlugin::default_nearest()).set(AnimationPlugin)
				.set(LogPlugin {
					level: log_level,
					filter: "info,cmp=trace,wgpu=error,bevy=warn".into(),
				}),
		)
		.register_asset_loader(bevy_qoi::QOIAssetLoader)
		// Fixed update runs every two seconds and performs slow work that can take this long.
		.insert_resource(Time::<Fixed>::from_seconds(0.5))
		.add_plugins((GUIInputPlugin, UIPlugin, TileManagement, AccommodationManagement, AreaManagement, NavManagement, ConfigPlugin(args.clone(), settings.clone()), ExternalPlugins(args)))
		.insert_resource(WindowIcon::default())
		.add_systems(Startup, (debug::create_stats, setup_window, model::spawn_test_tiles))
		.add_systems(PostStartup, print_program_info)
		.add_systems(Update, (set_window_icon, debug::print_stats, apply_window_settings));
	}
}

fn print_program_info() {
	info!("{}", program_info());
}

fn program_info() -> String {
	format!(
		"The Camping Madness Project version {}\nCopyright © 2023, kleines Filmröllchen. Licensed under a BSD \
		 2-clause license.",
		VERSION
	)
}

#[derive(Resource, Default)]
struct WindowIcon(Handle<Image>);

fn setup_window(
	asset_server: Res<AssetServer>,
	mut icon: ResMut<WindowIcon>,
	mut windows: Query<&mut bevy::prelude::Window, With<PrimaryWindow>>,
) {
	icon.0 = asset_server.load::<Image>("logo-overscaled.png");

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
	for ev in ev_asset.read() {
		if let AssetEvent::LoadedWithDependencies { id } = ev {
			let maybe_id = images.iter().find_map(|(img_id, _)| if *id == img_id { Some(img_id) } else { None });
			if maybe_id.is_some_and(|id| id == window_icon.0.id()) {
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

					let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap();

					winit_window.set_window_icon(Some(icon));
				}
			}
		}
	}
}
