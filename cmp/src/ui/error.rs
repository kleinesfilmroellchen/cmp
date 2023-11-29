//! Error display in the UI.
use bevy::prelude::*;

/// A kind of error event that can be displayed in the UI.
pub trait DisplayableError: std::error::Error + Event {
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

impl<T: DisplayableError> From<T> for ErrorBox {
	fn from(value: T) -> Self {
		Self(Box::new(value))
	}
}

pub(super) fn show_errors(mut errors: EventReader<ErrorBox>) {
	for _ in errors.read() {}
}

pub(super) fn print_errors(mut errors: EventReader<ErrorBox>) {
	for error in errors.read() {
		error!("Error: {}", error);
	}
}
