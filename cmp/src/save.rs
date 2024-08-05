//! Saving and loading.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use brotli::enc::BrotliEncoderParams;
use brotli::BrotliCompress;
use directories::ProjectDirs;
use moonshine_save::prelude::*;
use tempfile::NamedTempFile;

use crate::config::APP_NAME;
use crate::model::nav::NavComponent;
use crate::ui::world_info::WorldInfoProperties;

#[derive(Resource, Event, Debug, Clone)]
pub struct LoadSave {
	temp_file: Arc<OnceLock<NamedTempFile>>,
	save_slot: u32,
}

/// Since event requests are broken in moonshine_save, we instead use a resource request that is just a clone of the
/// event.
#[derive(Resource, Event, Debug, Clone)]
pub struct StoreSave {
	temp_file: Arc<OnceLock<NamedTempFile>>,
	save_slot: u32,
}

impl StoreSave {
	pub fn new(slot: u32) -> Self {
		Self { temp_file: Arc::new(OnceLock::new()), save_slot: slot }
	}

	pub fn transfer_compressed_save(&self) {
		let result: anyhow::Result<()> = try {
			let mut file = self.temp_file.get().ok_or(anyhow::anyhow!("save didn't complete"))?;
			let mut params = BrotliEncoderParams::default();
			params.quality = 9;
			params.lgwin = 20;
			let output_path = path_for_slot(self.save_slot).ok_or(anyhow::anyhow!("couldn't get project directory"))?;
			let mut output = std::fs::File::options().write(true).truncate(true).create(true).open(&output_path)?;
			BrotliCompress(&mut file, &mut output, &params)?;
			info!("slot {}: saved to {:?}", self.save_slot, output_path);
		};
		if let Err(error) = result {
			error!("slot {}: save failed: {}", self.save_slot, error);
		}
	}
}

impl FilePath for StoreSave {
	fn path(&self) -> &Path {
		self.temp_file.get_or_init(|| NamedTempFile::new().unwrap()).path()
	}
}

/// Return the file system path for the numbered save slot.
fn path_for_slot(save_slot: u32) -> Option<PathBuf> {
	let project = ProjectDirs::from("rs", "", APP_NAME)?;
	let data_path = project.data_dir();
	std::fs::create_dir_all(data_path).ok()?;
	Some(data_path.join(format!("save_{}.cmpsave", save_slot)))
}

pub struct Saving;

impl Plugin for Saving {
	fn build(&self, app: &mut App) {
		app.add_plugins((SavePlugin, LoadPlugin)).add_event::<StoreSave>();

		// TODO: Disable this line when debugging loading.
		// app.add_systems(Startup, crate::model::spawn_test_tiles);
		// TODO: Enable this line when debugging loading.
		app.add_systems(PostStartup, load_from_file("testsave.cmpsave"));

		// TODO: Testing only.
		app.add_systems(
			PreUpdate,
			(
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
					.into_file_on_request::<StoreSave>()
					.before(transfer_save)
					.after(cause_test_save),
				transfer_save,
				cause_test_save.before(clone_save_to_resource),
				clone_save_to_resource,
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

fn transfer_save(mut save_event: EventReader<StoreSave>) {
	if let Some(event) = save_event.read().next() {
		event.transfer_compressed_save();
	}
}

fn cause_test_save(input: Res<ButtonInput<KeyCode>>, mut events: EventWriter<StoreSave>) {
	if input.just_pressed(KeyCode::KeyS) && input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
		events.send(StoreSave::new(0));
	}
}
