use std::sync::LazyLock;
use std::time::Duration;

use bevy::color::palettes::css::{DARK_GRAY, GRAY, ORANGE};
use bevy::prelude::*;
use bevy::text::LineBreak;
use bevy::ui::FocusPolicy;
use build::BuildPlugin;
use main_menu::MainMenuPlugin;

use self::animate::{AnimationPlugin, AnimationTargets, UIAnimation};
use self::controls::{ALL_BUILD_MENUS, BuildMenuContainer};
use crate::gamemode::GameState;
use crate::graphics::HIGH_RES_LAYERS;
use crate::graphics::library::{FontStyle, FontWeight, font_for, logo_for_build_menu, logo_for_buildable};
use crate::input::{InputState, move_camera};
use crate::model::ALL_BUILDABLES;
use crate::ui::animate::{StyleHeight, TransitionTimes};
use crate::util::{Tooltip, TooltipPlugin};

pub(crate) mod animate;
pub(crate) mod build;
pub mod error;
pub(crate) mod main_menu;
pub(crate) mod world_info;

pub struct UIPlugin;

impl Plugin for UIPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((BuildPlugin, TooltipPlugin, AnimationPlugin, MainMenuPlugin))
			.add_event::<controls::OpenBuildMenu>()
			.add_event::<controls::CloseBuildMenus>()
			.add_event::<error::ErrorBox>()
			.add_systems(
				OnEnter(GameState::InGame),
				(initialize_ingame_ui, initialize_dialogs, world_info::setup_world_info),
			)
			.add_systems(
				Update,
				(world_info::reassign_world_info, world_info::update_world_info)
					.run_if(in_state(InputState::Idle))
					.run_if(in_state(GameState::InGame)),
			)
			.add_systems(
				Update,
				(world_info::move_world_info, world_info::hide_world_info)
					.before(world_info::update_world_info)
					.after(move_camera)
					.run_if(in_state(GameState::InGame)),
			)
			.add_systems(
				Update,
				(
					update_build_menu_state,
					on_build_menu_button_press,
					on_start_build_preview.after(on_build_menu_button_press),
					close_dialog,
				)
					.run_if(in_state(GameState::InGame)),
			)
			.add_systems(PostUpdate, (error::show_errors, error::print_errors).run_if(in_state(GameState::InGame)));
	}
}

/// Components used for marking and identifying various UI controls.
pub mod controls {
	use bevy::prelude::*;

	use crate::model::Buildable;
	use crate::util::Tooltipable;

	/// The possible build menus.
	#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
	pub enum BuildMenu {
		/// Basic objects, like fences and pathways.
		Basics,
		/// Visitor pitch.
		Pitch,
		/// All pool-related objects.
		Pool,
	}

	impl Tooltipable for BuildMenu {
		fn description(&self) -> &'static str {
			match self {
				Self::Basics => "Fundamental buildings and objects.",
				Self::Pitch => "Pitches housing visitors, such as tent pitches, caravans or mobile homes.",
				Self::Pool => "Everything for swimming pools.",
			}
		}
	}

	impl std::fmt::Display for BuildMenu {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			write!(f, "{}", match self {
				Self::Pitch => "Pitches",
				Self::Basics => "The Basics",
				Self::Pool => "Swimming Pools",
			})
		}
	}

	pub(super) const ALL_BUILD_MENUS: [BuildMenu; 3] = [BuildMenu::Basics, BuildMenu::Pitch, BuildMenu::Pool];

	/// Marks a button that opens one of the several build menus.
	#[derive(Component, Reflect)]
	#[reflect(Component)]
	pub struct BuildMenuButton(pub BuildMenu);

	#[derive(Component, Reflect)]
	#[reflect(Component)]
	pub struct BuildMenuContainer(pub BuildMenu);

	/// Marks a button that starts the build process for a specific [`Buildable`].
	#[derive(Component, Reflect)]
	#[reflect(Component)]
	pub struct StartBuildButton(pub Buildable);

	/// An event notifying that a build menu has been opened.
	#[derive(Event)]
	pub struct OpenBuildMenu(pub BuildMenu);

	/// An event notifying that the open build menu has been closed.
	#[derive(Event)]
	pub struct CloseBuildMenus;

	#[derive(Component, Reflect, Clone, Copy, Debug)]
	#[reflect(Component)]
	pub struct DialogContainer;
	#[derive(Component, Reflect, Clone, Copy, Debug)]
	#[reflect(Component)]
	pub struct DialogBox;
	#[derive(Component, Reflect, Clone, Copy, Debug)]
	#[reflect(Component)]
	pub struct DialogTitle;
	#[derive(Component, Reflect, Clone, Copy, Debug)]
	#[reflect(Component)]
	pub struct DialogContents;
	#[derive(Component, Reflect, Clone, Copy, Debug)]
	#[reflect(Component)]
	pub struct DialogCloseButton;
}

