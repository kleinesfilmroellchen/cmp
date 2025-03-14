use std::marker::ConstParamTy;

use bevy::prelude::*;
use bevy::utils::HashMap;
use moonshine_save::save::Save;

use super::nav::{NavCategory, NavComponent};
use super::GridPosition;
use crate::gamemode::GameState;
use crate::graphics::library::{anchor_for_image, image_for_ground};
use crate::graphics::{BorderKind, ObjectPriority, Sides};
use crate::ui::world_info::WorldInfoProperties;
use crate::util::Tooltipable;

pub struct TileManagement;

impl Plugin for TileManagement {
	fn build(&self, app: &mut App) {
		app.register_type::<GroundKind>()
			.insert_resource(GroundMap::new())
			.add_systems(PreUpdate, update_map_from_world.run_if(in_state(GameState::InGame)))
			.add_systems(
				PostUpdate,
				(update_ground_textures, add_ground_textures, add_world_info).run_if(in_state(GameState::InGame)),
			)
			// .add_systems(Update, resize_tiles)
			.add_systems(
				FixedUpdate,
				(add_navigability.after(update_navigability_properties), update_navigability_properties)
					.run_if(in_state(GameState::InGame)),
			);
	}
}

/// The kinds of ground that exist; most have their own graphics.
#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
#[reflect(Component)]
pub enum GroundKind {
	Grass,
	Pathway,
	PoolPath,
	Pitch,
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
			Self::Pitch => "Pitch",
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
			Self::Pitch =>
				"Pitch ground looks like grass, but behaves very differently, since it defines where a pitch is \
				 situated.",
		}
	}
}

impl GroundKind {
	pub const fn border_kind(&self) -> Option<BorderKind> {
		match self {
			Self::Pitch => Some(BorderKind::Pitch),
			Self::Grass | Self::Pathway | Self::PoolPath => None,
		}
	}

	pub const fn navigability(&self) -> NavCategory {
		match self {
			Self::Grass | Self::PoolPath => NavCategory::People,
			Self::Pathway => NavCategory::Vehicles,
			Self::Pitch => NavCategory::None,
		}
	}

	/// The traversal speed needed for this ground type. The speed is given in tiles/second (i/s²) for a person.
	pub const fn traversal_speed(&self) -> u32 {
		match self {
			Self::Grass | Self::Pitch => 1,
			Self::Pathway => 2,
			Self::PoolPath => 1,
		}
	}
}

/// A single tile on the ground defining its size.
#[derive(Bundle)]
pub struct GroundTile {
	position:   GridPosition,
	priority:   ObjectPriority,
	sprite:     Sprite,
	kind:       GroundKind,
	world_info: WorldInfoProperties,
	navigable:  NavComponent,
	save:       Save,
}

fn sprite_object_for_image(image: &str, asset_server: &AssetServer) -> Sprite {
	Sprite {
		anchor: anchor_for_image(image),
		image: asset_server.load(image),
		// flip_x: ((position.x % 5) ^ (position.y % 7) ^ (position.z % 11)) & (1 << 3) == 0,
		..Default::default()
	}
}

// pub const VERTICAL_FIX_EPSILON: f32 = 0.04;

// fn resize_tiles(images: Res<Assets<Image>>, mut tiles: Query<&mut Sprite, With<GroundKind>>) {
// 	for mut relevant_tile in tiles.iter_mut().filter(|t| t.custom_size.is_none()) {
// 		relevant_tile.custom_size =
// 			images.get(&relevant_tile.image).map(|img| img.size_f32() * Vec2::new(1.0, 1.0 + VERTICAL_FIX_EPSILON));
// 	}
// }

