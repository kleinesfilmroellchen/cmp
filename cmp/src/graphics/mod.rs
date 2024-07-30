use std::sync::OnceLock;

use bevy::core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpeningSettings;
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::utils::HashMap;
use bitflags::bitflags;

use crate::model::area::{Area, ImmutableArea};
use crate::model::{ActorPosition, GridBox, GridPosition, GroundMap, WorldPosition};

pub(crate) mod library;

/// Plugin responsible for setting up a window and running and initializing graphics.
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Msaa::default())
			.init_resource::<BorderTextures>()
			.add_systems(Startup, initialize_graphics)
			.add_systems(
				PostUpdate,
				(position_objects::<ActorPosition>, position_objects::<GridPosition>, position_objects::<GridBox>)
					.before(move_edge_objects_in_front_of_boxes),
			)
			.add_systems(PostUpdate, move_edge_objects_in_front_of_boxes)
			.add_systems(Update, (update_area_borders, update_immutable_area_borders));
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderKind {
	Pitch,
}

#[derive(Resource, Default)]
pub struct BorderTextures {
	pub textures: HashMap<BorderKind, Handle<TextureAtlasLayout>>,
}

impl BorderTextures {
	pub fn get(
		&mut self,
		kind: BorderKind,
		atlas: &mut Assets<TextureAtlasLayout>,
		asset_server: &AssetServer,
	) -> (Handle<TextureAtlasLayout>, Handle<Image>) {
		let sprite = library::sprite_for_border_kind(kind);
		let image = asset_server.load(sprite);
		(
			self.textures
				.entry(kind)
				.or_insert_with(|| atlas.add(TextureAtlasLayout::from_grid((16, 16).into(), 4, 1, None, None)))
				.clone(),
			image,
		)
	}
}

/// Sprite representing a border of a larger area, such as a fence.
#[derive(Bundle)]
pub struct BorderSprite {
	pub side:          Sides,
	pub kind:          BorderKind,
	pub(crate) sprite: SpriteBundle,
	pub atlas:         TextureAtlas,
	pub offset:        ActorPosition,
	priority:          ObjectPriority,
}

bitflags! {
	#[repr(transparent)]
	#[derive(Debug, Component, Clone, Copy, Eq, PartialEq)]
	pub struct Sides : u8 {
		const Top = 0b0001;
		const Right = 0b0010;
		const Bottom = 0b0100;
		const Left = 0b1000;
	}
}

impl Sides {
	pub fn to_sprite_index(self) -> usize {
		match self {
			Self::Top => 0,
			Self::Right => 1,
			Self::Left => 2,
			Self::Bottom => 3,
			_ => panic!("no single sprite index exists for combined sides"),
		}
	}

	pub fn tile_offset(self) -> Vec2 {
		const BORDER_HEIGHT: f32 = 16.;
		const BORDER_SIZE: Vec2 = Vec2::new(TILE_WIDTH, BORDER_HEIGHT);
		// NOTE: The Bevy documentation for Anchor vectors is wrong in 0.11.
		// The bottom left is -.5, -.5 and the top right is .5, .5.
		(self
			.iter()
			.map(|side| match side {
				Self::Top => Vec2::new(4., 10.),
				Self::Right => Vec2::new(12., 10.),
				Self::Left => Vec2::new(4., 4.),
				Self::Bottom => Vec2::new(12., 4.),
				_ => Vec2::ZERO,
			})
			.sum::<Vec2>()
			- BORDER_SIZE / 2.)
			/ BORDER_SIZE
	}

	pub fn anchor(self) -> Anchor {
		// Anchor::Center
		Anchor::Custom(self.tile_offset())
	}

	pub fn world_offset(self) -> Vec3A {
		self.iter()
			.map(|side| match side {
				Self::Top => Vec3A::new(0.5, 1., 0.),
				Self::Right => Vec3A::new(1., 0.5, 0.),
				Self::Left => Vec3A::new(0., 0.5, 0.),
				Self::Bottom => Vec3A::new(0.5, 0., 0.),
				_ => Vec3A::ZERO,
			})
			.sum::<Vec3A>()
	}
}

impl BorderSprite {
	pub fn new<'a>(
		sides: Sides,
		kind: BorderKind,
		asset_server: &'a AssetServer,
		texture_atlases: &'a mut Assets<TextureAtlasLayout>,
		border_textures: &'a mut BorderTextures,
	) -> impl Iterator<Item = Self> + 'a {
		sides.iter_names().map(move |(_, side)| {
			let (layout, image) = border_textures.get(kind, texture_atlases, asset_server);
			Self {
				side,
				kind,
				sprite: SpriteBundle {
					sprite: Sprite { anchor: side.anchor(), ..Default::default() },
					texture: image,
					..Default::default()
				},
				atlas: TextureAtlas { layout, index: side.to_sprite_index() },
				offset: side.world_offset().into(),
				priority: ObjectPriority::Border,
			}
		})
	}
}

fn update_area_borders(
	ground_map: Res<GroundMap>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
	mut border_textures: ResMut<BorderTextures>,
	mut areas: Query<&mut Area, Changed<Area>>,
) {
	for area in &mut areas {
		area.instantiate_borders(&ground_map, &mut commands, &asset_server, &mut texture_atlases, &mut border_textures);
	}
}

fn update_immutable_area_borders(
	ground_map: Res<GroundMap>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
	mut border_textures: ResMut<BorderTextures>,
	mut areas: Query<&mut ImmutableArea, Changed<ImmutableArea>>,
) {
	for area in &mut areas {
		area.instantiate_borders(&ground_map, &mut commands, &asset_server, &mut texture_atlases, &mut border_textures);
	}
}

