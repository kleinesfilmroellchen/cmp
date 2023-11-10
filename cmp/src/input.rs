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
		app.add_state::<InputState>().insert_resource(LastScreenPosition::default()).add_systems(
			Update,
			(
				move_camera.run_if(in_state(InputState::Idle)),
				fix_camera.run_if(not(in_state(InputState::Idle))),
				zoom_camera,
			),
		);
	}
}

/// The last position on the screen where the user held the primary mouse button; used mainly for panning functionality.
#[derive(Resource, Default)]
struct LastScreenPosition(Option<Vec2>);

fn move_camera(
	mouse: Res<Input<MouseButton>>,
	window: Query<&Window, With<PrimaryWindow>>,
	mut camera_q: Query<(&Camera, &mut Transform, &GlobalTransform)>,
	mut last_screen_position: ResMut<LastScreenPosition>,
) {
	let window = window.single();
	let (camera, mut camera_transform, camera_global_transform) = camera_q.single_mut();

	if let Some(current_screen_position) = window.cursor_position() {
		let current_world_position =
			camera.viewport_to_world(camera_global_transform, current_screen_position).unwrap().origin.truncate();

		if let Some(last_screen_position) = last_screen_position.0
			&& mouse.pressed(MouseButton::Left)
		{
			let last_world_position =
				camera.viewport_to_world(camera_global_transform, last_screen_position).unwrap().origin.truncate();
			let delta = last_world_position - current_world_position;
			camera_transform.translation += Vec3::from((delta, 0.));
		}

		last_screen_position.0 = if mouse.pressed(MouseButton::Left) { Some(current_screen_position) } else { None };
	}
}

fn fix_camera(mut last_screen_position: ResMut<LastScreenPosition>) {
	// Prevents large screen jumps due to a press registering "across" the input mode change.
	last_screen_position.0 = None;
}

fn zoom_camera(
	mut scroll_events: EventReader<MouseWheel>,
	mut camera_q: Query<&mut OrthographicProjection, With<Camera>>,
) {
	let mut camera_projection = camera_q.single_mut();

	for scroll in &mut scroll_events {
		let amount = scroll.y;
		// Only allow power-of-two scales, since those will not cause off-by-one rendering glitches.
		camera_projection.scale = 2.0f32.powf(camera_projection.scale.log2().round() - amount).clamp(1. / 16., 2.);
		// HACK: Exact scale of 1 is very glitchy for some reason
		if camera_projection.scale == 1. {
			camera_projection.scale = 1.0001;
		}
	}
}
