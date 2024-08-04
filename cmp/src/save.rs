//! Saving and loading.

use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use moonshine_save::prelude::*;

use crate::model::nav::NavComponent;
use crate::ui::world_info::WorldInfoProperties;

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

		// TODO: Disable this line when debugging loading.
		// app.add_systems(Startup, crate::model::spawn_test_tiles);
		// TODO: Enable this line when debugging loading.
		app.add_systems(PostStartup, load_from_file("testsave.cmpsave"));

		// TODO: Testing only.
		app.add_systems(
			PostUpdate,
			save_default()
				.exclude_component::<Handle<Image>>()
				.exclude_component::<Sprite>()
				.exclude_component::<Transform>()
				.exclude_component::<GlobalTransform>()
				.exclude_component::<Visibility>()
				.exclude_component::<InheritedVisibility>()
				.exclude_component::<ViewVisibility>()
				.exclude_component::<Aabb>()
				.exclude_component::<NavComponent>()
				.exclude_component::<WorldInfoProperties>()
				.into_file("testsave.cmpsave")
				.run_if(save_key_pressed),
		);
	}
}

fn save_key_pressed(input: Res<ButtonInput<KeyCode>>) -> bool {
	input.just_pressed(KeyCode::KeyS) && input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
}