fn initialize_graphics(mut commands: Commands, _asset_server: Res<AssetServer>, mut msaa: ResMut<Msaa>) {
	let projection = OrthographicProjection { scale: 1. / 4., near: -100000., ..Default::default() };
	commands.spawn((
		Camera2dBundle {
			projection,
			deband_dither: DebandDither::Enabled,
			camera: Camera { hdr: true, ..Default::default() },
			..Default::default()
		},
		ContrastAdaptiveSharpeningSettings {
			enabled:             false,
			sharpening_strength: 0.3,
			denoise:             false,
		},
	));
	*msaa = Msaa::Off;
}

/// Graphical object priorities assist in z-sorting objects at the same position.
#[derive(Clone, Copy, Debug, Component)]
pub enum ObjectPriority {
	/// Ground objects have the lowest priority.
	Ground,
	/// Normal objects have a priority higher than ground objects so they always appear on top of ground objects on the
	/// same tile.
	Normal,
	/// Tiles on borders need to be elevated since their position makes them a lower z index than they should actually
	/// be.
	Border,
	/// Overlay objects use a very large z offset as to appear on top of any object, even ones that are logically in
	/// front of them.
	Overlay,
}

impl Default for ObjectPriority {
	fn default() -> Self {
		Self::Normal
	}
}

impl ObjectPriority {
	pub fn index(&self) -> f32 {
		match self {
			ObjectPriority::Ground => 0.,
			ObjectPriority::Normal => 1.,
			ObjectPriority::Border => 1.5,
			ObjectPriority::Overlay => 1000.,
		}
	}
}

pub static TRANSFORMATION_MATRIX: OnceLock<Mat3> = OnceLock::new();

/// BUG: This should be 12 but that commonly leads to off-by-one seams.
pub const TILE_HEIGHT: f32 = 11.999;
pub const TILE_WIDTH: f32 = 16.;

fn position_objects<PositionType: WorldPosition>(
	mut entities: Query<(&mut Transform, &PositionType, Option<&ObjectPriority>), Changed<PositionType>>,
) {
	TRANSFORMATION_MATRIX.get_or_init(|| {
		// Our iso grid is a simple affine transform away from the real world position.
		// We only have a small, roughly 45°-rotation to the right, then a vertical scale.
		// The exact parameters are calculated with the fact that the triangle describing a tile corner has width 8 and
		// height 6, so we know where the X and Y vectors must point exactly.
		let x_vector = ((TILE_WIDTH / 2.).round(), (TILE_HEIGHT / 2.).round() + 1., 0.).into();
		let y_vector = (-(TILE_WIDTH / 2.).round(), (TILE_HEIGHT / 2.).round() + 1., 0.).into();
		// Only map z onto the y and z axes. Applying it to z as well will make 2D z sorting work correctly.
		Mat3::from_cols(x_vector, y_vector, Vec3::Y * (TILE_HEIGHT / 4.).round() + Vec3::Z)
	});
	for (mut bevy_transform, world_position_type, priority) in &mut entities {
		let world_position = world_position_type.position();
		let matrix = TRANSFORMATION_MATRIX.get().cloned().unwrap();
		// The translation rounding here is about 90% of pixel-perfectness:
		// - Make sure everything is camera-space pixel aligned (this code)
		// - Make sure all sprite anchors fall on pixel corners (sprite initialization code)
		// - Make sure no sprites are scaled (sprite initialization code)
		bevy_transform.translation = (matrix * world_position).round().into();
		bevy_transform.translation.z =
			-world_position.x - world_position.y + priority.map(ObjectPriority::index).unwrap_or(0.);
	}
}

fn move_edge_objects_in_front_of_boxes(
	mut edge_objects: Query<(&mut Transform, &ActorPosition, Option<&Parent>), Changed<Transform>>,
	possible_parents: Query<&GridPosition, With<Children>>,
	boxed_entities: Query<&GridBox>,
) {
	edge_objects.par_iter_mut().for_each(|(mut bevy_transform, edge_object_position, parent)| {
		let own_position = if let Some(parent) = parent.and_then(|parent| possible_parents.get(parent.get()).ok()) {
			parent.position() + **edge_object_position
		} else {
			**edge_object_position
		};

		// PERFORMANCE: This is a prime optimization candidate.
		if let Some(smallest_edge_box) = boxed_entities
			.iter()
			.filter(|grid_box| grid_box.has_on_smaller_edges(own_position))
			.min_by_key(|grid_box| grid_box.corner.x + grid_box.corner.y)
		{
			let offset = smallest_edge_box.corner.position() - own_position;
			bevy_transform.translation.z -= offset.x + offset.y;
		}
	});
}

/// Translates from a bevy engine position back to world space. Note that z needs to be provided and generally
/// depends on the surface at the specific location.
pub fn engine_to_world_space(engine_position: Vec2, z: f32) -> ActorPosition {
	// The matrix is invertible, since we keep the z dimension when using it normally, so we can make use of that by
	// synthetically re-inserting the z coordinate into the 2D engine position and getting a precise inverse transform
	// for free.
	let matrix = TRANSFORMATION_MATRIX.get().unwrap().inverse();
	let engine_space_with_synthetic_z: Vec3 = (engine_position, z).into();
	// The z coordinate here is garbage; discard it and replace it with the given one.
	let mut world_space = matrix * engine_space_with_synthetic_z;
	world_space.z = z;
	ActorPosition(world_space.into())
}
