//! The city hall is the main building for the player. It doesn't produce
//! or consume any resources, but if it is destroyed the game is lost.

use bevy::{prelude::*, sprite::Anchor};

use crate::{
    screens::{
        Screen,
        gameplay::{
            BuildingMode, OnRedrawToolbar,
            building::{BuildingAssets, BuildingLocation, BuildingType},
        },
    },
    wildfire::GameMap,
};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<CityHall>();
    app.register_type::<RequiresCityHall>();
}

#[derive(Resource, Reflect, Debug, Default)]
#[reflect(Resource)]
pub struct RequiresCityHall;

#[derive(Debug, Clone, Copy)]
pub struct SpawnCityHall(pub Vec2);

impl Command for SpawnCityHall {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_city_hall, self);
    }
}

fn spawn_city_hall(
    In(config): In<SpawnCityHall>,
    mut commands: Commands,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    map: Res<GameMap>,
    existing_city_halls: Query<Entity, With<CityHall>>,
) {
    if !existing_city_halls.is_empty() {
        warn!("There can only be one city hall! Aborting placement");
        return;
    }
    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid coordinates for city hall, skipping placement");
        return;
    }

    let Some(cell) = map.get(coords) else {
        warn!("Can't find cell to verify city hall location. Aborting placement");
        return;
    };

    match cell.terrain {
        crate::wildfire::TerrainType::Grassland | crate::wildfire::TerrainType::Tree => {
            // nop
        }
        crate::wildfire::TerrainType::Dirt
        | crate::wildfire::TerrainType::Stone
        | crate::wildfire::TerrainType::Fire
        | crate::wildfire::TerrainType::Smoldering => {
            warn!("Can't place city hall on invalid terrain. Aborting placement");
            return;
        }
    }

    let clamped_world_coords = map.world_coords(coords);

    info!("Spawning city hall at {coords}");
    commands.spawn((
        BuildingLocation(coords),
        BuildingType::CityHall,
        CityHall,
        StateScoped(Screen::Gameplay),
        Transform::from_translation(clamped_world_coords.extend(0.1)),
        Visibility::Visible,
        Sprite {
            image: buildings.city_hall.clone(),
            custom_size: Some(Vec2::splat(16.0)),
            anchor: Anchor::Center,
            ..default()
        },
    ));

    *building_mode = BuildingMode::None;
    commands.remove_resource::<RequiresCityHall>();
    commands.trigger(OnRedrawToolbar);
}

/// A mana producing building
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct CityHall;
