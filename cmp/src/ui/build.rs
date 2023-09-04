use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use itertools::{EitherOrBoth, Itertools};

use super::on_start_build_preview;
use crate::graphics::library::{anchor_for_sprite, sprite_for_buildable};
use crate::graphics::{screen_to_world_space, StaticSprite};
use crate::input::InputState;
use crate::model::{
	AccommodationBundle, Buildable, GridPosition, GroundKind, GroundTile, GroundTileCleanupNeeded, NewGroundTile,
};

pub struct BuildPlugin;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<PerformBuild>()
			.add_event::<StartBuildPreview>()
			.add_systems(
				Update,
				update_building_preview
					.after(create_building_preview)
					.after(on_start_build_preview)
					.run_if(in_state(InputState::Building)),
			)
			.add_systems(
				Update,
				(handle_build_interactions, set_building_preview_start, end_building)
					.run_if(in_state(InputState::Building))
					.before(perform_build),
			)
			.add_systems(Update, create_building_preview)
			.add_systems(OnExit(InputState::Building), destroy_building_preview.after(update_building_preview))
			.add_systems(Update, perform_build);
	}
}

#[derive(Event)]
pub struct StartBuildPreview {
	pub buildable: Buildable,
}

#[derive(Event)]
struct PerformBuild {
	start_position: GridPosition,
	end_position:   GridPosition,
	buildable:      Buildable,
}

/// Component for the building preview's parent entity.
#[derive(Component, Clone, Copy, Debug)]
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
#[derive(Component)]
struct PreviewChild;

// #[derive(Bundle)]
// struct PreviewParentBundle {
// 	preview:  PreviewParent,
// 	position: GridPosition,
// 	size:     BoundingBox,
// 	sprite:   StaticSprite,
// }

// impl PreviewParentBundle {
// 	pub fn new(buildable: Buildable, asset_server: &AssetServer) -> Self {
// 		let sprite = sprite_for_buildable(buildable);
// 		Self {
// 			preview:  PreviewParent::new(buildable),
// 			position: GridPosition::default(),
// 			// Extremely high priority.
// 			size:     buildable.size().with_height(1000),
// 			sprite:   StaticSprite {
// 				bevy_sprite: SpriteBundle {
// 					sprite: Sprite {
// 						color: Color::Hsla { hue: 0., saturation: 1., lightness: 1., alpha: 0.4 },
// 						anchor: anchor_for_sprite(sprite),
// 						..Default::default()
// 					},
// 					visibility: Visibility::Hidden,
// 					texture: asset_server.load(sprite),
// 					..Default::default()
// 				},
// 			},
// 		}
// 	}
// }

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
		const PREVIEW_TINT: Color = Color::Hsla { hue: 0., saturation: 1., lightness: 1., alpha: 0.4 };

		match self {
			Self::Single => {
				// Using start_position has the effect of "locking" the building where the click started.
				let preview_position = start_position - (previewed.size() / 2).truncate();
				let any_child = current_children.next();
				if let Some((_, mut existing_child)) = any_child {
					*existing_child = preview_position;
				} else {
					let sprite = sprite_for_buildable(previewed);
					commands.entity(parent_entity).with_children(|parent| {
						parent.spawn((
							PreviewChild,
							preview_position,
							// Extremely high priority.
							previewed.size().with_height(1000),
							StaticSprite {
								bevy_sprite: SpriteBundle {
									sprite: Sprite {
										color: PREVIEW_TINT,
										anchor: anchor_for_sprite(sprite),
										..Default::default()
									},
									texture: asset_server.load(sprite),
									..Default::default()
								},
							},
						));
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
							let sprite = sprite_for_buildable(previewed);
							commands.entity(parent_entity).with_children(|parent| {
								parent.spawn((
									PreviewChild,
									position,
									// Extremely high priority.
									previewed.size().with_height(1000),
									StaticSprite {
										bevy_sprite: SpriteBundle {
											sprite: Sprite {
												color: PREVIEW_TINT,
												anchor: anchor_for_sprite(sprite),
												..Default::default()
											},
											texture: asset_server.load(sprite),
											..Default::default()
										},
									},
								));
							});
						},
						// Destroy not needed child.
						EitherOrBoth::Right((child, _)) => {
							commands.entity(child).despawn_recursive();
						},
					}
				}
			},
			Self::Rect => {},
		}
	}
}

