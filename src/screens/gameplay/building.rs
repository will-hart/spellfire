//! Logic + code for placing buildings

use std::time::Duration;

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    sprite::Anchor,
    time::common_conditions::on_timer,
};
use rand::Rng;

use crate::{
    Pause,
    asset_tracking::LoadResource,
    screens::{PlayerResources, Screen},
    wildfire::{GameMap, OnLightningStrike},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingLocation>();
    app.register_type::<ManaForge>();
    app.register_type::<BuildingAssets>();

    app.load_resource::<BuildingAssets>();

    app.add_systems(
        Update,
        (
            produce_from_mana_forge,
            blow_up_mana_forge.run_if(on_timer(Duration::from_millis(100))),
        )
            .run_if(
                in_state(Pause(false))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>),
            ),
    );
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct BuildingAssets {
    #[dependency]
    mana_forge: Handle<Image>,
}

impl FromWorld for BuildingAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();

        Self {
            mana_forge: assets.load_with_settings(
                "images/mana_forge.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnManaForge(pub Vec2);

impl Command for SpawnManaForge {
    fn apply(self, world: &mut World) -> () {
        let _ = world.run_system_cached_with(spawn_mana_forge, self);
    }
}

#[cfg_attr(target_os = "macos", hot)]
fn spawn_mana_forge(
    In(config): In<SpawnManaForge>,
    mut commands: Commands,
    buildings: Res<BuildingAssets>,
    map: Res<GameMap>,
) {
    let coords = map.tile_coords(config.0);
    info!("Spawning mana forge at {coords}");

    commands.spawn((
        BuildingLocation(coords),
        ManaForge::default(),
        Transform::from_xyz(
            // coords.x as f32 * map.sprite_size,
            // coords.y as f32 * map.sprite_size,
            config.0.x, config.0.y, 0.1,
        ),
        Visibility::Visible,
        Sprite {
            image: buildings.mana_forge.clone(),
            anchor: Anchor::TopLeft,
            ..default()
        },
    ));
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct BuildingLocation(pub IVec2);

/// A mana producing building
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ManaForge {
    pub mana_per_second: u32,
    time_since_last_tick: f32,
}

impl Default for ManaForge {
    fn default() -> Self {
        Self {
            mana_per_second: 3,
            time_since_last_tick: 0.,
        }
    }
}

fn produce_from_mana_forge(
    time: Res<Time>,
    mut player: ResMut<PlayerResources>,
    mut forges: Query<&mut ManaForge>,
) {
    let delta = time.delta_secs();

    for mut forge in &mut forges {
        if forge.time_since_last_tick + delta <= 1.0 {
            forge.time_since_last_tick += delta;
            continue;
        }

        forge.time_since_last_tick = 0.0;
        player.mana += forge.mana_per_second;
    }
}

fn blow_up_mana_forge(
    mut commands: Commands,
    map: Res<GameMap>,
    forges: Query<(Entity, &BuildingLocation), With<ManaForge>>,
) {
    for (entity, loc) in &forges {
        // check if there is fire near the mana forge
        if map.is_on_fire(loc.0)
            || map.is_on_fire(loc.0 + IVec2::new(1, 0))
            || map.is_on_fire(loc.0 + IVec2::new(1, 1))
            || map.is_on_fire(loc.0 + IVec2::new(0, 1))
        {
            info!("Mana forge destroyed by fire");
            commands.entity(entity).despawn();

            // spawn fires around
            let mut rng = rand::rng();
            let num_fires = rng.random_range(2..=5);
            info!("Spawning {num_fires} other fires");

            // TODO: JUICE! spawn fireballs to show the effects
            for _ in 0..num_fires {
                let fire_tile_coords =
                    loc.0 + IVec2::new(rng.random_range(-10..=10), rng.random_range(-10..10));
                commands.trigger(OnLightningStrike(fire_tile_coords));
            }
        }
    }
}
