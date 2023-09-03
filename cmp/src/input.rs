use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// What the player is currently doing in the UI.
#[derive(States, Hash, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputState {
	/// Not doing anything.
	Idle,
	/// Placing a building.
	Building,
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
		app.add_state::<InputState>().add_systems(Update, move_camera.run_if(in_state(InputState::Idle)));
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
