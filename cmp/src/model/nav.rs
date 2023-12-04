//! Navigation and navmesh information.

use std::cmp::Ordering;
use std::marker::ConstParamTy;

use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::utils::petgraph::graphmap::DiGraphMap;
use bevy::utils::Instant;

use super::GridPosition;
use crate::config::GameSettings;
use crate::graphics::{Sides, TRANSFORMATION_MATRIX};

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

/// A navigation mesh. This is not really a mesh, but it serves the same function as a 3D navmesh. Mathematically
/// speaking, the navmesh is a directed weighted graph.
#[derive(Resource, Debug, Default)]
pub struct NavMesh<const N: NavCategory> {
	/// Internal graph for the nav mesh.
	graph: DiGraphMap<GridPosition, f32>,
}

impl<const N: NavCategory> NavMesh<N> {
	/// Updates vertex state with minimal effort, leaving the mesh inconsistent.
	fn update_vertex_impl(&mut self, position: &GridPosition, vertex: NavVertex) {
		let belongs_in_mesh = vertex.navigability <= N;
		// Vertex is being added to the mesh or modified within it.
		if belongs_in_mesh {
			for neighbor in position.neighbors_for(vertex.exits.complement()) {
				self.graph.remove_edge(*position, neighbor);
			}
			let weight = 1. / vertex.speed;
			for neighbor in position.neighbors_for(vertex.exits) {
				self.graph.add_edge(*position, neighbor, weight);
			}
		} else {
			// Vertex is being removed from the mesh.
			self.graph.remove_node(*position);
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
	debug!("Navmesh {:?} update took {:?}", N, Instant::now() - start);
}

fn visualize_navmesh<const N: NavCategory>(mesh: Res<NavMesh<N>>, mut gizmos: Gizmos, settings: Res<GameSettings>) {
	if !settings.show_debug {
		return;
	}

	for (start, end, _) in mesh.graph.all_edges() {
		gizmos.line_2d(
			(*TRANSFORMATION_MATRIX.get().unwrap() * (start.as_vec3a() + Vec3A::new(0.5, 0.5, 0.))).truncate(),
			(*TRANSFORMATION_MATRIX.get().unwrap() * (end.as_vec3a() + Vec3A::new(0.5, 0.5, 0.))).truncate(),
			Color::BLUE,
		);
	}
}

pub struct NavManagement;

impl Plugin for NavManagement {
	fn build(&self, app: &mut App) {
		app.init_resource::<NavMesh<{ NavCategory::People }>>()
			.init_resource::<NavMesh<{ NavCategory::Vehicles }>>()
			.add_systems(
				FixedUpdate,
				(update_navmesh::<{ NavCategory::People }>, update_navmesh::<{ NavCategory::Vehicles }>),
			)
			.add_systems(Update, visualize_navmesh::<{ NavCategory::Vehicles }>);
	}
}
