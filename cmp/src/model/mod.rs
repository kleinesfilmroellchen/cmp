//! Internal world state data models and game mechanics.

pub mod accommodation;
pub mod area;
pub mod geometry;
pub mod tile;

use std::marker::ConstParamTy;

pub use accommodation::*;
use bevy::prelude::*;
pub use geometry::*;
pub use tile::*;

use crate::ui::build::BuildMode;
use crate::ui::controls::BuildMenu;
use crate::util::Tooltipable;

/// All build-able objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
#[repr(u8)]
pub enum Buildable {
	/// A [`GroundTile`] of some [`GroundKind`].
	Ground(GroundKind),
	/// Demarcates the [`area::Area`] of a pool; filled with [`GroundKind::PoolPath`].
	PoolArea,
	/// Demarcates an unspecified [`Accommodation`]-[`area::Area`].
	AccommodationSite,
	/// Some [`AccommodationType`] specifying the kind of an already existing [`Accommodation`].
	Accommodation(AccommodationType),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ConstParamTy)]
#[repr(u8)]
pub enum BuildableType {
	Ground,
	PoolArea,
	AccommodationSite,
	Accommodation,
}

impl From<Buildable> for BuildableType {
	fn from(value: Buildable) -> Self {
		unsafe { *(((&value) as *const Buildable) as *const Self) }
	}
}

impl std::fmt::Display for Buildable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::Accommodation(kind) => kind.to_string(),
			Self::AccommodationSite => "Accommodation Site".to_string(),
			Self::Ground(kind) => kind.to_string(),
			Self::PoolArea => "Pool Area".to_string(),
		})
	}
}

impl Tooltipable for Buildable {
	fn description(&self) -> &'static str {
		match self {
			Self::Accommodation(kind) => kind.description(),
			Self::AccommodationSite =>
				"Demarcate a new accommodation site. The accommodation will initially be empty and cannot take \
				 visitors. You have to specify the kind of accommodation by building an accommodation on top of this \
				 site.",
			Self::Ground(kind) => kind.description(),
			Self::PoolArea => "Demarcate a pool area to start building a pool.",
		}
	}
}

pub const ALL_BUILDABLES: [Buildable; 9] = [
	Buildable::Ground(GroundKind::Pathway),
	Buildable::Ground(GroundKind::Grass),
	Buildable::PoolArea,
	Buildable::AccommodationSite,
	Buildable::Accommodation(AccommodationType::TentSite),
	Buildable::Accommodation(AccommodationType::CaravanSite),
	Buildable::Accommodation(AccommodationType::PermanentTent),
	Buildable::Accommodation(AccommodationType::MobileHome),
	Buildable::Accommodation(AccommodationType::Cottage),
];

impl Buildable {
	pub fn menu(&self) -> BuildMenu {
		match self {
			Self::Ground(_) => BuildMenu::Basics,
			Self::PoolArea => BuildMenu::Pool,
			Self::AccommodationSite | Self::Accommodation(_) => BuildMenu::Accommodation,
		}
	}

	pub fn size(&self) -> BoundingBox {
		match self {
			Self::Ground(_) => (1, 1).into(),
			Self::AccommodationSite | Self::PoolArea => (1, 1).into(),
			Self::Accommodation(kind) => kind.size(),
		}
	}

	pub fn build_mode(&self) -> BuildMode {
		match self {
			Self::Ground(_) => BuildMode::Line,
			Self::AccommodationSite | Self::PoolArea => BuildMode::Rect,
			Self::Accommodation(_) => BuildMode::Single,
		}
	}
}

/// A general-purpose metric with a specific range. Metrics are always natural numbers to simplify UI and corresponding
/// game mechanics. Specific subsystems will define their own metric-derived types with specific ranges.
#[derive(Clone, Copy, Debug, Eq, Ord, Deref)]
pub struct Metric<const MIN: u64, const MAX: u64>(u64);

impl<const MIN: u64, const MAX: u64> TryFrom<u64> for Metric<MIN, MAX> {
	type Error = ();

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		if value < MIN || value > MAX {
			Err(())
		} else {
			Ok(Self(value))
		}
	}
}

impl<const MIN: u64, const MAX: u64> std::fmt::Display for Metric<MIN, MAX> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl<const MIN: u64, const MAX: u64> Metric<MIN, MAX> {
	#[allow(unused)]
	pub const MAX: Self = Self(MAX);
	#[allow(unused)]
	pub const MIN: Self = Self(MIN);
}

impl<const MIN: u64, const MAX: u64> Default for Metric<MIN, MAX> {
	fn default() -> Self {
		Self(MIN)
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
