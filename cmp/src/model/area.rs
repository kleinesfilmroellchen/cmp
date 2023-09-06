use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::utils::{HashSet, Instant};
use itertools::Itertools;

use super::{BoundingBox, GridBox, GridPosition, GroundKind, GroundMap};

/// A continuous area on the ground, containing various tiles (often of a homogenous type) and demarcating some
/// important region. For example, pools and accommodations are fundamentally areas.
#[derive(Component, Clone, Debug)]
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
		let mut aabb = GridBox::from_corners(first_corner, second_corner);
		let tiles = (aabb.smallest().x ..= aabb.largest().x)
			.cartesian_product(aabb.smallest().y ..= aabb.largest().y)
			.map(GridPosition::from)
			.collect();
		aabb.enlargen((1, 1, 0).into());
		Self { tiles, aabb }
	}

	fn recompute_bounds(&mut self) {
		let (smallest_x, largest_x) = self.tiles.iter().map(|tile| tile.x).minmax().into_option().unwrap_or((0, 0));
		let (smallest_y, largest_y) = self.tiles.iter().map(|tile| tile.y).minmax().into_option().unwrap_or((0, 0));
		self.aabb = GridBox::from_corners((smallest_x, smallest_y, 0).into(), (largest_x + 1, largest_y + 1, 0).into());
	}

	#[allow(unused)]
	fn is_empty(&self) -> bool {
		self.tiles.is_empty()
	}

	#[allow(unused)]
	fn contains(&self, position: &GridPosition) -> bool {
		self.tiles.contains(position)
	}
}

/// A marker component used with the [`Area`] component to mark the area of a specific type and to determine some
/// type-specific area properties.
trait AreaMarker: Component {
	fn is_allowed_ground_type(&self, kind: GroundKind) -> bool;
}

/// Marker for pool areas.
#[derive(Component, Default)]
pub struct Pool;

impl AreaMarker for Pool {
	fn is_allowed_ground_type(&self, kind: GroundKind) -> bool {
		kind == GroundKind::PoolPath
	}
}

pub struct AreaManagement;

impl Plugin for AreaManagement {
	fn build(&self, app: &mut App) {
		// Add event resource manually to circumvent automatic frame-wise event cleanup.
		app.init_resource::<Events<UpdateAreas>>().add_systems(FixedUpdate, update_areas::<Pool>);
	}
}

#[derive(Event, Default)]
pub struct UpdateAreas;

#[derive(Component)]
pub struct DebugAreaText;

fn update_areas<T: AreaMarker + Default>(
	tiles: Res<GroundMap>,
	mut areas: Query<(Entity, &mut Area, &T)>,
	mut commands: Commands,
	mut update: ResMut<Events<UpdateAreas>>,
	old_area_markers: Query<Entity, With<DebugAreaText>>,
	// debugging
	// asset_server: Res<AssetServer>,
) {
	let start = Instant::now();
	if update.is_empty() {
		return;
	}
	update.clear();

	old_area_markers.for_each(|x| commands.entity(x).despawn());

	// Perform flood fill on the areas to update them.
	let mut remaining_tiles = HashSet::<GridPosition>::new();
	for (_, area, marker) in &areas {
		remaining_tiles.extend(
			area.tiles
				.iter()
				.filter(|tile| tiles.kind_of(tile).is_some_and(|kind| marker.is_allowed_ground_type(kind))),
		);
	}

	let mut new_areas = Vec::new();
	let mut active_area = Area::default();
	let mut adjacent_tiles = VecDeque::new();
	adjacent_tiles.push_front(*remaining_tiles.iter().next().unwrap());
	while !remaining_tiles.is_empty() {
		// No more adjacent tiles; start new area.
		if adjacent_tiles.is_empty() {
			active_area.recompute_bounds();
			new_areas.push(active_area);
			active_area = Area::default();
			// Extract an arbitrary new tile to start the next area.
			adjacent_tiles.push_front(*remaining_tiles.iter().next().unwrap());
		}
		let next_tile = adjacent_tiles.pop_back().unwrap();
		let did_remove = remaining_tiles.remove(&next_tile);
		#[cfg(debug_assertions)]
		if !did_remove {
			debug!("BUG! {:?} wasn’t a remaining tile, but it was in the queue!", next_tile);
		}
		active_area.tiles.insert(next_tile);
		for delta in [(-1, 0), (1, 0), (0, 1), (0, -1)] {
			let new_tile = next_tile + IVec2::from(delta);
			// Not a queued tile already, but we need to handle it.
			if !adjacent_tiles.contains(&new_tile) && remaining_tiles.contains(&new_tile) {
				adjacent_tiles.push_front(new_tile);
			}
		}
	}
	new_areas.push(active_area);
	let computation_time = Instant::now() - start;

	debug!("after area unification, {} areas remain (in {:?})", new_areas.len(), computation_time);

	// debugging
	// for (i, area) in new_areas.iter().enumerate() {
	// 	for tile in &area.tiles {
	// 		commands.spawn((
	// 			*tile + IVec3::new(0, 0, 3),
	// 			Text2dBundle {
	// 				text: Text::from_section(format!("{}", i), TextStyle {
	// 					font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
	// 					font_size: 16.,
	// 					color:     Color::RED,
	// 				}),
	// 				text_anchor: bevy::sprite::Anchor::BottomCenter,
	// 				visibility: Visibility::Visible,
	// 				..default()
	// 			},
	// 			DebugAreaText,
	// 		));
	// 	}
	// }

	for result in new_areas.into_iter().zip_longest(areas.iter_mut()) {
		match result {
			itertools::EitherOrBoth::Both(new, (_, mut old_area, _)) => {
				*old_area = new;
			},
			itertools::EitherOrBoth::Left(new) => {
				commands.spawn((new, T::default()));
			},
			itertools::EitherOrBoth::Right((old_entity, ..)) => {
				commands.entity(old_entity).despawn();
			},
		}
	}
}
