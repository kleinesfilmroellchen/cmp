use std::sync::OnceLock;

use bevy::core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpeningSettings;
use bevy::prelude::*;

use crate::model::{ActorPosition, BoundingBox, GridBox, GridPosition, WorldPosition};

pub(crate) mod library;

/// Plugin responsible for setting up a window and running and initializing graphics.
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Msaa::default())
			.add_systems(Startup, initialize_graphics)
			.add_systems(
				PostUpdate,
				(position_objects::<ActorPosition>, position_objects::<GridPosition>, position_objects::<GridBox>)
					.before(sort_bounded_objects_by_z),
			)
			.add_systems(PostUpdate, sort_bounded_objects_by_z);
	}
}

/// Static, unchanging sprite.
#[derive(Bundle, Default)]
pub struct StaticSprite {
	// Types enforced by Bevy so that the sprite renders. Don’t modify those manually!
	pub(crate) bevy_sprite: SpriteBundle,
}

/// Sprite representing a border of a larger area, such as a fence.
#[derive(Bundle)]
pub struct BorderSprite {
}

pub fn initialize_graphics(mut commands: Commands, _asset_server: Res<AssetServer>, mut msaa: ResMut<Msaa>) {
	let projection = OrthographicProjection { scale: 1. / 4., near: -100000., ..Default::default() };
	commands.spawn((Camera2dBundle { projection, ..Default::default() }, ContrastAdaptiveSharpeningSettings {
		enabled:             false,
		sharpening_strength: 0.3,
		denoise:             false,
	}));
	*msaa = Msaa::Off;
}

static TRANSFORMATION_MATRIX: OnceLock<Mat3> = OnceLock::new();

const TILE_HEIGHT: f32 = 12.;
const TILE_WIDTH: f32 = 16.;

pub fn position_objects<PositionType: WorldPosition>(
	mut entities: Query<(&mut Transform, &PositionType), Changed<PositionType>>,
) {
	TRANSFORMATION_MATRIX.get_or_init(|| {
		// Our iso grid is a simple affine transform away from the real world position.
		// We only have a small, roughly 45°-rotation to the right, then a vertical scale.
		// The exact parameters are calculated with the fact that the triangle describing a tile corner has width 8 and
		// height 6, so we know where the X and Y vectors must point exactly.
		let x_vector = ((TILE_WIDTH / 2.).round(), (TILE_HEIGHT / 2.).round() + 1., 0.).into();
		let y_vector = (-(TILE_WIDTH / 2.).round(), (TILE_HEIGHT / 2.).round() + 1., 0.).into();
		// Only map z onto the y and z axes. Applying it to z as well will make 2D z sorting work correctly.
		Mat3::from_cols(x_vector, y_vector, Vec3::Y * (TILE_HEIGHT / 4.).round() + Vec3::Z)
	});
	for entity in &mut entities {
		let (mut bevy_transform, world_position_type) = entity;
		let world_position = world_position_type.position();
		let matrix = TRANSFORMATION_MATRIX.get().cloned().unwrap();
		// The translation rounding here is about 90% of pixel-perfectness:
		// - Make sure everything is camera-space pixel aligned (this code)
		// - Make sure all sprite anchors fall on pixel corners (sprite initialization code)
		// - Make sure no sprites are scaled (sprite initialization code)
		bevy_transform.translation = (matrix * world_position).round().into();
		bevy_transform.translation.z = -world_position.x - world_position.y;
	}
}

pub fn sort_bounded_objects_by_z(
	mut independent_bounded_entities: Query<(&mut Transform, &BoundingBox), (Without<GridBox>, Changed<Transform>)>,
	mut boxed_entities: Query<(&mut Transform, &GridBox), (Without<BoundingBox>, Changed<Transform>)>,
) {
	for (mut bevy_transform, bounding_box) in &mut independent_bounded_entities {
		// Higher objects have higher priority, and objects lower on the screen also have higher priority.
		bevy_transform.translation.z += bounding_box.height() as f32;
	}
	for (mut bevy_transform, grid_box) in &mut boxed_entities {
		bevy_transform.translation.z += grid_box.height() as f32;
	}
}

/// Translates from a screen pixel position back to orld space. Note that z needs to be provided and generally
/// depends on the surface at the specific location.
pub fn screen_to_world_space(screen_position: Vec2, z: f32) -> ActorPosition {
	// The matrix is invertible, since we keep the z dimension when using it normally, so we can make use of that by
	// synthetically re-inserting the z coordinate into the 2D screen position and getting a precise inverse transform
	// for free.
	let matrix = TRANSFORMATION_MATRIX.get().unwrap().inverse();
	let screen_space_with_synthetic_z: Vec3 = (screen_position, z).into();
	// The z coordinate here is garbage; discard it and replace it with the given one.
	let mut world_space = matrix * screen_space_with_synthetic_z;
	world_space.z = z;
	ActorPosition(world_space.into())
}
