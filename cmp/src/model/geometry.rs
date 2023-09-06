//! Geometric baseline components.

use bevy::ecs::component::TableStorage;
use bevy::math::Vec3A;
use bevy::prelude::*;

/// A position in world space, as opposed to screen space. There are several underlying implementations of world
/// positions, depending on how an entity’s position is constrained.
///
/// The unit of world positions is tiles. One tile width corresponds to one world position
pub trait WorldPosition: Component<Storage = TableStorage> {
	/// Returns this component’s position in world space. For entities with extents other than 1, this is the center of
	/// the larger entity.
	fn position(&self) -> Vec3A;
}

/// An actor’s position is unconstrained in all three axes, and it can have non-grid-aligned values.
#[derive(Component, Default, Clone, Copy, Debug, Deref, PartialEq)]
pub struct ActorPosition(pub(crate) Vec3A);

impl ActorPosition {
	#[inline]
	pub fn round(self) -> GridPosition {
		GridPosition(self.0.round().as_ivec3())
	}
}

impl WorldPosition for ActorPosition {
	#[inline]
	fn position(&self) -> Vec3A {
		self.0
	}
}

// FIXME: Should be statically polymorphic, but dynamic polymorphism messes up Rusts's lifetimes, and static
// polymorphism conflicts with the blanket impl<T> from the standard library.
impl From<GridPosition> for ActorPosition {
	#[inline]
	fn from(value: GridPosition) -> Self {
		Self(value.position())
	}
}

impl<T: Into<Vec3>> std::ops::Sub<T> for ActorPosition {
	type Output = Self;

	#[inline]
	fn sub(self, rhs: T) -> Self::Output {
		Self(self.0 - Vec3A::from(rhs.into()))
	}
}

/// A grid position can only take exact grid values.
#[derive(Component, Default, Copy, Clone, Debug, Deref, DerefMut, Eq, PartialEq, Hash)]
pub struct GridPosition(pub(crate) IVec3);

impl GridPosition {
	/// Returns all grid positions on the straight line to `target`, effectively performing line rasterization.
	/// FIXME: Respect the Z dimension; all positions will currently inherit the source's z height.
	pub fn line_to_2d(self, target: Self) -> impl Iterator<Item = Self> {
		// Relying on extra unstable features is fun! <https://rust-lang.github.io/rfcs/2033-experimental-coroutines.html>
		std::iter::from_generator(move || {
			// Bresenham's algorithm
			if (self.y - target.y).abs() < (self.x - target.x).abs() {
				// non-steep slopes
				let lower = *if self.x < target.x { self } else { target };
				let upper = *if self.x >= target.x { self } else { target };

				let dx = upper.x - lower.x;
				let mut dy = upper.y - lower.y;

				let mut y_dir = 1;
				if dy < 0 {
					y_dir = -1;
					dy = -dy;
				}
				let mut d = (2 * dy) - dx;
				let mut y = lower.y;
				for x in lower.x ..= upper.x {
					yield (x, y, self.z).into();
					if d > 0 {
						y += y_dir;
						d += 2 * (dy - dx);
					} else {
						d += 2 * dy;
					}
				}
			} else {
				// steep slopes: iterate on y instead of x
				let lower = *if self.y < target.y { self } else { target };
				let upper = *if self.y >= target.y { self } else { target };

				let mut dx = upper.x - lower.x;
				let dy = upper.y - lower.y;
				let mut x_dir = 1;
				if dx < 0 {
					x_dir = -1;
					dx = -dx;
				}
				let mut d = (2 * dx) - dy;
				let mut x = lower.x;
				for y in lower.y ..= upper.y {
					yield (x, y, self.z).into();
					if d > 0 {
						x += x_dir;
						d += 2 * (dx - dy);
					} else {
						d += 2 * dx;
					}
				}
			}
		})
	}
}

impl WorldPosition for GridPosition {
	#[inline]
	fn position(&self) -> Vec3A {
		self.0.as_vec3a()
	}
}

