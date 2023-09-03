use bevy::prelude::*;

use super::{GroundBox, Metric};
use crate::util::Tooltipable;

/// The different available types of accommodation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccommodationType {
	TentSite,
	LargeTentSite,
	PermanentTent,
	CaravanSite,
	MobileHome,
	Cottage,
}

type Comfort = Metric<0, 10>;

impl AccommodationType {
	pub fn size(&self) -> GroundBox {
		match self {
			Self::TentSite => (5, 5),
			Self::LargeTentSite => (7, 5),
			Self::PermanentTent => (4, 4),
			Self::CaravanSite => (5, 5),
			Self::MobileHome => (3, 4),
			Self::Cottage => (3, 4),
		}
		.into()
	}

	pub fn comfort(&self) -> Comfort {
		match self {
			Self::TentSite => 1u64,
			Self::LargeTentSite => 1,
			Self::PermanentTent => 4,
			Self::CaravanSite => 3,
			Self::MobileHome => 5,
			Self::Cottage => 6,
		}.into()
	}
}

impl std::fmt::Display for AccommodationType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			Self::TentSite => "Tent Site",
			Self::LargeTentSite => "Large Tent Site",
			Self::PermanentTent => "Permanent Tent",
			Self::CaravanSite => "Caravan Site",
			Self::MobileHome => "Mobile Home",
			Self::Cottage => "Cottage",
		})
	}
}

impl Tooltipable for AccommodationType {
	fn description(&self) -> &'static str {
		match self {
			Self::TentSite =>
				"A basic tent site, suitable for a small tent and two people. Tent sites are not more than demarcated \
				 patches of grass, and take almost no effort to maintain. Only the hardy tent-camping visitors will \
				 use tent sites, however. Tent sites also take up a relatively large area in comparison to the amount \
				 of people that can stay there.",
			Self::LargeTentSite =>
				"A larger tent size, suitable for five people. This tent site is not fundamentally different from the \
				 standard one, but it provides space for larger groups of hardy campers.",
			Self::PermanentTent =>
				"A permanently constructed tent for five campers. Due to its construction with wooden flooring under a \
				 cloth roof, this tent does provide better comfort than a bare camp site, though its spacial \
				 requirement is only a little less than the large tent site’s. It requires some more upkeep, of \
				 course, but it doesn’t need water or electricity. You can, however, connect those resources anyways, \
				 which will mildly improve visitor satisfaction.",
			Self::CaravanSite =>
				"A site for two or three campers to park their caravans. As opposed to tent sites, caravan sites need \
				 a permanent water and electricity supply for the vehicles. In turn, less hardy campers with their \
				 caravans will show up to these accommodation spots. As with tent sites, caravan sites provide ample \
				 space for the few visitors.",
			Self::MobileHome =>
				"A mobile home, the most basic form of permanent housing for four visitors. Mobile homes are parked \
				 semi-permanently, need water and electricity, and they provide much more comfort than even a caravan. \
				 In addition, mobile homes are parked on a rather small patch of land. However, their upkeep is \
				 significantly more resource-intensive than the simple sites, since campers no longer bring their own \
				 housing.",
			Self::Cottage =>
				"A basic cottage for up to six visitors. Cottages are not more than semi-permanent wooden huts set up \
				 on a relatively small patch of land, and they can accommodate a whole group of people pretty \
				 comfortably. Cottages require water and electricity, and will need to be maintained for visitor \
				 satisfaction.",
		}
	}
}

/// A proper accommodation for guests; essentially an instance of [`AccommodationType`].
#[derive(Component)]
pub struct Accommodation {
	kind: AccommodationType,
}
