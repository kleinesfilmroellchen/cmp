//! Look-up tables and functions defining graphics assets for various in-engine data types.

use crate::model::{Buildable, GroundKind};
use crate::ui::controls::BuildMenu;

pub fn sprite_for_kind(kind: GroundKind) -> &'static str {
	match kind {
		GroundKind::Grass => "grass.png",
		GroundKind::Pathway => "gravel.png",
		GroundKind::PoolPath => "pool.png",
	}
}

pub fn logo_for_build_menu(menu: BuildMenu) -> &'static str {
	match menu {
		BuildMenu::Basics => "concrete.png",
		BuildMenu::Accommodation => "caravan.png",
		BuildMenu::Pool => "pool.png",
	}
}

pub fn logo_for_buildable(buildable: Buildable) -> &'static str {
	match buildable {
		Buildable::Pathway => "gravel.png",
		Buildable::PoolArea => "pool.png",
		Buildable::Cottage => "caravan.png",
	}
}