impl PartialOrd for GridPosition {
	/// A grid position is considered smaller if its distance to negative infinity (sum of all coordinates) is smaller.
	/// However, if two grid positions have the same distance to negative infinity but distinct coordinates, an order
	/// cannot be determined (that's why there is intentionally no [`std::cmp::Ord`] implementation on this type).
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		if self.0 == other.0 {
			Some(std::cmp::Ordering::Equal)
		} else {
			let self_negative_inf_distance = self.0.x + self.0.y + self.0.z;
			let other_negative_inf_distance = other.0.x + other.0.y + other.0.z;
			match self_negative_inf_distance.cmp(&other_negative_inf_distance) {
				order @ (std::cmp::Ordering::Less | std::cmp::Ordering::Greater) => Some(order),
				// Distance is equal, but we already know the positions are not equal!
				std::cmp::Ordering::Equal => None,
			}
		}
	}
}

impl From<(i32, i32, i32)> for GridPosition {
	#[inline]
	fn from(value: (i32, i32, i32)) -> Self {
		GridPosition(value.into())
	}
}

impl From<(i32, i32)> for GridPosition {
	#[inline]
	fn from(value: (i32, i32)) -> Self {
		GridPosition((value.into(), 0).into())
	}
}

impl From<IVec3> for GridPosition {
	#[inline]
	fn from(value: IVec3) -> Self {
		GridPosition(value)
	}
}

impl std::ops::Sub<IVec2> for GridPosition {
	type Output = Self;

	#[inline]
	fn sub(self, rhs: IVec2) -> Self::Output {
		self - IVec3::from((rhs, 0))
	}
}

impl std::ops::Add<IVec2> for GridPosition {
	type Output = Self;

	#[inline]
	fn add(self, rhs: IVec2) -> Self::Output {
		self + IVec3::from((rhs, 0))
	}
}

impl std::ops::Sub<IVec3> for GridPosition {
	type Output = Self;

	#[inline]
	fn sub(self, rhs: IVec3) -> Self::Output {
		GridPosition(self.0 - rhs)
	}
}

impl std::ops::Add<IVec3> for GridPosition {
	type Output = Self;

	#[inline]
	fn add(self, rhs: IVec3) -> Self::Output {
		GridPosition(self.0 + rhs)
	}
}

/// A rectangular bounding box around an entity. The entity’s position is in the corner with the smallest distance to
/// negative infinity on all axes, so the box extends define how far the box stretches in each positive axis direction.

#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, PartialEq, Eq)]
pub struct BoundingBox(pub IVec3);

impl BoundingBox {
	#[inline]
	pub const fn height(&self) -> i32 {
		self.0.z
	}

	#[inline]
	pub const fn with_height(mut self, new_height: i32) -> Self {
		self.0.z = new_height;
		self
	}

	#[inline]
	pub const fn fixed<const X: i32, const Y: i32, const Z: i32>() -> Self {
		Self(IVec3 { x: X, y: Y, z: Z })
	}
}

impl From<(i32, i32)> for BoundingBox {
	#[inline]
	fn from(value: (i32, i32)) -> Self {
		Self(IVec3::from((value.into(), 0)))
	}
}

impl From<(i32, i32, i32)> for BoundingBox {
	#[inline]
	fn from(value: (i32, i32, i32)) -> Self {
		Self(value.into())
	}
}

impl From<IVec3> for BoundingBox {
	#[inline]
	fn from(value: IVec3) -> Self {
		Self(value)
	}
}

impl std::ops::Div<i32> for BoundingBox {
	type Output = IVec3;

	#[inline]
	fn div(self, rhs: i32) -> Self::Output {
		self.0 / rhs
	}
}

/// An axis-aligned bounding box (AABB) internal to CMP world space. It usually defines the extents of some permanent
/// structure like a building, and effectively combines a [`GridPosition`] with a [`BoundingBox`]. Collisions are
/// primarily computed between GridBox objects.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridBox {
	// Intentionally private fields:
	/// Lower left corner; the point with the smallest distance to negative infinity inside the box on all axes.
	corner:  GridPosition,
	extents: BoundingBox,
}