/// This always sets the building preview's current point to the mouse cursor. [`handle_build_interactions`] copies this
/// into the start point when needed.
fn set_building_preview_start(
	windows: Query<&Window, With<PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut preview: Query<&mut PreviewParent>,
) {
	let (camera, camera_transform) = camera_q.single();
	let window = windows.single();

	let cursor_position =
		window.cursor_position().and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor));
	if cursor_position.is_none() {
		return;
	}
	// Since the anchors are on the lower left corner of the sprite, we need to offset the cursor half a tile.
	let cursor_position = cursor_position.unwrap();
	// FIXME: Use ray casting + structure data to figure out the elevation under the cursor.
	let fake_z = 0.;
	// Since we measure positions from corners, offset the cursor half a tile so we move the preview around its center.
	let world_position = (screen_to_world_space(cursor_position, fake_z) - Vec3::new(0.5, 0.5, 0.)).round();
	for mut preview_data in &mut preview {
		preview_data.current_position = world_position;
	}
}

fn update_building_preview(
	mut commands: Commands,
	mut preview: Query<(Entity, Option<&mut Children>, &PreviewParent, &mut Visibility)>,
	preview_children: Query<&mut GridPosition, With<PreviewChild>>,
	asset_server: Res<AssetServer>,
) {
	for (parent_entity, children, preview_data, mut visibility) in &mut preview {
		// SAFETY: We never obtain the same component twice, since the entity IDs in the iterator are distinct.
		// Therefore, we do not alias a mutable pointer to the same component.
		let children = children
			.iter()
			.flatten()
			.map(|entity| (*entity, unsafe { preview_children.get_unchecked(*entity) }.unwrap()));
		preview_data.previewed.build_mode().update_preview(
			*preview_data,
			children,
			parent_entity,
			&mut commands,
			&asset_server,
		);
		*visibility = Visibility::Visible;
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
	for event in &mut events {
		commands.spawn((
			PreviewParent::new(event.buildable),
			Visibility::Hidden,
			// Bare minimum collection of components to make this entity and its children render.
			Transform::default(),
			GlobalTransform::default(),
			ComputedVisibility::default(),
		));
	}
}

fn perform_build(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut event: EventReader<PerformBuild>,
	mut ground_event: EventWriter<GroundTileCleanupNeeded>,
) {
	for event in &mut event {
		// TODO: Check legality of the build action.
		perform_build_action(event.buildable, &mut commands, event.start_position, event.end_position, &asset_server);
		ground_event.send_default();
	}
}

fn perform_build_action(
	kind: Buildable,
	commands: &mut Commands,
	start_position: GridPosition,
	end_position: GridPosition,
	asset_server: &AssetServer,
) {
	match kind {
		Buildable::Ground(kind) =>
			for line_element in start_position.line_to_2d(end_position) {
				commands.spawn((GroundTile::new(kind, line_element, asset_server), NewGroundTile));
			},
		Buildable::PoolArea => {
			// FIXME: not accurately modeled!
			commands.spawn(GroundTile::new(GroundKind::PoolPath, start_position, asset_server));
		},
		Buildable::BasicAccommodation(kind) => {
			commands.spawn(AccommodationBundle::new(kind, start_position, asset_server));
		},
	};
}

fn handle_build_interactions(
	mouse: Res<Input<MouseButton>>,
	mut state: ResMut<NextState<InputState>>,
	mut preview: Query<&mut PreviewParent>,
	all_interacted: Query<&Interaction, (With<Node>, Changed<Interaction>)>,
	mut event: EventWriter<PerformBuild>,
) {
	let any_ui_active = all_interacted.iter().any(|interaction| interaction != &Interaction::None);
	if any_ui_active {
		return;
	}

	for mut preview_data in &mut preview {
		if mouse.just_released(MouseButton::Left) {
			state.set(InputState::Idle);
			event.send(PerformBuild {
				start_position: preview_data.start_position,
				end_position:   preview_data.current_position,
				buildable:      preview_data.previewed,
			});
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

fn end_building(keys: Res<Input<KeyCode>>, mut state: ResMut<NextState<InputState>>) {
	if keys.just_pressed(KeyCode::Escape) {
		state.set(InputState::Idle);
	}
}