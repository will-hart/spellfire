//! Logic + code for placing minotaur hutch buildings

use bevy::{prelude::*, sprite::Anchor};
use rand::Rng;

use crate::{
    Pause,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode,
            building::{
                BUILDING_FOOTPRINT_OFFSETS, BuildingAssets, BuildingLocation, BuildingType,
                ManaLine, ManaLineBalls, ParentBuilding, mana_forge::ManaForge,
            },
        },
    },
    wildfire::{GameMap, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<Minotaur>();

    app.add_systems(
        Update,
        produce_from_minotaur.run_if(
            in_state(Pause(false))
                .and(in_state(Screen::Gameplay))
                .and(resource_exists::<PlayerResources>),
        ),
    );
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnMinotaur(pub Vec2);

impl Command for SpawnMinotaur {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_minotaur, self);
    }
}

fn spawn_minotaur(
    In(config): In<SpawnMinotaur>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    mut map: ResMut<GameMap>,
    parent_forge: Single<(Entity, &ParentBuilding)>,
    forges: Query<&Transform, With<ManaForge>>,
) {
    if resources.mana < 30 {
        warn!("Not enough resources to spawn minotaur");
        return;
    }

    let (parent_forge_entity, parent_forge) = *parent_forge;
    let Some(parent_forge) = parent_forge.entity else {
        warn!("No parent mana forge inside tracking, skipping minotaur placement");
        return;
    };

    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid map coordinates, aborting minotaur placement");
        return;
    }

    commands.entity(parent_forge_entity).despawn();
    resources.mana -= 30;
    resources.mana_drain -= 1;

    let world_coords = map.world_coords(coords);
    info!("Spawning minotaur at {coords}");

    let Ok(parent_tx) = forges.get(parent_forge) else {
        warn!("Unable to find parent mana forge");
        return;
    };

    commands.entity(parent_forge).with_children(|builder| {
        builder.spawn((
            BuildingLocation(coords),
            BuildingType::Minotaur,
            Minotaur::default(),
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
                image: buildings.minotaur.clone(),
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
pub struct Minotaur {
    /// The time the minotaur was last updated
    time_since_last_tick: f32,
    /// The offset from the building location to describe where this minotaur
    /// currently is
    location: IVec2,
    /// The range of the minotaur (i.e. distance from the building location)
    range: i32,
    /// Whether the minotaur consumed mana last tick
    consumed_last_tick: bool,
}

impl Default for Minotaur {
    fn default() -> Self {
        Self {
            time_since_last_tick: 0.0,
            location: IVec2::ZERO,
            range: 5,
            consumed_last_tick: true,
        }
    }
}

impl Minotaur {
    /// Move the minotaur to a random new position
    fn move_to_grass(&mut self, map: &mut GameMap, center: IVec2) {
        // first find all the available cells that are grass or trees
        let coords = map
            .cells_within_range(center, self.range)
            .filter(
                // limit to trees and grass
                |coord| {
                    matches!(
                        // no bounds checjk requried as cells_within_range
                        // only returns valid cvells
                        map.data[coord.y as usize][coord.x as usize].terrain,
                        TerrainType::Grassland | TerrainType::Tree
                    )
                },
            )
            .collect::<Vec<_>>();

        if coords.is_empty() {
            self.location = IVec2::ZERO;
            return;
        }

        // now pick one and move there
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..coords.len());
        self.location = coords[idx];
    }
}

// #[cfg_attr(target_os = "macos", hot)]
fn produce_from_minotaur(
    time: Res<Time>,
    mut map: ResMut<GameMap>,
    mut resources: ResMut<PlayerResources>,
    mut forges: Query<(&BuildingLocation, &mut Minotaur)>,
) {
    let delta = time.delta_secs();

    for (loc, mut minotaur) in &mut forges {
        if minotaur.time_since_last_tick + delta <= 0.5 {
            minotaur.time_since_last_tick += delta;
            continue;
        }

        minotaur.time_since_last_tick = 0.0;

        // check if we have enough mana
        if !minotaur.consumed_last_tick {
            if resources.mana <= 0 {
                info!("Not enough mana to produce from minotaur at {}", loc.0);
                continue;
            }
            resources.mana = (resources.mana - 1).max(0);
        }

        minotaur.consumed_last_tick = !minotaur.consumed_last_tick;

        // reduce the current cell
        if let Some(current) = map.get_mut(minotaur.location) {
            match current.terrain {
                TerrainType::Grassland => {
                    current.terrain = TerrainType::Dirt;
                    current.mark_dirty();
                }
                TerrainType::Tree => {
                    current.terrain = TerrainType::Grassland;
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

        // move the minotaur in a spiral
        minotaur.move_to_grass(&mut map, loc.0);
    }
}
