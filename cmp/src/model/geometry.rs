//! Geometric baseline components.

use bevy::ecs::component::TableStorage;
use bevy::math::Vec3A;
use bevy::prelude::*;
use itertools::Itertools;

/// A position in world space, as opposed to screen space. There are several underlying implementations of world
/// positions, depending on how an entity’s position is constrained.
///
/// The unit of world positions is tiles. One tile width corresponds to one world position
pub trait WorldPosition: Component<Storage = TableStorage> {
	/// Returns this component’s position in world space. For entities with extents other than 1, this is the bottom
	/// left corner of the larger entity.
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

impl<T: Into<Vec3A>> From<T> for ActorPosition {
	#[inline]
	fn from(value: T) -> Self {
		Self(value.into())
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
		std::iter::from_coroutine(move || {
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

	pub fn neighbors(&self) -> [GridPosition; 4] {
		[(-1, 0), (1, 0), (0, -1), (0, 1)].map(|(x, y)| *self + IVec2::from((x, y)))
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

impl From<UVec3> for GridPosition {
	#[inline]
	fn from(value: UVec3) -> Self {
		GridPosition(value.as_ivec3())
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

impl std::ops::Sub<GridPosition> for GridPosition {
	type Output = Self;

	#[inline]
	fn sub(self, rhs: GridPosition) -> Self::Output {
		GridPosition(*self - *rhs)
	}
}

impl std::ops::Add<GridPosition> for GridPosition {
	type Output = Self;

	#[inline]
	fn add(self, rhs: GridPosition) -> Self::Output {
		GridPosition(*self + *rhs)
	}
}

impl std::fmt::Display for GridPosition {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{},{},{}", self.x, self.y, self.z)
	}
}

/// A rectangular bounding box around an entity. The entity’s position is in the corner with the smallest distance to
/// negative infinity on all axes, so the box extends define how far the box stretches in each positive axis direction.

#[derive(Component, Clone, Copy, Debug, Default, Deref, DerefMut, PartialEq, Eq)]
pub struct BoundingBox(pub UVec3);

impl BoundingBox {
	#[inline]
	#[allow(unused)]
	pub const fn height(&self) -> u32 {
		self.0.z
	}

	#[inline]
	pub const fn with_height(mut self, new_height: u32) -> Self {
		self.0.z = new_height;
		self
	}

	#[inline]
	pub const fn fixed<const X: u32, const Y: u32, const Z: u32>() -> Self {
		Self(UVec3 { x: X, y: Y, z: Z })
	}

	#[inline]
	pub const fn flat(self) -> Self {
		self.with_height(1)
	}
}

impl From<(u32, u32)> for BoundingBox {
	#[inline]
	fn from(value: (u32, u32)) -> Self {
		Self(UVec3::from((value.into(), 0)))
	}
}

impl From<(u32, u32, u32)> for BoundingBox {
	#[inline]
	fn from(value: (u32, u32, u32)) -> Self {
		Self(value.into())
	}
}

impl From<UVec3> for BoundingBox {
	#[inline]
	fn from(value: UVec3) -> Self {
		Self(value)
	}
}

impl std::ops::Div<u32> for BoundingBox {
	type Output = UVec3;

	#[inline]
	fn div(self, rhs: u32) -> Self::Output {
		self.0 / rhs
	}
}

/// An axis-aligned bounding box (AABB) internal to CMP world space. It usually defines the extents of some permanent
/// structure like a building, and effectively combines a [`GridPosition`] with a [`BoundingBox`]. Collisions are
/// primarily computed between GridBox objects.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridBox {
	/// Lower left corner; the point with the smallest distance to negative infinity inside the box on all axes.
	pub corner: GridPosition,
	extents:    BoundingBox,
}

impl PartialOrd for GridBox {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		match self.corner.partial_cmp(&other.corner) {
			Some(core::cmp::Ordering::Equal) =>
			// Reuse the partial ordering properties of grid positions, so that the volumetrically smaller bounding
			// box is considered less (and equal bounding boxes are considered equal).
				GridPosition::from(*self.extents).partial_cmp(&GridPosition::from(*other.extents)),
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

pub trait Extent {
	fn as_ivec3(&self) -> IVec3;
}
impl Extent for IVec3 {
	#[inline]
	fn as_ivec3(&self) -> IVec3 {
		*self
	}
}
impl Extent for UVec3 {
	#[inline]
	fn as_ivec3(&self) -> IVec3 {
		self.as_ivec3()
	}
}
impl Extent for BoundingBox {
	#[inline]
	fn as_ivec3(&self) -> IVec3 {
		self.0.as_ivec3()
	}
}

impl GridBox {
	pub fn new<T: Extent>(position: GridPosition, extents: T) -> Self {
		// Actually determine the smallest corner ("normalize" the box properties) which allows the user to provide any
		// corner and extent kind.
		let first_corner = *position;
		let second_corner = *position + extents.as_ivec3() - IVec3::from((1, 1, 1));
		Self::from_corners(first_corner.into(), second_corner.into())
	}

	/// Creates a grid box with the extents centered at the given position.
	pub fn around(center: GridPosition, extents: BoundingBox) -> Self {
		// Actually determine the smallest corner ("normalize" the box properties) which allows the user to provide any
		// corner and extent kind.
		let first_corner = *center - (extents / 2).as_ivec3();
		let second_corner = first_corner + extents.as_ivec3() - IVec3::from((1, 1, 1));
		Self::from_corners(first_corner.into(), second_corner.into())
	}

	pub fn from_corners(first_corner: GridPosition, second_corner: GridPosition) -> Self {
		let smallest_corner = first_corner.min(*second_corner);
		let largest_corner = first_corner.max(*second_corner);
		let real_extents = largest_corner - smallest_corner;
		debug_assert!(real_extents.x >= 0 && real_extents.y >= 0 && real_extents.z >= 0);
		Self { corner: smallest_corner.into(), extents: real_extents.as_uvec3().into() }
	}

	#[inline]
	pub const fn smallest(&self) -> GridPosition {
		self.corner
	}

	/// Inclusive (!) upper corner of the box.
	#[inline]
	pub fn largest(&self) -> GridPosition {
		self.corner + self.extents.as_ivec3()
	}

	#[inline]
	#[allow(unused)]
	pub fn center(&self) -> GridPosition {
		self.corner + (self.extents / 2).as_ivec3()
	}

	/// Returns all positions on the floor (lowest z) of this AABB.
	pub fn floor_positions(&self) -> impl Iterator<Item = GridPosition> + '_ {
		(self.smallest().x ..= self.largest().x)
			.cartesian_product(self.smallest().y ..= self.largest().y)
			.map(|(x, y)| (x, y, self.smallest().z).into())
	}

	/// Raises or lowers the extents.
	pub fn enlargen(&mut self, delta: IVec3) {
		let new_extents = self.extents.as_ivec3() + delta;
		if new_extents.x < 0 || new_extents.y < 0 || new_extents.z < 0 {
			*self = Self::new(self.corner, new_extents);
			return;
		}
		*self.extents = new_extents.as_uvec3();
	}

	/// The corner must be the smallest corner on all axes, otherwise the grid box's invariants are broken and weird
	/// behavior may result.
	#[allow(unused)]
	pub unsafe fn from_raw(corner: GridPosition, extents: BoundingBox) -> Self {
		Self { corner, extents }
	}

	/// Returns whether the given position is exactly on the smaller edges (negative X and negative Y) of this AABB.
	#[inline]
	pub fn has_on_smaller_edges(&self, position: Vec3A) -> bool {
		let in_range = |start, end, value| value >= start && value < end;

		let (start_x, start_y, _) = (*self.smallest()).into();
		let (end_x, end_y, _) = (*self.largest() + IVec3::new(1, 1, 0)).into();
		(position.x == start_x as f32 && in_range(start_y as f32, end_y as f32, position.y))
			|| (position.y == start_y as f32 && in_range(start_x as f32, end_x as f32, position.x))
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
		let own_end = self.largest();
		let other_start = other.corner;
		let other_end = other.largest();

		axis_intersects(own_start.x, own_end.x, other_start.x, other_end.x)
			&& axis_intersects(own_start.y, own_end.y, other_start.y, other_end.y)
			&& axis_intersects(own_start.z, own_end.z, other_start.z, other_end.z)
	}

	/// Returns whether the other box object intersects this box object on the xy plane.
	///
	/// This is a lower-level API used by various high-level collision functions.
	#[allow(unused)]
	pub fn intersects_2d(&self, other: GridBox) -> bool {
		let axis_intersects = |own_start, own_end, other_start, other_end| {
			// Either of our points is between the other’s start and end.
			(own_start < other_end && own_start >= other_start) || (own_end < other_end && own_end >= other_start)
		};

		let own_start = self.corner;
		let own_end = own_start + self.extents.as_ivec3();
		let other_start = other.corner;
		let other_end = other_start + other.extents.as_ivec3();

		axis_intersects(own_start.x, own_end.x, other_start.x, other_end.x)
			&& axis_intersects(own_start.y, own_end.y, other_start.y, other_end.y)
	}

	/// Returns the box’s extents in world space. The extents define how large the entity is along each axis. Extents
	/// are used for various purposes, but most importantly, they are used to determine static entity collisions and
	/// intersections, such as for construction.
	///
	/// Extents use integer vectors, since the collision mechanics for boxes are snapped to the grid.
	#[allow(unused)]
	#[inline]
	pub const fn extents(&self) -> UVec3 {
		self.extents.0
	}
}
