#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cmp::{CmpPlugin, GraphicsPlugin, PostStartup, App, info};

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
