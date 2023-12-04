//! Navigation and navmesh information.

use std::cmp::Ordering;
use std::f32::consts::PI;
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
	None,
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
				(_, Self::None) => Some(Ordering::Greater),
				(Self::People, _) => Some(Ordering::Less),
				(Self::Vehicles, Self::People) => Some(Ordering::Greater),
				_ => None,
			}
		}
	}
}

/// A navigable vertex on the ground. The entities with these components make up the nav meshes in the world.
#[derive(Component, Clone, Copy, Debug)]
pub struct NavComponent {
	/// Which directions this vertex has exits in.
	pub exits:        Sides,
	/// What speed this vertex can be traversed at. In the navmesh graph this is used for traversing from this vertex
	/// to the next.
	pub speed:        f32,
	/// This determines the *base* navigability of the mesh component. As per the category's subset relationship, this
	/// vertex may be part of other navmeshes too.
	pub navigability: NavCategory,
}

#[derive(Clone, Copy, Debug)]
pub struct NavVertex {
	pub position: GridPosition,
	pub speed:    f32,
}

impl PartialEq for NavVertex {
	fn eq(&self, other: &Self) -> bool {
		self.position == other.position
	}
}
impl Eq for NavVertex {}
impl std::hash::Hash for NavVertex {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.position.hash(state);
	}
}
impl PartialOrd for NavVertex {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.position.partial_cmp(&other.position)
	}
}
impl Ord for NavVertex {
	fn cmp(&self, other: &Self) -> Ordering {
		self.position.cmp(&other.position)
	}
}

impl From<(GridPosition, f32)> for NavVertex {
	fn from(value: (GridPosition, f32)) -> Self {
		Self { position: value.0, speed: value.1 }
	}
}

/// A navigation mesh. This is not really a mesh, but it serves the same function as a 3D navmesh. Mathematically
/// speaking, the navmesh is a directed weighted graph.
#[derive(Resource, Debug, Default)]
pub struct NavMesh<const N: NavCategory> {
	/// Internal graph for the nav mesh.
	graph: DiGraphMap<NavVertex, ()>,
}

impl<const N: NavCategory> NavMesh<N> {
	fn update_vertex_impl(&mut self, position: &GridPosition, vertex: NavComponent) {
		let belongs_in_mesh = N <= vertex.navigability;
		// Vertex is being added to the mesh or modified within it.
		if belongs_in_mesh {
			self.graph.remove_node((*position, vertex.speed).into());
			self.graph.add_node((*position, vertex.speed).into());
			for neighbor in position.neighbors_for(vertex.exits) {
				if self.graph.contains_node((neighbor, 0.).into()) {
					self.graph.add_edge((*position, vertex.speed).into(), (neighbor, vertex.speed).into(), ());
					// TODO: We don’t really know whether the neighbor actually has a connection in this direction.
					self.graph.add_edge((neighbor, vertex.speed).into(), (*position, vertex.speed).into(), ());
				}
			}
		} else {
			// Vertex is being removed from the mesh.
			self.graph.remove_node((*position, 0.).into());
		}
	}

	pub fn update_vertices<'a>(&mut self, vertices: impl IntoIterator<Item = (&'a GridPosition, &'a NavComponent)>) {
		for (position, vertex) in vertices {
			self.update_vertex_impl(position, *vertex);
		}
	}
}

fn update_navmesh<const N: NavCategory>(
	mut mesh: ResMut<NavMesh<N>>,
	changed_navigables: Query<(&GridPosition, &NavComponent), Changed<NavComponent>>,
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

	let positive_angle = Vec2::from_angle(PI / 12.);
	let negative_angle = Vec2::from_angle(-PI / 12.);

	for (start_node, end_node, _) in mesh.graph.all_edges() {
		let start = (*TRANSFORMATION_MATRIX.get().unwrap()
			* (start_node.position.as_vec3a() + Vec3A::new(0.5, 0.5, 0.)))
		.truncate();
		let end = (*TRANSFORMATION_MATRIX.get().unwrap() * (end_node.position.as_vec3a() + Vec3A::new(0.5, 0.5, 0.)))
			.truncate();
		let dir = end - start;
		let tip1 = start + positive_angle.rotate(dir) * 0.7;
		let tip2 = start + negative_angle.rotate(dir) * 0.7;

		gizmos.linestrip_2d([start, start + dir * 0.9, tip1, start + dir * 0.9, tip2], Color::BLUE * start_node.speed);
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
