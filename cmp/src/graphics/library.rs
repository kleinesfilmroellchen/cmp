//! Look-up tables and functions defining graphics assets for various in-engine data types.

use crate::tile::GroundKind;

pub fn sprite_for_kind(kind: GroundKind) -> &'static str {
	match kind {
		GroundKind::Grass => "grass.png",
		GroundKind::Pathway => "gravel.png",
		GroundKind::PoolPath => "pool.png",
	}
}
