//! Navigation and navmesh information.

use std::cmp::Ordering;
use std::marker::ConstParamTy;

use bevy::prelude::*;
use bevy::utils::{HashMap, Instant};

use super::GridPosition;
use crate::graphics::Sides;

/// The kinds of navigability, used by different groups of actors.
/// Each kind has its own nav mesh.
/// Note that many nav categories are subcategories of others, practically speaking. This is expressed with the
/// category ordering; see [`PartialOrd`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ConstParamTy)]
#[repr(u8)]
pub enum NavCategory {
	#[default]
	People,
	Vehicles,
}

impl PartialOrd for NavCategory {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		if self == other {
			Some(Ordering::Equal)
		} else {
			match (self, other) {
				(NavCategory::People, _) => Some(Ordering::Less),
				(NavCategory::Vehicles, NavCategory::People) => Some(Ordering::Greater),
				_ => None,
			}
		}
	}
}

/// A navigable vertex on the ground. The entities with these components make up the nav meshes in the world.
#[derive(Component, Clone, Copy, Debug)]
pub struct NavVertex {
	/// Which directions this vertex has exits in.
	pub exits:        Sides,
	/// What speed this vertex can be traversed at. In the navmesh graph this is used for traversing from this vertex
	/// to the next.
	pub speed:        f32,
	/// This determines the *base* navigability of the mesh component. As per the category's subset relationship, this
	/// vertex may be part of other navmeshes too.
	pub navigability: NavCategory,
}

/// The edge weights are the *inverse* of the traversal speed. A weight of 0 means no connection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VertexEdges {
	up:     f32,
	right:  f32,
	bottom: f32,
	left:   f32,
}

impl From<(Sides, f32)> for VertexEdges {
	fn from((sides, weight): (Sides, f32)) -> Self {
		let mut this = VertexEdges::default();
		for side in sides {
			match side {
				Sides::Top => this.up = weight,
				Sides::Right => this.right = weight,
				Sides::Bottom => this.bottom = weight,
				Sides::Left => this.left = weight,
				_ => unreachable!(),
			}
		}
		this
	}
}

/// A navigation mesh. This is not really a mesh, but it serves the same function as a 3D navmesh. Mathematically
/// speaking, the navmesh is a directed weighted graph.
#[derive(Resource, Debug, Default)]
pub struct NavMesh<const N: NavCategory> {
	/// If an entry (a, b) exists, that means that this navmesh has an edge from position a to side b.
	edges: HashMap<GridPosition, VertexEdges>,
}

impl<const N: NavCategory> NavMesh<N> {
	/// Updates vertex state with minimal effort, leaving the mesh inconsistent.
	fn update_vertex_impl(&mut self, position: &GridPosition, vertex: NavVertex) {
		let belongs_in_mesh = vertex.navigability == N;
		// Vertex is being added to the mesh or modified within it.
		if belongs_in_mesh {
			self.edges.entry(*position).insert((vertex.exits, 1. / vertex.speed).into());
		} else {
			// Vertex is being removed from the mesh.
			self.edges.remove(position);
		}
	}

	pub fn update_vertices<'a>(&mut self, vertices: impl IntoIterator<Item = (&'a GridPosition, &'a NavVertex)>) {
		for (position, vertex) in vertices {
			self.update_vertex_impl(position, *vertex);
		}
	}
}

fn update_navmesh<const N: NavCategory>(
	mut mesh: ResMut<NavMesh<N>>,
	changed_navigables: Query<(&GridPosition, &NavVertex), Changed<NavVertex>>,
) {
	if changed_navigables.is_empty() {
		return;
	}
	let start = Instant::now();
	mesh.update_vertices(&changed_navigables);
	debug!("Navmesh update took {:?}", Instant::now() - start);
}

fn visualize_navmesh<const N: NavCategory>(mesh: Res<NavMesh<N>>) {}

pub struct NavManagement;

impl Plugin for NavManagement {
	fn build(&self, app: &mut App) {
		app.init_resource::<NavMesh<{ NavCategory::People }>>()
			.init_resource::<NavMesh<{ NavCategory::Vehicles }>>()
			.add_systems(
				FixedUpdate,
				(update_navmesh::<{ NavCategory::People }>, update_navmesh::<{ NavCategory::Vehicles }>),
			)
			.add_systems(Update, visualize_navmesh::<{ NavCategory::People }>);
	}
}
