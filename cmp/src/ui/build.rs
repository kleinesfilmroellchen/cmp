use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use itertools::{EitherOrBoth, Itertools};
use thiserror::Error;

use super::error::{DisplayableError, ErrorBox};
use super::on_start_build_preview;
use super::world_info::WorldInfoProperties;
use crate::gamemode::GameState;
use crate::graphics::library::{anchor_for_image, preview_image_for_buildable};
use crate::graphics::{engine_to_world_space, InGameCamera, ObjectPriority};
use crate::input::{camera_to_world, InputState};
use crate::model::area::{Area, ImmutableArea, Pool, UpdateAreas};
use crate::model::pitch::Pitch;
use crate::model::{
	AccommodationBuildingBundle, AccommodationBundle, Buildable, BuildableType, GridBox, GridPosition, GroundKind,
	GroundMap,
};

pub struct BuildPlugin;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<StartBuildPreview>()
			.add_event::<PerformBuild<{ BuildableType::Ground }>>()
			.add_event::<PerformBuild<{ BuildableType::Pitch }>>()
			.add_event::<PerformBuild<{ BuildableType::PitchType }>>()
			.add_event::<PerformBuild<{ BuildableType::PoolArea }>>()
			.add_event::<BuildError>()
			.add_systems(
				Update,
				update_building_preview
					.after(create_building_preview)
					.after(on_start_build_preview)
					.run_if(in_state(InputState::Building))
					.run_if(in_state(GameState::InGame)),
			)
			.add_systems(
				Update,
				(handle_build_interactions, set_building_preview_start, end_building)
					.run_if(in_state(InputState::Building))
					.run_if(in_state(GameState::InGame)),
			)
			.add_systems(Update, create_building_preview.run_if(in_state(GameState::InGame)))
			.add_systems(
				OnExit(InputState::Building),
				destroy_building_preview.after(update_building_preview).run_if(in_state(GameState::InGame)),
			)
			.add_systems(
				Update,
				(perform_pitch_build, perform_pitch_type_build, perform_ground_build, perform_pool_area_build)
					.run_if(in_state(GameState::InGame)),
			);
	}
}

#[derive(Event)]
pub struct StartBuildPreview {
	pub buildable: Buildable,
}

/// The [`BuildableType`] is a static parameter on the build event so that we can determine the correct receiver system
/// via the type system and bevy's system parameters.
#[derive(Event)]
struct PerformBuild<const BUILDABLE: BuildableType> {
	start_position: GridPosition,
	end_position:   GridPosition,
	buildable:      Buildable,
}

/// Any reason that the build could not be completed; eventually propagated to the end-user.
#[derive(Event, Error, Debug)]
pub(super) enum BuildError {
	#[error("There is no pitch to build on here.")]
	NoAccommodationHere,
	#[error("Building doesn’t have enough space to be built here.")]
	NoSpace,
	#[error(
		"The pitch area is too small for this pitch type; {} tiles are required but there are only {} \
		 tiles.", .required, .actual
	)]
	PitchTooSmall { required: usize, actual: usize },
}

impl DisplayableError for BuildError {
	fn name(&self) -> &str {
		"Build error"
	}
}

/// Component for the building preview's parent entity.
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
struct PreviewParent {
	/// What is to be built.
	pub previewed:        Buildable,
	/// Wherever the user started to place the building; the location where they started clicking.
	pub start_position:   GridPosition,
	/// Where the building is supposed to be located right now.
	pub current_position: GridPosition,
}

impl PreviewParent {
	fn new(previewed: Buildable) -> Self {
		Self { previewed, start_position: GridPosition::default(), current_position: GridPosition::default() }
	}
}

/// Marker component for anything that's part of a building preview.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct PreviewChild;

/// The way the user performs building, and the way the building is previewed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildMode {
	/// Single building is previewed; click and drag has no effect.
	Single,
	/// A straight line from click start to end will be built.
	Line,
	/// A rectangle with opposite corners at click start and end will be built.
	Rect,
}

