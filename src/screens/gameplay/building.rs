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
        gameplay::{BuildTextHint, BuildingMode, building::mana_forge::ManaForge},
    },
    wildfire::{GameMap, OnLightningStrike},
};

mod city_hall;
mod lumber_mill;
mod mana_forge;
mod mana_line;
mod minotaur;

pub use city_hall::{RequiresCityHall, SpawnCityHall};
pub use lumber_mill::SpawnLumberMill;
pub use mana_forge::SpawnManaForge;
pub use minotaur::SpawnMinotaur;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingAssets>();
    app.register_type::<ResourceAssets>();
    app.register_type::<BuildingType>();
    app.register_type::<BuildingLocation>();
    app.register_type::<ManaLine>();
    app.register_type::<ManaLineBalls>();
    app.register_type::<ParentManaForge>();

    app.load_resource::<BuildingAssets>();
    app.load_resource::<ResourceAssets>();

    app.add_plugins((
        city_hall::plugin,
        lumber_mill::plugin,
        mana_forge::plugin,
        mana_line::plugin,
        minotaur::plugin,
    ));

    app.add_systems(
        Update,
        (
            burn_buildings.run_if(
                on_timer(Duration::from_millis(100))
                    .and(in_state(Pause(false)))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>),
            ),
            track_building_parent_while_placing.run_if(
                in_state(Pause(false))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>),
            ),
        ),
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
    pub mana_forge: Handle<Image>,
    #[dependency]
    pub minotaur: Handle<Image>,
    #[dependency]
    pub lightning: Handle<Image>,
    #[dependency]
    pub lumber_mill: Handle<Image>,
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
        }
    }
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct BuildingLocation(pub IVec2);

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct ParentManaForge {
    pub entity: Option<Entity>,
    pub building_type: BuildingType,
}

impl ParentManaForge {
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
    forges: Query<(Entity, &BuildingLocation, &BuildingType)>,
) {
    for (entity, loc, building_type) in &forges {
        // check if there is fire near the mana forge
        if map.is_on_fire(loc.0)
            || map.is_on_fire(loc.0 + IVec2::new(1, 0))
            || map.is_on_fire(loc.0 + IVec2::new(1, 1))
            || map.is_on_fire(loc.0 + IVec2::new(0, 1))
        {
            info!("{building_type:?} destroyed by fire");
            commands.entity(entity).despawn();

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
}

fn handle_despawned_buildings(
    trigger: Trigger<OnDespawn, BuildingType>,
    mut resources: ResMut<PlayerResources>,
    mut hint: ResMut<BuildTextHint>,
    buildings: Query<&BuildingType>,
) {
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
        BuildingType::Minotaur => {
            resources.mana_drain += 1;
        }
        BuildingType::LumberMill => {}
    }
}

fn track_building_parent_while_placing(
    mouse: Res<MousePosition>,
    mode: Res<BuildingMode>,
    map: Res<GameMap>,
    mut parent_forge: Single<(&mut ParentManaForge, &mut ManaLine)>,
    forges: Query<(Entity, &Transform), With<ManaForge>>,
) {
    const MAX_DISTANCE_SQR: f32 = 50.0 * 50.0;

    // unlikely but exit early anyway
    if !matches!(*mode, BuildingMode::PlaceMinotaur) {
        return;
    }

    let (forge, parent_mana_line) = &mut *parent_forge;

    // clear previous closest
    let mouse_pos = mouse.world_pos;
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

    distances.sort_by(|(_, a, _), (_, b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let Some((closest_forge, tx)) = distances.first().map(|(e, _, tx)| (e, tx)) else {
        forge.entity = None;
        parent_mana_line.disabled = true;

        return;
    };

    forge.entity = Some(*closest_forge);
    parent_mana_line.from = tx.extend(0.05);
    parent_mana_line.to = mouse_pos.extend(0.05);
    parent_mana_line.disabled = false;
}
