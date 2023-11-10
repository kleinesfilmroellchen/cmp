use std::sync::OnceLock;

use bevy::core_pipeline::contrast_adaptive_sharpening::ContrastAdaptiveSharpeningSettings;
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::utils::HashMap;
use bitflags::bitflags;

use crate::model::area::{Area, ImmutableArea};
use crate::model::{ActorPosition, BoundingBox, GridBox, GridPosition, GroundMap, WorldPosition};

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
					.before(sort_bounded_objects_by_z)
					.before(move_high_priority_objects),
			)
			.add_systems(PostUpdate, (sort_bounded_objects_by_z, move_high_priority_objects))
			.add_systems(FixedUpdate, (update_area_borders, update_immutable_area_borders));
	}
}

/// Static, unchanging sprite.
#[derive(Bundle, Default)]
pub struct StaticSprite {
	// Types enforced by Bevy so that the sprite renders. Don’t modify those manually!
	pub(crate) bevy_sprite: SpriteBundle,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorderKind {
	Pitch,
}

#[derive(Resource, Default)]
pub struct BorderTextures {
	pub textures: HashMap<BorderKind, Handle<TextureAtlas>>,
}

impl BorderTextures {
	pub fn get(
		&mut self,
		kind: BorderKind,
		atlas: &mut Assets<TextureAtlas>,
		asset_server: &AssetServer,
	) -> Handle<TextureAtlas> {
		self.textures
			.entry(kind)
			.or_insert_with(|| {
				let sprite = library::sprite_for_border_kind(kind);
				let image = asset_server.load(sprite);
				atlas.add(TextureAtlas::from_grid(image, (16., 16.).into(), 4, 1, None, None))
			})
			.clone()
	}
}

/// Sprite representing a border of a larger area, such as a fence.
#[derive(Bundle)]
pub struct BorderSprite {
	pub sides:                BorderSides,
	pub kind:                 BorderKind,
	pub(crate) sprite_bundle: SpriteSheetBundle,
	pub offset:               ActorPosition,
	priority:                 HighPriority,
}

bitflags! {
	#[repr(transparent)]
	#[derive(Debug, Component, Clone, Copy, Eq, PartialEq)]
	pub struct BorderSides : u8 {
		const Top = 0b0001;
		const Right = 0b0010;
		const Bottom = 0b0100;
		const Left = 0b1000;
	}
}

impl BorderSides {
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
		const EDGE_OFFSET: Vec2 = Vec2::splat(1. / 4.);
		self.iter()
			.map(|side| match side {
				Self::Top => -EDGE_OFFSET,
				Self::Right => -EDGE_OFFSET + Vec2::X / 2.,
				// Self::Left => -EDGE_OFFSET + Vec2::Y / 2.,
				// Self::Bottom => EDGE_OFFSET,
				_ => Vec2::ZERO,
			})
			.sum()
	}

	pub fn anchor(self) -> Anchor {
		Anchor::Custom(self.tile_offset())
	}

	pub fn world_offset(self) -> Vec3A {
		self.iter()
			.map(|side| match side {
				Self::Top => Vec3A::Y,
				Self::Right => Vec3A::X,
				// Self::Left => Vec3A::NEG_X,
				// Self::Bottom => Vec3A::NEG_Y,
				_ => Vec3A::ZERO,
			})
			.sum::<Vec3A>()
			/ 2.
	}
}

impl BorderSprite {
	pub fn new<'a>(
		sides: BorderSides,
		kind: BorderKind,
		asset_server: &'a AssetServer,
		texture_atlases: &'a mut Assets<TextureAtlas>,
		border_textures: &'a mut BorderTextures,
	) -> impl Iterator<Item = Self> + 'a {
		sides.iter_names().map(move |(_, side)| {
			debug!("{:?}: {}", side, side.world_offset());
			Self {
				sides: side,
				kind,
				sprite_bundle: SpriteSheetBundle {
					sprite: TextureAtlasSprite {
						anchor: side.anchor(),
						index: side.to_sprite_index(),
						..Default::default()
					},
					texture_atlas: border_textures.get(kind, texture_atlases, asset_server),
					..Default::default()
				},
				offset: side.world_offset().into(),
				priority: HighPriority,
			}
		})
	}
}