impl BuildMode {
	fn update_preview<'a>(
		&self,
		PreviewParent { previewed, start_position, current_position }: PreviewParent,
		mut current_children: impl Iterator<Item = (Entity, Mut<'a, GridPosition>)>,
		parent_entity: Entity,
		commands: &mut Commands,
		asset_server: &AssetServer,
	) {
		const PREVIEW_TINT: Color = Color::hsla(0., 0.5, 1., 0.7);

		match self {
			Self::Single => {
				// Using start_position has the effect of "locking" the building where the click started.
				let preview_position = GridBox::around(start_position, previewed.size().flat()).smallest();
				let any_child = current_children.next();
				if let Some((_, mut existing_child)) = any_child {
					*existing_child = preview_position;
				} else {
					let image = preview_image_for_buildable(previewed);
					commands.entity(parent_entity).with_children(|parent| {
						parent.spawn((PreviewChild, preview_position, ObjectPriority::Overlay, Sprite {
							color: PREVIEW_TINT,
							anchor: anchor_for_image(image),
							image: asset_server.load(image),
							..Default::default()
						}));
					});
				}
			},
			Self::Line => {
				let required_positions = start_position.line_to_2d(current_position);
				for element in required_positions.zip_longest(current_children) {
					match element {
						EitherOrBoth::Both(position, (_, mut child)) => *child = position,
						// Create new child.
						EitherOrBoth::Left(position) => {
							let image = preview_image_for_buildable(previewed);
							commands.entity(parent_entity).with_children(|parent| {
								parent.spawn((PreviewChild, ObjectPriority::Overlay, position, Sprite {
									color: PREVIEW_TINT,
									anchor: anchor_for_image(image),
									image: asset_server.load(image),
									..Default::default()
								}));
							});
						},
						// Destroy not needed child.
						EitherOrBoth::Right((child, _)) => {
							commands.entity(child).despawn_recursive();
						},
					}
				}
			},
			Self::Rect => {
				let smaller_corner = start_position.component_wise_min(current_position);
				let larger_corner = start_position.component_wise_max(current_position);

				let mut parent = commands.entity(parent_entity);
				let image = preview_image_for_buildable(previewed);

				for x in smaller_corner.x ..= larger_corner.x {
					for y in smaller_corner.y ..= larger_corner.y {
						if let Some((_, mut old_child_position)) = current_children.next() {
							old_child_position.x = x;
							old_child_position.y = y;
						} else {
							parent.with_children(|parent| {
								parent.spawn((
									PreviewChild,
									ObjectPriority::Overlay,
									GridPosition::from((x, y, start_position.z)),
									Sprite {
										color: PREVIEW_TINT,
										anchor: anchor_for_image(image),
										image: asset_server.load(image),
										..Default::default()
									},
								));
							});
						}
					}
				}

				// Despawn all superfluous old children.
				for (superfluous_child, _) in current_children {
					commands.entity(superfluous_child).despawn_recursive();
				}
			},
		}
	}
}

/// This always sets the building preview's current point to the mouse cursor. [`handle_build_interactions`] copies this
/// into the start point when needed.
fn set_building_preview_start(
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform), With<InGameCamera>>,
	mut preview: Query<&mut PreviewParent>,
) {
	let (camera, camera_transform) = camera_q.single();
	let window = windows.single();

	let cursor_position =
		window.cursor_position().and_then(|cursor| camera_to_world(cursor, window, camera, camera_transform));
	if cursor_position.is_none() {
		return;
	}
	// Since the anchors are on the lower left corner of the sprite, we need to offset the cursor half a tile.
	let cursor_position = cursor_position.unwrap();
	// FIXME: Use ray casting + structure data to figure out the elevation under the cursor.
	let fake_z = 0.;
	// Since we measure positions from corners, offset the cursor half a tile so we move the preview around its center.
	let world_position = (engine_to_world_space(cursor_position, fake_z) - Vec3::new(0.5, 0.5, 0.)).round();
	for mut preview_data in &mut preview {
		preview_data.current_position = world_position;
	}
}

