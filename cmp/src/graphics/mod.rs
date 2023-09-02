use std::sync::OnceLock;

use bevy::prelude::*;

use crate::geometry::{GridPosition, WorldPosition};

/// Static, unchanging sprite.
#[derive(Bundle, Default)]
pub struct StaticSprite {
	// Types enforced by Bevy so that the sprite renders. Don’t modify those manually!
	pub(crate) bevy_sprite: SpriteBundle,
}

pub fn initialize_graphics(mut commands: Commands, _asset_server: Res<AssetServer>) {
	let projection = OrthographicProjection { scale: 1. / 4., near: -100000., ..Default::default() };
	commands.spawn(Camera2dBundle { projection, ..Default::default() });
}

static TRANSFORMATION_MATRIX: OnceLock<Mat3> = OnceLock::new();

const TILE_HEIGHT: f32 = 12.;
const TILE_WIDTH: f32 = 16.;

pub fn position_objects<PositionType: WorldPosition>(mut entities: Query<(&mut Transform, &PositionType)>) {
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
		bevy_transform.translation = (matrix * world_position).round().into();
	}
}

/// Translates from a screen pixel position back to discrete world space. Note that z needs to be provided and generally
/// depends on the surface at the specific location.
pub fn screen_to_discrete_world_space(screen_position: Vec2, z: i32) -> GridPosition {
	// The matrix is invertible, since we keep the z dimension when using it normally, so we can make use of that by
	// synthetically re-inserting the z coordinate into the 2D screen position and getting a precise inverse transform
	// for free.
	let matrix = TRANSFORMATION_MATRIX.get().unwrap().inverse();
	let screen_space_with_synthetic_z: Vec3 = (screen_position, z as f32).into();
	GridPosition((matrix * screen_space_with_synthetic_z).round().as_ivec3())
}
