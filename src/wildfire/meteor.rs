//! A plugin that adds meteor in a given grid location. Can be triggered
//! on click other other user input or randomly by triggering [OnMeteorStrike]

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};
use rand::Rng;

use crate::{
    Pause,
    asset_tracking::LoadResource,
    audio::sound_effect,
    screens::Screen,
    wildfire::{TerrainType, map::GameMap},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<OnMeteorStrike>();
    app.register_type::<Meteor>();
    app.register_type::<Fireball>();

    app.register_type::<MeteorAssets>();
    app.load_resource::<MeteorAssets>();

    app.add_observer(handle_meteor_strike);
    app.add_systems(
        Update,
        (handle_meteor_impacts, handle_fireball_impacts)
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
}

#[derive(Debug, Clone, Event, Reflect)]
pub struct OnMeteorStrike(pub IVec2);

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct Meteor {
    target_coord: IVec2,
    target_world_pos: Vec2,
    speed: f32,
}

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct Fireball {
    pub target_world_pos: Vec2,
}

fn handle_meteor_strike(
    trigger: Trigger<OnMeteorStrike>,
    mut commands: Commands,
    meteor_assets: Res<MeteorAssets>,
    map: ResMut<GameMap>,
) {
    let loc = trigger.event().0;
    let Some(cell) = map.get(loc) else {
        info!("Unable to find cell for meteor strike at {loc:?}");
        return;
    };

    match cell.terrain {
        TerrainType::Grassland | TerrainType::Tree => {
            info!("Spawning meteor strike at {loc}");

            commands.spawn(sound_effect(meteor_assets.boom_one.clone()));

            let world_pos = map.world_coords(loc);
            commands.spawn((
                Transform::from_xyz(-1000.0, 100.0, 1.0),
                Meteor {
                    target_coord: loc,
                    target_world_pos: world_pos,
                    speed: (Vec2::new(-1000.0, 100.0) - world_pos).length() / METEOR_FLIGHT_TIME,
                },
                Sprite {
                    image: meteor_assets.meteor.clone(),
                    ..default()
                },
            ));
        }
        TerrainType::Building
        | TerrainType::Fire
        | TerrainType::Dirt
        | TerrainType::Stone
        | TerrainType::Smoldering => {}
    }
}

/// Timed to the audio clip :D
const METEOR_FLIGHT_TIME: f32 = 0.9;

/// RANDOM I GUESS
const FIREBALL_SPEED: f32 = 70.0;
fn handle_fireball_impacts(
    mut commands: Commands,
    time: Res<Time>,
    mut fireballs: Query<(Entity, &mut Transform, &Fireball)>,
) {
    for (entity, mut tx, fireball) in &mut fireballs {
        let delta = (fireball.target_world_pos - tx.translation.truncate()).normalize_or_zero()
            * time.delta_secs()
            * FIREBALL_SPEED;
        tx.translation += delta.extend(0.0);

        if (fireball.target_world_pos - tx.translation.truncate()).length_squared() < 100.0 {
            // we hit
            commands.entity(entity).despawn();
        }
    }
}

fn handle_meteor_impacts(
    mut commands: Commands,
    time: Res<Time>,
    mut map: ResMut<GameMap>,
    mut meteors: Query<(Entity, &mut Transform, &Meteor)>,
) {
    for (entity, mut tx, meteor) in &mut meteors {
        let delta = (meteor.target_world_pos - tx.translation.truncate()).normalize_or_zero()
            * time.delta_secs()
            * meteor.speed;
        tx.translation += delta.extend(0.0);

        if (meteor.target_world_pos - tx.translation.truncate()).length_squared() < 100.0 {
            // we hit
            commands.entity(entity).despawn();

            // find some random thingos around the impact point and start fires
            let points = map
                .cells_within_range(meteor.target_coord, 5)
                .filter(|cell| {
                    if let Some(c) = map.get(*cell) {
                        matches!(
                            c.terrain,
                            TerrainType::Grassland | TerrainType::Tree | TerrainType::Building
                        )
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>();

            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(2..=4) {
                let idx = rng.gen_range(0..points.len());
                let coords = points[idx];
                if let Some(cell) = map.get_mut(coords) {
                    cell.dirty = true;
                    cell.terrain = TerrainType::Fire;
                }
            }
        }
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct MeteorAssets {
    #[dependency]
    boom_one: Handle<AudioSource>,
    #[dependency]
    meteor: Handle<Image>,
    #[dependency]
    pub fireball: Handle<Image>,
}

impl FromWorld for MeteorAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            boom_one: assets.load("audio/sound_effects/boom_one.ogg"),
            meteor: assets.load_with_settings(
                "images/meteor.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            fireball: assets.load_with_settings(
                "images/fireball.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
        }
    }
}
