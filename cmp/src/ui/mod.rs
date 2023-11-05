use std::time::Duration;

use bevy::prelude::*;
use build::BuildPlugin;

use self::controls::{BuildMenuContainer, ALL_BUILD_MENUS};
use crate::graphics::library::{logo_for_build_menu, logo_for_buildable};
use crate::input::InputState;
use crate::model::ALL_BUILDABLES;
use crate::util::physics_ease::MassDamperSystem;
use crate::util::{Lerpable, Tooltip, TooltipPlugin};

pub(crate) mod build;
pub(crate) mod world_info;

pub struct UIPlugin;

impl Plugin for UIPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((BuildPlugin, TooltipPlugin))
			.add_event::<controls::OpenBuildMenu>()
			.add_event::<controls::CloseBuildMenus>()
			.add_systems(Startup, (initialize_ui, world_info::setup_world_info))
			.add_systems(Update, (transition_button_interaction, animate_button, update_build_menu_state))
			.add_systems(
				Update,
				(world_info::reassign_world_info, world_info::update_world_info).run_if(in_state(InputState::Idle)),
			)
			.add_systems(
				Update,
				(world_info::move_world_info, world_info::hide_world_info).before(world_info::update_world_info),
			)
			.add_systems(
				Update,
				(on_build_menu_button_press, on_start_build_preview.after(on_build_menu_button_press)),
			);
	}
}

/// Components used for marking and identifying various UI controls.
pub mod controls {
	use bevy::prelude::*;

	use crate::model::Buildable;
	use crate::util::Tooltipable;

	/// The possible build menus.
	#[derive(Clone, Copy, PartialEq, Eq, Debug)]
	pub enum BuildMenu {
		/// Basic objects, like fences and pathways.
		Basics,
		/// Visitor accommodation.
		Accommodation,
		/// All pool-related objects.
		Pool,
	}

	impl Tooltipable for BuildMenu {
		fn description(&self) -> &'static str {
			match self {
				Self::Basics => "Fundamental buildings and objects.",
				Self::Accommodation => "Visitor accommodations, such as tent spots, caravans or mobile homes.",
				Self::Pool => "Everything for swimming pools.",
			}
		}
	}

	impl std::fmt::Display for BuildMenu {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}", match self {
				Self::Accommodation => "Accomodation",
				Self::Basics => "The Basics",
				Self::Pool => "Swimming Pools",
			})
		}
	}

	pub(super) const ALL_BUILD_MENUS: [BuildMenu; 3] = [BuildMenu::Basics, BuildMenu::Accommodation, BuildMenu::Pool];

	/// Marks a button that opens one of the several build menus.
	#[derive(Component)]
	pub struct BuildMenuButton(pub BuildMenu);

	#[derive(Component)]
	pub struct BuildMenuContainer(pub BuildMenu);

	/// Marks a button that starts the build process for a specific [`Buildable`].
	#[derive(Component)]
	pub struct StartBuildButton(pub Buildable);

	/// An event notifying that a build menu has been opened.
	#[derive(Event)]
	pub struct OpenBuildMenu(pub BuildMenu);

	/// An event notifying that the open build menu has been closed.
	#[derive(Event)]
	pub struct CloseBuildMenus;
}

/// This does not use Bevy's animation system, which is highly limited and mostly useful for 3D mesh transforms.
#[derive(Component, Clone)]
struct ButtonAnimations {
	/// Currently playing animation.
	target:          Interaction,
	/// Stores the original background color so that it can be restored.
	original_color:  Color,
	/// Stores the original height of the button so that it can be restored.
	original_height: f32,
	// Physics-based easing systems:
	height_system:   MassDamperSystem,
	color_system:    MassDamperSystem,
}

impl ButtonAnimations {
	pub fn new(current_color: BackgroundColor, current_style: &Style) -> Self {
		ButtonAnimations {
			target:          Interaction::None,
			height_system:   MassDamperSystem::new(16., 20., 1.),
			color_system:    MassDamperSystem::new(4., 4., 1.),
			original_color:  current_color.0,
			original_height: current_style.height.evaluate(0.).unwrap_or(0.),
		}
	}

	pub const fn transition_time_to(target: Interaction) -> Duration {
		match target {
			Interaction::Pressed => Duration::from_millis(80),
			Interaction::Hovered => Duration::from_millis(200),
			Interaction::None => Duration::from_millis(250),
		}
	}

