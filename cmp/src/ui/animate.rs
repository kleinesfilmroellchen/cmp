//! UI animation system

use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;

use crate::util::physics_ease::MassDamperSystem;
use crate::util::Lerpable;

/// Any property of a component that is animatable.
/// Note that one component may have multiple animated properties.
///
/// This trait defines statically how to access the property on the corresponding component. It is not expected that an
/// implementation of this exists in any such component (or any associated datastructure), though this is the case for
/// very simple components who are [`Lerpable`] themselves.
pub trait AnimatedProperty<C: Component, D: Lerpable> {
	fn set_data(component: &mut C, data: D);
}

impl<C: Lerpable + Component + Clone> AnimatedProperty<C, C> for C {
	fn set_data(component: &mut C, data: C) {
		*component = data;
	}
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StyleHeight;
impl AnimatedProperty<Style, Val> for StyleHeight {
	fn set_data(component: &mut Style, data: Val) {
		component.height = data;
	}
}

/// Defines the three end targets for an animation, in the logical sense.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnimationTargets {
	pub start:        f32,
	pub when_hovered: f32,
	pub when_pressed: f32,
}

impl AnimationTargets {
	/// Animate fully with the hover interaction (and don't change during the press interaction).
	pub const fn at_hover() -> Self {
		Self { start: 0., when_hovered: 1., when_pressed: 1. }
	}

	/// Animate fully only with the press interaction.
	pub const fn at_press() -> Self {
		Self { start: 0., when_hovered: 0., when_pressed: 1. }
	}

	pub const fn logical_position_of(&self, target: Interaction) -> f32 {
		match target {
			Interaction::Pressed => self.when_pressed,
			Interaction::Hovered => self.when_hovered,
			Interaction::None => self.start,
		}
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TransitionTimes {
	pub to_start:   Duration,
	pub to_hovered: Duration,
	pub to_pressed: Duration,
}

impl TransitionTimes {
	/// Use a uniform time for all three transitions.
	#[allow(unused)]
	pub const fn uniform(d: Duration) -> Self {
		Self { to_start: d, to_hovered: d, to_pressed: d }
	}

	/// Specify the hovered duration as a baseline and the other ones as fractions of that. This allows adapting the
	/// animation speed more easily.
	#[allow(unused)]
	pub const fn with_fractions(to_hovered_ms: f32, to_start_fraction: f32, to_pressed_fraction: f32) -> Self {
		Self {
			to_hovered: Duration::from_millis(to_hovered_ms as u64),
			to_start:   Duration::from_millis((to_hovered_ms * to_start_fraction) as u64),
			to_pressed: Duration::from_millis((to_hovered_ms * to_pressed_fraction) as u64),
		}
	}

	pub const fn transition_time_to(&self, interaction: Interaction) -> Duration {
		match interaction {
			Interaction::Pressed => self.to_pressed,
			Interaction::Hovered => self.to_hovered,
			Interaction::None => self.to_start,
		}
	}
}

/// A component that is necessary to animate any [`Animatable`] component on the same entity.
#[derive(Component)]
pub struct UIAnimation<D: Lerpable + Sync + Send + 'static, C: Component, P: AnimatedProperty<C, D>> {
	/// Currently playing animation.
	target:           Interaction,
	/// Stores the target values.
	target_values:    AnimationTargets,
	start_position:   D,
	end_position:     D,
	transition_times: TransitionTimes,
	// Physics-based easing systems.
	system:           MassDamperSystem,
	c_mark:           PhantomData<C>,
	p_mark:           PhantomData<P>,
}

impl<D: Lerpable + Sync + Send + 'static + Clone, C: Component + Clone, P: AnimatedProperty<C, D> + Clone> Clone
	for UIAnimation<D, C, P>
{
	fn clone(&self) -> Self {
		Self {
			target:           self.target.clone(),
			target_values:    self.target_values.clone(),
			start_position:   self.start_position.clone(),
			end_position:     self.end_position.clone(),
			transition_times: self.transition_times.clone(),
			system:           self.system.clone(),
			c_mark:           self.c_mark.clone(),
			p_mark:           self.p_mark.clone(),
		}
	}
}

impl<D: Lerpable + Sync + Send + 'static, C: Component, P: AnimatedProperty<C, D>> UIAnimation<D, C, P> {
	pub fn new(
		start: D,
		end: D,
		targets: AnimationTargets,
		damper_force: f32,
		spring_force: f32,
		transition_times: TransitionTimes,
	) -> Self {
		Self {
			target: Interaction::None,
			target_values: targets,
			start_position: start,
			end_position: end,
			system: MassDamperSystem::new(damper_force, spring_force, 1.),
			transition_times,
			c_mark: PhantomData,
			p_mark: PhantomData,
		}
	}

	// color_system:    MassDamperSystem::new(4., 4., 1.),

	/// Starts an animation that transitions to the specific interaction target.
	pub fn start_transition_to(&mut self, target: Interaction) {
		self.target = target;
		self.system.set_target(self.target_values.logical_position_of(target));
	}

	/// Runs the regular update of the animation.
	pub fn update(&mut self, time: &Time, component: &mut C) {
		let normalized_delta =
			time.delta().as_secs_f32() / self.transition_times.transition_time_to(self.target).as_secs_f32();
		self.system.simulate(normalized_delta);

		let current_value = self.start_position.lerp(&self.end_position, self.system.position());
		P::set_data(component, current_value);
	}

	// pub fn update(&mut self, time: &Time, color: &mut BackgroundColor, style: &mut Style) {
	// 	let normalized_delta = time.delta().as_secs_f32() / Self::transition_time_to(self.target).as_secs_f32();
	// 	self.height_system.simulate(normalized_delta);
	// 	self.color_system.simulate(normalized_delta);

	// 	let target_color = {
	// 		let [hue, saturation, mut lightness, alpha] = self.original_color.as_hsla_f32();
	// 		lightness = (lightness - 0.3).clamp(0., 1.);
	// 		Color::hsla(hue, saturation, lightness, alpha)
	// 	};
	// 	let target_height = self.original_height + 20.;

	// 	let current_color = self.original_color.lerp(&target_color, self.color_system.position());
	// 	let current_height = self.original_height.lerp(&target_height, self.height_system.position()).round_ties_even();
	// 	*color = BackgroundColor(current_color);
	// 	style.height = Val::Px(current_height);
	// }
}

pub fn transition_animation<
	D: Lerpable + Send + Sync + 'static,
	C: Component,
	P: AnimatedProperty<C, D> + Send + Sync + 'static,
>(
	mut button: Query<(&Interaction, &mut UIAnimation<D, C, P>), Changed<Interaction>>,
) {
	for (interaction, mut animations) in &mut button {
		animations.start_transition_to(*interaction);
	}
}

pub fn update_animation<
	D: Lerpable + Send + Sync + 'static,
	C: Component,
	P: AnimatedProperty<C, D> + Send + Sync + 'static,
>(
	time: Res<Time>,
	mut buttons: Query<(&mut UIAnimation<D, C, P>, &mut C)>,
) {
	for (mut animations, mut component) in &mut buttons {
		animations.update(&time, &mut component);
	}
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				transition_animation::<Val, Style, StyleHeight>,
				transition_animation::<BackgroundColor, BackgroundColor, BackgroundColor>,
			),
		)
		.add_systems(
			Update,
			(
				update_animation::<Val, Style, StyleHeight>,
				update_animation::<BackgroundColor, BackgroundColor, BackgroundColor>,
			),
		);
	}
}
