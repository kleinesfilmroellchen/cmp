//! Game configuration and settings management
use bevy::prelude::*;
use log::error;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Resource, Clone, Copy, Debug)]
pub struct GameSettings {
	pub use_vsync: bool,
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
			.add_systems(Startup, load_settings)
			.add_systems(Update, (save_settings, modify_graphics_settings));
	}
}

fn load_settings(mut settings: ResMut<GameSettings>) {
	let maybe_config = confy::load(APP_NAME, CONFIG_NAME);
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
