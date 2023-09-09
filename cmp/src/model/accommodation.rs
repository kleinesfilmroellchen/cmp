use std::marker::ConstParamTy;

use bevy::prelude::*;
use bevy::utils::HashSet;

use super::area::{Area, AreaMarker, ImmutableArea};
use super::{BoundingBox, GridBox, GridPosition, GroundKind, GroundMap, Metric};
use crate::graphics::library::{anchor_for_sprite, sprite_for_accommodation};
use crate::graphics::StaticSprite;
use crate::util::Tooltipable;

/// The different available types of accommodation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
pub enum AccommodationType {
	TentSite,
	PermanentTent,
	CaravanSite,
	MobileHome,
	Cottage,
}

type Comfort = Metric<0, 10>;

impl AccommodationType {
	pub const fn size(&self) -> BoundingBox {
		match self {
			Self::CaravanSite | Self::TentSite => BoundingBox::fixed::<1, 1, 1>(),
			Self::PermanentTent => BoundingBox::fixed::<2, 2, 2>(),
			Self::MobileHome => BoundingBox::fixed::<1, 2, 2>(),
			Self::Cottage => BoundingBox::fixed::<2, 3, 3>(),
		}
	}

	pub const fn required_area(&self) -> usize {
		match self {
			Self::CaravanSite | Self::TentSite => 5 * 5,
			Self::PermanentTent => 4 * 4,
			Self::MobileHome => 2 * 4,
			Self::Cottage => 3 * 4,
		}
	}

	pub fn comfort(&self) -> Comfort {
		match self {
			Self::TentSite => 1,
			Self::PermanentTent => 4,
			Self::CaravanSite => 3,
			Self::MobileHome => 5,
			Self::Cottage => 6,
		}
		.try_into()
		.unwrap()
	}

	/// Determines whether this accommodation type is actually a building, so that when creating it an actual building
	/// entity must be constructed.
	pub const fn is_real_building(&self) -> bool {
		match self {
			Self::CaravanSite | Self::TentSite => false,
			Self::PermanentTent | Self::MobileHome | Self::Cottage => true,
		}
	}
}

impl std::fmt::Display for AccommodationType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::TentSite => "Tent Site",
			Self::PermanentTent => "Permanent Tent",
			Self::CaravanSite => "Caravan Site",
			Self::MobileHome => "Mobile Home",
			Self::Cottage => "Cottage",
		})
	}
}

impl Tooltipable for AccommodationType {
	fn description(&self) -> &'static str {
		match self {
			Self::TentSite =>
				"A basic tent site, suitable for a small tent and two people. Tent sites are not more than demarcated \
				 patches of grass, and take almost no effort to maintain. Only the hardy tent-camping visitors will \
				 use tent sites, however. Tent sites also take up a relatively large area in comparison to the amount \
				 of people that can stay there.",
			Self::PermanentTent =>
				"A permanently constructed tent for five campers. Due to its construction with wooden flooring under a \
				 cloth roof, this tent does provide better comfort than a bare camp site, though its spacial \
				 requirement is only a little less than the large tent site’s. It requires some more upkeep, of \
				 course, but it doesn’t need water or electricity. You can, however, connect those resources anyways, \
				 which will mildly improve visitor satisfaction.",
			Self::CaravanSite =>
				"A site for two or three campers to park their caravans. As opposed to tent sites, caravan sites need \
				 a permanent water and electricity supply for the vehicles. In turn, less hardy campers with their \
				 caravans will show up to these accommodation spots. As with tent sites, caravan sites provide ample \
				 space for the few visitors.",
			Self::MobileHome =>
				"A mobile home, the most basic form of permanent housing for four visitors. Mobile homes are parked \
				 semi-permanently, need water and electricity, and they provide much more comfort than even a caravan. \
				 In addition, mobile homes are parked on a rather small patch of land. However, their upkeep is \
				 significantly more resource-intensive than the simple sites, since campers no longer bring their own \
				 housing.",
			Self::Cottage =>
				"A basic cottage for up to six visitors. Cottages are not more than semi-permanent wooden huts set up \
				 on a relatively small patch of land, and they can accommodate a whole group of people pretty \
				 comfortably. Cottages require water and electricity, and will need to be maintained for visitor \
				 satisfaction.",
		}
	}
}

type AccommodationMultiplicity = Metric<1, 2>;

/// A proper accommodation for guests; essentially an instance of [`AccommodationType`].
#[derive(Component, Default)]
pub struct Accommodation {
	/// When the kind is [`None`], the accommodation type is unassigned and this accommodation is not functional.
	pub kind:         Option<AccommodationType>,
	/// How many of the same accommodation are available here. This value rarely goes beyond 1 except for specific
	/// accommodation types.
	pub multiplicity: AccommodationMultiplicity,
}

