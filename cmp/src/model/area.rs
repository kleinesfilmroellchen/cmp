use std::collections::VecDeque;

use bevy::color::palettes::css::RED;
use bevy::prelude::*;
use bevy::utils::Instant;
use itertools::Itertools;
use moonshine_save::save::Save;

use super::{BoundingBox, GridBox, GridPosition, GroundKind, GroundMap, Pitch};
use crate::config::GameSettings;
use crate::gamemode::GameState;
use crate::graphics::{BorderSprite, BorderTextures, ObjectPriority, Sides};
use crate::ui::world_info::WorldInfoProperties;
use crate::HashSet;

/// A continuous area on the ground, containing various tiles (often of a homogenous type) and demarcating some
/// important region. For example, pools and pitches are fundamentally areas.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct Area {
	tiles: HashSet<GridPosition>,
	// A bounding box for intersection acceleration.
	aabb:  GridBox,
}

impl Default for Area {
	fn default() -> Self {
		Self { tiles: HashSet::new(), aabb: GridBox::new(GridPosition::default(), BoundingBox::fixed::<0, 0, 0>()) }
	}
}

impl Area {
	pub fn from_rect(first_corner: GridPosition, second_corner: GridPosition) -> Self {
		let smallest_corner = first_corner.component_wise_min(second_corner);
		let largest_corner = first_corner.component_wise_max(second_corner);
		let mut aabb = GridBox::from_corners(smallest_corner, largest_corner);
		let tiles = (aabb.smallest().x ..= aabb.largest().x)
			.cartesian_product(aabb.smallest().y ..= aabb.largest().y)
			.map(GridPosition::from)
			.map(|x| (x, ()))
			.collect();
		aabb.enlargen((1, 1, 1).into());
		Self { tiles, aabb }
	}

	pub fn recompute_bounds(&mut self) {
		let (smallest_x, largest_x) = self.tiles.keys().map(|tile| tile.x).minmax().into_option().unwrap_or((0, 0));
		let (smallest_y, largest_y) = self.tiles.keys().map(|tile| tile.y).minmax().into_option().unwrap_or((0, 0));
		self.aabb = GridBox::from_corners((smallest_x, smallest_y, 0).into(), (largest_x + 1, largest_y + 1, 1).into());
	}

	pub fn retain_tiles(&mut self, predicate: impl Fn(&GridPosition) -> bool) {
		self.tiles.retain(|x, _| predicate(x));
		self.recompute_bounds();
	}

	#[allow(unused)]
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.tiles.is_empty()
	}

	pub fn is_discontinuous(&self) -> bool {
		if self.is_empty() {
			return true;
		}
		// Flood fill to determine continuity.
		let mut candidate_tiles = self.tiles.clone();
		let mut nearby_tiles = VecDeque::new();
		nearby_tiles.push_back(*candidate_tiles.keys().next().unwrap());
		while !nearby_tiles.is_empty() {
			let current_tile = nearby_tiles.pop_front().unwrap();
			for neighbor in current_tile
				.neighbors()
				.into_iter()
				.filter(|neighbor| candidate_tiles.contains_key(neighbor))
				.collect_vec()
			{
				nearby_tiles.push_back(neighbor);
				candidate_tiles.remove(&neighbor);
			}
		}
		// If candidates remain, we have a discontinuity.
		!candidate_tiles.is_empty()
	}

	#[inline]
	pub fn size(&self) -> usize {
		self.tiles.len()
	}

	#[inline]
	pub fn contains(&self, position: &GridPosition) -> bool {
		self.tiles.contains_key(position)
	}

	pub fn fits(&self, aabb: &GridBox) -> bool {
		aabb.floor_positions().all(|grid_position| self.contains(&grid_position))
	}

	#[inline]
	pub fn tiles_iter(&self) -> impl Iterator<Item = GridPosition> + '_ {
		self.tiles.keys().copied()
	}

	pub fn instantiate_borders(
		&self,
		ground_map: &GroundMap,
		commands: &mut Commands,
		asset_server: &AssetServer,
		texture_atlases: &mut Assets<TextureAtlasLayout>,
		border_textures: &mut BorderTextures,
	) {
		for position in self.tiles.keys() {
			if let Some((entity, kind)) = ground_map.get(position) {
				if let Some(border_kind) = kind.border_kind() {
					let mut sides = Sides::all();
					for neighbor in position.neighbors().into_iter().filter(|neighbor| {
						self.tiles.contains_key(neighbor)
							&& ground_map.kind_of(neighbor).is_some_and(|neighbor_kind| neighbor_kind == kind)
					}) {
						sides ^= match *(neighbor - *position) {
							IVec3::X => Sides::Right,
							IVec3::NEG_X => Sides::Left,
							IVec3::Y => Sides::Top,
							IVec3::NEG_Y => Sides::Bottom,
							_ => unreachable!(),
						};
					}
					let borders = BorderSprite::new(sides, border_kind, asset_server, texture_atlases, border_textures);
					commands.entity(entity).despawn_descendants().with_children(|tile_parent| {
						for border in borders {
							tile_parent.spawn(border);
						}
					});
				}
			}
		}
	}
}

