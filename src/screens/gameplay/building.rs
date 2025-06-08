//! Logic + code for placing buildings

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    input::common_conditions::input_just_pressed,
    prelude::*,
};

use crate::{
    Pause,
    asset_tracking::LoadResource,
    input::MousePosition,
    screens::{
        PlayerResources, Screen,
        gameplay::{StormMagePlacementRotation, building::mana_forge::ManaForge},
    },
    wildfire::GameMap,
};

mod city_hall;
mod destroy;
mod lumber_mill;
mod mana_forge;
mod mana_line;
mod minotaur;
mod storm_mage;
mod water_golem;

pub use city_hall::{CityHall, RequiresCityHall, SpawnCityHall};
pub use lumber_mill::SpawnLumberMill;
pub use mana_forge::SpawnManaForge;
pub use minotaur::SpawnMinotaur;
pub use storm_mage::{MageRotation, SpawnStormMage};
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
pub const STORM_MAGE_COST_MANA: i32 = 40;
pub const WATER_GOLEM_COST_MANA: i32 = 30;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingAssets>();
    app.register_type::<ResourceAssets>();
    app.register_type::<BuildingType>();
    app.register_type::<BuildingLocation>();
    app.register_type::<ManaLine>();
    app.register_type::<ManaLineBalls>();
    app.register_type::<ManaEntityLink>();
    app.register_type::<TrackParentBuildingWhilePlacing>();

    app.load_resource::<BuildingAssets>();
    app.load_resource::<ResourceAssets>();

    app.add_plugins((
        city_hall::plugin,
        destroy::plugin,
        lumber_mill::plugin,
        mana_forge::plugin,
        mana_line::plugin,
        minotaur::plugin,
        storm_mage::plugin,
        water_golem::plugin,
    ));

    app.add_systems(
        Update,
        ((
            track_building_parent_while_placing,
            rotate_storm_mage.run_if(
                input_just_pressed(KeyCode::KeyR)
                    .and(resource_exists::<StormMagePlacementRotation>),
            ),
        )
            .run_if(
                in_state(Pause(false))
                    .and(in_state(Screen::Gameplay))
                    .and(resource_exists::<PlayerResources>),
            ),),
    );
}

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq)]
#[reflect(Component)]
pub enum BuildingType {
    CityHall,
    ManaForge,
    Minotaur,
    LumberMill,
    StormMage,
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
    pub meteor: Handle<Image>,
    #[dependency]
    pub lumber_mill: Handle<Image>,
    #[dependency]
    pub mana_forge: Handle<Image>,
    #[dependency]
    pub minotaur: Handle<Image>,
    #[dependency]
    pub storm_mage: Handle<Image>,
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
            meteor: assets.load_with_settings(
                "images/meteor_icon.png",
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
            storm_mage: assets.load_with_settings(
                "images/storm_mage.png",
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
pub struct TrackParentBuildingWhilePlacing {
    pub entity: Option<Entity>,
    pub building_type: BuildingType,
}

impl TrackParentBuildingWhilePlacing {
    pub fn new(building_type: BuildingType) -> Self {
        Self {
            entity: None,
            building_type,
        }
    }
}

/// Indicates a line should be drawn between two components to show mana flow
#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct ManaLine {
    pub from: Vec3,
    pub to: Vec3,
    pub disabled: bool,
    pub destroying: bool,
}

impl ManaLine {
    pub fn new(from: Vec3, to: Vec3) -> Self {
        Self {
            from,
            to,
            disabled: false,
            destroying: false,
        }
    }
}

/// used to link parent mana forge/city hall to child
#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct ManaEntityLink {
    pub from_entity: Entity,
    pub to_entity: Entity,
    pub destruction_time: Option<f32>,
}

#[derive(Component, Reflect, Debug, Copy, Clone, Default)]
#[reflect(Component)]
pub struct ManaLineBalls {
    pub mana_dot_distance: f32,
}

fn rotate_storm_mage(mut mode: ResMut<StormMagePlacementRotation>) {
    mode.0 = mode.0.next();
}

fn track_building_parent_while_placing(
    mouse: Res<MousePosition>,
    map: Res<GameMap>,
    mut parent_building: Single<(&mut TrackParentBuildingWhilePlacing, &mut ManaLine)>,
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
