//! Spawn the main level and trigger a map to be spawned

use bevy::prelude::*;
use rand::Rng;

use crate::{
    asset_tracking::LoadResource,
    audio::music,
    screens::{
        BuildingMode, EndlessMode, NextStoryLevel, PlayerResources, RequiresCityHall, Screen,
        get_level_data,
    },
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
            music: assets.load("audio/music/spellfire_main_theme.ogg"),
        }
    }
}

/// A system that spawns the main level.
pub fn spawn_level(
    mut commands: Commands,
    endless_mode: Option<Res<EndlessMode>>,
    next_story_level: Res<NextStoryLevel>,
    level_assets: Res<LevelAssets>,
    mut mode: ResMut<BuildingMode>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let endless_mode = endless_mode.is_some();
    commands.init_resource::<PlayerResources>();

    if endless_mode {
        info!("Spawning random level ixn endless mode");

        let seed = rand::thread_rng().r#gen();
        commands.trigger(OnSpawnMap::new(seed));

        *mode = BuildingMode::PlaceCityHall;
        commands.init_resource::<RequiresCityHall>();
    } else {
        let Some(level_data) = get_level_data(next_story_level.0) else {
            warn!("No level exists, aborting");
            next_screen.set(Screen::Title);
            return;
        };

        commands.trigger(OnSpawnMap::new(level_data.map_seed));
        commands.queue(level_data.clone());
        commands.insert_resource(level_data);
        commands.remove_resource::<RequiresCityHall>();
    }

    commands.spawn((
        Name::new("Soundtrack"),
        StateScoped(Screen::Gameplay),
        children![
            Name::new("Gameplay music"),
            music(level_assets.music.clone())
        ],
    ));
}

fn despawn_maps(mut commands: Commands, maps: Query<Entity, With<SpawnedMap>>) {
    for entity in &maps {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<GameMap>();
    commands.remove_resource::<PlayerResources>();
}
