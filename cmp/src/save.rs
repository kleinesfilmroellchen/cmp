//! Saving and loading.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use brotli::enc::BrotliEncoderParams;
use brotli::{BrotliCompress, BrotliDecompress};
use directories::ProjectDirs;
use moonshine_save::load::{load_from_file_on_event, load_from_file_on_request};
use moonshine_save::prelude::*;
use tempfile::NamedTempFile;

use crate::config::APP_NAME;
use crate::gamemode::GameState;
use crate::model::nav::NavComponent;
use crate::ui::world_info::WorldInfoProperties;

#[derive(Resource, Event, Debug, Clone)]
pub struct LoadSave {
	temp_file: Arc<OnceLock<NamedTempFile>>,
	save_name: String,
}

/// Since event requests are broken in moonshine_save, we instead use a resource request that is just a clone of the
/// event.
#[derive(Resource, Event, Debug, Clone)]
pub struct StoreSave {
	temp_file: Arc<OnceLock<NamedTempFile>>,
	save_name: String,
}

impl StoreSave {
	pub fn new(save_name: String) -> Self {
		Self { temp_file: Arc::new(OnceLock::new()), save_name }
	}

	pub fn transfer_compressed_save(&self) {
		let result: anyhow::Result<()> = try {
			let mut file = self.temp_file.get().ok_or(anyhow::anyhow!("save didn't complete"))?;
			let mut params = BrotliEncoderParams::default();
			params.quality = 9;
			params.lgwin = 20;
			let output_path =
				path_for_slot(&self.save_name).ok_or(anyhow::anyhow!("couldn’t get project directory"))?;
			let mut output = std::fs::File::options().write(true).truncate(true).create(true).open(&output_path)?;
			BrotliCompress(&mut file, &mut output, &params)?;
			info!("slot {}: saved to {:?}", self.save_name, output_path);
		};
		if let Err(error) = result {
			error!("slot {}: save failed: {}", self.save_name, error);
		}
	}
}

impl LoadSave {
	pub fn new(save_name: String) -> Self {
		Self { temp_file: Arc::new(OnceLock::new()), save_name }
	}

	pub fn decompress_save(&self) {
		let result: anyhow::Result<()> = try {
			let source_path =
				path_for_slot(&self.save_name).ok_or(anyhow::anyhow!("couldn’t get project directory"))?;
			let mut source = std::fs::File::options().read(true).open(&source_path)?;
			let mut temp_file = self.temp_file.get_or_init(|| NamedTempFile::new().unwrap());
			BrotliDecompress(&mut source, &mut temp_file)?;
			info!("slot {}: decompressed from {:?}", self.save_name, source_path);
		};
		if let Err(error) = result {
			error!("slot {}: decompression failed: {}", self.save_name, error);
		}
	}
}

impl GetFilePath for LoadSave {
	fn path(&self) -> &Path {
		self.temp_file.get().unwrap().path()
	}
}

impl GetFilePath for StoreSave {
	fn path(&self) -> &Path {
		self.temp_file.get_or_init(|| NamedTempFile::new().unwrap()).path()
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
		app.add_systems(Startup, crate::model::spawn_test_tiles);
		// TODO: Enable this line when debugging loading.

		app.add_systems(
			First,
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
					.into_file_on_request::<StoreSave>()
					.before(transfer_save)
					.after(cause_test_save),
				load_from_file_on_request::<LoadSave>().after(cause_test_load),
				transfer_save,
				cause_test_save.before(clone_save_to_resource).run_if(in_state(GameState::InGame)),
				cause_test_load.before(clone_load_to_resource).run_if(in_state(GameState::InGame)),
				clone_save_to_resource,
				clone_load_to_resource,
			),
		);
	}
}

/// HACK: Clones the store event into a resource so that moonshine_save sees it.
fn clone_save_to_resource(mut save_event: EventReader<StoreSave>, mut commands: Commands) {
	if let Some(event) = save_event.read().next() {
		commands.insert_resource(event.clone());
	}
}

fn clone_load_to_resource(mut load_event: EventReader<LoadSave>, mut commands: Commands) {
	if let Some(event) = load_event.read().next() {
		event.decompress_save();
		commands.insert_resource(event.clone());
	}
}

fn transfer_save(mut save_event: EventReader<StoreSave>) {
	if let Some(event) = save_event.read().next() {
		event.transfer_compressed_save();
	}
}

fn cause_test_save(input: Res<ButtonInput<KeyCode>>, mut events: EventWriter<StoreSave>) {
	if input.just_pressed(KeyCode::KeyS) && input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		events.send(StoreSave::new("Test".to_string()));
	}
}

fn cause_test_load(input: Res<ButtonInput<KeyCode>>, mut events: EventWriter<LoadSave>) {
	if input.just_pressed(KeyCode::KeyO) && input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		events.send(LoadSave::new("Test".to_string()));
	}
}
