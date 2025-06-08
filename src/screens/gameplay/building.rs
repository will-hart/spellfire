//! Logic + code for placing buildings

use std::time::Duration;

use bevy::{
    ecs::world::OnDespawn,
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    time::common_conditions::on_timer,
};
use rand::Rng;

use crate::{
    Pause,
    asset_tracking::LoadResource,
    input::MousePosition,
    screens::{
        PlayerResources, Screen,
        gameplay::{BuildTextHint, building::mana_forge::ManaForge},
    },
    wildfire::{GameMap, OnLightningStrike},
};

mod city_hall;
mod lumber_mill;
mod mana_forge;
mod mana_line;
mod minotaur;
mod water_golem;

pub use city_hall::{CityHall, RequiresCityHall, SpawnCityHall};
pub use lumber_mill::SpawnLumberMill;
pub use mana_forge::SpawnManaForge;
pub use minotaur::SpawnMinotaur;
pub use water_golem::SpawnWaterGolem;

pub const BUILDING_FOOTPRINT_OFFSETS: [IVec2; 4] = [
    IVec2::ZERO,
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
];

pub const LUMBER_MILL_COST_LUMBER: i32 = 30;
pub const MANA_FORGE_COST_LUMBER: i32 = 40;
pub const MINOTAUR_COST_MANA: i32 = 35;
pub const WATER_GOLEM_COST_MANA: i32 = 30;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingAssets>();
    app.register_type::<ResourceAssets>();
    app.register_type::<BuildingType>();
    app.register_type::<BuildingLocation>();
    app.register_type::<ManaLine>();
    app.register_type::<ManaLineBalls>();
    app.register_type::<ParentBuilding>();

    app.load_resource::<BuildingAssets>();
    app.load_resource::<ResourceAssets>();

    app.add_plugins((
        city_hall::plugin,
        lumber_mill::plugin,
        mana_forge::plugin,
        mana_line::plugin,
        minotaur::plugin,
        water_golem::plugin,
    ));

    app.add_systems(
        Update,
        ((
            burn_buildings.run_if(on_timer(Duration::from_millis(100))),
            track_building_parent_while_placing,
        )
            .run_if(
                in_state(Pause(false))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>),
            ),),
    );

    app.add_observer(handle_despawned_buildings);
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub enum BuildingType {
    CityHall,
    ManaForge,
    Minotaur,
    LumberMill,
    WaterGolem,
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct ResourceAssets {
    #[dependency]
    pub resource_icons: Handle<Image>,
}

impl FromWorld for ResourceAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();

        Self {
            resource_icons: assets.load_with_settings(
                "images/resource_icons.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
        }
    }
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct BuildingAssets {
    #[dependency]
    pub city_hall: Handle<Image>,
    #[dependency]
    pub lightning: Handle<Image>,
    #[dependency]
    pub lumber_mill: Handle<Image>,
    #[dependency]
    pub mana_forge: Handle<Image>,
    #[dependency]
    pub minotaur: Handle<Image>,
    #[dependency]
    pub water_golem: Handle<Image>,
}

impl FromWorld for BuildingAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();

        Self {
            city_hall: assets.load_with_settings(
                "images/city_hall.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            lightning: assets.load_with_settings(
                "images/lightning.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            lumber_mill: assets.load_with_settings(
                "images/lumbermill.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            mana_forge: assets.load_with_settings(
                "images/mana_forge.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            minotaur: assets.load_with_settings(
                "images/minotaur.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
            water_golem: assets.load_with_settings(
                "images/water_golem.png",
                |settings: &mut ImageLoaderSettings| {
                    // Use `nearest` image sampling to preserve pixel art style.
                    settings.sampler = ImageSampler::nearest();
                },
            ),
        }
    }
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct BuildingLocation(pub IVec2);

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct ParentBuilding {
    pub entity: Option<Entity>,
    pub building_type: BuildingType,
}

impl ParentBuilding {
    pub fn new(building_type: BuildingType) -> Self {
        Self {
            entity: None,
            building_type,
        }
    }
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct ManaLine {
    pub from: Vec3,
    pub to: Vec3,
    pub disabled: bool,
}

#[derive(Component, Reflect, Debug, Copy, Clone, Default)]
#[reflect(Component)]
pub struct ManaLineBalls {
    pub mana_dot_distance: f32,
}

/// Burns buildings that are consumed by fire
fn burn_buildings(
    mut commands: Commands,
    map: Res<GameMap>,
    buildings: Query<(Entity, &BuildingLocation, &BuildingType)>,
) {
    let mut despawn_all = false;

    for (entity, loc, building_type) in &buildings {
        // check if there is fire near the building
        if map.check_on_fire(&[
            loc.0,
            loc.0 + IVec2::new(1, 0),
            loc.0 + IVec2::new(1, 1),
            loc.0 + IVec2::new(0, 1),
        ]) {
            // currently despawns child buildings of a mana forge too due to
            // hierarchy.
            // TODO: use some other method that enables juice
            info!("{building_type:?} destroyed by fire");
            commands.entity(entity).despawn();

            match building_type {
                BuildingType::CityHall => {
                    despawn_all = true;
                }
                BuildingType::Minotaur | BuildingType::LumberMill | BuildingType::WaterGolem => {
                    // no follow up booms
                    return;
                }
                BuildingType::ManaForge => {
                    // there will be a follow up boom
                }
            }

            // spawn fires around
            let mut rng = rand::thread_rng();
            let num_fires = rng.gen_range(2..=5);
            info!("Spawning {num_fires} other fires");

            // TODO: JUICE! spawn fireballs to show the effects
            for _ in 0..num_fires {
                let fire_tile_coords =
                    loc.0 + IVec2::new(rng.gen_range(-10..=10), rng.gen_range(-10..10));
                commands.trigger(OnLightningStrike(fire_tile_coords));
            }
        }
    }

    if despawn_all {
        for (entity, _, _) in &buildings {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_despawned_buildings(
    trigger: Trigger<OnDespawn, BuildingType>,
    resources: Option<ResMut<PlayerResources>>,
    mut hint: ResMut<BuildTextHint>,
    buildings: Query<&BuildingType>,
) {
    let Some(mut resources) = resources else {
        // probably because we're exiting the game or to menu
        return;
    };

    let target = trigger.target();
    let Ok(building_type) = buildings.get(target) else {
        warn!("Unable to find building to handle despawn");
        return;
    };

    match building_type {
        BuildingType::CityHall => {
            hint.set("GAME OVER");
        }
        BuildingType::ManaForge => {
            resources.mana_drain -= 3;
        }
        BuildingType::Minotaur | BuildingType::WaterGolem => {
            resources.mana_drain += 1;
        }
        BuildingType::LumberMill => {}
    }
}

fn track_building_parent_while_placing(
    mouse: Res<MousePosition>,
    map: Res<GameMap>,
    mut parent_building: Single<(&mut ParentBuilding, &mut ManaLine)>,
    forges: Query<(Entity, &Transform), With<ManaForge>>,
    hall: Single<(Entity, &Transform), With<CityHall>>,
) {
    const MAX_DISTANCE_SQR: f32 = 60.0 * 60.0;

    let (parent, parent_mana_line) = &mut *parent_building;

    let mouse_pos = mouse.world_pos;

    let closest = if matches!(parent.building_type, BuildingType::ManaForge) {
        let pos = hall.1.translation.truncate();

        let distance_to_forge = mouse_pos.distance_squared(pos);
        if distance_to_forge > MAX_DISTANCE_SQR * map.sprite_size {
            None
        } else {
            Some((hall.0, hall.1.translation.truncate()))
        }
    } else {
        let mut distances = forges
            .iter()
            .filter_map(|(e, tx)| {
                let distance_to_forge = mouse_pos.distance_squared(tx.translation.truncate());
                if distance_to_forge > MAX_DISTANCE_SQR * map.sprite_size {
                    return None;
                }

                Some((e, distance_to_forge, tx.translation.truncate()))
            })
            .collect::<Vec<_>>();

        distances
            .sort_by(|(_, a, _), (_, b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        distances
            .first()
            .map(|(e, _, target_location)| (*e, *target_location))
    };

    let Some((closest_forge, tx)) = closest else {
        parent.entity = None;
        parent_mana_line.disabled = true;
        return;
    };

    parent.entity = Some(closest_forge);
    parent_mana_line.from = tx.extend(0.05);
    parent_mana_line.to = mouse_pos.extend(0.05);
    parent_mana_line.disabled = false;
}
