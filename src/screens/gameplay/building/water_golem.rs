//! Logic + code for placing water golem buildings

use bevy::{color::palettes::tailwind::INDIGO_600, prelude::*, sprite::Anchor};
use bevy_vector_shapes::{prelude::ShapePainter, shapes::DiscPainter};
use rand::Rng;

use crate::{
    Pause,
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode, WATER_GOLEM_COST_MANA,
            building::{
                BUILDING_FOOTPRINT_OFFSETS, BuildingAssets, BuildingLocation, BuildingType,
                ManaEntityLink, ManaLine, ManaLineBalls, TrackParentBuildingWhilePlacing,
                mana_forge::ManaForge,
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
                .and(resource_exists::<PlayerResources>)
                .and(resource_exists::<GameMap>),
        ),
    );

    app.add_systems(
        Update,
        draw_golem_areas.run_if(
            in_state(Screen::Gameplay).and(in_state(Pause(false)).and(resource_exists::<GameMap>)),
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
    parent_forge: Single<(Entity, &TrackParentBuildingWhilePlacing)>,
    forges: Query<&Transform, With<ManaForge>>,
) {
    if resources.mana < WATER_GOLEM_COST_MANA {
        warn!("Not enough resources to spawn water golem");
        return;
    }

    let (parent_tracking_entity, parent_forge) = *parent_forge;
    let Some(parent_forge) = parent_forge.entity else {
        warn!("No parent mana forge inside tracking, skipping water golem placement");
        return;
    };

    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid map coordinates, aborting water golem placement");
        return;
    }

    commands.entity(parent_tracking_entity).despawn();
    resources.mana -= WATER_GOLEM_COST_MANA;
    resources.mana_drain -= 2;

    let world_coords = map.world_coords(coords);
    info!("Spawning water golem at {coords}");

    let Ok(parent_tx) = forges.get(parent_forge) else {
        warn!("Unable to find parent mana forge");
        return;
    };

    commands.spawn((
        BuildingLocation(coords),
        BuildingType::WaterGolem,
        WaterGolem::default(),
        ManaLine::new(
            parent_tx.translation.truncate().extend(0.05),
            config.0.extend(0.05),
        ),
        ManaLineBalls::default(),
        ManaEntityLink {
            from_entity: parent_forge,
            destruction_time: None,
        },
        StateScoped(Screen::Gameplay),
        Transform::from_xyz(world_coords.x, world_coords.y, 0.1),
        Visibility::Visible,
        Sprite {
            image: buildings.water_golem.clone(),
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
            range: 4,
        }
    }
}

fn draw_golem_areas(
    mut painter: ShapePainter,
    map: Res<GameMap>,
    golems: Query<(&Transform, &WaterGolem)>,
) {
    let original_tx = painter.transform;

    for (tx, golem) in &golems {
        let mut color = INDIGO_600;
        color.alpha = 0.4;

        painter.hollow = true;
        painter.set_color(color);
        painter.translate(tx.translation - Vec3::new(0.0, 0.0, 0.05));
        painter.circle(golem.range as f32 * map.sprite_size);

        painter.transform = original_tx;
    }
}

pub const WATER_GOLEM_PRODUCTION_TIME: f32 = 2.0;
pub const WATER_GOLEM_QUENCH_CHANCE: f64 = 0.3;
pub const WATER_GOLEM_MOISTURE_INCREASE: f32 = 0.05;
pub const WATER_GOLEM_MANA_CONSUMPTION: i32 = 4;

fn produce_from_water_golem(
    time: Res<Time>,
    mut map: ResMut<GameMap>,
    mut resources: ResMut<PlayerResources>,
    mut golems: Query<(&BuildingLocation, &mut WaterGolem)>,
) {
    let delta = time.delta_secs();

    for (loc, mut golem) in &mut golems {
        if golem.time_since_last_tick + delta <= WATER_GOLEM_PRODUCTION_TIME {
            golem.time_since_last_tick += delta;
            continue;
        }

        golem.time_since_last_tick = 0.0;

        // check if we have enough mana
        if resources.mana < WATER_GOLEM_MANA_CONSUMPTION {
            info!("Not enough mana to produce from minotaur at {}", loc.0);
            continue;
        }
        resources.mana = (resources.mana - WATER_GOLEM_MANA_CONSUMPTION).max(0);

        // find all cells in rand and handle them
        let neighbours = map
            .cells_within_range(loc.0, golem.range)
            .collect::<Vec<_>>();

        let mut rng = rand::thread_rng();

        for coord in &neighbours {
            if let Some(cell) = map.get_mut(*coord) {
                match cell.terrain {
                    TerrainType::Fire => {
                        if rng.gen_bool(WATER_GOLEM_QUENCH_CHANCE) {
                            cell.terrain = TerrainType::Smoldering;
                            cell.mark_dirty();
                        }
                    }
                    TerrainType::Grassland | TerrainType::Tree => {
                        cell.moisture =
                            (cell.moisture + WATER_GOLEM_MOISTURE_INCREASE).clamp(0.0, 1.0);
                        cell.mark_dirty();
                    }
                    TerrainType::Dirt
                    | TerrainType::Building
                    | TerrainType::Stone
                    | TerrainType::Smoldering => {
                        // nop
                    }
                }
            }
        }
    }
}
