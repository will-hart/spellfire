//! Stuff for having a story mode

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::{
    Pause,
    screens::{Screen, gameplay::building::SpawnCityHall},
    wildfire::{GOOD_SEEDS, GameMap, OnLightningStrike},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<StoryModeLevel>();

    app.add_systems(
        Update,
        (spawn_story_lightning_bolts, update_story_elapsed_time).run_if(
            in_state(Screen::Gameplay)
                .and(in_state(Pause(false)))
                .and(resource_exists::<StoryModeLevel>),
        ),
    );
}

/// Contains the details for a story mode level
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct StoryModeLevel {
    /// The seed to use when spwaning the map
    pub map_seed: i32,
    /// The lightning bolts to deploy to start the wildfire
    pub bolts: VecDeque<(f32, IVec2)>,
    /// The starting location for the city hall (in tile coords)
    pub starting_location: IVec2,
    /// The amount of time since this story level was started
    pub elapsed_time: f32,
}

impl Command for StoryModeLevel {
    fn apply(self, world: &mut World) -> () {
        let _ = world.run_system_cached_with(spawn_story, self);
    }
}

fn spawn_story(In(config): In<StoryModeLevel>, mut commands: Commands, map: Res<GameMap>) {
    info!("Spawning items for level");

    let world_coords = map.world_coords(config.starting_location);
    commands.queue(SpawnCityHall(world_coords));
}

/// Tick the level elapsed time while unpaused
fn update_story_elapsed_time(time: Res<Time>, mut level: ResMut<StoryModeLevel>) {
    level.elapsed_time += time.delta_secs();
}

/// If a bolt is due, spawn it
fn spawn_story_lightning_bolts(mut commands: Commands, mut level: ResMut<StoryModeLevel>) {
    // max one lightning bolt per frame just because its easier to write about
    if let Some((bolt_time, bolt_loc)) = level.bolts.front() {
        if level.elapsed_time < *bolt_time {
            return;
        }

        commands.trigger(OnLightningStrike(*bolt_loc));
        let _ = level.bolts.pop_front();
    }
}

/// Extremely lazy way to create level data :D
pub fn get_level_data(lvl: usize) -> Option<StoryModeLevel> {
    if lvl == 1 {
        Some(StoryModeLevel {
            map_seed: GOOD_SEEDS[0],
            starting_location: IVec2 { x: 168, y: 243 },
            bolts: vec![(10.0, IVec2 { x: 53, y: 15 })].into(),
            elapsed_time: 0.0,
        })
    } else {
        None
    }
}
