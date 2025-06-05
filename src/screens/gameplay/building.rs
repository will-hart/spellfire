//! Logic + code for placing buildings

use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

use crate::asset_tracking::LoadResource;

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
    app.register_type::<ParentManaForge>();

    app.load_resource::<BuildingAssets>();

    app.add_plugins((mana_forge::plugin, mana_line::plugin, minotaur::plugin));
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
    mana_forge: Handle<Image>,
    #[dependency]
    minotaur: Handle<Image>,
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
        }
    }
}

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct BuildingLocation(pub IVec2);

#[derive(Resource, Reflect, Debug, Copy, Clone, Default)]
#[reflect(Resource)]
pub struct ParentManaForge(pub Option<Entity>);

#[derive(Component, Reflect, Debug, Copy, Clone)]
#[reflect(Component)]
pub struct ManaLine {
    pub from: Vec3,
    pub to: Vec3,
    pub mana_dot_distance: f32,
}
