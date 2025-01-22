//! Saving and loading.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use brotli::enc::BrotliEncoderParams;
use directories::ProjectDirs;
use moonshine_save::prelude::*;
use moonshine_save::{stream_from_resource, GetStream};

use crate::config::APP_NAME;
use crate::gamemode::GameState;
use crate::model::nav::NavComponent;
use crate::ui::world_info::WorldInfoProperties;

#[derive(Resource, Event, Debug, Clone)]
pub struct LoadSave {
	save_name: String,
}

/// Since event requests are broken in moonshine_save, we instead use a resource request that is just a clone of the
/// event.
#[derive(Resource, Event, Debug, Clone)]
pub struct StoreSave {
	save_name: String,
}

const BUFFER_SIZE: usize = 10 * 1024;

impl StoreSave {
	pub fn new(save_name: String) -> Self {
		Self { save_name }
	}

	fn save_file(&self) -> anyhow::Result<std::fs::File> {
		let output_path = path_for_slot(&self.save_name).ok_or(anyhow::anyhow!("couldn’t get project directory"))?;
		debug!("initiated save to {output_path:?}");
		Ok(std::fs::File::options().write(true).truncate(true).create(true).open(&output_path)?)
	}

	fn brotli_params() -> BrotliEncoderParams {
		let mut params = BrotliEncoderParams::default();
		params.quality = 9;
		params.lgwin = 20;
		params
	}
}

impl LoadSave {
	pub fn new(save_name: String) -> Self {
		Self { save_name }
	}

	fn save_file(&self) -> anyhow::Result<std::fs::File> {
		let output_path = path_for_slot(&self.save_name).ok_or(anyhow::anyhow!("couldn’t get project directory"))?;
		debug!("initiated load from {output_path:?}");
		Ok(std::fs::File::options().read(true).open(&output_path)?)
	}
}

impl GetStream for StoreSave {
	type Stream = brotli::CompressorWriter<std::fs::File>;

	fn stream(&self) -> Self::Stream {
		brotli::CompressorWriter::with_params(self.save_file().unwrap(), BUFFER_SIZE, &Self::brotli_params())
	}
}

impl GetStream for LoadSave {
	type Stream = brotli::Decompressor<std::fs::File>;

	fn stream(&self) -> Self::Stream {
		brotli::Decompressor::new(self.save_file().unwrap(), BUFFER_SIZE)
	}
}

/// Return the file system path for the numbered save slot.
fn path_for_slot(save_name: &str) -> Option<PathBuf> {
	let project = ProjectDirs::from("rs", "", APP_NAME)?;
	let data_path = project.data_dir();
	std::fs::create_dir_all(data_path).ok()?;
	Some(data_path.join(format!("{}.cmpsave", save_name)))
}

pub struct Saving;

impl Plugin for Saving {
	fn build(&self, app: &mut App) {
		app.add_plugins((SavePlugin, LoadPlugin)).add_event::<StoreSave>().add_event::<LoadSave>();

		// TODO: Disable this line when debugging loading.
		// app.add_systems(Startup, crate::model::spawn_test_tiles);
		// TODO: Enable this line when debugging loading.

		app.add_systems(
			FixedPreUpdate,
			(
				save_default()
					.exclude_component::<Sprite>()
					.exclude_component::<Transform>()
					.exclude_component::<GlobalTransform>()
					.exclude_component::<Visibility>()
					.exclude_component::<InheritedVisibility>()
					.exclude_component::<ViewVisibility>()
					.exclude_component::<Aabb>()
					.exclude_component::<NavComponent>()
					.exclude_component::<WorldInfoProperties>()
					.into(stream_from_resource::<StoreSave>()),
				load(stream_from_resource::<LoadSave>()),
			),
		);

		app.add_systems(
			First,
			(cause_test_save.run_if(in_state(GameState::InGame)), cause_test_load.run_if(in_state(GameState::InGame))),
		);
	}
}

fn cause_test_save(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
	if input.just_pressed(KeyCode::KeyS) && input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		commands.insert_resource(StoreSave::new("Test".to_string()));
	}
}

fn cause_test_load(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
	if input.just_pressed(KeyCode::KeyO) && input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		commands.insert_resource(LoadSave::new("Test".to_string()));
	}
}
