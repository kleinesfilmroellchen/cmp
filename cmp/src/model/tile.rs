use bevy::prelude::*;

use super::{FixedBox, GridPosition};
use crate::graphics::library::sprite_for_kind;
use crate::graphics::StaticSprite;
use crate::util::Tooltipable;

/// The kinds of ground that exist; most have their own graphics.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GroundKind {
	Grass,
	Pathway,
	PoolPath,
}

impl Default for GroundKind {
	fn default() -> Self {
		Self::Grass
	}
}

impl std::fmt::Display for GroundKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::Grass => "Grass",
			Self::Pathway => "Pathway",
			Self::PoolPath => "Pool Path",
		})
	}
}

impl Tooltipable for GroundKind {
	fn description(&self) -> &'static str {
		match self {
			Self::Grass => "Grass is the default ground. Everyone can walk here, but not very fast.",
			Self::Pathway => "Pathways increase walking speed and allow vehicles to traverse the site.",
			Self::PoolPath =>
				"Pool paths are similar to pathways, but they instead serve as the floor material of all pools. You \
				 can therefore easily identify a pool area by this flooring.",
		}
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

// pub fn wave_tiles(time: Res<Time>, mut tiles: Query<&mut GridPosition, With<FixedBox<1, 1, 0>>>) {
// 	for mut tile in &mut tiles {
// 		let position = &mut tile.0;
// 		position.z =
// 			((time.elapsed_seconds() + position.x as f32 / 2. + position.y as f32 / 3.).sin() * 3f32).round() as i32;
// 	}
// }