/// Stores an area's data, but makes it not participate in area combination anymore.
#[derive(Component, Reflect, Debug, Deref, DerefMut)]
#[reflect(Component)]
pub struct ImmutableArea(pub Area);

impl From<ImmutableArea> for Area {
	fn from(value: ImmutableArea) -> Self {
		value.0
	}
}

/// A marker component used with the [`Area`] component to mark the area of a specific type and to determine some
/// type-specific area properties.
pub trait AreaMarker: Component {
	fn is_allowed_ground_type(&self, kind: GroundKind) -> bool;
	fn init_new(area: Area, commands: &mut Commands);
}

/// Marker for pool areas.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Pool;

impl AreaMarker for Pool {
	fn is_allowed_ground_type(&self, kind: GroundKind) -> bool {
		kind == GroundKind::PoolPath
	}

	fn init_new(area: Area, commands: &mut Commands) {
		commands.spawn((area, Pool, Save));
	}
}

pub struct AreaManagement;

impl Plugin for AreaManagement {
	fn build(&self, app: &mut App) {
		// Add event resource manually to circumvent automatic frame-wise event cleanup.
		app.init_resource::<Events<UpdateAreas>>()
			.register_type::<Pool>()
			.register_type::<DebugAreaText>()
			.register_type::<Area>()
			.register_type::<ImmutableArea>()
			.add_systems(
				FixedUpdate,
				(update_areas::<Pool>, update_areas::<Pitch>)
					.before(clean_area_events)
					.before(update_area_world_info)
					.run_if(in_state(GameState::InGame)),
			)
			.add_systems(FixedUpdate, (clean_area_events, update_area_world_info).run_if(in_state(GameState::InGame)))
			.add_systems(Update, (add_area_world_info, add_area_transforms).run_if(in_state(GameState::InGame)));
	}
}

#[derive(Event, Default)]
pub struct UpdateAreas;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct DebugAreaText;

