use bevy::prelude::*;

use crate::geometry::{FixedBox, GridPosition};
use crate::graphics::library::sprite_for_kind;
use crate::graphics::StaticSprite;

/// The kinds of ground that exist; most have their own graphics.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GroundKind {
	/// Grass is the default ground and considered walkable, but not very fast.
	Grass,
	/// Pathways increase walking speed and allow vehicles to traverse it.
	Pathway,
	/// Pool paths are similar to pathways, but they can only be placed in pool areas. They serve as a basement for
	/// most objects that can only be placed inside a pool area, like pools themselves.
	PoolPath,
}

impl Default for GroundKind {
	fn default() -> Self {
		Self::Grass
	}
}

/// A single tile on the ground defining its size.
#[derive(Bundle, Default)]
pub struct GroundTile {
	position: GridPosition,
	bounds:   FixedBox<1, 1, 0>,
	sprite:   StaticSprite,
	kind:     GroundKind,
}

impl GroundTile {
	fn new(kind: GroundKind, position: GridPosition, asset_server: &Res<AssetServer>) -> Self {
		GroundTile {
			position,
			sprite: StaticSprite {
				bevy_sprite: SpriteBundle {
					sprite: Sprite {
						anchor: bevy::sprite::Anchor::Center,
						// flip_y: ((position.0.x % 5) >= (position.0.y % 7)) ^ (position.0.z % 3 == 0),
						..Default::default()
					},
					texture: asset_server.load(sprite_for_kind(kind)),
					..Default::default()
				},
			},
			kind,
			..Default::default()
		}
	}
}

// For testing purposes:

pub fn spawn_test_tiles(mut commands: Commands, asset_server: Res<AssetServer>) {
	for x in -100i32 .. 100 {
		for y in -100i32 .. 100 {
			let kind = if x.abs() < 2 || y.abs() < 2 {
				GroundKind::Pathway
			} else if x > 10 {
				GroundKind::PoolPath
			} else {
				GroundKind::Grass
			};
			commands.spawn(GroundTile::new(kind, (x, y, 0).into(), &asset_server));
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