fn update_area_borders(
	ground_map: Res<GroundMap>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
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
	mut texture_atlases: ResMut<Assets<TextureAtlas>>,
	mut border_textures: ResMut<BorderTextures>,
	mut areas: Query<&mut ImmutableArea, Changed<ImmutableArea>>,
) {
	for area in &mut areas {
		area.instantiate_borders(&ground_map, &mut commands, &asset_server, &mut texture_atlases, &mut border_textures);
	}
}

fn initialize_graphics(mut commands: Commands, _asset_server: Res<AssetServer>, mut msaa: ResMut<Msaa>) {
	let projection = OrthographicProjection { scale: 1. / 4., near: -100000., ..Default::default() };
	commands.spawn((Camera2dBundle { projection, ..Default::default() }, ContrastAdaptiveSharpeningSettings {
		enabled:             false,
		sharpening_strength: 0.3,
		denoise:             false,
	}));
	*msaa = Msaa::Off;
}

#[derive(Clone, Copy, Debug, Default, Component)]
pub struct HighPriority;

static TRANSFORMATION_MATRIX: OnceLock<Mat3> = OnceLock::new();

pub const TILE_HEIGHT: f32 = 12.;
pub const TILE_WIDTH: f32 = 16.;

fn position_objects<PositionType: WorldPosition>(
	mut entities: Query<(&mut Transform, &PositionType), Changed<PositionType>>,
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
	for entity in &mut entities {
		let (mut bevy_transform, world_position_type) = entity;
		let world_position = world_position_type.position();
		let matrix = TRANSFORMATION_MATRIX.get().cloned().unwrap();
		// The translation rounding here is about 90% of pixel-perfectness:
		// - Make sure everything is camera-space pixel aligned (this code)
		// - Make sure all sprite anchors fall on pixel corners (sprite initialization code)
		// - Make sure no sprites are scaled (sprite initialization code)
		bevy_transform.translation = (matrix * world_position).round().into();
		bevy_transform.translation.z = -world_position.x - world_position.y;
	}
}

fn sort_bounded_objects_by_z(
	mut independent_bounded_entities: Query<&mut Transform, (With<BoundingBox>, Without<GridBox>, Changed<Transform>)>,
	mut boxed_entities: Query<&mut Transform, (With<GridBox>, Without<BoundingBox>, Changed<Transform>)>,
) {
	for mut bevy_transform in independent_bounded_entities.iter_mut().chain(boxed_entities.iter_mut()) {
		bevy_transform.translation.z += 1.;
	}
}

fn move_high_priority_objects(mut boxed_entities: Query<&mut Transform, (With<HighPriority>, Changed<Transform>)>) {
	for mut bevy_transform in &mut boxed_entities {
		bevy_transform.translation.z += 1000.0;
	}
}

/// Translates from a screen pixel position back to world space. Note that z needs to be provided and generally
/// depends on the surface at the specific location.
pub fn screen_to_world_space(screen_position: Vec2, z: f32) -> ActorPosition {
	// The matrix is invertible, since we keep the z dimension when using it normally, so we can make use of that by
	// synthetically re-inserting the z coordinate into the 2D screen position and getting a precise inverse transform
	// for free.
	let matrix = TRANSFORMATION_MATRIX.get().unwrap().inverse();
	let screen_space_with_synthetic_z: Vec3 = (screen_position, z).into();
	// The z coordinate here is garbage; discard it and replace it with the given one.
	let mut world_space = matrix * screen_space_with_synthetic_z;
	world_space.z = z;
	ActorPosition(world_space.into())
}
