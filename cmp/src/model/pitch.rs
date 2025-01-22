use std::marker::ConstParamTy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::prelude::*;
use moonshine_save::save::Save;

use super::area::{Area, AreaMarker, ImmutableArea, UpdateAreas};
use super::{BoundingBox, GridBox, GridPosition, GroundKind, GroundMap, Metric};
use crate::gamemode::GameState;
use crate::graphics::library::{anchor_for_image, image_for_pitch};
use crate::graphics::ObjectPriority;
use crate::ui::world_info::{WorldInfoProperties, WorldInfoProperty};
use crate::util::Tooltipable;
use crate::HashSet;

/// The different available types of pitch.
#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
pub enum PitchType {
	TentPitch,
	PermanentTent,
	CaravanPitch,
	MobileHome,
	Cottage,
}

pub type Comfort = Metric<0, 10>;

impl PitchType {
	pub const fn size(&self) -> BoundingBox {
		match self {
			Self::CaravanPitch | Self::TentPitch => BoundingBox::fixed::<1, 1, 1>(),
			Self::PermanentTent => BoundingBox::fixed::<2, 2, 2>(),
			Self::MobileHome => BoundingBox::fixed::<1, 2, 2>(),
			Self::Cottage => BoundingBox::fixed::<2, 3, 3>(),
		}
	}

	pub const fn required_area(&self) -> usize {
		match self {
			Self::CaravanPitch | Self::TentPitch => 5 * 5,
			Self::PermanentTent => 4 * 4,
			Self::MobileHome => 2 * 4,
			Self::Cottage => 3 * 4,
		}
	}

	pub fn comfort(&self) -> Comfort {
		match self {
			Self::TentPitch => 1,
			Self::PermanentTent => 4,
			Self::CaravanPitch => 3,
			Self::MobileHome => 5,
			Self::Cottage => 6,
		}
		.try_into()
		.unwrap()
	}

	/// Determines whether this pitch type is actually a building, so that when creating it an actual building
	/// entity must be constructed.
	pub const fn is_real_building(&self) -> bool {
		match self {
			Self::CaravanPitch | Self::TentPitch => false,
			Self::PermanentTent | Self::MobileHome | Self::Cottage => true,
		}
	}
}

impl std::fmt::Display for PitchType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::TentPitch => "Tent Pitch",
			Self::PermanentTent => "Permanent Tent",
			Self::CaravanPitch => "Caravan Pitch",
			Self::MobileHome => "Mobile Home",
			Self::Cottage => "Cottage",
		})
	}
}

impl Tooltipable for PitchType {
	fn description(&self) -> &'static str {
		match self {
			Self::TentPitch =>
				"A basic tent pitch, suitable for a small tent and two people. Tent pitches are not more than \
				 demarcated patches of grass, and take almost no effort to maintain. Only the hardy tent-camping \
				 visitors will use tent pitches, however. Tent pitches also take up a relatively large area in \
				 comparison to the amount of people that can stay there.",
			Self::PermanentTent =>
				"A permanently constructed tent for five campers. Due to its construction with wooden flooring under a \
				 cloth roof, this tent does provide better comfort than a bare tent pitch, though its spacial \
				 requirement is only a little less than the large tent pitch’s. It requires some more upkeep, of \
				 course, but it doesn’t need water or electricity. You can, however, connect those resources anyways, \
				 which will mildly improve visitor satisfaction.",
			Self::CaravanPitch =>
				"A pitch for two or three campers to park their caravans. As opposed to tent pitches, caravan pitches \
				 need a permanent water and electricity supply for the vehicles. In turn, less hardy campers with \
				 their caravans will show up to these pitches. As with tent pitches, caravan pitches provide ample \
				 space for the few visitors.",
			Self::MobileHome =>
				"A mobile home, the most basic form of permanent housing for four visitors. Mobile homes are parked \
				 semi-permanently, need water and electricity, and they provide much more comfort than even a caravan. \
				 In addition, mobile homes are parked on a rather small pitch. However, their upkeep is significantly \
				 more resource-intensive than the simple pitches, since campers no longer bring their own housing.",
			Self::Cottage =>
				"A basic cottage for up to six visitors. Cottages are not more than semi-permanent wooden huts set up \
				 on a relatively small pitch, and they can accommodate a whole group of people pretty comfortably. \
				 Cottages require water and electricity, and will need to be maintained for visitor satisfaction.",
		}
	}
}