fn update_building_preview(
	mouse: Res<ButtonInput<MouseButton>>,
	mut commands: Commands,
	mut preview: Query<(Entity, Option<&mut Children>, &PreviewParent, &mut Visibility)>,
	preview_children: Query<&mut GridPosition, With<PreviewChild>>,
	asset_server: Res<AssetServer>,
) {
	for (parent_entity, children, preview_data, mut visibility) in &mut preview {
		// SAFETY: We never obtain the same component twice, since the entity IDs in the iterator are distinct.
		// Therefore, we do not alias a mutable pointer to the same component.
		let children = children.iter().flatten().flat_map(|entity| {
			if let Ok(child) = unsafe { preview_children.get_unchecked(*entity) } {
				Some((*entity, child))
			} else {
				None
			}
		});
		preview_data.previewed.build_mode().update_preview(
			*preview_data,
			children,
			parent_entity,
			&mut commands,
			&asset_server,
		);
		// Make sure to delay displaying the preview until after the user releases the mouse after clicking the button.
		// On second click, since we never set the building to invisible again, it doesn't matter.
		if !mouse.pressed(MouseButton::Left) {
			*visibility = Visibility::Visible;
		}
	}
}

fn create_building_preview(
	mut commands: Commands,
	current_preview: Query<Entity, With<PreviewParent>>,
	mut events: EventReader<StartBuildPreview>,
) {
	if !events.is_empty() {
		for old_preview in &current_preview {
			commands.entity(old_preview).despawn_recursive();
		}
	}
	for event in events.read() {
		commands.spawn((
			PreviewParent::new(event.buildable),
			ObjectPriority::Overlay,
			Visibility::Hidden,
			// Bare minimum collection of components to make this entity and its children render.
			Transform::default(),
			GlobalTransform::default(),
			InheritedVisibility::default(),
			ViewVisibility::default(),
		));
	}
}

fn perform_ground_build(
	mut event: EventReader<PerformBuild<{ BuildableType::Ground }>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut ground_map: ResMut<GroundMap>,
	mut tile_query: Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
	mut area_update_event: EventWriter<UpdateAreas>,
) {
	for event in event.read() {
		let kind = match event.buildable {
			Buildable::Ground(kind) => kind,
			_ => unreachable!(),
		};
		for line_element in event.start_position.line_to_2d(event.end_position) {
			ground_map.set(line_element, kind, &mut tile_query, &mut commands, &asset_server);
		}
		// Either we or the tiles we overwrote might be part of areas.
		area_update_event.send_default();
	}
	event.clear();
}

fn perform_pitch_build(
	mut event: EventReader<PerformBuild<{ BuildableType::Pitch }>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut ground_map: ResMut<GroundMap>,
	mut tile_query: Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
	mut area_update_event: EventWriter<UpdateAreas>,
) {
	for event in event.read() {
		ground_map.fill_rect(
			event.start_position,
			event.end_position,
			GroundKind::Pitch,
			&mut tile_query,
			&mut commands,
			&asset_server,
		);
		commands.spawn(AccommodationBundle::new(event.start_position, event.end_position));
		area_update_event.send_default();
	}
	event.clear();
}

fn perform_pool_area_build(
	mut event: EventReader<PerformBuild<{ BuildableType::PoolArea }>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut ground_map: ResMut<GroundMap>,
	mut tile_query: Query<(Entity, &GridPosition, &mut GroundKind, &mut WorldInfoProperties)>,
	mut area_update_event: EventWriter<UpdateAreas>,
) {
	for event in event.read() {
		ground_map.fill_rect(
			event.start_position,
			event.end_position,
			GroundKind::PoolPath,
			&mut tile_query,
			&mut commands,
			&asset_server,
		);
		commands.spawn((Area::from_rect(event.start_position, event.end_position), Pool));
		area_update_event.send_default();
	}
	event.clear();
}

