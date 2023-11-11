use bevy::prelude::*;
use cmp::{CmpPlugin, GraphicsPlugin};

fn main() {
	App::new().add_plugins((CmpPlugin, GraphicsPlugin)).add_systems(PostStartup, print_program_info).run();
}

const VERSION: &str = env!(
	"CARGO_PKG_VERSION",
	"The CMP client must be built under Cargo, or set the CARGO_PKG_VERSION variable manually."
);

fn print_program_info() {
	info!("CMP client version {}", VERSION);
}