type AccommodationMultiplicity = Metric<1, 2>;

/// A proper pitch for guests; essentially an instance of [`PitchType`].
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Pitch {
	/// When the kind is [`None`], the pitch type is unassigned and this pitch is not functional.
	pub kind:         Option<PitchType>,
	/// How many of the same pitch are available here. This value rarely goes beyond 1 except for specific
	/// pitch types.
	pub multiplicity: AccommodationMultiplicity,
}

impl AreaMarker for Pitch {
	fn is_allowed_ground_type(&self, kind: super::GroundKind) -> bool {
		kind == Self::GROUND_TYPE
	}

	fn init_new(area: Area, commands: &mut Commands) {
		commands.spawn(AccommodationBundle::from_area(area));
	}
}

impl Pitch {
	pub const GROUND_TYPE: GroundKind = GroundKind::Pitch;

	pub fn required_area(&self) -> usize {
		self.kind.map(|kind| kind.required_area() * (*self.multiplicity as usize)).unwrap_or(0)
	}

	pub fn apply_properties(&self, properties: &mut WorldInfoProperties, area: &Area) {
		properties.clear();
		properties.name = AccommodationBundle::info_base().name;
		properties.description =
			self.kind.map_or(AccommodationBundle::info_base().description.as_str(), |x| x.description()).to_string();
		if let Some(kind) = self.kind {
			properties.push(WorldInfoProperty::PitchType(kind));
			properties.push(WorldInfoProperty::Comfort(kind.comfort()));
			properties.push(WorldInfoProperty::MinArea(kind.required_area()));
		}
		properties.push(WorldInfoProperty::Multiplicity(*self.multiplicity));
		properties.push(WorldInfoProperty::Area(area.size()));
	}
}

#[derive(Bundle)]
pub struct AccommodationBundle {
	area:                 Area,
	pitch:                Pitch,
	global_transform:     GlobalTransform,
	transform:            Transform,
	view_visibility:      ViewVisibility,
	inherited_visibility: InheritedVisibility,
	visibility:           Visibility,
	properties:           WorldInfoProperties,
	save:                 Save,
}

impl AccommodationBundle {
	pub fn new(start_position: GridPosition, end_position: GridPosition) -> Self {
		Self {
			area:                 Area::from_rect(start_position, end_position),
			pitch:                Pitch::default(),
			// Make various graphical children of the pitch area (borders, trees, buildings) visible.
			global_transform:     GlobalTransform::default(),
			transform:            Transform::default(),
			inherited_visibility: InheritedVisibility::default(),
			view_visibility:      ViewVisibility::default(),
			visibility:           Visibility::Visible,
			properties:           Self::info_base(),
			save:                 Save,
		}
	}

	pub fn from_area(area: Area) -> Self {
		Self {
			area,
			pitch: Pitch::default(),
			global_transform: GlobalTransform::default(),
			transform: Transform::default(),
			inherited_visibility: InheritedVisibility::default(),
			view_visibility: ViewVisibility::default(),
			visibility: Visibility::Visible,
			properties: Self::info_base(),
			save: Save,
		}
	}

