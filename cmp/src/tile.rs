use bevy::prelude::*;

use crate::geometry::{FixedBox, GridPosition};
use crate::graphics::StaticSprite;

/// A single tile on the ground defining its size.
#[derive(Bundle, Default)]
pub struct GroundTile {
	position: GridPosition,
	bounds:   FixedBox<1, 1, 0>,
	sprite:   StaticSprite,
}

// For testing purposes:

pub fn spawn_test_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
	let texture = asset_server.load("grass.png");
	for x in -100 .. 100 {
		for y in -100 .. 100 {
			let sprite = StaticSprite { bevy_sprite: SpriteBundle { texture: texture.clone(), ..Default::default() } };
			commands.spawn(GroundTile { position: (x, y, 0).into(), sprite, ..Default::default() });
		}
	}
}

pub fn wave_tiles(time: Res<Time>, mut tiles: Query<&mut GridPosition, With<FixedBox<1, 1, 0>>>) {
	for mut tile in &mut tiles {
		let position = &mut tile.0;
		position.z =
			((time.elapsed_seconds() + position.x as f32 / 2. + position.y as f32 / 3.).sin() * 3f32).round() as i32;
	}
}