impl AreaMarker for Accommodation {
	fn is_allowed_ground_type(&self, kind: super::GroundKind) -> bool {
		kind == Self::GROUND_TYPE
	}

	fn init_new(area: Area, commands: &mut Commands) {
		commands.spawn(AccommodationBundle::from_area(area));
	}
}

impl Accommodation {
	pub const GROUND_TYPE: GroundKind = GroundKind::Accommodation;

	pub fn required_area(&self) -> usize {
		self.kind.map(|kind| kind.required_area() * (*self.multiplicity as usize)).unwrap_or(0)
	}
}

#[derive(Bundle)]
pub struct AccommodationBundle {
	area:                Area,
	accommodation:       Accommodation,
	global_transform:    GlobalTransform,
	transform:           Transform,
	computed_visibility: ComputedVisibility,
	visibility:          Visibility,
}

impl AccommodationBundle {
	pub fn new(start_position: GridPosition, end_position: GridPosition) -> Self {
		Self {
			area:                Area::from_rect(start_position, end_position),
			accommodation:       Accommodation::default(),
			// Make various graphical children of the accommodation area (borders, trees, buildings) visible.
			global_transform:    GlobalTransform::default(),
			transform:           Transform::default(),
			computed_visibility: ComputedVisibility::default(),
			visibility:          Visibility::Visible,
		}
	}

	pub fn from_area(area: Area) -> Self {
		Self {
			area,
			accommodation: Accommodation::default(),
			global_transform: GlobalTransform::default(),
			transform: Transform::default(),
			computed_visibility: ComputedVisibility::default(),
			visibility: Visibility::Visible,
		}
	}
}

#[derive(Component)]
pub struct AccommodationBuilding;

#[derive(Bundle)]
pub struct AccommodationBuildingBundle {
	pub position: GridBox,
	pub sprite:   StaticSprite,
	marker:       AccommodationBuilding,
}

impl AccommodationBuildingBundle {
	pub fn new(kind: AccommodationType, position: GridPosition, asset_server: &AssetServer) -> Option<Self> {
		if !kind.is_real_building() {
			None
		} else {
			let sprite = sprite_for_accommodation(kind);
			Some(AccommodationBuildingBundle {
				position: GridBox::around(position, kind.size().flat()),
				sprite:   StaticSprite {
					bevy_sprite: SpriteBundle {
						sprite: Sprite { anchor: anchor_for_sprite(sprite), ..Default::default() },
						texture: asset_server.load(sprite),
						..Default::default()
					},
				},
				marker:   AccommodationBuilding,
			})
		}
	}
}

pub struct AccommodationManagement;
impl Plugin for AccommodationManagement {
	fn build(&self, app: &mut App) {
		app.add_systems(FixedUpdate, update_built_accommodations);
	}
}

pub fn update_built_accommodations(
	commands: ParallelCommands,
	mut accommodations: Query<(Entity, &Accommodation, &Children, &mut ImmutableArea)>,
	other_areas: Query<&Area>,
	accommodation_building_children: Query<&GridBox, With<AccommodationBuilding>>,
	ground_map: Res<GroundMap>,
) {
	if ground_map.is_changed() {
		let relevant_tiles =
			|tile: &'_ _| ground_map.kind_of(tile).is_some_and(|kind| kind == Accommodation::GROUND_TYPE);
		// When the player places accommodation tiles over this finalized accommodation, we have to detect that and
		// delete the tiles from our area.
		let foreign_area_tiles =
			other_areas.into_iter().flat_map(|area| area.tiles_iter().filter(relevant_tiles)).collect::<HashSet<_>>();

		accommodations.par_iter_mut().for_each_mut(|(entity, accommodation, children, mut area)| {
			area.retain_tiles(|tile| relevant_tiles(tile) && !foreign_area_tiles.contains(tile));
			let mut should_destroy = false;
			// Check the three conditions for destroying an updated accommodation:
			// 1. Area doesn't provide enough tiles for the accommodation type.
			// 2. Accommodation building is not physically on the area anymore.
			// 3. Area is discontinuous (for simplification purposes, this always deletes the entire pitch).

			if area.is_empty() || area.is_discontinuous() {
				should_destroy = true;
			} else {
				// 2.
				for child in children.iter() {
					let child_position = accommodation_building_children.get(*child).unwrap();
					if !area.fits(child_position) {
						should_destroy = true;
						break;
					}
				}
				// 1.
				if area.size() < accommodation.required_area() {
					should_destroy = true;
				}
			}
			if should_destroy {
				commands.command_scope(|mut commands| commands.entity(entity).despawn_recursive());
			}
		});
	}
}
