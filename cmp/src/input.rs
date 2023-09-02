use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::geometry::GridPosition;
use crate::graphics::{screen_to_discrete_world_space, StaticSprite};

/// What the player is currently doing in the UI.
#[derive(States, Hash, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputState {
	/// Not doing anything.
	Idle,
	/// Placing a building.
	Building,
}

#[derive(Event)]
struct PerformBuild {
	building_position: GridPosition,
}

impl Default for InputState {
	fn default() -> Self {
		Self::Idle
	}
}

#[derive(Default)]
pub struct GUIInputPlugin;

impl Plugin for GUIInputPlugin {
	fn build(&self, app: &mut App) {
		app.add_state::<InputState>()
			.add_event::<PerformBuild>()
			.add_systems(Update, display_building_preview.run_if(in_state(InputState::Building)))
			.add_systems(OnEnter(InputState::Building), create_building_preview.before(display_building_preview))
			.add_systems(OnExit(InputState::Building), destroy_building_preview.after(display_building_preview))
			.add_systems(Update, enter_build_mode.before(create_building_preview).before(destroy_building_preview))
			.add_systems(Update, move_camera.run_if(in_state(InputState::Idle)))
			.add_systems(Update, try_building.after(enter_build_mode).run_if(in_state(InputState::Building)))
			.add_systems(Update, perform_build.after(try_building));
	}
}

/// Marker component for the building acting as a preview.
#[derive(Component)]
struct PreviewBuilding;

fn display_building_preview(
	windows: Query<&Window, With<PrimaryWindow>>,
	mut preview: Query<&mut GridPosition, With<PreviewBuilding>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
) {
	let (camera, camera_transform) = camera_q.single();
	let window = windows.single();

	let cursor_position = window
		.cursor_position()
		.and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
		.map(|ray| ray.origin.truncate());
	if cursor_position.is_none() {
		return;
	}
	let cursor_position = cursor_position.unwrap();
	// FIXME: Use ray casting + structure data to figure out the elevation under the cursor.
	let fake_z = 3;
	let world_position = screen_to_discrete_world_space(cursor_position, fake_z);
	for mut preview in &mut preview {
		*preview = world_position;
	}
}

fn create_building_preview(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn((
		PreviewBuilding,
		StaticSprite {
			bevy_sprite: SpriteBundle {
				texture: asset_server.load("2x3-house-template.png"),
				sprite: Sprite {
					color: Color::Hsla { hue: 0., saturation: 1., lightness: 1., alpha: 0.8 },
					anchor: Anchor::Center,
					..Default::default()
				},
				..Default::default()
			},
		},
		GridPosition::default(),
	));
}

fn perform_build(mut commands: Commands, asset_server: Res<AssetServer>, mut event: EventReader<PerformBuild>) {
	for event in &mut event {
		commands.spawn((
			StaticSprite {
				bevy_sprite: SpriteBundle {
					texture: asset_server.load("2x3-house-template.png"),
					sprite: Sprite { anchor: Anchor::Center, ..Default::default() },
					..Default::default()
				},
			},
			event.building_position,
		));
	}
}

fn try_building(
	mouse: Res<Input<MouseButton>>,
	mut state: ResMut<NextState<InputState>>,
	preview: Query<&GridPosition, With<PreviewBuilding>>,
	mut event: EventWriter<PerformBuild>,
) {
	for preview in &preview {
		if mouse.just_pressed(MouseButton::Left) {
			state.set(InputState::Idle);
			event.send(PerformBuild { building_position: *preview });
		}
	}
}

fn destroy_building_preview(mut commands: Commands, preview: Query<(Entity, &PreviewBuilding)>) {
	for (entity, _) in &preview {
		commands.get_entity(entity).unwrap().despawn();
	}
}

fn enter_build_mode(
	keys: Res<Input<KeyCode>>,
	current_state: Res<State<InputState>>,
	mut state: ResMut<NextState<InputState>>,
) {
	if keys.just_pressed(KeyCode::B) && *current_state != InputState::Building {
		state.set(InputState::Building);
	} else if keys.just_pressed(KeyCode::Escape) {
		state.set(InputState::Idle);
	}
}

const ZOOM_SPEED: f32 = 0.2;

fn move_camera(
	mut scroll_events: EventReader<MouseWheel>,
	mouse: Res<Input<MouseButton>>,
	window: Query<&Window, With<PrimaryWindow>>,
	mut camera_q: Query<(&Camera, &mut OrthographicProjection, &mut Transform, &GlobalTransform)>,
	mut last_screen_position: Local<Option<Vec2>>,
) {
	let window = window.single();
	let (camera, mut camera_projection, mut camera_transform, camera_global_transform) = camera_q.single_mut();

	if let Some(current_screen_position) = window.cursor_position() {
		let current_world_position =
			camera.viewport_to_world(camera_global_transform, current_screen_position).unwrap().origin.truncate();

		if let Some(last_screen_position) = *last_screen_position && mouse.pressed(MouseButton::Left) {
			let last_world_position =
				camera.viewport_to_world(camera_global_transform, last_screen_position).unwrap().origin.truncate();
			let delta = last_world_position - current_world_position;
			camera_transform.translation += Vec3::from((delta, 0.));
		}

		*last_screen_position = if mouse.pressed(MouseButton::Left) { Some(current_screen_position) } else { None };
	}

	for scroll in &mut scroll_events {
		let amount = scroll.y;
		camera_projection.scale -= amount * ZOOM_SPEED * camera_projection.scale;
	}
}
