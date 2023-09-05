//! CMP's plugin system.
//!
//! A plugin must contain a function with this exact signature:
//! ```ignore
//! fn _bevy_create_plugin() -> *mut dyn bevy::app::Plugin;
//! ```
//! The plugin function is passed the exact app used for the game. Several important caveats apply:
//! - As of bevy 0.11, `#[derive(DynamicPlugin)]` which should generate a wrapper with the above signature that
//!   constructs and returns your plugin, is broken due to an ABI oversight in the macro. Implement it manually for the
//!   time being.
//! - For ABI compatibility, make sure that the plugin is compiled with the EXACT SAME bevy version as CMP itself.
//!   Otherwise, strange errors (most likely segmentation faults) may happen. The --version option of CMP binaries will
//!   report the bevy version in use.
//! - bevy MUST be a dynamic dependency of the plugin, since CMP itself links bevy dynamically; this way, the engine
//!   code is not included twice in the resulting binary and the exact same code is used everywhere.
//! - CMP's own [`Plugin`] as well as some set of default bevy [`Plugin`]s are already loaded by the time this function
//!   is called on any CMP plugin. You may experiment and figure out which extra default bevy plugins are loadable, but
//!   any of them may cause a runtime panic due to duplicate plugins. The set of plugins loaded across CMP versions may
//!   change in any way. The safest option is to only load [`Plugin`]s of your own design or from some other third-party
//!   library.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use libloading::Library;

use crate::config::CommandLineArguments;

#[derive(Deref, DerefMut)]
struct LoadedPluginLibraries {
	libraries: Vec<Library>,
}

impl LoadedPluginLibraries {
	const fn new() -> Self {
		Self { libraries: Vec::new() }
	}
}

/// A structure keeping all loaded libraries around and alive. [`libloading::Library`] (sensibly) is an RAII type,
/// meaning that a dropped library would be unloaded. By using a global variable here, we make sure that libraries are
/// never unloaded by Bevy's engine while they might still interact with it (which would crash the program), but only
/// unloaded by the Rust runtime or the operating system on program exit.
static PLUGIN_LIBRARIES: Mutex<LoadedPluginLibraries> = Mutex::new(LoadedPluginLibraries::new());

/// Bevy does not implement [`Plugin`] for Box<dyn Plugin> for some reason, so we have to do it ourselves with a simple
/// wrapper bridge type.
#[derive(Deref, DerefMut)]
struct DynamicPluginBridge(Box<dyn Plugin>);

impl Plugin for DynamicPluginBridge {
	fn build(&self, app: &mut App) {
		self.0.build(app);
	}

	fn ready(&self, app: &App) -> bool {
		self.0.ready(app)
	}

	fn finish(&self, app: &mut App) {
		self.0.finish(app);
	}

	fn cleanup(&self, app: &mut App) {
		self.0.cleanup(app);
	}

	fn name(&self) -> &str {
		self.0.name()
	}

	fn is_unique(&self) -> bool {
		self.0.is_unique()
	}
}

/// A plugin responsible for adding external plugins.
#[derive(Deref)]
pub(crate) struct ExternalPlugins(pub(crate) Arc<CommandLineArguments>);

impl Plugin for ExternalPlugins {
	fn build(&self, app: &mut App) {
		let mut plugin_libraries = PLUGIN_LIBRARIES.lock().unwrap();

		#[cfg(any(target_family = "windows", target_family = "unix"))]
		{
			let mut successful = 0;
			let mut failed = 0;
			for plugin_path in &self.plugins {
				let result: Result<(), bevy_dynamic_plugin::DynamicPluginLoadError> = try {
					let (library, plugin) = unsafe { bevy_dynamic_plugin::dynamically_load_plugin(plugin_path) }?;
					app.add_plugins(DynamicPluginBridge(plugin));
					plugin_libraries.push(library);
					info!("Successfully loaded plugin {}", plugin_path.to_string_lossy());
					successful += 1;
				};
				if let Err(why) = result {
					error!("Could not load plugin {}: {}", plugin_path.to_string_lossy(), why);
					failed += 1;
				}
			}
			info!("Loaded {} plugins total ({} successful, {} failed)", successful + failed, successful, failed);
		}
		#[cfg(not(any(target_family = "windows", target_family = "unix")))]
		{
			if !self.plugins.is_empty() {
				info!(
					"Cannot load the requested plugins {} on this platform family ({}), since plugin loading is not \
					 supported here.",
					self.plugins.iter().map(|p| p.to_string_lossy()).intersperse(", ".into()).collect::<String>(),
					std::env::consts::FAMILY,
				);
			}
		}
	}
}
