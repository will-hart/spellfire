//! Spawn the main level and trigger a map to be spawned

use bevy::prelude::*;

use crate::{
    asset_tracking::LoadResource,
    audio::music,
    demo::player::{PlayerAssets, player},
    screens::Screen,
    wildfire::{OnSpawnMap, SpawnedMap},
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
    level_assets: Res<LevelAssets>,
    player_assets: Res<PlayerAssets>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn((
        Name::new("Level"),
        Transform::default(),
        Visibility::default(),
        StateScoped(Screen::Gameplay),
        children![
            player(400.0, &player_assets, &mut texture_atlas_layouts),
            (
                Name::new("Gameplay Music"),
                music(level_assets.music.clone())
            )
        ],
    ));

    commands.trigger(OnSpawnMap {
        size: UVec2::splat(256),
        sprite_size: 8.0,
    });
}

fn despawn_maps(mut commands: Commands, maps: Query<Entity, With<SpawnedMap>>) {
    for entity in &maps {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<OnSpawnMap>();
}
