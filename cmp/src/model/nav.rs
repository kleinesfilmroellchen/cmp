//! Navigation and navmesh information.

use std::cmp::Ordering;
use std::collections::{BTreeSet, VecDeque};
use std::f32::consts::PI;
use std::marker::ConstParamTy;

use bevy::color::palettes::css::{BLUE, RED};
use bevy::math::Vec3A;
use bevy::prelude::*;
use bevy::utils::Instant;
use petgraph::graphmap::DiGraphMap;

use super::{GridPosition, WorldPosition};
use crate::config::GameSettings;
use crate::gamemode::GameState;
use crate::graphics::{engine_to_world_space, Sides, TRANSFORMATION_MATRIX};
use crate::input::MouseClick;

/// The kinds of navigability, used by different groups of actors.
/// Each kind has its own nav mesh.
/// Note that many nav categories are subcategories of others, practically speaking. This is expressed with the
/// category ordering; see [`PartialOrd`].
#[derive(Reflect, Clone, Copy, Debug, Default, Eq, PartialEq, ConstParamTy)]
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
#[derive(Component, Reflect, Clone, Copy, Debug)]
#[reflect(Component)]
pub struct NavComponent {
	/// Which directions this vertex has exits in.
	pub exits:        Sides,
	/// What speed this vertex can be traversed at. In the navmesh graph this is used for traversing from this vertex
	/// to the next.
	pub speed:        u32,
	/// This determines the *base* navigability of the mesh component. As per the category's subset relationship, this
	/// vertex may be part of other navmeshes too.
	pub navigability: NavCategory,
}

#[derive(Clone, Copy, Debug)]
pub struct NavVertex {
	pub position: GridPosition,
	pub speed:    u32,
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
		Some(self.cmp(other))
	}
}
impl Ord for NavVertex {
	fn cmp(&self, other: &Self) -> Ordering {
		self.position.cmp(&other.position)
	}
}

impl From<(GridPosition, u32)> for NavVertex {
	fn from(value: (GridPosition, u32)) -> Self {
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

#[derive(Debug, Default)]
pub struct Path {
	segments: VecDeque<GridPosition>,
}

impl Path {
	pub fn start(&self) -> Option<&GridPosition> {
		self.segments.front()
	}

	pub fn end(&self) -> Option<&GridPosition> {
		self.segments.back()
	}
}

impl<const N: NavCategory> NavMesh<N> {
	fn update_vertex_impl(&mut self, position: &GridPosition, vertex: NavComponent) {
		let belongs_in_mesh = N <= vertex.navigability;
		// Vertex is being added to the mesh or modified within it.
		if belongs_in_mesh {
			self.graph.remove_node((*position, vertex.speed).into());
			self.graph.add_node((*position, vertex.speed).into());
			for neighbor in position.neighbors_for(vertex.exits) {
				if self.graph.contains_node((neighbor, 0).into()) {
					self.graph.add_edge((*position, vertex.speed).into(), (neighbor, vertex.speed).into(), ());
					// TODO: We don’t really know whether the neighbor actually has a connection in this direction.
					self.graph.add_edge((neighbor, vertex.speed).into(), (*position, vertex.speed).into(), ());
				}
			}
		} else {
			// Vertex is being removed from the mesh.
			self.graph.remove_node((*position, 0).into());
		}
	}

	pub fn update_vertices<'a>(&mut self, vertices: impl IntoIterator<Item = (&'a GridPosition, &'a NavComponent)>) {
		for (position, vertex) in vertices {
			self.update_vertex_impl(position, *vertex);
		}
	}

	/// Pathfind via A* from start to end.
	pub fn pathfind(&self, start: GridPosition, end: GridPosition) -> Option<Path> {
		/// Manhattan distance between X and Y components of the grid position.
		fn heuristic(from: GridPosition, to: GridPosition) -> u32 {
			from.x.abs_diff(to.x) + from.y.abs_diff(to.y)
		}

		#[derive(Clone, Copy, Debug, Default)]
		struct OpenSetEntry {
			position:    GridPosition,
			// Total cost; used for ordering entries.
			cost:        u32,
			// Cost to this node.
			g:           u32,
			predecessor: GridPosition,
		}
		impl Eq for OpenSetEntry {}
		impl PartialEq for OpenSetEntry {
			fn eq(&self, other: &Self) -> bool {
				self.position == other.position
			}
		}
		impl std::hash::Hash for OpenSetEntry {
			fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
				self.position.hash(state);
			}
		}
		impl PartialOrd for OpenSetEntry {
			fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
				Some(self.cmp(other))
			}
		}
		impl Ord for OpenSetEntry {
			fn cmp(&self, other: &Self) -> Ordering {
				match self.position.cmp(&other.position) {
					Ordering::Equal => Ordering::Equal,
					position_ord => match self.cost.cmp(&other.cost) {
						// never equal because of above comparison
						Ordering::Equal => position_ord,
						cost_ord => cost_ord,
					},
				}
			}
		}
		impl From<GridPosition> for OpenSetEntry {
			fn from(value: GridPosition) -> Self {
				Self { position: value, cost: 0, g: 0, predecessor: value }
			}
		}

