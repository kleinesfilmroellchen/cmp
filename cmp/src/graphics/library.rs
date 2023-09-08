//! Look-up tables and functions defining graphics assets for various in-engine data types.

use bevy::sprite::Anchor;

use super::BorderKind;
use crate::model::{AccommodationType, Buildable, GroundKind};
use crate::ui::controls::BuildMenu;

pub fn sprite_for_ground(kind: GroundKind) -> &'static str {
	match kind {
		GroundKind::Grass => "grass.png",
		GroundKind::Pathway => "gravel.png",
		GroundKind::PoolPath => "pool.png",
		GroundKind::Accommodation => "accommodation-site-tile.png",
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
		Buildable::Ground(kind) => sprite_for_ground(kind),
		Buildable::Accommodation(kind) => sprite_for_accommodation(kind),
		Buildable::AccommodationSite => "accommodation-logo.png",
		Buildable::PoolArea => "pool.png",
	}
}

pub fn preview_sprite_for_buildable(buildable: Buildable) -> &'static str {
	match buildable {
		Buildable::Ground(kind) => sprite_for_ground(kind),
		Buildable::Accommodation(kind) => sprite_for_accommodation(kind),
		Buildable::AccommodationSite => "accommodation-site-tile.png",
		Buildable::PoolArea => "pool.png",
	}
}

pub fn sprite_for_accommodation(kind: AccommodationType) -> &'static str {
	match kind {
		AccommodationType::TentSite => todo!(),
		AccommodationType::PermanentTent => todo!(),
		AccommodationType::CaravanSite => todo!(),
		AccommodationType::MobileHome => todo!(),
		AccommodationType::Cottage => "caravan.png",
	}
}

pub fn sprite_for_border_kind(kind: BorderKind) -> &'static str {
	match kind {
		BorderKind::Accommodation => "accommodation-site.png",
	}
}

/// The anchors must always be on the bottom left (in world space!) of the bottom left world-space (isometric) tile. For
/// simple 1x1 tiles, this is the bottom center of the sprite, but for other tiles, a more complex computation is in
/// order. This needs to be updated to keep in sync with graphics.
pub fn anchor_for_sprite(sprite: &'static str) -> Anchor {
	match sprite {
		"grass.png" | "gravel.png" | "pool.png" => Anchor::BottomCenter,
		"caravan.png" => Anchor::Custom(((23. - 20.) / 40., -0.5).into()),
		_ => Anchor::BottomCenter,
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
	Regular,
	Bold,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
	Regular,
	Italic,
}

pub fn font_for(weight: FontWeight, style: FontStyle) -> String {
	format!(
		"CrimsonPro-{}{}.ttf",
		if weight == FontWeight::Bold { "Bold" } else { "" },
		if style == FontStyle::Italic { "Italic" } else { "" }
	)
}
