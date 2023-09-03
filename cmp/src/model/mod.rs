//! Internal world state data models and game mechanics.

pub mod accommodation;
pub mod geometry;
pub mod tile;

pub use accommodation::*;
pub use geometry::*;
pub use tile::*;

use crate::ui::controls::BuildMenu;
use crate::util::Tooltipable;

/// All build-able objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buildable {
	/// A [`GroundTile`] of type [`GroundKind::Pathway`].
	Pathway,
	/// Demarcates the area of a pool; filled with [`GroundKind::PoolPath`].
	PoolArea,
	/// An [`Accommodation`] of type [`AccommodationType::Cottage`].
	Cottage,
}

impl std::fmt::Display for Buildable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::Cottage => AccommodationType::Cottage.to_string(),
			Self::Pathway => GroundKind::Pathway.to_string(),
			Self::PoolArea => "Pool Area".to_string(),
		})
	}
}

impl Tooltipable for Buildable {
	fn description(&self) -> &'static str {
		match self {
			Self::Cottage => AccommodationType::Cottage.description(),
			Self::Pathway => GroundKind::Pathway.description(),
			Self::PoolArea => "Demarcate a pool area to start building a pool.",
		}
	}
}

// FIXME: Use reflection for this.
pub const ALL_BUILDABLES: [Buildable; 3] = [Buildable::Pathway, Buildable::PoolArea, Buildable::Cottage];

impl Buildable {
	pub fn menu(&self) -> BuildMenu {
		match self {
			Self::Pathway => BuildMenu::Basics,
			Self::PoolArea => BuildMenu::Pool,
			Self::Cottage => BuildMenu::Accommodation,
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
