//! Spawn the main level and trigger a map to be spawned

use bevy::prelude::*;

use crate::{
    asset_tracking::LoadResource,
    screens::{PlayerResources, Screen},
    wildfire::{GameMap, OnSpawnMap, SpawnedMap},
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
pub fn spawn_level(mut commands: Commands) {
    commands.trigger(OnSpawnMap {
        size: UVec2::splat(256),
        sprite_size: 4.0,
    });

    commands.init_resource::<PlayerResources>();
}

fn despawn_maps(mut commands: Commands, maps: Query<Entity, With<SpawnedMap>>) {
    for entity in &maps {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<GameMap>();
    commands.remove_resource::<PlayerResources>();
}
