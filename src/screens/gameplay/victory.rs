//! Logic for victory or defeat

use bevy::prelude::*;

use crate::screens::{RequiresCityHall, Screen, gameplay::CityHall};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        track_defeat_conditions
            .run_if(in_state(Screen::Gameplay).and(not(resource_exists::<RequiresCityHall>))),
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