impl GroundTile {
	pub fn new(kind: GroundKind, position: GridPosition, asset_server: &AssetServer) -> Self {
		let image = image_for_ground(kind);
		GroundTile {
			position,
			sprite: sprite_object_for_image(image, asset_server),
			priority: ObjectPriority::Ground,
			kind,
			world_info: WorldInfoProperties::basic(kind.to_string(), kind.description().to_string()),
			navigable: NavComponent {
				exits:        Sides::all(),
				speed:        kind.traversal_speed(),
				navigability: kind.navigability(),
			},
			save: Save,
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
		tile_query: &mut Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
		commands: &mut Commands,
		asset_server: &AssetServer,
	) {
		self.set_impl(position, kind, tile_query, commands, asset_server);
	}

	fn set_impl(
		&mut self,
		position: GridPosition,
		kind: GroundKind,
		tile_query: &mut Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
		commands: &mut Commands,
		asset_server: &AssetServer,
	) {
		if let Some((responsible_entity, old_kind)) = self.map.get_mut(&position) {
			let (_, _, mut in_world_kind, mut world_info) = tile_query.get_mut(*responsible_entity).unwrap();
			// Avoid mutation if there is no change, reducing the pressure on update_ground_textures
			in_world_kind.set_if_neq(kind);
			*world_info = WorldInfoProperties::basic(kind.to_string(), kind.description().to_string());
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
		tile_query: &mut Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
		commands: &mut Commands,
		asset_server: &AssetServer,
	) {
		let smaller_corner = start_position.component_wise_min(end_position);
		let larger_corner = start_position.component_wise_max(end_position);
		for x in smaller_corner.x ..= larger_corner.x {
			for y in smaller_corner.y ..= larger_corner.y {
				let position = (x, y, start_position.z).into();
				self.set_impl(position, kind, tile_query, commands, asset_server);
			}
		}
	}

	pub fn kind_of(&self, position: &GridPosition) -> Option<GroundKind> {
		self.map.get(position).map(|(_, kind)| *kind)
	}

	pub fn get(&self, position: &GridPosition) -> Option<(Entity, GroundKind)> {
		self.map.get(position).cloned()
	}

	/// Enter an existing tile into the ground map. This is only to be used with already correctly set up tiles (from a
	/// game load), and not for entering tile changes and additions into the map.
	pub(super) fn update_with_existing_tile(&mut self, entity: Entity, position: GridPosition, kind: GroundKind) {
		self.map.insert(position, (entity, kind));
	}
}

fn update_map_from_world(
	new_entries: Query<(Entity, &GridPosition, &GroundKind), Added<GroundKind>>,
	mut map: ResMut<GroundMap>,
) {
	for (entity, position, kind) in &new_entries {
		map.update_with_existing_tile(entity, *position, *kind);
	}
}

// For testing purposes:

pub fn spawn_test_tiles(
	mut commands: Commands,
	mut tile_query: Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
	mut map: ResMut<GroundMap>,
	asset_server: Res<AssetServer>,
) {
	for x in -100i32 .. 100 {
		for y in -100i32 .. 100 {
			let kind = if x.abs() < 2 || y.abs() < 2 { GroundKind::Pathway } else { GroundKind::Grass };
			map.set((x, y, 0).into(), kind, &mut tile_query, &mut commands, &asset_server);
		}
	}
}

pub fn update_ground_textures(
	mut ground_textures: Query<(Entity, &GroundKind, &mut Sprite), Changed<GroundKind>>,
	asset_server: Res<AssetServer>,
	mut commands: Commands,
) {
	for (entity, kind, mut sprite) in &mut ground_textures {
		// remove any children of the old tile
		commands.entity(entity).despawn_descendants();
		let image = image_for_ground(*kind);
		sprite.image = asset_server.load(image);
	}
}

pub fn add_ground_textures(
	mut ground_textures: Query<(Entity, &GroundKind), Without<Sprite>>,
	asset_server: Res<AssetServer>,
	mut commands: Commands,
) {
	for (entity, kind) in &mut ground_textures {
		let image = image_for_ground(*kind);
		let sprite = sprite_object_for_image(image, &asset_server);
		commands.entity(entity).insert(sprite);
	}
}

fn add_navigability(mut ground_vertices: Query<(Entity, &GroundKind), Without<NavComponent>>, mut commands: Commands) {
	for (entity, kind) in &mut ground_vertices {
		commands.entity(entity).insert(NavComponent {
			navigability: kind.navigability(),
			exits:        Sides::all(),
			speed:        kind.traversal_speed(),
		});
	}
}

fn update_navigability_properties(mut ground_vertices: Query<(&GroundKind, &mut NavComponent), Changed<GroundKind>>) {
	for (kind, mut vertex) in &mut ground_vertices {
		vertex.navigability = kind.navigability();
		// TODO: Check border objects in another system and remove sides with borders.
		vertex.exits = Sides::all();
		vertex.speed = kind.traversal_speed();
	}
}

fn add_world_info(mut commands: Commands, ground_vertices: Query<(Entity, &GroundKind), Without<WorldInfoProperties>>) {
	for (entity, kind) in &ground_vertices {
		commands.entity(entity).insert(WorldInfoProperties::basic(kind.to_string(), kind.description().to_string()));
	}
}
