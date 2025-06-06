//! Logic + code for placing mana forge buildings

use std::time::Duration;

use bevy::{prelude::*, sprite::Anchor, time::common_conditions::on_timer};
use rand::Rng;

use crate::{
    Pause,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode,
            building::{BuildingAssets, BuildingLocation, BuildingType},
        },
    },
    wildfire::{GameMap, OnLightningStrike},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<ManaForge>();

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

#[derive(Debug, Clone, Copy)]
pub struct SpawnManaForge(pub Vec2);

impl Command for SpawnManaForge {
    fn apply(self, world: &mut World) -> () {
        let _ = world.run_system_cached_with(spawn_mana_forge, self);
    }
}

fn spawn_mana_forge(
    In(config): In<SpawnManaForge>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    map: Res<GameMap>,
) {
    if resources.mana < 50 {
        warn!("Not enough resources to place mana forge!");
        return;
    }
    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid coordinates for manaforge, skipping placement");
        return;
    }

    let clamped_world_coords = map.world_coords(coords);

    info!("Spawning mana forge at {coords}");
    resources.mana -= 50;

    commands.spawn((
        BuildingLocation(coords),
        BuildingType::ManaForge,
        ManaForge::default(),
        StateScoped(Screen::Gameplay),
        Transform::from_translation(clamped_world_coords.extend(0.1)),
        Visibility::Visible,
        Sprite {
            image: buildings.mana_forge.clone(),
            custom_size: Some(Vec2::splat(16.0)),
            anchor: Anchor::Center,
            ..default()
        },
    ));

    *building_mode = BuildingMode::None;
}

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
