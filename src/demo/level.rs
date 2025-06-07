//! Spawn the main level and trigger a map to be spawned

use bevy::prelude::*;
use rand::Rng;

use crate::{
    asset_tracking::LoadResource,
    screens::{BuildingMode, EndlessMode, PlayerResources, RequiresCityHall, Screen},
    wildfire::{GOOD_SEEDS, GameMap, OnSpawnMap, SpawnedMap},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<LevelAssets>();
    app.load_resource::<LevelAssets>();

    app.add_systems(OnExit(Screen::Gameplay), despawn_maps);
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency]
    music: Handle<AudioSource>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/caves-of-dawn-10376.ogg"),
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(
    mut commands: Commands,
    endless_mode: Option<Res<EndlessMode>>,
    mut mode: ResMut<BuildingMode>,
) {
    let endless_mode = endless_mode.is_some();

    let seed = if endless_mode {
        info!("Spawning random level in endless mode");
        rand::thread_rng().r#gen()
    } else {
        info!("Spawning first level");
        GOOD_SEEDS[0]
    };

    if endless_mode {
        *mode = BuildingMode::PlaceCityHall;
        commands.init_resource::<RequiresCityHall>();
    } else {
        commands.remove_resource::<RequiresCityHall>();
        // TODO: spawn level for story mode
    }

    commands.trigger(OnSpawnMap::new(seed));
    commands.init_resource::<PlayerResources>();
}

fn despawn_maps(mut commands: Commands, maps: Query<Entity, With<SpawnedMap>>) {
    for entity in &maps {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<GameMap>();
    commands.remove_resource::<PlayerResources>();
}
