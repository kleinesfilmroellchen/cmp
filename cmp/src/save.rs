//! Saving and loading.

use bevy::prelude::*;
use moonshine_save::prelude::*;

#[derive(Event)]
pub struct Load {
	filename: String,
}

#[derive(Event)]
pub struct StoreSave {
	filename: String,
}

pub struct Saving;

impl Plugin for Saving {
	fn build(&self, app: &mut App) {
		app.add_plugins((SavePlugin, LoadPlugin));
	}
}
