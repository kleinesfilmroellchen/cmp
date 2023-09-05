use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::config::GameSettings;
use crate::graphics::library::{font_for, FontStyle, FontWeight};

// Account for up to 600fps and the 10 second metrics.
const FRAME_TIMES_COUNT: usize = 600 * 11;
/// Marker component for the text that’s responsible for performance statistics display.
#[derive(Component, Default)]
pub struct StatUI {
	last_frame_times: VecDeque<Duration>,
}

impl StatUI {
	fn average(&self, average_time: Duration) -> Duration {
		let (total, count) = self
			.last_frame_times
			.iter()
			.scan((Duration::ZERO, 0), |(total, count), time| {
				*total += *time;
				*count += 1;
				if *total > average_time {
					None
				} else {
					Some((*total, *count))
				}
			})
			.last()
			.unwrap_or((Duration::ZERO, 0));
		total / count.max(1)
	}

	fn percentile(&self, average_time: Duration, percentile: f32) -> Duration {
		let mut values = self
			.last_frame_times
			.iter()
			.scan((Duration::ZERO, Duration::ZERO), |(total, _), new| {
				*total += *new;
				if *total > average_time {
					None
				} else {
					Some((*total, *new))
				}
			})
			.map(|(_, value)| value)
			.collect::<Vec<_>>();
		if values.is_empty() {
			return Duration::ZERO;
		}

		values.sort();
		let index = (percentile * values.len() as f32).floor() as usize;
		values[index]
	}

	fn worst(&self) -> Duration {
		*self.last_frame_times.iter().max().unwrap_or(&Duration::ZERO)
	}
}

pub fn create_stats(mut commands: Commands) {
	commands
		.spawn(NodeBundle {
			style: Style {
				width: Val::Percent(100.),
				height: Val::Percent(100.),
				display: Display::Flex,
				position_type: PositionType::Absolute,
				..default()
			},
			// Debug stats should always appear on top.
			z_index: ZIndex::Global(1000),
			..default()
		})
		.with_children(|parent| {
			parent.spawn((
				TextBundle::from_section("FPS Data", TextStyle::default())
					.with_style(Style { margin: UiRect::all(Val::Px(5.0)), ..default() })
					.with_text_alignment(TextAlignment::Left),
				StatUI::default(),
			));
		});
}

pub fn print_stats(
	time: Res<Time>,
	settings: Res<GameSettings>,
	mut stat_ui: Query<(&mut Text, &mut StatUI)>,
	asset_server: Res<AssetServer>,
) {
	let (mut ui, mut stats) = stat_ui.single_mut();

	stats.last_frame_times.push_front(time.delta());
	if stats.last_frame_times.len() > FRAME_TIMES_COUNT {
		stats.last_frame_times.pop_back();
	}

	let last_second_avg = stats.average(Duration::SECOND);
	let last_second_95p = stats.percentile(Duration::SECOND, 0.95);
	let last_10s_avg = stats.average(Duration::SECOND * 10);
	let last_10s_95p = stats.percentile(Duration::SECOND * 10, 0.95);
	let worst = stats.worst();

	if settings.show_fps {
		*ui = Text::from_section(
			format!(
				"Current: {:4.1} fps, {:6.2}ms\nLast second: {:4.1} fps, {:6.2}ms\nLast second (95%): {:4.1} fps, \
				 {:6.2}ms\n10s: {:4.1} fps, {:6.2}ms\n10s (95%): {:4.1} fps, {:6.2}ms\nWorst frame: {:4.1} fps, \
				 {:6.2}ms",
				1. / time.delta_seconds_f64(),
				time.delta_seconds_f64() * 1000.,
				1. / last_second_avg.as_secs_f64(),
				last_second_avg.as_secs_f64() * 1000.,
				1. / last_second_95p.as_secs_f64(),
				last_second_95p.as_secs_f64() * 1000.,
				1. / last_10s_avg.as_secs_f64(),
				last_10s_avg.as_secs_f64() * 1000.,
				1. / last_10s_95p.as_secs_f64(),
				last_10s_95p.as_secs_f64() * 1000.,
				1. / worst.as_secs_f64(),
				worst.as_secs_f64() * 1000.,
			),
			TextStyle {
				font:      asset_server.load(font_for(FontWeight::Regular, FontStyle::Regular)),
				font_size: 15.0,
				color:     Color::WHITE,
			},
		);
	} else {
		ui.sections.clear();
	}
}
