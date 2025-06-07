//! The game's main screen states and transitions between them.

mod game_over;
mod gameplay;
mod loading;
mod splash;
mod title;
mod victory;

pub use gameplay::{
    BuildingMode, BuildingType, EndlessMode, OnRedrawToolbar, PlayerResources, RequiresCityHall,
    story_mode::{NextStoryLevel, StoryModeLevel, get_level_data},
};

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();

    app.add_plugins((
        gameplay::plugin,
        game_over::plugin,
        loading::plugin,
        splash::plugin,
        title::plugin,
        victory::plugin,
    ));
}

/// The game's main screen states.
#[derive(States, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[states(scoped_entities)]
pub enum Screen {
    #[default]
    Splash,
    Title,
    Loading,
    Gameplay,
    GameOver,
    LevelWon,
}