fn perform_pitch_type_build(
	mut event: EventReader<PerformBuild<{ BuildableType::PitchType }>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut pitches: Query<(Entity, &Area, &mut Pitch)>,
	mut build_error: EventWriter<ErrorBox>,
	mut area_update_event: EventWriter<UpdateAreas>,
) {
	for event in event.read() {
		let kind = match event.buildable {
			Buildable::PitchType(kind) => kind,
			_ => unreachable!(),
		};
		let start_position = event.start_position;
		let mut pitch = OnceLock::new();
		pitches.par_iter_mut().for_each(|(entity, area, pitch_candidate)| {
			// Perform work immediately, since only one pitch should contain this pitch type.
			if area.contains(&start_position) {
				let _ = pitch.set((entity, area, pitch_candidate));
			}
		});

		if pitch.get().is_none() {
			build_error.send(BuildError::NoAccommodationHere.into());
			return;
		}
		let (pitch_entity, area, pitch) = pitch.get_mut().unwrap();
		let pitch_box = GridBox::around(start_position, kind.size().flat());
		if !area.fits(&pitch_box) {
			build_error.send(BuildError::NoSpace.into());
			return;
		}
		if area.size() < kind.required_area() {
			build_error.send(BuildError::PitchTooSmall { required: kind.required_area(), actual: area.size() }.into());
			return;
		}

		pitch.kind = Some(kind);
		if let Some(bundle) = AccommodationBuildingBundle::new(kind, start_position, &asset_server) {
			commands.entity(*pitch_entity).with_children(|parent| {
				parent.spawn(bundle);
			});
		}

		commands.entity(*pitch_entity).remove::<Area>().insert(ImmutableArea((*area).clone()));
		area_update_event.send_default();
	}
	event.clear();
}

fn handle_build_interactions(
	mouse: Res<ButtonInput<MouseButton>>,
	mut state: ResMut<NextState<InputState>>,
	mut preview: Query<&mut PreviewParent>,
	all_interacted: Query<&Interaction, (With<Node>, Changed<Interaction>)>,
	mut pitch_type_build_event: EventWriter<PerformBuild<{ BuildableType::PitchType }>>,
	mut ground_build_event: EventWriter<PerformBuild<{ BuildableType::Ground }>>,
	mut pitch_build_event: EventWriter<PerformBuild<{ BuildableType::Pitch }>>,
	mut pool_build_event: EventWriter<PerformBuild<{ BuildableType::PoolArea }>>,
) {
	let any_ui_active = all_interacted.iter().any(|interaction| interaction != &Interaction::None);

	for mut preview_data in &mut preview {
		// Probably before the user released the mouse from clicking the build button.
		if any_ui_active {
			preview_data.start_position = preview_data.current_position;
			return;
		}

		if mouse.just_released(MouseButton::Left) {
			state.set(InputState::Idle);
			// Transform a "dynamic" PerformBuild instantiation into a static one.
			match BuildableType::from(preview_data.previewed) {
				BuildableType::Ground => {
					ground_build_event.send(PerformBuild {
						start_position: preview_data.start_position,
						end_position:   preview_data.current_position,
						buildable:      preview_data.previewed,
					});
				},
				BuildableType::PoolArea => {
					pool_build_event.send(PerformBuild {
						start_position: preview_data.start_position,
						end_position:   preview_data.current_position,
						buildable:      preview_data.previewed,
					});
				},
				BuildableType::Pitch => {
					pitch_build_event.send(PerformBuild {
						start_position: preview_data.start_position,
						end_position:   preview_data.current_position,
						buildable:      preview_data.previewed,
					});
				},
				BuildableType::PitchType => {
					pitch_type_build_event.send(PerformBuild {
						start_position: preview_data.start_position,
						end_position:   preview_data.current_position,
						buildable:      preview_data.previewed,
					});
				},
			}
		}
		// Keep start and current identical as long as the mouse is not pressed.
		// This has the effect that it establishes the building's start corner once the user starts clicking.
		if !mouse.pressed(MouseButton::Left) {
			preview_data.start_position = preview_data.current_position;
		}
	}
}

fn destroy_building_preview(mut commands: Commands, preview: Query<Entity, With<PreviewParent>>) {
	for entity in &preview {
		commands.get_entity(entity).unwrap().despawn_recursive();
	}
}

fn end_building(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<NextState<InputState>>) {
	if keys.just_pressed(KeyCode::Escape) {
		state.set(InputState::Idle);
	}
}
