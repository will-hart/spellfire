//! Wind module for generating and updating wind

use bevy::prelude::*;
use rand::Rng;

use crate::Pause;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<WindDirection>();
    app.init_resource::<WindDirection>();

    app.add_systems(Update, wandery_wind.run_if(in_state(Pause(false))));
}

#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct WindDirection(pub Vec2);

impl Default for WindDirection {
    fn default() -> Self {
        Self(Vec2::ONE * 2000.0)
    }
}

fn wandery_wind(mut time_left: Local<f32>, time: Res<Time>, mut wind: ResMut<WindDirection>) {
    if *time_left <= 0.0 {
        *time_left = 1.0;
    } else {
        *time_left -= time.delta_secs();
        return;
    }

    // update the wind
    let mut rng = rand::rng();
    let strength_delta = rng.random_range(-0.1..=0.1);
    let current_strength = wind.0.length() + strength_delta;
    let normed_wind = wind.0.normalize_or(Vec2::ONE);

    wind.0 = Vec2::new(
        normed_wind.x + rng.random_range(-0.05..=0.05),
        normed_wind.y + rng.random_range(-0.05..=0.05),
    ) * current_strength;
}
