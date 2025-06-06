//! Logic + code for placing buildings

use std::time::Duration;

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    time::common_conditions::on_timer,
};
use rand::Rng;

use crate::{
    Pause,
    asset_tracking::LoadResource,
    screens::{PlayerResources, Screen},
    wildfire::{GameMap, OnLightningStrike},
};

mod mana_forge;
mod mana_line;
mod minotaur;

pub use mana_forge::SpawnManaForge;
pub use minotaur::SpawnMinotaur;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingAssets>();
    app.register_type::<BuildingType>();
    app.register_type::<BuildingLocation>();
    app.register_type::<ManaLine>();
    app.register_type::<ManaLineBalls>();
    app.register_type::<ParentManaForge>();

    app.load_resource::<BuildingAssets>();

    app.add_plugins((mana_forge::plugin, mana_line::plugin, minotaur::plugin));

    app.add_systems(
        Update,
        burn_buildings.run_if(
            on_timer(Duration::from_millis(100))
                .and(in_state(Pause(false)))
                .and(in_state(Screen::Gameplay))
                .and(resource_exists::<PlayerResources>),
        ),
    );
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub enum BuildingType {
    ManaForge,
    Minotaur,
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct BuildingAssets {
    #[dependency]
    pub mana_forge: Handle<Image>,
    #[dependency]
    pub minotaur: Handle<Image>,
    #[dependency]
    pub lightning: Handle<Image>,
}

impl FromWorld for BuildingAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();

        Self {
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
        }
    }
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct BuildingLocation(pub IVec2);

#[derive(Component, Reflect, Debug, Copy, Clone, Default)]
#[reflect(Component)]
pub struct ParentManaForge(pub Option<Entity>);

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

            if !matches!(building_type, BuildingType::ManaForge) {
                return;
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
}
