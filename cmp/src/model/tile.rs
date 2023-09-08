use std::marker::ConstParamTy;

use bevy::prelude::*;
use bevy::utils::HashMap;

use super::{BoundingBox, GridPosition};
use crate::graphics::library::{anchor_for_sprite, sprite_for_ground};
use crate::graphics::StaticSprite;
use crate::util::Tooltipable;

pub struct TileManagement;

impl Plugin for TileManagement {
	fn build(&self, app: &mut App) {
		app.insert_resource(GroundMap::new()).add_systems(PostUpdate, update_ground_textures);
	}
}

/// The kinds of ground that exist; most have their own graphics.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
pub enum GroundKind {
	Grass,
	Pathway,
	PoolPath,
	/// Pitch surface.
	Accommodation,
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
			Self::Accommodation => "Accommodation",
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
			Self::Accommodation =>
				"Accommodation ground looks like grass, but behaves very differently, since it defines where an \
				 accommodation is situated.",
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

/// A map of all ground tiles for fast access.
#[derive(Resource)]
pub struct GroundMap {
	map: HashMap<GridPosition, (Entity, GroundKind)>,
}

impl GroundMap {
	pub fn new() -> Self {
		Self { map: HashMap::new() }
	}

	pub fn set(
		&mut self,
		position: GridPosition,
		kind: GroundKind,
		tile_query: &mut Query<(Entity, &GridPosition, &mut GroundKind)>,
		commands: &mut Commands,
		asset_server: &AssetServer,
	) {
		if let Some((responsible_entity, old_kind)) = self.map.get_mut(&position) {
			let (_, _, mut in_world_kind) = tile_query.get_mut(*responsible_entity).unwrap();
			// Avoid mutation if there is no change, reducing the pressure on update_ground_textures
			in_world_kind.set_if_neq(kind);
			*old_kind = kind;
		} else {
			let new_entity = commands.spawn(GroundTile::new(kind, position, asset_server)).id();
			self.map.entry(position).insert((new_entity, kind));
		}
	}

	pub fn fill_rect(
		&mut self,
		start_position: GridPosition,
		end_position: GridPosition,
		kind: GroundKind,
		tile_query: &mut Query<(Entity, &GridPosition, &mut GroundKind)>,
		commands: &mut Commands,
		asset_server: &AssetServer,
	) {
		let smaller_corner = start_position.min(*end_position);
		let larger_corner = start_position.max(*end_position);
		for x in smaller_corner.x ..= larger_corner.x {
			for y in smaller_corner.y ..= larger_corner.y {
				self.set((x, y, start_position.z).into(), kind, tile_query, commands, asset_server);
			}
		}
	}

	pub fn kind_of(&self, position: &GridPosition) -> Option<GroundKind> {
		self.map.get(position).map(|(_, kind)| *kind)
	}
}

// For testing purposes:

pub fn spawn_test_tiles(
	mut commands: Commands,
	mut tile_query: Query<(Entity, &GridPosition, &mut GroundKind)>,
	mut map: ResMut<GroundMap>,
	asset_server: Res<AssetServer>,
) {
	for x in -100i32 .. 100 {
		for y in -100i32 .. 100 {
			let kind = if x.abs() < 2 || y.abs() < 2 {
				GroundKind::Pathway
			} else if x > 60 && y > 60 {
				GroundKind::PoolPath
			} else {
				GroundKind::Grass
			};
			map.set((x, y, 0).into(), kind, &mut tile_query, &mut commands, &asset_server);
		}
	}
}

pub fn update_ground_textures(
	mut ground_textures: Query<(&GroundKind, &mut Handle<Image>), Changed<GroundKind>>,
	asset_server: Res<AssetServer>,
) {
	for (kind, mut texture) in &mut ground_textures {
		let sprite = sprite_for_ground(*kind);
		*texture = asset_server.load(sprite);
	}
}
