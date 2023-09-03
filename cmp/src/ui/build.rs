use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::graphics::library::{anchor_for_sprite, sprite_for_buildable};
use crate::graphics::{screen_to_world_space, StaticSprite};
use crate::input::InputState;
use crate::model::{BoundingBox, Buildable, GridPosition};

pub struct BuildPlugin;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<PerformBuild>()
			.add_event::<StartBuildPreview>()
			.add_systems(Update, display_building_preview.run_if(in_state(InputState::Building)))
			.add_systems(OnEnter(InputState::Building), create_building_preview.before(display_building_preview))
			.add_systems(OnExit(InputState::Building), destroy_building_preview.after(display_building_preview))
			.add_systems(Update, try_building.run_if(in_state(InputState::Building)))
			.add_systems(Update, perform_build.after(try_building));
	}
}

#[derive(Event)]
pub struct StartBuildPreview {
	pub buildable: Buildable,
}

#[derive(Event)]
struct PerformBuild {
	building_position: GridPosition,
	buildable:         Buildable,
}

/// Component for the building acting as a preview.
#[derive(Component)]
struct PreviewBuilding(pub Buildable);

#[derive(Bundle)]
struct PreviewBundle {
	marker:   PreviewBuilding,
	position: GridPosition,
	size:     BoundingBox,
	sprite:   StaticSprite,
}

impl PreviewBundle {
	pub fn new(buildable: Buildable, asset_server: &AssetServer) -> Self {
		let sprite = sprite_for_buildable(buildable);
		Self {
			marker:   PreviewBuilding(buildable),
			position: GridPosition::default(),
			// Extremely high priority.
			size:     buildable.size().with_height(1000),
			sprite:   StaticSprite {
				bevy_sprite: SpriteBundle {
					sprite: Sprite {
						color: Color::Hsla { hue: 0., saturation: 1., lightness: 1., alpha: 0.4 },
						anchor: anchor_for_sprite(sprite),
						..Default::default()
					},
					texture: asset_server.load(sprite),
					..Default::default()
				},
			},
		}
	}
}

fn display_building_preview(
	windows: Query<&Window, With<PrimaryWindow>>,
	mut preview: Query<(&mut GridPosition, &BoundingBox), With<PreviewBuilding>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
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
	// FIXME: Empirically determined cursor offset error.
	let world_position = (screen_to_world_space(cursor_position, fake_z) - Vec3::new(0.9, 0., 0.)).round();
	for (mut preview_position, preview_size) in &mut preview {
		*preview_position = world_position - (*preview_size / 2).truncate();
		info!("{:?}, {:?}", preview_position.0, (*preview_size / 2).truncate());
	}
}

fn create_building_preview(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut event: EventReader<StartBuildPreview>,
) {
	for event in &mut event {
		commands.spawn(PreviewBundle::new(event.buildable, &asset_server));
	}
}

fn perform_build(mut commands: Commands, asset_server: Res<AssetServer>, mut event: EventReader<PerformBuild>) {
	for event in &mut event {
		event.buildable.spawn_entity(&mut commands, event.building_position, &asset_server);
	}
}

fn try_building(
	mouse: Res<Input<MouseButton>>,
	mut state: ResMut<NextState<InputState>>,
	preview: Query<(&GridPosition, &PreviewBuilding)>,
	mut event: EventWriter<PerformBuild>,
) {
	for (preview_position, preview_data) in &preview {
		if mouse.just_pressed(MouseButton::Left) {
			state.set(InputState::Idle);
			event.send(PerformBuild { building_position: *preview_position, buildable: preview_data.0 });
		}
	}
}

fn destroy_building_preview(mut commands: Commands, preview: Query<Entity, With<PreviewBuilding>>) {
	for entity in &preview {
		commands.get_entity(entity).unwrap().despawn();
	}
}