	fn info_base() -> WorldInfoProperties {
		WorldInfoProperties::basic(
			"Pitch".to_string(),
			"A pitch, providing residency to visitors. This pitch is unassigned and cannot house visitors currently."
				.to_string(),
		)
	}
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AccommodationBuilding;

#[derive(Bundle)]
pub struct AccommodationBuildingBundle {
	pub position: GridBox,
	pub sprite:   Sprite,
	marker:       AccommodationBuilding,
	priority:     ObjectPriority,
	save:         Save,
}

impl AccommodationBuildingBundle {
	pub fn new(kind: PitchType, position: GridPosition, asset_server: &AssetServer) -> Option<Self> {
		if !kind.is_real_building() {
			None
		} else {
			let image = image_for_pitch(kind);
			Some(Self {
				position: GridBox::around(position, kind.size().flat()),
				sprite:   Sprite {
					anchor: anchor_for_image(image),
					image: asset_server.load(image),
					..Default::default()
				},
				marker:   AccommodationBuilding,
				priority: ObjectPriority::Normal,
				save:     Save,
			})
		}
	}
}

pub struct AccommodationManagement;
impl Plugin for AccommodationManagement {
	fn build(&self, app: &mut App) {
		app.register_type::<AccommodationBuilding>()
			.register_type::<PitchType>()
			.register_type::<Pitch>()
			.register_type::<Comfort>()
			.register_type::<AccommodationMultiplicity>()
			.add_systems(Update, add_pitch_graphics.run_if(in_state(GameState::InGame)))
			.add_systems(FixedUpdate, update_built_pitches.run_if(in_state(GameState::InGame)))
			.add_systems(
				FixedUpdate,
				update_pitch_world_info.after(update_built_pitches).run_if(in_state(GameState::InGame)),
			);
	}
}

fn update_built_pitches(
	commands: ParallelCommands,
	mut pitches: Query<(Entity, &mut Pitch, &Children, &mut ImmutableArea)>,
	other_areas: Query<&Area>,
	pitch_building_children: Query<&GridBox, With<AccommodationBuilding>>,
	ground_map: Res<GroundMap>,
	mut update: ResMut<Events<UpdateAreas>>,
) {
	if ground_map.is_changed() {
		let relevant_tiles = |tile: &'_ _| ground_map.kind_of(tile).is_some_and(|kind| kind == Pitch::GROUND_TYPE);
		// When the player places pitch tiles over this finalized pitch, we have to detect that and
		// delete the tiles from our area.
		let foreign_area_tiles = other_areas
			.into_iter()
			.flat_map(|area| area.tiles_iter().filter(relevant_tiles).map(|x| (x, ())))
			.collect::<HashSet<_>>();

		let needs_update = Arc::new(AtomicBool::new(false));

		pitches.par_iter_mut().for_each(|(entity, mut pitch, children, mut area)| {
			area.retain_tiles(|tile| relevant_tiles(tile) && !foreign_area_tiles.contains_key(tile));
			let mut should_destroy = false;
			// Check the three conditions for destroying an updated pitch:
			// 1. Area doesn't provide enough tiles for the pitch type.
			// 2. Pitch building is not physically on the area anymore.
			// 3. Area is discontinuous (for simplification purposes, this always deletes the entire pitch).

			if area.is_empty() || area.is_discontinuous() {
				should_destroy = true;
			} else {
				// 2.
				for child in children.iter() {
					let child_position = pitch_building_children.get(*child).unwrap();
					if !area.fits(child_position) {
						should_destroy = true;
						break;
					}
				}
				// 1.
				if area.size() < pitch.required_area() {
					should_destroy = true;
				}
			}
			if should_destroy {
				// Reset the pitch type into a mutable area without a type.
				commands.command_scope(|mut commands| {
					let inner_area: Area = area.clone();
					let mut entity_commands = commands.entity(entity);
					entity_commands.remove::<ImmutableArea>();
					entity_commands.insert(inner_area);
					entity_commands.despawn_descendants();
				});
				pitch.kind = None;
				pitch.multiplicity = AccommodationMultiplicity::default();
				needs_update.store(true, Ordering::Release);
			}
		});

		if needs_update.load(Ordering::Acquire) {
			update.send_default();
		}
	}
}

fn update_pitch_world_info(
	mut immutable_pitches: Query<(&mut WorldInfoProperties, Ref<Pitch>, Ref<ImmutableArea>), Without<Area>>,
	mut pitches: Query<(&mut WorldInfoProperties, Ref<Pitch>, Ref<Area>), Without<ImmutableArea>>,
) {
	for (mut properties, pitch, area) in pitches.iter_mut().filter(|(_, _, a)| a.is_changed()) {
		pitch.apply_properties(&mut properties, &area);
	}
	for (mut properties, pitch, area) in immutable_pitches.iter_mut().filter(|(_, _, a)| a.is_changed()) {
		pitch.apply_properties(&mut properties, &area.0);
	}
}

fn add_pitch_graphics(
	buildings: Query<Entity, (With<AccommodationBuilding>, Without<Sprite>)>,
	pitches: Query<(&Pitch, &Children), Without<AccommodationBuilding>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	for entity in &buildings {
		let result: Option<()> = try {
			let (parent_pitch, _) = pitches.iter().find(|(_, children)| children.contains(&entity))?;
			let image = image_for_pitch(parent_pitch.kind?);
			commands.entity(entity).insert(Sprite {
				anchor: anchor_for_image(image),
				image: asset_server.load(image),
				..Default::default()
			});
		};
		if result.is_none() {
			commands.entity(entity).despawn_recursive();
		}
	}
}