	pub fn logical_height_position_of(target: Interaction) -> f32 {
		match target {
			Interaction::Pressed => 1.,
			Interaction::Hovered => 1.,
			Interaction::None => 0.,
		}
	}

	pub fn logical_color_position_of(target: Interaction) -> f32 {
		match target {
			Interaction::Pressed => 1.,
			Interaction::Hovered => 0.,
			Interaction::None => 0.,
		}
	}

	/// Starts an animation that transitions to the specific interaction target.
	pub fn start_transition_to(&mut self, target: Interaction) {
		self.target = target;
		self.height_system.set_target(Self::logical_height_position_of(target));
		self.color_system.set_target(Self::logical_color_position_of(target));
	}

	/// Runs the regular update of the animation, performing the button animation itself if necessary.
	pub fn update(&mut self, time: &Time, color: &mut BackgroundColor, style: &mut Style) {
		// Animation was finished and finalized before already, no need to do anything.
		// This order of operations guarantees that we actually reach the target values without needing to set them
		// every time.

		let normalized_delta = time.delta().as_secs_f32() / Self::transition_time_to(self.target).as_secs_f32();
		self.height_system.simulate(normalized_delta);
		self.color_system.simulate(normalized_delta);

		let target_color = {
			let [hue, saturation, mut lightness, alpha] = self.original_color.as_hsla_f32();
			lightness = (lightness - 0.3).clamp(0., 1.);
			Color::hsla(hue, saturation, lightness, alpha)
		};
		let target_height = self.original_height + 20.;

		let current_color = self.original_color.lerp(&target_color, self.color_system.position());
		let current_height = self.original_height.lerp(&target_height, self.height_system.position()).round_ties_even();
		*color = BackgroundColor(current_color);
		style.height = Val::Px(current_height);
	}
}

const BUTTON_SPACING: Val = Val::Px(5.);

fn initialize_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn(NodeBundle {
			style: Style {
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				display: Display::Grid,
				// Absolute positioning for top-level containers allows us to make all UI layers independent.
				position_type: PositionType::Absolute,
				grid_template_columns: vec![
					// Left spacing column
					RepeatedGridTrack::percent(1, 8.),
					// Center column for main UI
					RepeatedGridTrack::auto(1),
					// Right spacing column
					RepeatedGridTrack::percent(1, 8.),
				],
				grid_template_rows: vec![
					// Top controls (main statistics, global game menus)
					RepeatedGridTrack::percent(1, 5.),
					// Center spacing for viewing the world.
					RepeatedGridTrack::auto(1),
					// Bottom controls (build menu); expandable from a minimum size since the menus open upwards
					RepeatedGridTrack::minmax(
						1,
						MinTrackSizingFunction::Percent(5.),
						MaxTrackSizingFunction::MaxContent,
					),
				],
				..Default::default()
			},
			..Default::default()
		})
		.with_children(|parent| {
			parent
				.spawn(NodeBundle {
					style: Style {
						grid_row: GridPlacement::start(3),
						grid_column: GridPlacement::start(2),
						display: Display::Grid,
						align_items: AlignItems::End,
						align_content: AlignContent::End,
						grid_template_columns: vec![RepeatedGridTrack::auto(1)],
						grid_template_rows: vec![RepeatedGridTrack::auto(1), RepeatedGridTrack::min_content(1)],
						padding: UiRect::all(BUTTON_SPACING),
						row_gap: BUTTON_SPACING,
						..Default::default()
					},
					..Default::default()
				})
				.with_children(|parent| {
					parent
						.spawn(NodeBundle {
							style: Style {
								grid_row: GridPlacement::start(2),
								display: Display::Flex,
								flex_direction: FlexDirection::Row,
								align_items: AlignItems::Baseline,
								column_gap: BUTTON_SPACING,
								..Default::default()
							},
							..Default::default()
						})
						.with_children(|parent| {
							// TODO: Use iter_variants to dynamically access all variants.
							for menu_type in controls::ALL_BUILD_MENUS {
								let background_color = BackgroundColor(Color::DARK_GRAY);
								let style = Style {
									justify_content: JustifyContent::Center,
									align_items: AlignItems::Center,
									width: Val::Px(50.),
									height: Val::Px(50.),
									..Default::default()
								};
								parent
									.spawn((
										ButtonAnimations::new(background_color, &style),
										ButtonBundle { style, background_color, ..Default::default() },
										controls::BuildMenuButton(menu_type),
										Tooltip::from(&menu_type),
									))
									.with_children(|button| {
										button.spawn(ImageBundle {
											image: UiImage {
												texture: asset_server.load(logo_for_build_menu(menu_type)),
												..Default::default()
											},
											style: Style { width: Val::Percent(90.), ..Default::default() },
											..Default::default()
										});
									});
							}
						});
					// All build menus.
					for menu_type in ALL_BUILD_MENUS {
						parent
							.spawn((
								NodeBundle {
									style: Style {
										grid_row: GridPlacement::start(1),
										display: Display::None,
										flex_direction: FlexDirection::Row,
										align_items: AlignItems::Baseline,
										align_self: AlignSelf::Baseline,
										column_gap: BUTTON_SPACING,
										padding: UiRect::all(BUTTON_SPACING),
										min_height: Val::Px(50.),
										..Default::default()
									},
									background_color: BackgroundColor(Color::GRAY),
									..Default::default()
								},
								BuildMenuContainer(menu_type),
								// Make the menu background receive and block mouse clicks, e.g. to prevent accidental
								// building.
								Interaction::None,
							))
							.with_children(|build_menu| {
								// May be a little slow to iterate all buildable types each time, but we only do it once
								// on startup anyways.
								for buildable in ALL_BUILDABLES.iter().filter(|buildable| buildable.menu() == menu_type)
								{
									let background_color = BackgroundColor(Color::DARK_GRAY);
									let style = Style {
										justify_content: JustifyContent::Center,
										align_items: AlignItems::Center,
										width: Val::Px(50.),
										height: Val::Px(50.),
										..Default::default()
									};
									build_menu
										.spawn((
											ButtonAnimations::new(background_color, &style),
											ButtonBundle { style, background_color, ..Default::default() },
											Tooltip::from(buildable),
											controls::StartBuildButton(*buildable),
										))
										.with_children(|button| {
											button.spawn(ImageBundle {
												image: UiImage {
													texture: asset_server.load(logo_for_buildable(*buildable)),
													..Default::default()
												},
												style: Style { width: Val::Percent(90.), ..Default::default() },
												..Default::default()
											});
										});
								}
							});
					}
				});
		});
}