		let mut open_set = BTreeSet::new();
		let mut closed_set: bevy::utils::HashSet<OpenSetEntry> = bevy::utils::HashSet::new();

		open_set.insert(OpenSetEntry { position: start, cost: 0, g: 0, predecessor: start });
		while let Some(current @ OpenSetEntry { position: current_position, g: current_g, .. }) = open_set.pop_first() {
			closed_set.insert(current);
			if current_position == end {
				let mut backtrack = end;
				let mut segments = VecDeque::new();
				while let Some(backtrack_entry) = closed_set.get(&OpenSetEntry::from(backtrack)) {
					segments.push_front(backtrack_entry.position);
					if backtrack_entry.position == start {
						break;
					}
					backtrack = backtrack_entry.predecessor;
				}
				return Some(Path { segments });
			}

			for neighbor in self
				.graph
				.neighbors((current_position, 0).into())
				.filter(|neighbor| !closed_set.contains(&OpenSetEntry::from(neighbor.position)))
			{
				let edge_cost = neighbor.speed;
				let g = current_g + edge_cost;
				if let Some(neighbor_in_set) = open_set.get(&neighbor.position.into())
					&& g >= neighbor_in_set.g
				{
					continue;
				}
				let cost = g + heuristic(neighbor.position, end);
				open_set.replace(OpenSetEntry { position: neighbor.position, cost, g, predecessor: current_position });
			}
		}

		None
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
			* (start_node.position.position() + Vec3A::new(0.5, 0.5, 0.)))
		.truncate();
		let end = (*TRANSFORMATION_MATRIX.get().unwrap() * (end_node.position.position() + Vec3A::new(0.5, 0.5, 0.)))
			.truncate();
		let dir = end - start;
		let tip1 = start + positive_angle.rotate(dir) * 0.7;
		let tip2 = start + negative_angle.rotate(dir) * 0.7;

		gizmos
			.linestrip_2d([start, start + dir * 0.9, tip1, start + dir * 0.9, tip2], BLUE * (start_node.speed as f32));
	}
}

fn debug_pathfinding<const N: NavCategory>(
	mesh: Res<NavMesh<N>>,
	mut gizmos: Gizmos,
	settings: Res<GameSettings>,
	mut path: Local<Path>,
	mut clicks: EventReader<MouseClick>,
) {
	if !settings.show_debug {
		return;
	}

	for click in clicks.read() {
		let new_end = (engine_to_world_space(click.engine_position, 0.) - Vec3A::new(0.5, 0.5, 0.)).round();
		let new_start = path.end();
		if let Some(new_start) = new_start {
			let start_time = Instant::now();
			if let Some(new_path) = mesh.pathfind(*new_start, new_end) {
				*path = new_path;
			} else {
				path.segments = VecDeque::from_iter(Some(new_end).into_iter());
			}
			debug!("Pathfind took {:?}", Instant::now() - start_time);
		} else {
			path.segments = VecDeque::from_iter(Some(new_end).into_iter());
		}
	}

	let mut last_position = path.start().cloned();
	for position in &path.segments {
		if let Some(last_position) = last_position {
			gizmos.line_2d(
				(*TRANSFORMATION_MATRIX.get().unwrap() * (last_position.position() + Vec3A::new(0.4, 0.4, 0.)))
					.truncate(),
				(*TRANSFORMATION_MATRIX.get().unwrap() * (position.position() + Vec3A::new(0.4, 0.4, 0.))).truncate(),
				RED,
			);
		}
		last_position = Some(*position);
	}
}

pub struct NavManagement;

impl Plugin for NavManagement {
	fn build(&self, app: &mut App) {
		app.init_resource::<NavMesh<{ NavCategory::People }>>()
			.init_resource::<NavMesh<{ NavCategory::Vehicles }>>()
			.register_type::<NavComponent>()
			.register_type::<NavCategory>()
			.add_systems(
				FixedUpdate,
				(update_navmesh::<{ NavCategory::People }>, update_navmesh::<{ NavCategory::Vehicles }>).run_if(in_state(GameState::InGame)),
			)
			.add_systems(
				Update,
				(visualize_navmesh::<{ NavCategory::Vehicles }>, debug_pathfinding::<{ NavCategory::Vehicles }>).run_if(in_state(GameState::InGame)),
			);
	}
}
