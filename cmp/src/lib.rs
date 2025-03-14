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
	trait_alias
)]

extern crate test;

use std::sync::Arc;
#[allow(unused)]
use std::time::Duration;

use bevy::asset::AssetMetaCheck;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::{EnabledButtons, PresentMode, PrimaryWindow, WindowResolution};
use bevy::winit::WinitWindows;
use config::{CommandLineArguments, ConfigPlugin, GameSettings};
use gamemode::{pause_fixed_timer, GameState};
use input::GUIInputPlugin;
use model::area::AreaManagement;
use model::nav::NavManagement;
use model::{
	AccommodationManagement, ActorPosition, BoundingBox, Buildable, BuildableType, GridBox, GridPosition,
	TileManagement,
};
use save::Saving;
use ui::UIPlugin;
use winit::window::Icon;

pub(crate) mod config;
pub(crate) mod debug;
pub(crate) mod gamemode;
pub(crate) mod graphics;
pub(crate) mod input;
pub(crate) mod model;
pub(crate) mod save;
pub(crate) mod ui;
pub mod util;

pub use graphics::GraphicsPlugin;
// re-export bevy symbols needed by the client and server, so that they don’t have to depend on bevy themselves
pub use bevy::prelude::{App, PostStartup, info};

/// Hash set wrapper, because bevy doesn't have a serialization implementation for HashSet.
pub type HashSet<T> = bevy::utils::HashMap<T, ()>;

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

		println!("environment variables:");
		for (key, value) in std::env::vars() {
			println!("{key}: {value}");
		}
		println!("directory: {:?}", std::env::current_dir());

		let settings = Arc::new(GameSettings::from_arg_path(&args));
		let log_level = if settings.show_debug { Level::TRACE } else { Level::INFO };

		app.add_plugins(
			DefaultPlugins
				.build()
				.set(AssetPlugin {
					file_path:       "assets".into(),
					processed_file_path: "../processed-assets".into(),
					#[cfg(debug_assertions)]
        			watch_for_changes_override: Some(true),
					#[cfg(not(debug_assertions))]
					watch_for_changes_override: Some(false),
        			mode: AssetMode::Unprocessed,
					meta_check: AssetMetaCheck::Always,
				})
				.set(ImagePlugin::default_nearest()).set(AnimationPlugin)
				.set(LogPlugin {
					level: log_level,
					filter: "info,cmp=trace,wgpu=error,bevy=warn".into(),
					..Default::default()
				// }).set(RenderPlugin {
				// 	render_creation: RenderCreation::Automatic(WgpuSettings {
				// 		// backends: Some(Backends::VULKAN),
				// 		..default()
				// 	}),
				// 	..default()
				}).set(WindowPlugin {
					primary_window: Some(Window {
						resolution: WindowResolution::new(1920.0, 1080.0),
						enabled_buttons: EnabledButtons {
							maximize: false,
							..Default::default()
						},
						..Default::default()
					}),
					..Default::default()
				}),
		)
		.register_type::<HashSet<GridPosition>>()
		.register_type::<GridBox>()
		.register_type::<BoundingBox>()
		.register_type::<Buildable>()
		.register_type::<GridPosition>()
		.register_type::<BuildableType>()
		.register_type::<ActorPosition>()
		.register_asset_loader(bevy_qoi::QOIAssetLoader)
		// Fixed update runs every two seconds and performs slow work that can take this long.
		.insert_resource(Time::<Fixed>::from_seconds(0.5))
		.init_state::<GameState>()
		.add_plugins((GUIInputPlugin, UIPlugin, TileManagement, AccommodationManagement, AreaManagement, NavManagement, Saving, ConfigPlugin(args.clone(), settings.clone())))
		.insert_resource(WindowIcon::default())
		.add_systems(Startup, (debug::create_stats, setup_window))
		.add_systems(PostStartup, print_program_info)
		.add_systems(Update, (set_window_icon, debug::print_stats, apply_window_settings))
		.add_systems(Update, pause_fixed_timer.run_if(state_changed::<GameState>))
		.add_systems(PreStartup, go_to_game);

		configure_set(app, PreUpdate);
		configure_set(app, Update);
		configure_set(app, FixedPostUpdate);
		configure_set(app, FixedPreUpdate);
		configure_set(app, FixedUpdate);
		configure_set(app, First);
		configure_set(app, Last);
		configure_set(app, Startup);
		configure_set(app, PreStartup);
		configure_set(app, PostStartup);
	}
}

fn configure_set<S>(app: &mut App, set: S)
where
	S: ScheduleLabel,
{
	app.configure_sets(
		set,
		(
			GameState::InGame.run_if(in_state(GameState::InGame)),
			GameState::MainMenu.run_if(in_state(GameState::MainMenu)),
			GameState::Paused.run_if(in_state(GameState::Paused)),
		),
	);
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
	if settings.is_changed() {
		let mut window = windows.single_mut();
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

fn go_to_game(mut next: ResMut<NextState<GameState>>) {
	next.set(GameState::InGame);
}
