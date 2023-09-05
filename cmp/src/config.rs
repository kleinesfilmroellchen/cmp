//! Game configuration and settings management
use std::path::PathBuf;

use bevy::prelude::*;
use log::error;
use serde_derive::{Deserialize, Serialize};

#[derive(Resource, Clone, Debug, Default)]
pub struct CommandLineArguments {
	pub settings_file: Option<PathBuf>,
	/// External game plugins ("mods") to load. A plugin must contain a function with this exact signature:
	/// ```ignore
	/// fn initialize_cmp_plugin(app: &mut bevy::app::App);
	/// ```
	/// The plugin function is passed the exact app used for the game. Several important caveats apply:
	/// - For ABI compatibility, make sure that the plugin is compiled with the EXACT SAME bevy version as CMP itself.
	///   Otherwise, strange errors (most likely segmentation faults) may happen. The --version option of CMP binaries
	///   will report the bevy version in use. It is actually recommended to specify bevy as a dynamic dependency,
	///   since CMP itself links bevy dynamically; this way, the engine code is not included twice in the binary and
	///   the exact same code is used everywhere.
	/// - CMP's own [`Plugin`] as well as some set of default bevy [`Plugin`]s are already loaded by the time this
	///   function is called on any CMP plugin. You may experiment and figure out which extra default bevy plugins are
	///   loadable, but any of them may cause a runtime panic due to duplicate plugins. The set of plugins loaded
	///   across CMP versions may change in any way. The safest option is to only load [`Plugin`]s of your own design
	///   or from some other third-party library.
	pub plugins:       Vec<PathBuf>,
}

/// Game settings for CMP. Game settings are stored by [`confy`] in TOML format in a system-defined config path. For
/// instance, on Linux it's `~/.config/cmp/game-settings.toml` and on Windows it's `%APPDATA%/cmp/game-settings.toml`.
/// It is possible to use a different game settings path by overriding the path on the command line.
#[derive(Serialize, Deserialize, Resource, Clone, Copy, Debug)]
pub struct GameSettings {
	/// Whether to enable VSync.
	#[serde(default)]
	pub use_vsync: bool,
	/// Whether to show a detailed FPS display in the upper left corner of the game window.
	#[serde(default)]
	pub show_fps:  bool,
}

impl Default for GameSettings {
	fn default() -> Self {
		Self { use_vsync: true, show_fps: false }
	}
}

const APP_NAME: &str = "cmp";
const CONFIG_NAME: &str = "game-settings";

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GameSettings>()
			.init_resource::<CommandLineArguments>()
			.add_systems(Startup, load_settings)
			.add_systems(Update, (save_settings, modify_graphics_settings));
	}
}

fn load_settings(mut settings: ResMut<GameSettings>, cli_arguments: Res<CommandLineArguments>) {
	let maybe_config = if let Some(alternate_settings_file) = &cli_arguments.settings_file {
		confy::load_path(alternate_settings_file)
	} else {
		confy::load(APP_NAME, CONFIG_NAME)
	};
	if let Err(why) = &maybe_config {
		error!("Couldn’t load game settings: {}, falling back to defaults.", why);
	}

	*settings = maybe_config.unwrap_or_default();
}

fn save_settings(settings: Res<GameSettings>) {
	if settings.is_changed() {
		let result = confy::store(APP_NAME, CONFIG_NAME, *settings);
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
