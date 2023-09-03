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
#[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
pub struct ActorPosition(pub(crate) Vec3A);

impl ActorPosition {
	pub fn round(self) -> GridPosition {
		GridPosition(self.0.round().as_ivec3())
	}
}

impl WorldPosition for ActorPosition {
	fn position(&self) -> Vec3A {
		self.0
	}
}

// FIXME: Should be statically polymorphic, but dynamic polymorphism messes up Rusts's lifetimes, and static
// polymorphism conflicts with the blanket impl<T> from the standard library.
impl From<GridPosition> for ActorPosition {
	fn from(value: GridPosition) -> Self {
		Self(value.position())
	}
}

impl<T: Into<Vec3>> std::ops::Sub<T> for ActorPosition {
	type Output = Self;

	fn sub(self, rhs: T) -> Self::Output {
		Self(self.0 - Vec3A::from(rhs.into()))
	}
}

/// A grid position can only take exact grid values.
#[derive(Component, Default, Copy, Clone, Debug)]
pub struct GridPosition(pub(crate) IVec3);

impl WorldPosition for GridPosition {
	fn position(&self) -> Vec3A {
		self.0.as_vec3a()
	}
}

impl From<(i32, i32, i32)> for GridPosition {
	fn from(value: (i32, i32, i32)) -> Self {
		GridPosition(value.into())
	}
}

impl std::ops::Sub<IVec2> for GridPosition {
	type Output = Self;

	fn sub(self, rhs: IVec2) -> Self::Output {
		GridPosition(self.0 - IVec3::from((rhs, 0)))
	}
}

impl std::ops::Add<IVec2> for GridPosition {
	type Output = Self;

	fn add(self, rhs: IVec2) -> Self::Output {
		GridPosition(self.0 + IVec3::from((rhs, 0)))
	}
}

/// A rectangular bounding box around an entity. The entity’s position is in the corner with the smallest distance to
/// the origin, so the box extends define how far the box stretches in each positive axis direction.

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox(IVec3);
impl BoundingBox {
	/// Returns the box’s extents in world space. The extents define how large the entity is along each axis. Extents
	/// are used for various purposes, but most importantly, they are used to determine static entity collisions and
	/// intersections, such as for construction.
	///
	/// Extents use integer vectors, since the collision mechanics for boxes are snapped to the grid.
	pub const fn extents(&self) -> IVec3 {
		self.0
	}

	pub const fn height(&self) -> i32 {
		self.0.z
	}

	pub const fn with_height(mut self, new_height: i32) -> Self {
		self.0.z = new_height;
		self
	}

	/// Returns whether the other box object intersects this box object. For this, each box’s position must be supplied.
	///
	/// This is a lower-level API used by various high-level entity bundle collision functions.
	pub fn intersects(
		&self,
		position: &dyn WorldPosition,
		other: BoundingBox,
		other_position: &dyn WorldPosition,
	) -> bool {
		let axis_intersects = |own_start, own_end, other_start, other_end| {
			// Either of our points is between the other’s start and end.
			(own_start < other_end && own_start >= other_start) || (own_end < other_end && own_end >= other_start)
		};

		let own_start = position.position();
		let own_end = own_start + self.extents().as_vec3a();
		let other_start = other_position.position();
		let other_end = other_start + other.extents().as_vec3a();

		axis_intersects(own_start.x, own_end.x, other_start.x, other_end.x)
			&& axis_intersects(own_start.y, own_end.y, other_start.y, other_end.y)
			&& axis_intersects(own_start.z, own_end.z, other_start.z, other_end.z)
	}

	pub const fn fixed<const X: i32, const Y: i32, const Z: i32>() -> Self {
		Self(IVec3 { x: X, y: Y, z: Z })
	}
}

impl From<(i32, i32)> for BoundingBox {
	fn from(value: (i32, i32)) -> Self {
		Self(IVec3::from((value.into(), 0)))
	}
}

impl From<(i32, i32, i32)> for BoundingBox {
	fn from(value: (i32, i32, i32)) -> Self {
		Self(value.into())
	}
}

impl std::ops::Div<i32> for BoundingBox {
	type Output = IVec3;

	fn div(self, rhs: i32) -> Self::Output {
		self.0 / rhs
	}
}
