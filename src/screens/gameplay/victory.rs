//! Logic for victory or defeat

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};

use crate::{
    screens::{RequiresCityHall, Screen, StoryModeLevel, gameplay::CityHall},
    wildfire::GameMap,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        track_defeat_conditions
            .run_if(in_state(Screen::Gameplay).and(not(resource_exists::<RequiresCityHall>))),
    );

    app.add_systems(
        Update,
        track_victory_conditions.run_if(
            in_state(Screen::Gameplay)
                .and(resource_exists::<StoryModeLevel>)
                .and(on_timer(Duration::from_millis(500))),
        ),
    );
}

/// If the resource exists and the city hall was removed, go to the defeated
/// state
fn track_defeat_conditions(
    mut next_state: ResMut<NextState<Screen>>,
    halls: Query<Entity, With<CityHall>>,
) {
    if halls.is_empty() {
        next_state.set(Screen::GameOver);
    }
}

/// In story mode, victory is when there is no more fire and the last lightning
/// bolt has been launched
fn track_victory_conditions(map: Res<GameMap>, level: Res<StoryModeLevel>) {
    if level.bolts.is_empty() && !map.any_on_fire() {
        error!("VICTORY");
    }
}
