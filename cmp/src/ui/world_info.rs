use std::ops::DerefMut;
use std::sync::Arc;

use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::ui::FocusPolicy;
use bevy::utils::Instant;
use parking_lot::Mutex;

use crate::graphics::library::{font_for, FontStyle, FontWeight};
use crate::graphics::{TILE_HEIGHT, TILE_WIDTH};
use crate::model::{AccommodationType, Comfort};

#[derive(Component, Default)]
pub struct WorldInfoUI {
	attached_entity: Option<Entity>,
}
#[derive(Component)]
pub struct WorldInfoTitle;
#[derive(Component)]
pub struct WorldInfoBody;
/// Used for all property columns.
#[derive(Component)]
pub enum WorldInfoPropertyDisplay {
	Description,
	Value,
}

/// A property displayed in the world info UI.
///
/// Since this is a large sum type which is unwieldy to access, the data stored in here is not the primary source of
/// data for the game logic. Instead, various systems update these properties with the real data which is stored
/// somewhere else.
#[derive(Clone, Debug)]
pub enum WorldInfoProperty {
	/// Current area of some object.
	Area(usize),
	/// Minimum area of some object.
	MinArea(usize),
	/// Comfort level of an [`crate::model::Accommodation`].
	Comfort(Comfort),
	/// [`AccommodationType`] of an accommodation.
	AccommodationType(AccommodationType),
	/// Various properties called "multiplicity".
	Multiplicity(u64),
}

impl WorldInfoProperty {
	/// Short name of the property.
	fn property_name(&self) -> String {
		match self {
			Self::Area(_) => "Area",
			Self::MinArea(_) => "Minimum area",
			Self::Comfort(_) => "Comfort",
			Self::AccommodationType(_) => "Type",
			Self::Multiplicity(_) => "Multiplicity",
		}
		.to_string()
	}

	/// Formatted value of the property.
	fn property_value(&self) -> String {
		match self {
			Self::MinArea(area) | Self::Area(area) => format!("{}i²", area),
			Self::Comfort(comfort) => format!("{}", comfort),
			Self::AccommodationType(kind) => kind.to_string(),
			Self::Multiplicity(multiplicity) => format!("{}", multiplicity),
		}
	}
}

#[derive(Component, Clone)]
pub struct WorldInfoProperties {
	properties:      Vec<WorldInfoProperty>,
	pub name:        String,
	pub description: String,
}

impl std::ops::Deref for WorldInfoProperties {
	type Target = Vec<WorldInfoProperty>;

	fn deref(&self) -> &Self::Target {
		&self.properties
	}
}

impl DerefMut for WorldInfoProperties {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.properties
	}
}

impl WorldInfoProperties {
	pub fn basic(name: String, description: String) -> Self {
		Self { properties: Vec::default(), name, description }
	}
}

pub fn setup_world_info(mut commands: Commands) {
	commands
		.spawn((
			NodeBundle {
				style: Style {
					position_type: PositionType::Absolute,
					display: Display::Grid,
					grid_template_rows: vec![
						RepeatedGridTrack::min_content(1),
						RepeatedGridTrack::min_content(1),
						RepeatedGridTrack::repeat_many(GridTrackRepetition::AutoFit, vec![GridTrack::auto()]),
					],
					grid_auto_columns: vec![],
					grid_auto_rows: vec![],
					grid_template_columns: vec![RepeatedGridTrack::auto(1), RepeatedGridTrack::min_content(1)],
					grid_auto_flow: GridAutoFlow::Row,
					padding: UiRect::all(Val::Px(5.)),
					row_gap: Val::Px(5.),
					width: Val::Percent(20.),
					min_height: Val::Percent(10.),
					..Default::default()
				},
				background_color: BackgroundColor(Color::DARK_GRAY),
				focus_policy: FocusPolicy::Pass,
				z_index: ZIndex::Global(1),
				visibility: Visibility::Hidden,
				..Default::default()
			},
			WorldInfoUI::default(),
		))
		.with_children(|parent| {
			parent.spawn((WorldInfoTitle, TextBundle {
				text: Text { linebreak_behavior: BreakLineOn::WordBoundary, ..Default::default() },
				style: Style { grid_column: GridPlacement::start_span(0, 2), ..Default::default() },
				..Default::default()
			}));
			parent.spawn((WorldInfoBody, TextBundle {
				text: Text { linebreak_behavior: BreakLineOn::WordBoundary, ..Default::default() },
				style: Style { grid_column: GridPlacement::start_span(0, 2), ..Default::default() },
				..Default::default()
			}));
		});
}

pub fn move_world_info(
	windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	mut world_info: Query<(&mut Style, &mut Visibility, &WorldInfoUI)>,
	interactable_world_info_entities: Query<&GlobalTransform>,
) {
	let (camera, camera_transform) = camera_q.single();
	let window = windows.get_single();
	if window.is_err() {
		return;
	}
	let window = window.unwrap();
	let cursor_position = window.cursor_position();
	if cursor_position.is_none() {
		return;
	}
	let (mut world_info_style, mut world_info_visibility, world_info_ui) = world_info.single_mut();

	if let Some(Ok(attached_transform)) =
		world_info_ui.attached_entity.map(|attached_entity| interactable_world_info_entities.get(attached_entity))
	{
		world_info_visibility.set_if_neq(Visibility::Visible);
		let bevy_world_position = attached_transform.translation() + Vec3::from((0., TILE_HEIGHT / 2., 0.));
		if let Some(screen_position) = camera.world_to_viewport(camera_transform, bevy_world_position) {
			world_info_style.bottom = Val::Px(-screen_position.y + window.height());
			world_info_style.left = Val::Px(screen_position.x);
		}
	} else {
		world_info_visibility.set_if_neq(Visibility::Hidden);
	}
}

