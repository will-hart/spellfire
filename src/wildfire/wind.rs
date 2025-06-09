//! Wind module for generating and updating wind

use bevy::{math::CompassOctant, prelude::*};
use rand::Rng;

use crate::{Pause, screens::EndlessMode};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<WindDirection>();
    app.init_resource::<WindDirection>();

    app.add_systems(
        Update,
        wandery_wind.run_if(in_state(Pause(false)).and(resource_exists::<EndlessMode>)),
    );
}

#[derive(Resource, Debug, Clone, Copy, Reflect)]
#[reflect(Resource)]
pub struct WindDirection {
    angle: f32,
    strength: f32,
    variance: f32,
    target: f32,
}

impl Default for WindDirection {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..360.0);
        let target = rng.gen_range(angle - 45.0..angle + 45.0) % 360.0;

        Self {
            angle,
            strength: 10.0,
            variance: std::f32::consts::FRAC_PI_2.to_degrees(),
            target,
        }
    }
}

impl std::fmt::Display for WindDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if cfg!(debug_assertions) {
            write!(
                f,
                "{:.0}kts, from {} [{:.1}-->{:.1}]",
                self.strength,
                self.compass(),
                self.angle,
                self.target
            )
        } else {
            write!(f, "{:.0}kts, from {}", self.strength / 10.0, self.compass())
        }
    }
}

impl WindDirection {
    pub fn as_vec(&self) -> Vec2 {
        (Quat::from_axis_angle(Vec3::Z, self.angle.to_radians()) * Vec3::X).truncate()
            * self.strength
    }

    /// Gets the current wind direction as a vector
    pub fn get_wind_vec(&self) -> Vec2 {
        (Quat::from_axis_angle(Vec3::Z, self.angle.to_radians()) * Vec3::X * self.strength)
            .truncate()
    }

    fn compass(&self) -> &'static str {
        let vec = Dir2::new(self.get_wind_vec()).unwrap_or(Dir2::NORTH);

        match CompassOctant::from(vec) {
            CompassOctant::North => "N",
            CompassOctant::NorthEast => "NE",
            CompassOctant::East => "E",
            CompassOctant::SouthEast => "SE",
            CompassOctant::South => "S",
            CompassOctant::SouthWest => "SW",
            CompassOctant::West => "W",
            CompassOctant::NorthWest => "NW",
        }
    }

    pub fn r#override(&mut self, angle: f32, strength: f32) {
        self.angle = angle;
        self.strength = strength;
    }
}

/// the speed the wind changes in degrees per second
const WIND_CHANGE_SPEED: f32 = 5.0;
const WIND_STRENGTH_VARIANCE: f32 = 1.0;
const MIN_WIND_SPEED: f32 = 10.0;
const MAX_WIND_SPEED: f32 = 100.0;

fn wandery_wind(time: Res<Time>, mut wind: ResMut<WindDirection>) {
    // find out which rotation direction is faster
    // probably a much neater way to do this but whatever
    let raw_delta = wind.target - wind.angle;
    let positive_delta = if raw_delta < 0.0 {
        raw_delta + 360.0
    } else {
        raw_delta
    };
    let negative_delta = if raw_delta > 0.0 {
        360.0 + raw_delta
    } else {
        raw_delta.abs()
    };

    let direction_sign = if positive_delta > negative_delta {
        -1.0
    } else {
        1.0
    };

    // move the wind towards its target angle
    let wind_delta = time.delta_secs() * WIND_CHANGE_SPEED;
    wind.angle += wind_delta * direction_sign;

    // if it has reached it, find a new target angle within range
    // happy to keep the "definition of delta" smallish because its ok if we
    // wobble around a little
    let mut rng = rand::thread_rng();
    if ((wind.angle % 360.0) - wind.target).abs() < 1.0 {
        wind.angle = wind.target;
        wind.target =
            rng.gen_range((wind.angle - wind.variance)..(wind.angle + wind.variance)) % 360.0;
    }

    // random walk the strength between some limits as defined by the wind equation
    // in wildire/map.rs. See docs/wind_sim.png for a graph of the proposed (black) and
    // revised (red) wind influence equation
    wind.strength = (wind.strength
        + rng.gen_range(-WIND_STRENGTH_VARIANCE..WIND_STRENGTH_VARIANCE))
    .clamp(MIN_WIND_SPEED, MAX_WIND_SPEED);
}
