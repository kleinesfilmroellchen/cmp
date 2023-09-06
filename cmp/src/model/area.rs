use bevy::prelude::*;
use itertools::Itertools;

use super::{BoundingBox, GridBox, GridPosition, GroundKind, GroundMap};

/// A continuous area on the ground, containing various tiles (often of a homogenous type) and demarcating some
/// important region. For example, pools and accommodations are fundamentally areas.
#[derive(Component, Clone, Debug)]
pub struct Area {
	tiles: Vec<GridPosition>,
	// A bounding box for intersection acceleration.
	aabb:  GridBox,
}

impl Default for Area {
	fn default() -> Self {
		Self { tiles: Vec::new(), aabb: GridBox::new(GridPosition::default(), BoundingBox::fixed::<0, 0, 0>()) }
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

	fn from_overlapping(old_areas: impl IntoIterator<Item = Area>) -> Self {
		let mut new =
			Self { tiles: old_areas.into_iter().flat_map(|area| area.tiles).collect(), aabb: GridBox::default() };
		new.recompute_bounds();
		new
	}

	fn recompute_bounds(&mut self) {
		let (smallest_x, largest_x) = self.tiles.iter().map(|tile| tile.x).minmax().into_option().unwrap_or((0, 0));
		let (smallest_y, largest_y) = self.tiles.iter().map(|tile| tile.y).minmax().into_option().unwrap_or((0, 0));
		self.aabb = GridBox::from_corners((smallest_x, smallest_y, 0).into(), (largest_x + 1, largest_y + 1, 0).into());
	}

	fn is_empty(&self) -> bool {
		self.tiles.is_empty()
	}
}

/// A marker component used with the [`Area`] component to mark the area of a specific type and to determine some
/// type-specific area properties.
trait AreaMarker: Component {
	fn is_allowed_ground_type(&self, kind: GroundKind) -> bool;
}

/// Marker for pool areas.
#[derive(Component)]
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

fn update_areas<T: AreaMarker>(
	tiles: Res<GroundMap>,
	mut areas: Query<(Entity, &mut Area, &T)>,
	mut commands: Commands,
	mut update: ResMut<Events<UpdateAreas>>,
) {
	if update.is_empty() {
		return;
	}
	update.clear();

	let mut active_areas = Vec::new();

	// Perform flood fill on the areas to update them.
	for (old_area_entity, mut area, marker) in &mut areas {
		area.tiles.retain(|tile| tiles.kind_of(tile).is_some_and(|kind| marker.is_allowed_ground_type(kind)));
		if area.is_empty() {
			continue;
		}

		let remaining_tiles = area.tiles.iter();
	}

	// Shrink areas to remove tiles that are not area tiles.
	for (area_entity, mut area, marker) in &mut areas {
		area.tiles.retain(|tile| tiles.kind_of(tile).is_some_and(|kind| marker.is_allowed_ground_type(kind)));
		area.recompute_bounds();
		if area.is_empty() {
			commands.entity(area_entity).despawn_recursive();
		} else {
			active_areas.push((area_entity, area, marker));
		}
	}

	// Merge overlapping areas.
	// FIXME: Type shouldn't be necessary; rust-analyzer can deduce it just fine.
	let mut areas_to_merge: Vec<(Area, _)> = Vec::new();

	for (id, area, _) in &active_areas {
		let overlapping_areas = areas_to_merge
			.extract_if(|(other_area, _)| area.aabb.intersects_2d(other_area.aabb))
			.map(|(other_area, _)| other_area)
			.chain(Some((**area).clone()))
			.collect::<Vec<_>>();
		let new_area = Area::from_overlapping(overlapping_areas);
		areas_to_merge.push((new_area, id));
	}

	debug!("{}", areas_to_merge.len());

	let now_deleted_areas = active_areas.iter().filter_map(|(old_area, ..)| {
		if !areas_to_merge.iter().any(|(_, area)| *area == old_area) {
			Some(*old_area)
		} else {
			None
		}
	});

	for area_to_delete in now_deleted_areas {
		commands.entity(area_to_delete).despawn_recursive();
	}
}