const BUTTON_SPACING: Val = Val::Px(5.);

static COLUMN_TEMPLATE: LazyLock<Vec<RepeatedGridTrack>> = LazyLock::new(|| {
	vec![
		// Left spacing column
		RepeatedGridTrack::percent(1, 8.),
		// Center column for main UI
		RepeatedGridTrack::auto(1),
		// Right spacing column
		RepeatedGridTrack::percent(1, 8.),
	]
});

fn initialize_ingame_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn((
			Node {
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				display: Display::Grid,
				// Absolute positioning for top-level containers allows us to make all UI layers independent.
				position_type: PositionType::Absolute,
				grid_template_columns: COLUMN_TEMPLATE.clone(),
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
			HIGH_RES_LAYERS,
		))
		.with_children(|parent| {
			parent
				.spawn(Node {
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
				})
				.with_children(|parent| {
					let background_color = BackgroundColor(DARK_GRAY.into());
					const PIXEL_SIZE: f32 = 50.;
					const TRANSITION_TIMES: TransitionTimes = TransitionTimes {
						to_start:   Duration::from_millis(250),
						to_hovered: Duration::from_millis(200),
						to_pressed: Duration::from_millis(80),
					};
					let height_animation = UIAnimation::<_, _, StyleHeight>::new(
						Val::Px(PIXEL_SIZE),
						Val::Px(PIXEL_SIZE + 20.),
						AnimationTargets::at_hover(),
						16.,
						20.,
						TRANSITION_TIMES,
					);
					let press_animation = UIAnimation::<_, _, BackgroundColor>::new(
						background_color,
						BackgroundColor({
							let Hsla { hue, saturation, mut lightness, alpha } = background_color.0.into();
							lightness = (lightness - 0.3).clamp(0., 1.);
							Color::hsla(hue, saturation, lightness, alpha)
						}),
						AnimationTargets::at_press(),
						4.,
						4.,
						TransitionTimes::uniform(Duration::from_millis(100)),
					);
					parent
						.spawn((
							Node {
								grid_row: GridPlacement::start(2),
								display: Display::Flex,
								flex_direction: FlexDirection::Row,
								align_items: AlignItems::Baseline,
								column_gap: BUTTON_SPACING,
								..Default::default()
							},
							FocusPolicy::Block,
							Interaction::default(),
						))
						.with_children(|parent| {
							// TODO: Use iter_variants to dynamically access all variants.
							for menu_type in controls::ALL_BUILD_MENUS {
								let node = Node {
									justify_content: JustifyContent::Center,
									align_items: AlignItems::Center,
									width: Val::Px(PIXEL_SIZE),
									height: Val::Px(PIXEL_SIZE),
									..Default::default()
								};
								parent
									.spawn((
										Button,
										height_animation.clone(),
										press_animation.clone(),
										node,
										background_color,
										controls::BuildMenuButton(menu_type),
										Tooltip::from(&menu_type),
									))
									.with_children(|button| {
										button.spawn((
											ImageNode {
												image: asset_server.load(logo_for_build_menu(menu_type)),
												..Default::default()
											},
											Node { width: Val::Percent(90.), ..Default::default() },
										));
									});
							}
						});
					// All build menus.
					for menu_type in ALL_BUILD_MENUS {
						parent
							.spawn((
								Node {
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
								BackgroundColor(GRAY.into()),
								FocusPolicy::Block,
								BuildMenuContainer(menu_type),
								Interaction::default(),
							))
							.with_children(|build_menu| {
								// May be a little slow to iterate all buildable types each time, but we only do it once
								// on startup anyways.
								for buildable in ALL_BUILDABLES.iter().filter(|buildable| buildable.menu() == menu_type)
								{
									let background_color = BackgroundColor(DARK_GRAY.into());
									let node = Node {
										justify_content: JustifyContent::Center,
										align_items: AlignItems::Center,
										width: Val::Px(50.),
										height: Val::Px(50.),
										..Default::default()
									};
									build_menu
										.spawn((
											Button,
											height_animation.clone(),
											press_animation.clone(),
											node,
											background_color,
											Tooltip::from(buildable),
											controls::StartBuildButton(*buildable),
										))
										.with_children(|button| {
											button.spawn((
												ImageNode {
													image: asset_server.load(logo_for_buildable(*buildable)),
													..Default::default()
												},
												Node { width: Val::Percent(90.), ..Default::default() },
											));
										});
								}
							});
					}
				});
		});
}

fn initialize_dialogs(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn((
			Node {
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				display: Display::Grid,
				// Absolute positioning for top-level containers allows us to make all UI layers independent.
				position_type: PositionType::Absolute,
				grid_template_columns: vec![
					RepeatedGridTrack::fr(1, 1.),
					RepeatedGridTrack::percent(1, 50.),
					RepeatedGridTrack::fr(1, 1.),
				],
				grid_template_rows: vec![
					RepeatedGridTrack::fr(1, 1.),
					RepeatedGridTrack::minmax(
						1,
						MinTrackSizingFunction::Percent(50.),
						MaxTrackSizingFunction::MinContent,
					),
					RepeatedGridTrack::fr(1, 1.),
				],
				..Default::default()
			},
			HIGH_RES_LAYERS,
			Visibility::Hidden,
			BackgroundColor(Color::Srgba(DARK_GRAY).with_alpha(0.5)),
			controls::DialogContainer,
		))
		.with_children(|parent| {
			parent
				.spawn((
					Node {
						grid_row: GridPlacement::start(2),
						grid_column: GridPlacement::start(2),
						display: Display::Grid,
						align_items: AlignItems::Start,
						justify_content: JustifyContent::Center,
						align_content: AlignContent::Center,
						grid_template_columns: vec![RepeatedGridTrack::auto(1), RepeatedGridTrack::min_content(1)],
						grid_template_rows: vec![RepeatedGridTrack::min_content(1), RepeatedGridTrack::auto(1)],
						padding: UiRect::all(BUTTON_SPACING),
						row_gap: BUTTON_SPACING,
						..Default::default()
					},
					FocusPolicy::Block,
					BackgroundColor(DARK_GRAY.into()),
					Interaction::default(),
					controls::DialogBox,
				))
				.with_children(|parent| {
					parent.spawn((
						Node {
							grid_row: GridPlacement::start(1),
							grid_column: GridPlacement::span(1),
							justify_self: JustifySelf::Center,
							align_self: AlignSelf::Center,
							..Default::default()
						},
						Text(String::new()),
						TextLayout { justify: JustifyText::Center, linebreak: LineBreak::WordBoundary },
						TextColor(ORANGE.into()),
						TextFont {
							font: asset_server.load(font_for(FontWeight::Bold, FontStyle::Regular)),
							font_size: 32.,
							..Default::default()
						},
						controls::DialogTitle,
					));
					parent.spawn((
						Node {
							grid_row: GridPlacement::start(1),
							grid_column: GridPlacement::start(2),
							min_width: Val::Px(30.),
							min_height: Val::Px(30.),
							..Default::default()
						},
						Button,
						BackgroundColor(Color::BLACK),
						controls::DialogCloseButton,
					));
				});
		});
}

fn close_dialog(
	mut dialog_container: Query<&mut Visibility, With<controls::DialogContainer>>,
	interacted_button: Query<&Interaction, (Changed<Interaction>, With<controls::DialogCloseButton>)>,
) -> Result {
	let mut dialog_container_visibility = dialog_container.single_mut()?;
	if matches!(interacted_button.single(), Ok(&Interaction::Pressed)) {
		dialog_container_visibility.set_if_neq(Visibility::Hidden);
	}
	Ok(())
}

fn on_build_menu_button_press(
	mut interacted_button: Query<(&Interaction, &controls::BuildMenuButton), (Changed<Interaction>, With<Button>)>,
	mut open_menu_event: EventWriter<controls::OpenBuildMenu>,
) {
	for (interaction, button_kind) in &mut interacted_button {
		if interaction == &Interaction::Pressed {
			open_menu_event.write(controls::OpenBuildMenu(button_kind.0));
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
			start_preview_event.write(build::StartBuildPreview { buildable: button_kind.0 });
			state.set(InputState::Building);
		}
	}
}

fn update_build_menu_state(
	mut build_menus: Query<(&controls::BuildMenuContainer, &mut Node)>,
	mut open_menu_event: EventReader<controls::OpenBuildMenu>,
	mut close_menu_event: EventReader<controls::CloseBuildMenus>,
) {
	for open_event in open_menu_event.read() {
		let kind = open_event.0;
		for (container_type, mut node) in &mut build_menus {
			// The second check will also close any currently-open menu on second click.
			node.display =
				if container_type.0 == kind && node.display == Display::None { Display::Flex } else { Display::None };
		}
	}
	for _ in close_menu_event.read() {
		for (_, mut node) in &mut build_menus {
			node.display = Display::None;
		}
	}
}
