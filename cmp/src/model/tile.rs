use bevy::prelude::*;

use super::{BoundingBox, GridPosition};
use crate::graphics::library::{anchor_for_sprite, sprite_for_ground};
use crate::graphics::StaticSprite;
use crate::util::Tooltipable;

/// The kinds of ground that exist; most have their own graphics.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Bundle)]
pub struct GroundTile {
	position: GridPosition,
	bounds:   BoundingBox,
	sprite:   StaticSprite,
	kind:     GroundKind,
}

impl GroundTile {
	pub fn new(kind: GroundKind, position: GridPosition, asset_server: &AssetServer) -> Self {
		let sprite = sprite_for_ground(kind);
		GroundTile {
			position,
			sprite: StaticSprite {
				bevy_sprite: SpriteBundle {
					sprite: Sprite {
						anchor: anchor_for_sprite(sprite),
						// flip_y: ((position.0.x % 5) >= (position.0.y % 7)) ^ (position.0.z % 3 == 0),
						..Default::default()
					},
					texture: asset_server.load(sprite),
					..Default::default()
				},
			},
			kind,
			bounds: BoundingBox::fixed::<1, 1, 0>(),
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

#[derive(Component)]
pub struct NewGroundTile;
/// Various external processes invoke this event to clean up the state of the ground tiles. All tiles that overlap other
/// tiles marked with [`NewGroundTile`] are deleted. The marker is removed so that these new tiles now become regular,
/// old tiles.
#[derive(Event, Default)]
pub struct GroundTileCleanupNeeded;

pub fn cleanup_ground_tiles(
	mut event: EventReader<GroundTileCleanupNeeded>,
	old_tiles: Query<(Entity, &GridPosition), Without<NewGroundTile>>,
	new_tiles: Query<(Entity, &GridPosition), With<NewGroundTile>>,
	mut commands: Commands,
) {
	for _ in &mut event {
		for (new_tile, new_tile_position) in &new_tiles {
			for (old_tile, old_tile_position) in &old_tiles {
				if old_tile_position == new_tile_position {
					commands.entity(old_tile).despawn_recursive();
				}
			}
			commands.entity(new_tile).remove::<NewGroundTile>();
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