fn update_areas<T: AreaMarker + Default>(
	tiles: Res<GroundMap>,
	mut areas: Query<(Entity, &mut Area, &T)>,
	mut commands: Commands,
	update: Res<Events<UpdateAreas>>,
	old_area_markers: Query<Entity, With<DebugAreaText>>,
	// debugging
	asset_server: Res<AssetServer>,
	settings: Res<GameSettings>,
) {
	let start = Instant::now();
	if update.is_empty() {
		return;
	}

	old_area_markers.iter().for_each(|x| commands.entity(x).despawn());

	// Perform flood fill on the areas to update them.
	let mut remaining_tiles = HashSet::<GridPosition>::new();
	for (_, area, marker) in &areas {
		remaining_tiles.extend(
			area.tiles
				.iter()
				.filter(|(tile, _)| tiles.kind_of(tile).is_some_and(|kind| marker.is_allowed_ground_type(kind))),
		);
	}

	let mut new_areas = Vec::new();
	let mut active_area = Area::default();
	let mut adjacent_tiles = VecDeque::new();
	if !remaining_tiles.is_empty() {
		adjacent_tiles.push_front(*remaining_tiles.keys().next().unwrap());
	}
	while !remaining_tiles.is_empty() {
		// No more adjacent tiles; start new area.
		if adjacent_tiles.is_empty() {
			active_area.recompute_bounds();
			new_areas.push(active_area);
			active_area = Area::default();
			// Extract an arbitrary new tile to start the next area.
			adjacent_tiles.push_front(*remaining_tiles.keys().next().unwrap());
		}
		let next_tile = adjacent_tiles.pop_back().unwrap();

		let did_remove = remaining_tiles.remove(&next_tile).is_some();
		if !did_remove {
			debug!("BUG! {:?} wasn’t a remaining tile, but it was in the queue!", next_tile);
		}

		active_area.tiles.insert(next_tile, ());
		for new_tile in next_tile.neighbors() {
			// Not a queued tile already, but we need to handle it.
			if !adjacent_tiles.contains(&new_tile) && remaining_tiles.contains_key(&new_tile) {
				adjacent_tiles.push_front(new_tile);
			}
		}
	}
	active_area.recompute_bounds();
	new_areas.push(active_area);
	let computation_time = Instant::now() - start;

	debug!("after unification, {} areas remain (in {:?})", new_areas.len(), computation_time);

	// debugging
	if settings.show_debug {
		for (i, area) in new_areas.iter().enumerate() {
			for tile in area.tiles.keys() {
				commands.spawn((
					*tile + IVec3::new(0, 0, 3),
					Text2dBundle {
						text: Text::from_section(format!("{}", i), TextStyle {
							font:      asset_server.load(crate::graphics::library::font_for(
								crate::graphics::library::FontWeight::Regular,
								crate::graphics::library::FontStyle::Regular,
							)),
							font_size: 16.,
							color:     RED.into(),
						}),
						text_anchor: bevy::sprite::Anchor::BottomCenter,
						visibility: Visibility::Visible,
						..default()
					},
					DebugAreaText,
					ObjectPriority::Overlay,
				));
			}
		}
	}

	for result in new_areas.into_iter().zip_longest(areas.iter_mut()) {
		match result {
			itertools::EitherOrBoth::Both(new, (old_entity, mut old_area, _)) => {
				*old_area = new;
				commands.entity(old_entity).despawn_descendants();
			},
			itertools::EitherOrBoth::Left(new) => {
				T::init_new(new, &mut commands);
			},
			itertools::EitherOrBoth::Right((old_entity, ..)) => {
				commands.entity(old_entity).despawn_recursive();
			},
		}
	}
}

fn clean_area_events(mut update: ResMut<Events<UpdateAreas>>) {
	update.clear();
}

fn add_area_world_info(
	finalized_pitches: Query<Entity, (Without<Area>, With<ImmutableArea>, Without<WorldInfoProperties>)>,
	unfinalized_pitches: Query<Entity, (Without<ImmutableArea>, With<Area>, Without<WorldInfoProperties>)>,
	mut commands: Commands,
) {
	for entity in unfinalized_pitches.iter().chain(finalized_pitches.iter()) {
		commands.entity(entity).insert(WorldInfoProperties::basic(String::new(), String::new()));
	}
}

fn add_area_transforms(
	area: Query<Entity, (Or<(With<Area>, With<ImmutableArea>)>, Without<GlobalTransform>)>,
	mut commands: Commands,
) {
	for entity in &area {
		commands.entity(entity).insert((GlobalTransform::default(), Transform::default(), VisibilityBundle::default()));
	}
}

fn update_area_world_info(
	finalized_pitches: Query<(&WorldInfoProperties, &ImmutableArea), (Without<Area>, Changed<WorldInfoProperties>)>,
	unfinalized_pitches: Query<(&WorldInfoProperties, &Area), (Without<ImmutableArea>, Changed<WorldInfoProperties>)>,
	ground_map: Res<GroundMap>,
	mut tiles: Query<&mut WorldInfoProperties, (With<GroundKind>, Without<ImmutableArea>, Without<Area>)>,
) {
	for (properties, area) in
		unfinalized_pitches.iter().chain(finalized_pitches.iter().map(|(properties, area)| (properties, &area.0)))
	{
		for tile in area.tiles_iter() {
			let (tile_entity, _) = ground_map.get(&tile).unwrap();
			if let Ok(mut tile_properties) = tiles.get_mut(tile_entity) {
				*tile_properties = properties.clone();
			}
		}
	}
}
