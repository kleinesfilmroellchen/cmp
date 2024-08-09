use bevy::prelude::*;

/// Current game state, affects how game runs.
#[derive(States, SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
	#[default]
	MainMenu,
	InGame,
	Paused,
}

pub fn pause_fixed_timer(state: Res<State<GameState>>, mut game_time: ResMut<Time<Virtual>>) {
	if state.get() != &GameState::InGame {
		game_time.pause();
	} else {
		game_time.unpause();
	}
}
