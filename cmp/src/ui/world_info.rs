use std::ops::DerefMut;
use std::sync::Arc;

use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::ui::FocusPolicy;
use parking_lot::Mutex;

use crate::graphics::library::{font_for, FontStyle, FontWeight};
use crate::graphics::TILE_WIDTH;

#[derive(Component)]
pub struct WorldInfoUI;
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

/// A component that can provide a property to the world info UI.
pub trait WorldInfoProperty: Send {
	/// Short name of the property.
	fn property_name(&self) -> String;
	/// Formatted value of the property.
	fn property_value(&self) -> String;

	fn clone(&self) -> Box<dyn WorldInfoProperty>;
}

// FIXME: Shouldn't need a mutex here, but bevy has very strong send/sync requirements and the trait objects break all
// of them (why do they propagate through Vec, Box, Arc, RwLock, Pin, ...???)
#[derive(Component)]
pub struct WorldInfoProperties {
	properties:  Mutex<Vec<Box<dyn WorldInfoProperty>>>,
	name:        String,
	description: String,
}

impl Clone for WorldInfoProperties {
	fn clone(&self) -> Self {
		let mut new_vec = Vec::with_capacity(self.properties.lock().len());
		for element in self.properties.lock().iter() {
			new_vec.push(WorldInfoProperty::clone(element.as_ref()));
		}
		Self { properties: Mutex::new(new_vec), name: self.name.clone(), description: self.description.clone() }
	}
}

impl std::ops::Deref for WorldInfoProperties {
	type Target = Mutex<Vec<Box<dyn WorldInfoProperty>>>;

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
		Self { properties: Mutex::default(), name, description }
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
					grid_template_columns: vec![RepeatedGridTrack::auto(1), RepeatedGridTrack::min_content(1)],
					grid_auto_flow: GridAutoFlow::Row,
					padding: UiRect::all(Val::Px(5.)),
					row_gap: Val::Px(5.),
					width: Val::Percent(10.),
					min_height: Val::Percent(10.),
					..Default::default()
				},
				background_color: BackgroundColor(Color::DARK_GRAY),
				focus_policy: FocusPolicy::Block,
				z_index: ZIndex::Global(1),
				..Default::default()
			},
			WorldInfoUI,
		))
		.with_children(|parent| {
			parent.spawn((WorldInfoTitle, TextBundle {
				text: Text { linebreak_behavior: BreakLineOn::WordBoundary, ..Default::default() },
				style: Style { grid_column: GridPlacement::span(2), ..Default::default() },
				..Default::default()
			}));
			parent.spawn((WorldInfoBody, TextBundle {
				text: Text { linebreak_behavior: BreakLineOn::WordBoundary, ..Default::default() },
				style: Style { grid_column: GridPlacement::span(2), ..Default::default() },
				..Default::default()
			}));
		});
}

pub fn update_world_info(
	mouse_buttons: Res<Input<MouseButton>>,
	windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
	camera_q: Query<(&Camera, &GlobalTransform)>,
	// We don't need mutable access to the properties, but it signals to the borrow checker that we have exclusive
	// (read) access to it.
	mut interactable_world_info_entities: Query<(&GlobalTransform, &mut WorldInfoProperties)>,
	mut world_info: Query<(Entity, &mut Style), With<WorldInfoUI>>,
	mut world_info_header: Query<&mut Text, (With<WorldInfoTitle>, Without<WorldInfoBody>)>,
	mut world_info_body: Query<&mut Text, (With<WorldInfoBody>, Without<WorldInfoTitle>)>,
	asset_server: Res<AssetServer>,
	property_displays: Query<
		Entity,
		(With<Text>, With<WorldInfoPropertyDisplay>, Without<WorldInfoBody>, Without<WorldInfoTitle>),
	>,
	mut commands: Commands,
) {
	if mouse_buttons.just_pressed(MouseButton::Left) {
		for entity in property_displays.into_iter() {
			commands.entity(entity).despawn_recursive();
		}

		let (camera, camera_transform) = camera_q.single();
		let window = windows.single();
		let cursor_position = window.cursor_position();
		if cursor_position.is_none() {
			return;
		}
		let cursor_position = cursor_position.unwrap();
		let (world_info_entity, mut world_info_style) = world_info.single_mut();
		world_info_style.bottom = Val::Px(-cursor_position.y + window.height() + 10.);
		world_info_style.left = Val::Px(cursor_position.x + 10.);

		let cursor_position = camera.viewport_to_world_2d(camera_transform, cursor_position);
		if cursor_position.is_none() {
			return;
		}
		let cursor_position = Vec3A::from((cursor_position.unwrap(), 0.));

		let node_under_cursor: Arc<Mutex<Option<(f32, WorldInfoProperties)>>> = Arc::default();
		// PERFORMANCE: Run distance checks in parallel, only locking the current-best node once we have something
		// that's within the click tolerance anyways.
		interactable_world_info_entities.par_iter_mut().for_each_mut(|(node_position, properties)| {
			let distance_to_cursor = node_position.translation_vec3a().distance(cursor_position).abs();
			if distance_to_cursor < 2. * TILE_WIDTH {
				let mut node_under_cursor = node_under_cursor.lock();
				if let Some((distance, node_under_cursor)) = node_under_cursor.as_mut() {
					debug!("got something! {:?}", node_position);
					if *distance > distance_to_cursor {
						*node_under_cursor = properties.clone();
						*distance = distance_to_cursor;
					}
				} else {
					*node_under_cursor = Some((distance_to_cursor, properties.clone()));
				}
			}
		});

		let mut world_info_header = world_info_header.single_mut();
		let mut world_info_body = world_info_body.single_mut();
		if let Some((_, node_under_cursor)) = &*node_under_cursor.lock() {
			world_info_header.sections = vec![TextSection::new(&node_under_cursor.name, TextStyle {
				font:      asset_server.load(font_for(FontWeight::Bold, FontStyle::Regular)),
				font_size: 20.,
				color:     Color::WHITE,
			})];
			world_info_body.sections = vec![TextSection::new(&node_under_cursor.description, TextStyle {
				font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
				font_size: 14.,
				color:     Color::WHITE,
			})];

			let mut info_ui = commands.entity(world_info_entity);
			info_ui.with_children(|parent| {
				for property in node_under_cursor.lock().iter() {
					let property_name = property.property_name();
					let property_value = property.property_value();
					parent.spawn((
						TextBundle {
							style: Style { grid_column: GridPlacement::start(1), ..Default::default() },
							text: Text::from_section(property_name, TextStyle {
								font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
								font_size: 10.,
								color:     Color::WHITE,
							}),
							..Default::default()
						},
						WorldInfoPropertyDisplay::Description,
					));
					parent.spawn((
						TextBundle {
							style: Style {
								grid_column: GridPlacement::start(2),
								align_self: AlignSelf::End,
								..Default::default()
							},
							text: Text::from_section(property_value, TextStyle {
								font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
								font_size: 10.,
								color:     Color::ANTIQUE_WHITE,
							}),
							..Default::default()
						},
						WorldInfoPropertyDisplay::Value,
					));
				}
			});
		} else {
			world_info_header.sections.clear();
			world_info_body.sections.clear();
		};
	}
}
