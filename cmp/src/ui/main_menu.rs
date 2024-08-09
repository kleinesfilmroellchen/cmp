use bevy::prelude::*;

use super::{BUTTON_SPACING, COLUMN_TEMPLATE};
use crate::graphics::library::{font_for, FontStyle, FontWeight};

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_main_menu);
	}
}

pub fn setup_main_menu(mut commands: Commands, assets: Res<AssetServer>) {
	commands
		.spawn(NodeBundle {
			style: Style {
				width: Val::Vw(100.),
				height: Val::Vh(100.),
				display: Display::Grid,
				position_type: PositionType::Absolute,
				grid_template_columns: COLUMN_TEMPLATE.clone(),
				grid_template_rows: vec![
					RepeatedGridTrack::percent(1, 10.),
					RepeatedGridTrack::minmax(10, MinTrackSizingFunction::Px(1.), MaxTrackSizingFunction::MinContent),
					RepeatedGridTrack::percent(1, 10.),
				],
				..Default::default()
			},
			..Default::default()
		})
		.with_children(|parent| {
			parent.spawn(TextBundle {
				style: Style {
					margin: UiRect::all(BUTTON_SPACING),
					grid_row: GridPlacement::start(2),
					grid_column: GridPlacement::start(2),
					..Default::default()
				},
				text: Text::from_sections([
					TextSection::new("CMP", TextStyle {
						font: assets.load(font_for(FontWeight::Bold, FontStyle::Regular)),
						font_size: 120.,
						..Default::default()
					}),
					TextSection::new("\nThe Camping Madness Project", TextStyle {
						font: assets.load(font_for(FontWeight::Bold, FontStyle::Regular)),
						font_size: 40.,
						..Default::default()
					}),
				])
				.with_justify(JustifyText::Center),
				..Default::default()
			});
		});
}