pub fn hide_world_info(mut world_info: Query<&mut WorldInfoUI>, input: Res<Input<KeyCode>>) {
	let mut world_info_ui = world_info.single_mut();
	if input.just_pressed(KeyCode::Escape) {
		world_info_ui.attached_entity = None;
	}
}

pub fn reassign_world_info(
	mouse_buttons: Res<Input<MouseButton>>,
	windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	blocking_ui_elements: Query<&Interaction, (With<Node>, Changed<Interaction>)>,
	mut interactable_world_info_entities: Query<(Entity, &GlobalTransform, &mut WorldInfoProperties)>,
	mut world_info: Query<&mut WorldInfoUI>,
) {
	if mouse_buttons.just_released(MouseButton::Left)
		&& !blocking_ui_elements.iter().any(|interaction| interaction != &Interaction::None)
	{
		let start = Instant::now();

		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		let cursor_position = window.cursor_position();
		if cursor_position.is_none() {
			return;
		}
		let cursor_position = cursor_position.unwrap();
		let mut world_info_data = world_info.single_mut();

		let cursor_position = camera.viewport_to_world_2d(camera_transform, cursor_position);
		if cursor_position.is_none() {
			return;
		}
		let cursor_position = Vec3A::from((cursor_position.unwrap(), 0.)) - Vec3A::from((0., TILE_HEIGHT / 2., 0.));

		let node_under_cursor: Arc<Mutex<Option<_>>> = Arc::default();
		// PERFORMANCE: Run distance checks in parallel, only locking the current-best node once we have something
		// that's within the click tolerance anyways.
		interactable_world_info_entities.par_iter_mut().for_each_mut(|(entity, node_position, mut properties)| {
			let mut node_position = node_position.translation_vec3a();
			node_position.z = 0.;
			let distance_to_cursor = node_position.distance(cursor_position).abs();

			if distance_to_cursor < 2. * TILE_WIDTH {
				let mut node_under_cursor = node_under_cursor.lock();
				if let Some((old_entity, distance)) = node_under_cursor.as_mut() {
					if *distance > distance_to_cursor {
						// Set this world info to changed so that update_world_info definitely runs the next time.
						properties.set_changed();
						*old_entity = entity;
						*distance = distance_to_cursor;
					}
				} else {
					*node_under_cursor = Some((entity, distance_to_cursor));
				}
			}
		});

		if let Some((entity, _)) = &*node_under_cursor.lock() {
			world_info_data.attached_entity = Some(*entity);
		}

		let duration = Instant::now() - start;
		debug!("Regenerating world info took {:?}", duration);
	}
}

pub fn update_world_info(
	interactable_world_info_entities: Query<&WorldInfoProperties>,
	mut world_info: Query<(Entity, &mut WorldInfoUI)>,
	mut world_info_header: Query<&mut Text, (With<WorldInfoTitle>, Without<WorldInfoBody>)>,
	mut world_info_body: Query<&mut Text, (With<WorldInfoBody>, Without<WorldInfoTitle>)>,
	asset_server: Res<AssetServer>,
	property_displays: Query<
		Entity,
		(With<Text>, With<WorldInfoPropertyDisplay>, Without<WorldInfoBody>, Without<WorldInfoTitle>),
	>,
	mut commands: Commands,
) {
	let (world_info_style, mut world_info_ui) = world_info.single_mut();

	let mut world_info_header = world_info_header.single_mut();
	let mut world_info_body = world_info_body.single_mut();
	if let Some(Ok(node_under_cursor)) =
		world_info_ui.attached_entity.map(|attached_entity| interactable_world_info_entities.get(attached_entity))
	{
		for entity in property_displays.into_iter() {
			commands.entity(entity).despawn_recursive();
		}

		world_info_header.sections = vec![TextSection::new(&node_under_cursor.name, TextStyle {
			font:      asset_server.load(font_for(FontWeight::Bold, FontStyle::Regular)),
			font_size: 24.,
			color:     Color::WHITE,
		})];
		world_info_body.sections = vec![TextSection::new(&node_under_cursor.description, TextStyle {
			font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
			font_size: 16.,
			color:     Color::WHITE,
		})];

		let mut info_ui = commands.entity(world_info_style);
		info_ui.with_children(|parent| {
			for property in node_under_cursor.iter() {
				let property_name = property.property_name();
				let property_value = property.property_value();
				parent.spawn((
					TextBundle {
						style: Style { grid_column: GridPlacement::start(0), ..Default::default() },
						text: Text::from_section(property_name, TextStyle {
							font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
							font_size: 18.,
							color:     Color::WHITE,
						}),
						..Default::default()
					},
					WorldInfoPropertyDisplay::Description,
				));
				parent.spawn((
					TextBundle {
						style: Style {
							grid_column: GridPlacement::start(1),
							align_self: AlignSelf::End,
							..Default::default()
						},
						text: Text::from_section(property_value, TextStyle {
							font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
							font_size: 18.,
							color:     Color::ANTIQUE_WHITE,
						}),
						..Default::default()
					},
					WorldInfoPropertyDisplay::Value,
				));
			}
		});
	} else {
		world_info_ui.attached_entity = None;
		world_info_header.sections.clear();
		world_info_body.sections.clear();
	}
}
