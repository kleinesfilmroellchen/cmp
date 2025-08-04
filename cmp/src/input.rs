use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};

use crate::gamemode::GameState;
use crate::graphics::{InGameCamera, RES_HEIGHT, RES_WIDTH};

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
		app.init_state::<InputState>().init_resource::<DragStartPosition>().add_event::<MouseClick>().add_systems(
			Update,
			(
				move_camera.run_if(in_state(InputState::Idle)),
				fix_camera.run_if(not(in_state(InputState::Idle))),
				zoom_camera,
				fullscreen,
			)
				.in_set(GameState::InGame),
		);
	}
}

/// Both a world-space and camera-space position.
#[derive(Default, Copy, Clone)]
struct MatchedPosition {
	screen_pos: Vec2,
	camera_pos: Vec3,
}
#[derive(Resource, Default, Copy, Clone)]
pub(crate) struct DragStartPosition(Option<MatchedPosition>);

const DRAG_THRESHOLD: f32 = 0.2;

#[derive(Event, Debug, Clone, Copy)]
pub struct MouseClick {
	#[allow(unused)]
	pub screen_position: Vec2,
	pub engine_position: Vec2,
}

pub fn camera_to_world(
	position: Vec2,
	window: &Window,
	camera: &Camera,
	camera_transform: &GlobalTransform,
) -> Option<Vec2> {
	let width_size_ratio = window.width() / RES_WIDTH as f32;
	let height_size_ratio = window.height() / RES_HEIGHT as f32;
	// Transform the window position into the kind of position that the pixel perfect camera would see
	let real_position = position / Vec2::new(width_size_ratio, height_size_ratio);
	camera.viewport_to_world(camera_transform, real_position).map(|p| p.origin.truncate()).ok()
}

pub fn world_to_camera(
	position: Vec3,
	window: &Window,
	camera: &Camera,
	camera_transform: &GlobalTransform,
) -> Option<Vec2> {
	let width_size_ratio = window.width() / RES_WIDTH as f32;
	let height_size_ratio = window.height() / RES_HEIGHT as f32;
	// Transform the window position into the kind of position that the pixel perfect camera would see
	let unscaled_position = camera.world_to_viewport(camera_transform, position).ok()?;
	let real_position = unscaled_position * Vec2::new(width_size_ratio, height_size_ratio);
	Some(real_position)
}

pub(crate) fn move_camera(
	mouse: Res<ButtonInput<MouseButton>>,
	window: Query<&Window, With<PrimaryWindow>>,
	mut camera_q: Query<(&Camera, &mut Transform, &GlobalTransform), With<InGameCamera>>,
	mut drag_start_position: ResMut<DragStartPosition>,
	mut click_event: EventWriter<MouseClick>,
) -> Result {
	let window = window.single()?;
	let (camera, mut camera_transform, camera_global_transform) = camera_q.single_mut()?;

	if let Some(current_screen_position) = window.cursor_position() {
		let Some(current_engine_position) =
			camera_to_world(current_screen_position, window, camera, camera_global_transform)
		else {
			return Ok(());
		};

		'pos: {
			if let Some(drag_start_screen_position) = drag_start_position.0
				&& mouse.pressed(MouseButton::Left)
			{
				let Some(drag_start_engine_position) =
					camera_to_world(drag_start_screen_position.screen_pos, window, camera, camera_global_transform)
				else {
					break 'pos;
				};
				// Snap camera to pixel grid. This masks all issues with slightly misaligned objects that wouldn’t move
				// in sync when dragging the camera. The steppy movement is only really noticeable at large zoom
				// levels, and not too jarring since it works correctly no matter the drag speed.
				let delta = (drag_start_engine_position - current_engine_position).round();
				camera_transform.translation =
					(drag_start_screen_position.camera_pos + Vec3::from((delta, 0.))).round();
			}
		}

		if mouse.just_pressed(MouseButton::Left) {
			drag_start_position.0 =
				Some(MatchedPosition { screen_pos: current_screen_position, camera_pos: camera_transform.translation });
		}

		'drag: {
			if let Some(drag_start_screen_position) = drag_start_position.0
				&& mouse.just_released(MouseButton::Left)
			{
				let Some(drag_start_world_position) =
					camera_to_world(drag_start_screen_position.screen_pos, window, camera, camera_global_transform)
				else {
					break 'drag;
				};
				let delta = drag_start_world_position - current_engine_position;

				if delta.length() < DRAG_THRESHOLD {
					click_event.write(MouseClick {
						screen_position: current_screen_position,
						engine_position: current_engine_position,
					});
				}
			}
		}

		if mouse.just_released(MouseButton::Left) {
			drag_start_position.0 = None;
		}
	}
	Ok(())
}

fn fix_camera(mut drag_start_position: ResMut<DragStartPosition>) {
	// Prevents large screen jumps due to a press registering "across" the input mode change.
	drag_start_position.0 = None;
}

/// `accumulated_scroll` takes care of small-increment smooth scrolling devices like trackpads.
fn zoom_camera(
	mut scroll_events: EventReader<MouseWheel>,
	mut camera_q: Query<&mut Projection, With<InGameCamera>>,
	mut accumulated_scroll: Local<f32>,
) -> Result {
	let mut camera_projection = camera_q.single_mut()?;
	let Projection::Orthographic(camera_projection) = camera_projection.as_mut() else {
		return Ok(());
	};

	let amount = scroll_events.read().map(|scroll| scroll.y).sum::<f32>();
	if amount == 0. {
		return Ok(());
	}

	// If changing scroll direction, snap accumulation to 0 so that it doesn’t take longer to zoom than if you didn’t
	// change direction.
	if accumulated_scroll.signum() != amount.signum() {
		*accumulated_scroll = 0.;
	}
	// Accumulate scroll so that small scroll increments don’t get lost.
	*accumulated_scroll += amount;
	if accumulated_scroll.abs() < 1. {
		// Below a total scroll of 1, nothing happens due to the zoom math below, so we can skip updating the camera
		// transform altogether.
		return Ok(());
	}

	// Only allow power-of-two scales, since those will not cause off-by-one rendering glitches.
	camera_projection.scale =
		2f32.powf(camera_projection.scale.log2().round() - *accumulated_scroll).clamp(1. / 16., 8.);
	// HACK: Exact scale of 1 is very glitchy for some reason
	// if camera_projection.scale == 1. {
	// 	camera_projection.scale = 1.0001;
	// }

	// Since we just scrolled, reset the accumulator.
	*accumulated_scroll = 0.;

	Ok(())
}

fn fullscreen(
	keys: Res<ButtonInput<KeyCode>>,
	mut windows: Query<&mut bevy::prelude::Window, With<PrimaryWindow>>,
) -> Result {
	let mut window = windows.single_mut()?;

	if keys.just_pressed(KeyCode::F11) {
		window.mode = match window.mode {
			// FIXME: only use borderless fullscreen on Wayland?
			WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
			_ => WindowMode::Windowed,
		};
	}
	Ok(())
}
