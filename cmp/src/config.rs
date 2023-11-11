//! Game configuration and settings management
use std::path::PathBuf;
use std::sync::Arc;

use argh::FromArgs;
use bevy::prelude::*;
use serde_derive::{Deserialize, Serialize};

/// The Camping Madness Project
#[derive(FromArgs, Resource, Clone, Debug, Default)]
pub struct CommandLineArguments {
	/// an alternative settings file to use instead of the system default
	#[argh(option)]
	pub settings_file: Option<PathBuf>,
	/// external game plugins ("mods") to load; a path to a plugin's shared library file (.dll, .so, ...)
	#[argh(option)]
	pub plugins:       Vec<PathBuf>,
}

/// Game settings for CMP. Game settings are stored by [`confy`] in TOML format in a system-defined config path. For
/// instance, on Linux it's `~/.config/cmp/game-settings.toml` and on Windows it's `%APPDATA%/cmp/game-settings.toml`.
/// It is possible to use a different game settings path by overriding the path on the command line.
#[derive(Serialize, Deserialize, Resource, Clone, Copy, Debug)]
pub struct GameSettings {
	/// Whether to enable VSync.
	#[serde(default = "_true")]
	pub use_vsync:  bool,
	/// Whether to show a detailed FPS display in the upper left corner of the game window.
	#[serde(default = "_false")]
	pub show_fps:   bool,
	/// Whether to show various debugging information in the world.
	#[serde(default = "_false")]
	pub show_debug: bool,
}

fn _true() -> bool {
	true
}
fn _false() -> bool {
	false
}

impl Default for GameSettings {
	fn default() -> Self {
		Self { use_vsync: true, show_fps: false, show_debug: false }
	}
}

impl GameSettings {
	pub fn from_arg_path(cli_arguments: &CommandLineArguments) -> Self {
		let maybe_config = if let Some(alternate_settings_file) = &cli_arguments.settings_file {
			confy::load_path(alternate_settings_file)
		} else {
			confy::load(APP_NAME, CONFIG_NAME)
		};
		match &maybe_config {
			Err(why) => {
				error!("Couldn’t load game settings: {}, falling back to defaults.", why);
				Self::default()
			},
			Ok(config) => *config,
		}
	}
}

const APP_NAME: &str = "cmp";
const CONFIG_NAME: &str = "game-settings";

pub struct ConfigPlugin(pub Arc<CommandLineArguments>, pub Arc<GameSettings>);

#[derive(Resource, Deref, DerefMut)]
pub struct CLIResource(pub Arc<CommandLineArguments>);

impl Plugin for ConfigPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(*self.1)
			.insert_resource(CLIResource(self.0.clone()))
			.add_systems(Update, (save_settings, modify_graphics_settings));
	}
}

fn save_settings(settings: Res<GameSettings>, cli_arguments: Res<CLIResource>) {
	if settings.is_changed() {
		let result = if let Some(alternate_settings_file) = &cli_arguments.settings_file {
			confy::store_path(alternate_settings_file, *settings)
		} else {
			confy::store(APP_NAME, CONFIG_NAME, *settings)
		};
		if let Err(why) = result {
			error!("Couldn’t save game settings: {}", why);
		}
	}
}

fn modify_graphics_settings(mut settings: ResMut<GameSettings>, keys: Res<Input<KeyCode>>) {
	if keys.just_pressed(KeyCode::V) && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		settings.use_vsync = !settings.use_vsync;
	}
}