impl PartialOrd for GridBox {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		match self.corner.partial_cmp(&other.corner) {
			Some(core::cmp::Ordering::Equal) =>
			// Reuse the partial ordering properties of grid positions, so that the volumetrically smaller bounding
			// box is considered less (and equal bounding boxes are considered equal).
				GridPosition(*self.extents).partial_cmp(&GridPosition(*other.extents)),
			ord => ord,
		}
	}
}

impl WorldPosition for GridBox {
	#[inline]
	fn position(&self) -> Vec3A {
		self.corner.position()
	}
}

impl From<GridPosition> for GridBox {
	fn from(value: GridPosition) -> Self {
		Self { corner: value, ..Default::default() }
	}
}

impl GridBox {
	pub fn new(position: GridPosition, extents: BoundingBox) -> Self {
		// Actually determine the smallest corner ("normalize" the box properties) which allows the user to provide any
		// corner and extent kind.
		let first_corner = *position;
		let second_corner = *position + *extents;
		Self::from_corners(first_corner.into(), second_corner.into())
	}

	pub fn from_corners(first_corner: GridPosition, second_corner: GridPosition) -> Self {
		let smallest_corner = first_corner.min(*second_corner);
		let largest_corner = first_corner.max(*second_corner);
		let real_extents = largest_corner - smallest_corner;
		Self { corner: smallest_corner.into(), extents: real_extents.into() }
	}

	pub fn smallest(&self) -> GridPosition {
		self.corner
	}

	pub fn largest(&self) -> GridPosition {
		self.corner + *self.extents
	}

	/// Raises or lowers the extents.
	pub fn enlargen(&mut self, delta: IVec3) {
		(*self.extents) += delta;
		if self.extents.x < 0 || self.extents.y < 0 || self.extents.z < 0 {
			*self = Self::new(self.corner, self.extents);
		}
	}

	/// The corner must be the smallest corner on all axes, otherwise the grid box's invariants are broken and weird
	/// behavior may result.
	#[allow(unused)]
	pub unsafe fn from_raw(corner: GridPosition, extents: BoundingBox) -> Self {
		Self { corner, extents }
	}

	#[inline]
	pub const fn height(&self) -> i32 {
		self.extents.height()
	}

	/// Returns whether the other box object intersects this box object.
	///
	/// This is a lower-level API used by various high-level collision functions.
	#[allow(unused)]
	pub fn intersects(&self, other: GridBox) -> bool {
		let axis_intersects = |own_start, own_end, other_start, other_end| {
			// Either of our points is between the other’s start and end.
			(own_start < other_end && own_start >= other_start) || (own_end < other_end && own_end >= other_start)
		};

		let own_start = self.corner;
		let own_end = own_start + self.extents.0;
		let other_start = other.corner;
		let other_end = other_start + other.extents.0;

		axis_intersects(own_start.x, own_end.x, other_start.x, other_end.x)
			&& axis_intersects(own_start.y, own_end.y, other_start.y, other_end.y)
			&& axis_intersects(own_start.z, own_end.z, other_start.z, other_end.z)
	}

	/// Returns whether the other box object intersects this box object on the xy plane.
	///
	/// This is a lower-level API used by various high-level collision functions.
	pub fn intersects_2d(&self, other: GridBox) -> bool {
		let axis_intersects = |own_start, own_end, other_start, other_end| {
			// Either of our points is between the other’s start and end.
			(own_start < other_end && own_start >= other_start) || (own_end < other_end && own_end >= other_start)
		};

		let own_start = self.corner;
		let own_end = own_start + self.extents.0;
		let other_start = other.corner;
		let other_end = other_start + other.extents.0;

		axis_intersects(own_start.x, own_end.x, other_start.x, other_end.x)
			&& axis_intersects(own_start.y, own_end.y, other_start.y, other_end.y)
	}

	/// Returns the box’s extents in world space. The extents define how large the entity is along each axis. Extents
	/// are used for various purposes, but most importantly, they are used to determine static entity collisions and
	/// intersections, such as for construction.
	///
	/// Extents use integer vectors, since the collision mechanics for boxes are snapped to the grid.
	#[inline]
	pub const fn extents(&self) -> IVec3 {
		self.extents.0
	}
}
