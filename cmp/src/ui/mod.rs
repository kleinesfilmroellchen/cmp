use std::sync::LazyLock;
use std::time::Duration;

use bevy::color::palettes::css::{DARK_GRAY, GRAY, ORANGE};
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::ui::FocusPolicy;
use build::BuildPlugin;
use main_menu::MainMenuPlugin;

use self::animate::{AnimationPlugin, AnimationTargets, UIAnimation};
use self::controls::{BuildMenuContainer, ALL_BUILD_MENUS};
use crate::gamemode::GameState;
use crate::graphics::library::{font_for, logo_for_build_menu, logo_for_buildable, FontStyle, FontWeight};
use crate::input::InputState;
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
		.spawn(NodeBundle {
			style: Style {
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
							NodeBundle {
								style: Style {
									grid_row: GridPlacement::start(2),
									display: Display::Flex,
									flex_direction: FlexDirection::Row,
									align_items: AlignItems::Baseline,
									column_gap: BUTTON_SPACING,
									..Default::default()
								},
								focus_policy: FocusPolicy::Block,
								..Default::default()
							},
							Interaction::default(),
						))
						.with_children(|parent| {
							// TODO: Use iter_variants to dynamically access all variants.
							for menu_type in controls::ALL_BUILD_MENUS {
								let style = Style {
									justify_content: JustifyContent::Center,
									align_items: AlignItems::Center,
									width: Val::Px(PIXEL_SIZE),
									height: Val::Px(PIXEL_SIZE),
									..Default::default()
								};
								parent
									.spawn((
										height_animation.clone(),
										press_animation.clone(),
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
									background_color: BackgroundColor(GRAY.into()),
									focus_policy: FocusPolicy::Block,
									..Default::default()
								},
								BuildMenuContainer(menu_type),
								Interaction::default(),
							))
							.with_children(|build_menu| {
								// May be a little slow to iterate all buildable types each time, but we only do it once
								// on startup anyways.
								for buildable in ALL_BUILDABLES.iter().filter(|buildable| buildable.menu() == menu_type)
								{
									let background_color = BackgroundColor(DARK_GRAY.into());
									let style = Style {
										justify_content: JustifyContent::Center,
										align_items: AlignItems::Center,
										width: Val::Px(50.),
										height: Val::Px(50.),
										..Default::default()
									};
									build_menu
										.spawn((
											height_animation.clone(),
											press_animation.clone(),
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

fn initialize_dialogs(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn((
			NodeBundle {
				style: Style {
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
				visibility: Visibility::Hidden,
				background_color: BackgroundColor(Color::Srgba(DARK_GRAY).with_alpha(0.5)),
				..Default::default()
			},
			controls::DialogContainer,
		))
		.with_children(|parent| {
			parent
				.spawn((
					NodeBundle {
						style: Style {
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
						focus_policy: FocusPolicy::Block,
						background_color: BackgroundColor(DARK_GRAY.into()),
						..Default::default()
					},
					Interaction::default(),
					controls::DialogBox,
				))
				.with_children(|parent| {
					parent.spawn((
						TextBundle {
							style: Style {
								grid_row: GridPlacement::start(1),
								grid_column: GridPlacement::span(1),
								justify_self: JustifySelf::Center,
								align_self: AlignSelf::Center,
								..Default::default()
							},
							text: Text {
								justify:            JustifyText::Center,
								linebreak_behavior: BreakLineOn::WordBoundary,
								sections:           vec![TextSection::new("", TextStyle {
									font:      asset_server.load(font_for(FontWeight::Bold, FontStyle::Regular)),
									font_size: 32.,
									color:     ORANGE.into(),
								})],
							},
							..Default::default()
						},
						controls::DialogTitle,
					));
					parent.spawn((
						ButtonBundle {
							style: Style {
								grid_row: GridPlacement::start(1),
								grid_column: GridPlacement::start(2),
								min_width: Val::Px(30.),
								min_height: Val::Px(30.),
								..Default::default()
							},
							background_color: BackgroundColor(Color::BLACK),
							..Default::default()
						},
						controls::DialogCloseButton,
					));
				});
		});
}

fn close_dialog(
	mut dialog_container: Query<&mut Visibility, With<controls::DialogContainer>>,
	interacted_button: Query<&Interaction, (Changed<Interaction>, With<controls::DialogCloseButton>)>,
) {
	let mut dialog_container_visibility = dialog_container.single_mut();
	if matches!(interacted_button.get_single(), Ok(&Interaction::Pressed)) {
		dialog_container_visibility.set_if_neq(Visibility::Hidden);
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
	for open_event in open_menu_event.read() {
		let kind = open_event.0;
		for (container_type, mut style) in &mut build_menus {
			// The second check will also close any currently-open menu on second click.
			style.display =
				if container_type.0 == kind && style.display == Display::None { Display::Flex } else { Display::None };
		}
	}
	for _ in close_menu_event.read() {
		for (_, mut style) in &mut build_menus {
			style.display = Display::None;
		}
	}
}
