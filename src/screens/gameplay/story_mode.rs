//! Stuff for having a story mode

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    Pause,
    screens::{Screen, gameplay::building::SpawnCityHall},
    wildfire::{GOOD_SEEDS, GameMap, OnMeteorStrike, WindDirection},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<StoryModeLevel>();
    app.register_type::<NextStoryLevel>();

    app.init_resource::<NextStoryLevel>();

    app.add_systems(
        Update,
        (spawn_story_meteor_bolts, update_story_elapsed_time).run_if(
            in_state(Screen::Gameplay)
                .and(in_state(Pause(false)))
                .and(resource_exists::<StoryModeLevel>),
        ),
    );
}

#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct NextStoryLevel(pub usize);

impl Default for NextStoryLevel {
    fn default() -> Self {
        Self(1)
    }
}

/// Contains the details for a story mode level
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct StoryModeLevel {
    /// The number of this level
    pub level_number: usize,
    /// The seed to use when spwaning the map
    pub map_seed: i32,
    /// The meteor bolts to deploy to start the wildfire
    pub bolts: VecDeque<(f32, IVec2)>,
    /// The starting location for the city hall (in tile coords)
    pub starting_location: IVec2,
    /// The amount of time since this story level was started
    pub elapsed_time: f32,

    /// Store the wind speed and angle, which is constant for story mode
    pub wind_speed: f32,
    pub wind_angle: f32,
}

impl Command for StoryModeLevel {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_story, self);
    }
}

fn spawn_story(
    In(config): In<StoryModeLevel>,
    mut commands: Commands,
    map: Res<GameMap>,
    mut wind: ResMut<WindDirection>,
) {
    info!("Spawning items for level");

    let world_coords = map.world_coords(config.starting_location);
    commands.queue(SpawnCityHall(world_coords));

    wind.r#override(config.wind_angle, config.wind_speed);
}

/// Tick the level elapsed time while unpaused
fn update_story_elapsed_time(time: Res<Time>, mut level: ResMut<StoryModeLevel>) {
    level.elapsed_time += time.delta_secs();
}

/// If a bolt is due, spawn it
fn spawn_story_meteor_bolts(mut commands: Commands, mut level: ResMut<StoryModeLevel>) {
    // max one meteor bolt per frame just because its easier to write about
    if let Some((bolt_time, bolt_loc)) = level.bolts.front() {
        if level.elapsed_time < *bolt_time {
            return;
        }

        commands.trigger(OnMeteorStrike(*bolt_loc));
        let _ = level.bolts.pop_front();
        info!("Level has {} bolts remaining", level.bolts.len());
    }
}

/// Extremely lazy way to create level data :D
pub fn get_level_data(lvl: usize) -> Option<StoryModeLevel> {
    if lvl == 1 {
        Some(StoryModeLevel {
            level_number: lvl,
            map_seed: GOOD_SEEDS[lvl - 1],
            starting_location: IVec2 { x: 168, y: 243 },
            bolts: vec![
                (10.0, IVec2 { x: 21, y: 46 }),
                (30.0, IVec2 { x: 27, y: 175 }),
                (30.2, IVec2 { x: 29, y: 177 }),
                (30.2, IVec2 { x: 25, y: 173 }),
            ]
            .into(),
            wind_speed: 15.0,
            wind_angle: 32.0,
            elapsed_time: 0.0,
        })
    } else if lvl == 2 {
        Some(StoryModeLevel {
            level_number: lvl,
            map_seed: GOOD_SEEDS[lvl - 1],
            starting_location: IVec2 { x: 27, y: 228 },
            bolts: vec![
                (20.0, IVec2 { x: 29, y: 177 }),
                (21.0, IVec2 { x: 175, y: 17 }),
            ]
            .into(),

            wind_speed: 15.0,
            wind_angle: 32.0,
            elapsed_time: 0.0,
        })
    } else {
        None
    }
}
