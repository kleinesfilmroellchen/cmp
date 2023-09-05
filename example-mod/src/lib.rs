use std::time::Duration;

use bevy::prelude::*;

/// As of bevy 0.11, `#[derive(DynamicPlugin)]` is broken due to it specifying the wrong ABI (C instead of Rust, which
/// the loader expects). For the time being, DO NOT USE `#[derive(DynamicPlugin)]` (it will segfault CMP every time),
/// use the below implementation instead.
struct ExamplePlugin;

/// A fixed version of `#[derive(DynamicPlugin]`, see comment above.
#[no_mangle]
pub fn _bevy_create_plugin() -> *mut dyn bevy::app::Plugin {
	// make sure the constructor is the correct type.
	let object = ExamplePlugin {};
	let boxed = Box::new(object);
	Box::into_raw(boxed)
}

// Everything beyond this is normal Bevy code.

impl Plugin for ExamplePlugin {
	fn build(&self, app: &mut App) {
		info!("Hello from the example plugin!");
		app.add_systems(Update, report_slow_frames);
	}
}

/// An example system demonstrating (and testing) that the plugin stays loaded and can use Bevy functionality as normal.
fn report_slow_frames(time: Res<Time>) {
	if time.delta() > Duration::from_millis(50) {
		info!("Uh oh, this frame was slow, it took {:?} to render :(", time.delta());
	}
}