fn transition_button_interaction(
	mut button: Query<(&Interaction, &mut ButtonAnimations), (Changed<Interaction>, With<Button>)>,
) {
	for (interaction, mut animations) in &mut button {
		animations.start_transition_to(*interaction);
	}
}

fn animate_button(
	time: Res<Time>,
	mut buttons: Query<(&mut ButtonAnimations, &mut BackgroundColor, &mut Style), With<Button>>,
) {
	for (mut animations, mut color, mut style) in &mut buttons {
		animations.update(&time, &mut color, &mut style);
	}
}

fn on_build_menu_button_press(
	mut interacted_button: Query<(&Interaction, &controls::BuildMenuButton), (Changed<Interaction>, With<Button>)>,
	mut open_menu_event: EventWriter<controls::OpenBuildMenu>,
) {
	for (interaction, button_kind) in &mut interacted_button {
		if interaction == &Interaction::Pressed {
			open_menu_event.send(controls::OpenBuildMenu(button_kind.0));
		}
	}
}

fn on_start_build_preview(
	mut interacted_button: Query<(&Interaction, &controls::StartBuildButton), (Changed<Interaction>, With<Button>)>,
	mut start_preview_event: EventWriter<build::StartBuildPreview>,
	current_state: ResMut<State<InputState>>,
	mut state: ResMut<NextState<InputState>>,
) {
	for (interaction, button_kind) in &mut interacted_button {
		// Only start building if we're doing nothing or already building.
		if interaction == &Interaction::Pressed && [InputState::Building, InputState::Idle].contains(&current_state) {
			start_preview_event.send(build::StartBuildPreview { buildable: button_kind.0 });
			state.set(InputState::Building);
		}
	}
}

fn update_build_menu_state(
	mut build_menus: Query<(&controls::BuildMenuContainer, &mut Style)>,
	mut open_menu_event: EventReader<controls::OpenBuildMenu>,
	mut close_menu_event: EventReader<controls::CloseBuildMenus>,
) {
	for open_event in &mut open_menu_event {
		let kind = open_event.0;
		for (container_type, mut style) in &mut build_menus {
			// The second check will also close any currently-open menu on second click.
			style.display =
				if container_type.0 == kind && style.display == Display::None { Display::Flex } else { Display::None };
		}
	}
	for _ in &mut close_menu_event {
		for (_, mut style) in &mut build_menus {
			style.display = Display::None;
		}
	}
}
