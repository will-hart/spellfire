//! Logic + code for placing mana forge buildings

use bevy::{prelude::*, sprite::Anchor};

use crate::{
    Pause,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode, MANA_FORGE_COST_LUMBER,
            building::{
                BUILDING_FOOTPRINT_OFFSETS, BuildingAssets, BuildingLocation, BuildingType,
                ManaLine, ParentBuilding, city_hall::CityHall,
            },
        },
    },
    wildfire::{GameMap, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<ManaForge>();

    app.add_systems(
        Update,
        (produce_from_mana_forge,).run_if(
            in_state(Pause(false))
                .and(in_state(Screen::Gameplay))
                .and(resource_exists::<PlayerResources>),
        ),
    );
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnManaForge(pub Vec2);

impl Command for SpawnManaForge {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_mana_forge, self);
    }
}

fn spawn_mana_forge(
    In(config): In<SpawnManaForge>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    mut map: ResMut<GameMap>,
    parent_forge: Single<(Entity, &ParentBuilding)>,
    hall: Single<&Transform, With<CityHall>>,
) {
    if resources.lumber < MANA_FORGE_COST_LUMBER {
        warn!("Not enough lumber to place mana forge!");
        return;
    }

    let Some(_) = parent_forge.1.entity else {
        warn!("No parent city hall, aborting mana forge placement");
        return;
    };

    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid coordinates for manaforge, skipping placement");
        return;
    }

    let clamped_world_coords = map.world_coords(coords);

    info!("Spawning mana forge at {coords}");
    resources.lumber -= 50;
    resources.mana_drain += 3;

    commands.spawn((
        BuildingLocation(coords),
        BuildingType::ManaForge,
        ManaForge::default(),
        ManaLine {
            from: hall.translation,
            to: clamped_world_coords.extend(0.05),
            disabled: false,
        },
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

    // update the map underneath to turn to buildings
    BUILDING_FOOTPRINT_OFFSETS.iter().for_each(|offset| {
        if let Some(cell) = map.get_mut(coords + *offset) {
            cell.terrain = TerrainType::Building;
        }
    });

    *building_mode = BuildingMode::None;
}

/// A mana producing building
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct ManaForge {
    pub mana_per_second: i32,
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
