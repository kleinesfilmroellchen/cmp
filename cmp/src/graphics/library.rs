//! Look-up tables and functions defining graphics assets for various in-engine data types.

use bevy::sprite::Anchor;

use super::BorderKind;
use crate::model::{Buildable, GroundKind, PitchType};
use crate::ui::controls::BuildMenu;

pub fn image_for_ground(kind: GroundKind) -> &'static str {
	match kind {
		GroundKind::Grass => "grass.qoi",
		GroundKind::Pathway => "gravel.qoi",
		GroundKind::PoolPath => "pool.qoi",
		GroundKind::Pitch => "pitch-tile.qoi",
	}
}

pub fn logo_for_build_menu(menu: BuildMenu) -> &'static str {
	match menu {
		BuildMenu::Basics => "concrete.qoi",
		BuildMenu::Pitch => "pitch-logo.qoi",
		BuildMenu::Pool => "pool.qoi",
	}
}

pub fn logo_for_buildable(buildable: Buildable) -> &'static str {
	match buildable {
		Buildable::Ground(kind) => image_for_ground(kind),
		Buildable::PitchType(kind) => image_for_pitch(kind),
		Buildable::Pitch => "pitch-area-logo.qoi",
		Buildable::PoolArea => "pool.qoi",
	}
}

pub fn preview_image_for_buildable(buildable: Buildable) -> &'static str {
	match buildable {
		Buildable::Ground(kind) => image_for_ground(kind),
		Buildable::PitchType(kind) => image_for_pitch(kind),
		Buildable::Pitch => "pitch-tile.qoi",
		Buildable::PoolArea => "pool.qoi",
	}
}

pub fn image_for_pitch(kind: PitchType) -> &'static str {
	match kind {
		PitchType::TentPitch => "tent-post.qoi",
		PitchType::PermanentTent => "permanent-tent.qoi",
		PitchType::CaravanPitch => "caravan-post.qoi",
		PitchType::MobileHome => "mobile-home.qoi",
		PitchType::Cottage => "cottage.qoi",
	}
}

pub fn image_for_border_kind(kind: BorderKind) -> &'static str {
	match kind {
		BorderKind::Pitch => "pitch-border.qoi",
	}
}

/// The anchors must always be on the bottom left (in world space!) of the bottom left world-space (isometric) tile. For
/// simple 1x1 tiles, this is the bottom center of the sprite, but for other tiles, a more complex computation is in
/// order. This needs to be updated to keep in sync with graphics.
pub fn anchor_for_image(image: &str) -> Anchor {
	match image {
		"grass.qoi" | "gravel.qoi" | "pool.qoi" => Anchor::BottomCenter,
		"cottage.qoi" => Anchor::Custom(((23. - 20.) / 40., -0.5).into()),
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
