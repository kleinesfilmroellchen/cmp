//! Look-up tables and functions defining graphics assets for various in-engine data types.

use bevy::sprite::Anchor;

use crate::model::{AccommodationType, Buildable, GroundKind};
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

pub fn sprite_for_buildable(buildable: Buildable) -> &'static str {
	match buildable {
		Buildable::Pathway => "gravel.png",
		Buildable::PoolArea => "pool.png",
		Buildable::Cottage => "caravan.png",
	}
}

pub fn sprite_for_accommodation(kind: AccommodationType) -> &'static str {
	match kind {
		AccommodationType::TentSite => todo!(),
		AccommodationType::LargeTentSite => todo!(),
		AccommodationType::PermanentTent => todo!(),
		AccommodationType::CaravanSite => todo!(),
		AccommodationType::MobileHome => todo!(),
		AccommodationType::Cottage => "caravan.png",
	}
}

/// The anchors must always be on the bottom left of the bottom left world-space (isometric) tile. For simple 1x1 tiles,
/// this is the bottom left of the sprite, but for other tiles, a more complex computation is in order. This needs to be
/// updated to keep in sync with graphics.
pub fn anchor_for_sprite(sprite: &'static str) -> Anchor {
	match sprite {
		"grass.png" | "gravel.png" | "pool.png" => Anchor::BottomLeft,
		"caravan.png" => Anchor::Custom((-4. / 40., -0.5).into()),
		_ => Anchor::BottomLeft,
	}
}
