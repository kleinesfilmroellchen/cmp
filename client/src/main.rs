use bevy::prelude::*;
use cmp::{CmpPlugin, GraphicsPlugin};

fn main() {
	App::new().add_plugins((CmpPlugin, GraphicsPlugin)).run();
}
