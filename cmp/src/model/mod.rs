//! Internal world state data models and game mechanics.

pub mod accommodation;
pub mod geometry;
pub mod pool;
pub mod tile;

pub use accommodation::*;
pub use geometry::*;
pub use tile::*;

use crate::ui::build::BuildMode;
use crate::ui::controls::BuildMenu;
use crate::util::Tooltipable;

/// All build-able objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buildable {
	/// A [`GroundTile`] of some [`GroundKind`].
	Ground(GroundKind),
	/// Demarcates the area of a pool; filled with [`GroundKind::PoolPath`].
	PoolArea,
	/// An [`Accommodation`] of some [`AccommodationType`]. This is a placeholder type until the proper accommodation
	/// system is in place.
	BasicAccommodation(AccommodationType),
}

impl std::fmt::Display for Buildable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::BasicAccommodation(kind) => kind.to_string(),
			Self::Ground(kind) => kind.to_string(),
			Self::PoolArea => "Pool Area".to_string(),
		})
	}
}

impl Tooltipable for Buildable {
	fn description(&self) -> &'static str {
		match self {
			Self::BasicAccommodation(kind) => kind.description(),
			Self::Ground(kind) => kind.description(),
			Self::PoolArea => "Demarcate a pool area to start building a pool.",
		}
	}
}

pub const ALL_BUILDABLES: [Buildable; 4] = [
	Buildable::Ground(GroundKind::Pathway),
	Buildable::Ground(GroundKind::Grass),
	Buildable::PoolArea,
	Buildable::BasicAccommodation(AccommodationType::Cottage),
];

impl Buildable {
	pub fn menu(&self) -> BuildMenu {
		match self {
			Self::Ground(_) => BuildMenu::Basics,
			Self::PoolArea => BuildMenu::Pool,
			Self::BasicAccommodation(_) => BuildMenu::Accommodation,
		}
	}

	pub fn size(&self) -> BoundingBox {
		match self {
			Self::Ground(_) => (1, 1).into(),
			Self::PoolArea => (1, 1).into(),
			Self::BasicAccommodation(kind) => kind.size(),
		}
	}

	pub fn build_mode(&self) -> BuildMode {
		match self {
			Buildable::Ground(_) => BuildMode::Line,
			Buildable::PoolArea => BuildMode::Rect,
			Buildable::BasicAccommodation(_) => BuildMode::Single,
		}
	}
}

/// A general-purpose metric with a specific range. Metrics are always natural numbers to simplify UI and corresponding
/// game mechanics. Specific subsystems will define their own metric-derived types with specific ranges.
#[derive(Clone, Copy, Debug, Default, Eq, Ord)]
pub struct Metric<const MIN: u64, const MAX: u64>(u64);

impl<const MIN: u64, const MAX: u64, T: Into<u64>> From<T> for Metric<MIN, MAX> {
	fn from(value: T) -> Self {
		Metric(value.into())
	}
}

impl<const MIN: u64, const MAX: u64, const OTHER_MIN: u64, const OTHER_MAX: u64>
	PartialOrd<Metric<OTHER_MIN, OTHER_MAX>> for Metric<MIN, MAX>
{
	fn partial_cmp(&self, other: &Metric<OTHER_MIN, OTHER_MAX>) -> Option<std::cmp::Ordering> {
		self.0.partial_cmp(&other.0)
	}
}

impl<const MIN: u64, const MAX: u64, const OTHER_MIN: u64, const OTHER_MAX: u64> PartialEq<Metric<OTHER_MIN, OTHER_MAX>>
	for Metric<MIN, MAX>
{
	fn eq(&self, other: &Metric<OTHER_MIN, OTHER_MAX>) -> bool {
		self.0.eq(&other.0)
	}
}
