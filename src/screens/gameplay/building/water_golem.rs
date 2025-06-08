//! Logic + code for placing water golem buildings

use bevy::{prelude::*, sprite::Anchor};
use rand::Rng;

use crate::{
    Pause,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode, WATER_GOLEM_COST_MANA,
            building::{
                BUILDING_FOOTPRINT_OFFSETS, BuildingAssets, BuildingLocation, BuildingType,
                ManaLine, ManaLineBalls, ParentBuilding, mana_forge::ManaForge,
            },
        },
    },
    wildfire::{GameMap, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<WaterGolem>();

    app.add_systems(
        Update,
        produce_from_water_golem.run_if(
            in_state(Pause(false))
                .and(in_state(Screen::Gameplay))
                .and(resource_exists::<PlayerResources>),
        ),
    );
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnWaterGolem(pub Vec2);

impl Command for SpawnWaterGolem {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_water_golem, self);
    }
}

fn spawn_water_golem(
    In(config): In<SpawnWaterGolem>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    mut map: ResMut<GameMap>,
    parent_forge: Single<(Entity, &ParentBuilding)>,
    forges: Query<&Transform, With<ManaForge>>,
) {
    if resources.mana < WATER_GOLEM_COST_MANA {
        warn!("Not enough resources to spawn water golem");
        return;
    }

    let (parent_forge_entity, parent_forge) = *parent_forge;
    let Some(parent_forge) = parent_forge.entity else {
        warn!("No parent mana forge inside tracking, skipping water golem placement");
        return;
    };

    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid map coordinates, aborting water golem placement");
        return;
    }

    commands.entity(parent_forge_entity).despawn();
    resources.mana -= WATER_GOLEM_COST_MANA;
    resources.mana_drain -= 1;

    let world_coords = map.world_coords(coords);
    info!("Spawning water golem at {coords}");

    let Ok(parent_tx) = forges.get(parent_forge) else {
        warn!("Unable to find parent mana forge");
        return;
    };

    commands.entity(parent_forge).with_children(|builder| {
        builder.spawn((
            BuildingLocation(coords),
            BuildingType::WaterGolem,
            WaterGolem::default(),
            ManaLine {
                from: parent_tx.translation.truncate().extend(0.05),
                to: config.0.extend(0.05),
                disabled: false,
            },
            ManaLineBalls::default(),
            StateScoped(Screen::Gameplay),
            Transform::from_xyz(
                world_coords.x - parent_tx.translation.x,
                world_coords.y - parent_tx.translation.y,
                0.1,
            ),
            Visibility::Visible,
            Sprite {
                image: buildings.water_golem.clone(),
                custom_size: Some(Vec2::splat(16.0)),
                anchor: Anchor::Center,
                ..default()
            },
        ));
    });

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
pub struct WaterGolem {
    /// The time the golem  was last updated
    time_since_last_tick: f32,
    /// The range of the golem (i.e. distance from the building location)
    range: i32,
}

impl Default for WaterGolem {
    fn default() -> Self {
        Self {
            time_since_last_tick: 0.0,
            range: 6,
        }
    }
}

impl WaterGolem {
    /// Find the next tree for the lumber mill to harvest
    fn find_next_target(&mut self, map: &mut GameMap, center: IVec2) -> Option<IVec2> {
        // first find all the available cells that are trees
        let coords = map
            .cells_within_range(center, self.range)
            .filter(
                // limit to trees and grass
                |coord| {
                    // direct access ok here as we only have valid coords
                    match map.data[coord.y as usize][coord.x as usize].terrain {
                        TerrainType::Grassland | TerrainType::Tree => true,
                        _ => false,
                    }
                },
            )
            .collect::<Vec<_>>();

        if coords.is_empty() {
            return None;
        }

        // now pick one and move there
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..coords.len());
        Some(coords[idx])
    }
}

// #[cfg_attr(target_os = "macos", hot)]
fn produce_from_water_golem(
    time: Res<Time>,
    mut map: ResMut<GameMap>,
    mut resources: ResMut<PlayerResources>,
    mut forges: Query<(&BuildingLocation, &mut WaterGolem)>,
) {
    let delta = time.delta_secs();

    for (loc, mut golem) in &mut forges {
        if golem.time_since_last_tick + delta <= 1.0 {
            golem.time_since_last_tick += delta;
            continue;
        }

        golem.time_since_last_tick = 0.0;

        // check if we have enough mana
        if resources.mana <= 0 {
            info!("Not enough mana to produce from minotaur at {}", loc.0);
            continue;
        }
        resources.mana = (resources.mana - 1).max(0);

        // reduce the current cell
        let Some(next_target) = golem.find_next_target(&mut map, loc.0) else {
            warn!("Unable to find cell to moisten. Skipping water golem production.");
            continue;
        };

        if let Some(current) = map.get_mut(next_target) {
            match current.terrain {
                TerrainType::Grassland | TerrainType::Tree => {
                    current.moisture = (current.moisture + 0.2).clamp(0.0, 1.0);
                    current.mark_dirty();
                    // continue, don't move on until we have dirt
                    continue;
                }
                TerrainType::Building
                | TerrainType::Dirt
                | TerrainType::Stone
                | TerrainType::Fire
                | TerrainType::Smoldering => {
                    // nop
                }
            }
        }
    }
}
