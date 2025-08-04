//! Generic utilities not specific to CMP.

use bevy::color::palettes::css::DARK_GRAY;
use bevy::prelude::*;
use bevy::text::LineBreak;

use crate::graphics::library::{FontStyle, FontWeight, font_for};

pub mod physics_ease;

/// Any property which can be linerarly interpolated with itself. Linear interpolation is a useful tool for many things
/// in games, like animations and transitions.
pub trait Lerpable {
	/// t determines the interpolation point and *should* be between 0 and 1. t values outside will usually extrapolate
	/// properly.
	fn lerp(&self, other: &Self, t: f32) -> Self;
}

impl Lerpable for f32 {
	fn lerp(&self, other: &Self, t: f32) -> Self {
		self + t * (other - self)
	}
}

impl Lerpable for f64 {
	fn lerp(&self, other: &Self, t: f32) -> Self {
		self + t as f64 * (other - self)
	}
}

impl Lerpable for Color {
	fn lerp(&self, other: &Self, t: f32) -> Self {
		// It is VERY IMPORTANT that we interpolate colors in linear color space, otherwise the lightness will be off!
		let LinearRgba { red: this_red, green: this_green, blue: this_blue, alpha: this_alpha } = self.to_linear();
		let LinearRgba { red: other_red, green: other_green, blue: other_blue, alpha: other_alpha } = other.to_linear();
		Self::linear_rgba(
			this_red.lerp(other_red, t),
			this_green.lerp(other_green, t),
			this_blue.lerp(other_blue, t),
			this_alpha.lerp(other_alpha, t),
		)
	}
}

impl Lerpable for Val {
	fn lerp(&self, other: &Self, t: f32) -> Self {
		match (self, other) {
			// Interpolating from/to Auto doesn't really make sense, but we can stay at Auto.
			(Val::Auto, _) | (_, Val::Auto) => Val::Auto,
			(Val::Px(this), Val::Px(other)) => Val::Px(this.lerp(other, t)),
			(Val::Percent(this), Val::Percent(other)) => Val::Percent(this.lerp(other, t)),
			(Val::Vw(this), Val::Vw(other)) => Val::Vw(this.lerp(other, t)),
			(Val::Vh(this), Val::Vh(other)) => Val::Vh(this.lerp(other, t)),
			(Val::VMin(this), Val::VMin(other)) => Val::VMin(this.lerp(other, t)),
			(Val::VMax(this), Val::VMax(other)) => Val::VMax(this.lerp(other, t)),
			_ => panic!("Can't lerp between {:?} and {:?}", self, other),
		}
	}
}

impl Lerpable for BackgroundColor {
	fn lerp(&self, other: &Self, t: f32) -> Self {
		Self(self.0.lerp(&other.0, t))
	}
}

/// Shows information about a UI element on hover.
#[derive(Component, Reflect)]
pub struct Tooltip {
	/// The headline of the tooltip.
	pub title: String,
	/// The longer body text of the tooltip.
	pub body:  String,
}

/// Any object that one can easily create a tooltip from.
pub trait Tooltipable: std::fmt::Display {
	/// Returns the description used for the body.
	fn description(&self) -> &'static str;
}

impl<T: Tooltipable> From<&T> for Tooltip {
	fn from(value: &T) -> Self {
		Self { title: value.to_string(), body: value.description().to_string() }
	}
}

/// Plugin displaying tooltips on anything that has a Tooltipable component and is part of the UI.
pub struct TooltipPlugin;

impl Plugin for TooltipPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, setup_tooltip)
			.add_systems(Update, (move_tooltip_to_mouse, show_tooltip, update_tooltip));
	}
}

#[derive(Component, Reflect)]
struct TooltipHeaderText;
#[derive(Component, Reflect)]
struct TooltipBodyText;

#[derive(Component, Reflect, Default)]
struct TooltipUI;

fn tooltip_style(asset_server: &AssetServer, is_body: bool) -> impl Bundle {
	(
		TextFont {
			font: asset_server
				.load(font_for(if is_body { FontWeight::Regular } else { FontWeight::Bold }, FontStyle::Regular)),
			font_size: if is_body { 20. } else { 30. },
			..Default::default()
		},
		TextColor(Color::WHITE),
	)
}

fn setup_tooltip(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands
		.spawn((
			Node {
				min_width: Val::Percent(2.),
				min_height: Val::Percent(2.),
				max_width: Val::Percent(30.),
				display: Display::Grid,
				position_type: PositionType::Absolute,
				grid_template_columns: vec![RepeatedGridTrack::auto(1)],
				padding: UiRect::all(Val::Px(5.)),
				grid_template_rows: vec![
					// Heading
					RepeatedGridTrack::min_content(1),
					// Body
					RepeatedGridTrack::auto(1),
				],
				row_gap: Val::Px(5.),
				..Default::default()
			},
			BackgroundColor(DARK_GRAY.into()),
			TooltipUI,
		))
		.with_children(|container| {
			container.spawn((
				Text::default(),
				TextLayout { linebreak: LineBreak::WordBoundary, ..Default::default() },
				TooltipHeaderText,
				tooltip_style(&asset_server, false),
			));
			container.spawn((
				Text::default(),
				TextLayout { linebreak: LineBreak::WordBoundary, ..Default::default() },
				TooltipBodyText,
				tooltip_style(&asset_server, true),
			));
		});
}

fn move_tooltip_to_mouse(
	windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
	mut tooltip: Query<&mut Node, With<TooltipUI>>,
) -> Result {
	let window = windows.single()?;
	let mut tooltip_style = tooltip.single_mut()?;
	if let Some(cursor_position) = window.cursor_position() {
		// Some hacks to translate screen coordinates to UI behavior...
		tooltip_style.bottom = Val::Px(-cursor_position.y + window.height() + 10.);
		tooltip_style.left = Val::Px(cursor_position.x + 10.);
	}
	Ok(())
}

fn update_tooltip(
	mut tooltip_header_text: Query<(&mut Text, &TooltipHeaderText), Without<TooltipBodyText>>,
	mut tooltip_body_text: Query<(&mut Text, &TooltipBodyText), Without<TooltipHeaderText>>,
	interacted_tooltipable_node: Query<(&Interaction, &Tooltip), (Changed<Interaction>, With<Node>)>,
) -> Result {
	let (mut tooltip_header_text, _) = tooltip_header_text.single_mut()?;
	let (mut tooltip_body_text, _) = tooltip_body_text.single_mut()?;
	for (interaction, tooltip) in &interacted_tooltipable_node {
		if interaction == &Interaction::None {
			continue;
		}
		**tooltip_header_text = tooltip.title.clone();
		**tooltip_body_text = tooltip.body.clone();
	}
	Ok(())
}

fn show_tooltip(
	mut tooltip: Query<&mut Node, With<TooltipUI>>,
	any_tooltipable_node: Query<(&Interaction, &Tooltip), With<Node>>,
) -> Result {
	let mut hovers_any = false;
	for (interaction, _) in &any_tooltipable_node {
		hovers_any |= interaction != &Interaction::None;
	}

	tooltip.single_mut()?.display = if hovers_any { Display::Grid } else { Display::None };
	Ok(())
}
