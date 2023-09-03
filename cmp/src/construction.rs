use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;

use crate::geometry::GridPosition;
use crate::graphics::{screen_to_discrete_world_space, StaticSprite};
use crate::input::InputState;

pub struct ConstructionPlugin;

impl Plugin for ConstructionPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<PerformBuild>()
			.add_systems(Update, display_building_preview.run_if(in_state(InputState::Building)))
			.add_systems(OnEnter(InputState::Building), create_building_preview.before(display_building_preview))
			.add_systems(OnExit(InputState::Building), destroy_building_preview.after(display_building_preview))
			.add_systems(Update, enter_build_mode.before(create_building_preview).before(destroy_building_preview))
			.add_systems(Update, try_building.after(enter_build_mode).run_if(in_state(InputState::Building)))
			.add_systems(Update, perform_build.after(try_building));
	}
}

#[derive(Event)]
struct PerformBuild {
	building_position: GridPosition,
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
	let fake_z = 0;
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
