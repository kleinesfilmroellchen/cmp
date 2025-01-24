//! A mass-spring-damper dynamic system, providing a physically-based easing function.

use bevy::prelude::*;

/// A physics simulation of a mass-spring-damper dynamic system, useful for simulating dampened motion. (Warning:
/// Physics explanation for the implementation ahead, including differential equations and linear algebra!)
///
/// The system consists of:
/// - a mass m at position x trying to reach the target position w
/// - a spring with spring force `F_S = k_P (w − x)`
/// - a damper with dampening force `F_D = −k_D ẋ`
///
/// A displacement force is omitted, thereby the system is described by the differential equation:
///
/// `m ẍ = k_P (w − x) − k_D ẋ`
///
/// which we can transform into a standard inhomogenous differential equation:
///
/// `ẍ + k_D/m ẋ + k_P/m x = k_P/m w`
///
/// Using the state space vector `x̄ = [x, ẋ]ᵀ` and the control quantity `u = w` we obtain the system in state space
/// via standard transformation procedure:
///
/// ```math
/// [ ẋ ]   [   0       1     ] [ x ]   [ 0 ]
/// [ ẍ ] = [ -k_P/m  -k_D/m  ] [ ẋ ] + [ 1 ] u
/// ```
///
/// and `y = k_P/m x` (theory says `y = cᵀ x + d u` with `cᵀ = [k_P/m, 0]` and `d = 0` but no need for vector
/// math here)
///
/// We then use the resulting derivation of the state space vector to perform Euler integration (`x̄ += dt x̄̇`) with some
/// small time step. Since in practice the simulation is run frame rate bound, this could lead to incorrect simulation
/// due to large time steps, so we split the time step up into sufficiently small steps (<1/100 s).
#[derive(Clone, Copy, Debug, Component, Reflect)]
#[reflect(Component)]
pub struct MassDamperSystem {
	/// State space vector `x̄ = [x, ẋ]ᵀ`, consisting of position and velocity.
	state:            Vec2,
	/// k_D; how quickly the system slows down while it approaches the target position.
	pub damper_force: f32,
	/// k_P; how quickly the system moves towards the target position.
	pub spring_force: f32,
	/// m; a scaling factor for the system's speed.
	pub mass:         f32,
	/// w; Target position.
	target:           f32,
}

impl Default for MassDamperSystem {
	fn default() -> Self {
		Self { state: (0., 0.).into(), damper_force: 1., spring_force: 1., mass: 1., target: 0. }
	}
}

impl MassDamperSystem {
	const MAX_DT: f32 = 1. / 100.;

	/// Creates a new system with the given damper and spring forces and mass.
	pub fn new(damper_force: f32, spring_force: f32, mass: f32) -> Self {
		Self { damper_force, spring_force, mass, ..default() }
	}

	/// Returns the current position of the system, which is the output variable.
	pub fn position(&self) -> f32 {
		self.c().dot(self.state)
	}

	/// Sets the system's target position w.
	pub fn set_target(&mut self, target: f32) {
		self.target = target;
	}

	/// Simulate the system for the given time step.
	pub fn simulate(&mut self, dt: f32) {
		// Maximum dt to use
		let used_dt = Self::MAX_DT.min(dt);
		let mut simulated_time = 0.;
		// make sure to not run into float imprecision infinite loops
		while (simulated_time - dt).abs() > 0.0001 {
			// Either run a step with used_dt, or until the end of dt.
			let step_dt = used_dt.min(dt - simulated_time);
			self.simulate_single_step(step_dt);
			simulated_time += step_dt;
		}
	}

	/// Returns the state derivation transfer matrix A.
	pub const fn a(&self) -> Mat2 {
		Mat2::from_cols_array_2d(&[[0., -self.spring_force / self.mass], [1., -self.damper_force / self.mass]])
	}

	/// Returns the input transfer vector b.
	pub const fn b(&self) -> Vec2 {
		Vec2::from_array([0., 1.])
	}

	/// Returns the output transfer vector c.
	pub const fn c(&self) -> Vec2 {
		Vec2::from_array([self.spring_force / self.mass, 0.])
	}

	/// dt < 1/100s needs to hold or else simulation will be inaccurate!
	fn simulate_single_step(&mut self, dt: f32) {
		// x̄̇
		let state_d = self.a() * self.state + self.b() * self.target;
		self.state += state_d * dt;
	}
}

mod test {
	#[bench]
	fn bench_mass_spring_damper_system(bench: &mut test::Bencher) {
		let mut system = super::MassDamperSystem::new(1.4, 2.33, 0.7);
		bench.iter(|| {
			system.simulate(60.);
			test::black_box(());
		})
	}
}
