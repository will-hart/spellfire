//! Logic + code for placing lumber mill buildings

use bevy::{prelude::*, sprite::Anchor};
use rand::Rng;

use crate::{
    Pause,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode, LUMBER_MILL_COST_LUMBER,
            building::{BuildingAssets, BuildingLocation, BuildingType},
        },
    },
    wildfire::{GameMap, TerrainType},
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<LumberMill>();

    app.add_systems(
        Update,
        produce_from_lumber_mill.run_if(
            in_state(Pause(false))
                .and(in_state(Screen::Gameplay))
                .and(resource_exists::<PlayerResources>)
                .and(resource_exists::<GameMap>),
        ),
    );
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnLumberMill(pub Vec2);

impl Command for SpawnLumberMill {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_lumber_mill, self);
    }
}

fn spawn_lumber_mill(
    In(config): In<SpawnLumberMill>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    map: Res<GameMap>,
) {
    if resources.lumber < LUMBER_MILL_COST_LUMBER {
        warn!("Not enough resources to spawn lumber mill");
        return;
    }

    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid map coordinates, aborting lumber mill placement");
        return;
    }

    resources.lumber -= 30;

    let world_coords = map.world_coords(coords);
    info!("Spawning lumber mill at {coords}");

    commands.spawn((
        BuildingLocation(coords),
        BuildingType::LumberMill,
        LumberMill::default(),
        StateScoped(Screen::Gameplay),
        Transform::from_translation(world_coords.extend(0.1)),
        Visibility::Visible,
        Sprite {
            image: buildings.lumber_mill.clone(),
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
pub struct LumberMill {
    /// The time the minotaur was last updated
    time_since_last_tick: f32,
    /// The range of the minotaur (i.e. distance from the building location)
    range: i32,
}

impl Default for LumberMill {
    fn default() -> Self {
        Self {
            time_since_last_tick: 0.0,
            range: 5,
        }
    }
}

impl LumberMill {
    /// Find the next tree for the lumber mill to harvest
    fn find_next_target(
        &mut self,
        map: &mut GameMap,
        center: IVec2,
        target_terrain: TerrainType,
    ) -> Option<IVec2> {
        // first find all the available cells that are trees
        let coords = map
            .cells_within_range(center, self.range)
            .filter(
                // limit to trees and grass
                |coord| {
                    // direct access ok here as we only have valid coords
                    map.data[coord.y as usize][coord.x as usize].terrain == target_terrain
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

const CHANCE_LUMBER_MILL_PLANTS_TREE: f64 = 0.15;

fn produce_from_lumber_mill(
    time: Res<Time>,
    mut map: ResMut<GameMap>,
    mut resources: ResMut<PlayerResources>,
    mut mills: Query<(&BuildingLocation, &mut LumberMill)>,
) {
    let delta = time.delta_secs();
    let mut rng = rand::thread_rng();

    for (loc, mut mill) in &mut mills {
        let (target_terrain, new_terrain) = if rng.gen_bool(CHANCE_LUMBER_MILL_PLANTS_TREE) {
            (TerrainType::Grassland, TerrainType::Tree)
        } else {
            (TerrainType::Tree, TerrainType::Grassland)
        };

        if mill.time_since_last_tick + delta <= 1.0 {
            mill.time_since_last_tick += delta;
            continue;
        }

        mill.time_since_last_tick = 0.0;

        // reduce the current cell
        let Some(coord) = mill.find_next_target(&mut map, loc.0, target_terrain) else {
            return;
        };

        let Some(current) = map.get_mut(coord) else {
            warn!("Unable to find cell chosen for harvesting");
            return;
        };
        match current.terrain {
            TerrainType::Tree | TerrainType::Grassland => {
                current.terrain = new_terrain;
                current.mark_dirty();

                if new_terrain == TerrainType::Grassland {
                    resources.lumber += 2;
                }
            }
            TerrainType::Dirt
            | TerrainType::Building
            | TerrainType::Stone
            | TerrainType::Fire
            | TerrainType::Smoldering => {
                // nop
            }
        }
    }
}
