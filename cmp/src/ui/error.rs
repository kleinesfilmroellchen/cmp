//! Error display in the UI.
use bevy::color::palettes::css::{ORANGE, WHITE};
use bevy::prelude::*;

use super::controls::{DialogBox, DialogContainer, DialogContents, DialogTitle};
use crate::graphics::library::{FontStyle, FontWeight, font_for};

/// A kind of error event that can be displayed in the UI.
pub trait DisplayableError: std::error::Error {
	// The error's name; may not be static but depend on internal state.
	fn name(&self) -> &str;
}

/// The type-erased container for all errors. We accept the performance penalty of heap allocation since errors are rare
/// one-off events.
#[derive(Debug, Event)]
pub struct ErrorBox(Box<dyn DisplayableError + Send + Sync>);

impl std::fmt::Display for ErrorBox {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl<T: DisplayableError + Send + Sync + 'static> From<T> for ErrorBox {
	fn from(value: T) -> Self {
		Self(Box::new(value))
	}
}

pub(super) fn show_errors(
	mut errors: EventReader<ErrorBox>,
	mut dialog_container: Query<&mut Visibility, With<DialogContainer>>,
	dialog_box: Query<Entity, With<DialogBox>>,
	mut dialog_title: Query<(&mut Text, &mut TextColor), With<DialogTitle>>,
	mut dialog_contents: Query<Entity, With<DialogContents>>,
	asset_server: Res<AssetServer>,
	mut commands: Commands,
) -> Result {
	let mut dialog_container = dialog_container.single_mut()?;
	// Don't show another error while the dialog box is still open.
	if dialog_container.as_ref() == Visibility::Visible {
		return Ok(());
	}

	if let Some(ErrorBox(error)) = errors.read().next() {
		let title = error.name();
		let text = error.to_string();

		let (mut dialog_title, mut dialog_title_color) = dialog_title.single_mut()?;
		let dialog_box = dialog_box.single()?;

		dialog_contents.iter_mut().for_each(|entity| commands.entity(entity).despawn());

		*dialog_title = Text(title.into());
		*dialog_title_color = TextColor(ORANGE.into());

		commands.entity(dialog_box).with_children(|dialog_content_commands| {
			dialog_content_commands.spawn((
				Text(text),
				TextFont {
					font: asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
					font_size: 24.,
					..Default::default()
				},
				TextColor(WHITE.into()),
				DialogContents,
			));
		});

		dialog_container.set_if_neq(Visibility::Visible);
	}
	Ok(())
}

pub(super) fn print_errors(mut errors: EventReader<ErrorBox>) {
	for error in errors.read() {
		error!("Error: {}", error);
	}
}
