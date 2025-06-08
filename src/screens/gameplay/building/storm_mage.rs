//! Logic + code for placing storm mages buildings

use bevy::{ecs::world::OnDespawn, prelude::*, sprite::Anchor};

use crate::{
    screens::{
        PlayerResources, Screen,
        gameplay::{
            BuildingMode, STORM_MAGE_COST_MANA,
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
    app.register_type::<StormMage>();
    app.add_observer(remove_storm_mage);
}

fn remove_storm_mage(
    trigger: Trigger<OnDespawn, StormMage>,
    map: Option<ResMut<GameMap>>,
    mages: Query<(&BuildingLocation, &StormMage)>,
) {
    let Some(mut map) = map else {
        warn!("Unable to remove mage, no game map exists");
        return;
    };

    let target = trigger.target();
    let Ok((loc, mage)) = mages.get(target) else {
        error!(
            "Unable to find mage being removed, aborting `remove_storm_mage`. The map will be out of date."
        );
        return;
    };

    for coord in &mage.cells {
        let Some(cell) = map.get_mut(*coord + loc.0) else {
            warn!("No cell found for storm mage");
            continue;
        };

        cell.wind -= mage.wind;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnStormMage(pub Vec2, pub MageRotation);

impl Command for SpawnStormMage {
    fn apply(self, world: &mut World) {
        let _ = world.run_system_cached_with(spawn_storm_mage, self);
    }
}

fn spawn_storm_mage(
    In(config): In<SpawnStormMage>,
    mut commands: Commands,
    mut resources: ResMut<PlayerResources>,
    mut building_mode: ResMut<BuildingMode>,
    buildings: Res<BuildingAssets>,
    mut map: ResMut<GameMap>,
    parent_forge: Single<(Entity, &TrackParentBuildingWhilePlacing)>,
    forges: Query<&Transform, With<ManaForge>>,
) {
    if resources.mana < 30 {
        warn!("Not enough resources to spawn storm mage");
        return;
    }

    let (parent_forge_entity, parent_forge) = *parent_forge;
    let Some(parent_forge) = parent_forge.entity else {
        warn!("No parent mana forge inside tracking, skipping storm mage placement");
        return;
    };

    let coords = map.tile_coords(config.0);
    if !map.is_valid_coords(coords) {
        warn!("Invalid map coordinates, aborting storm mage placement");
        return;
    }

    commands.entity(parent_forge_entity).despawn();
    resources.mana -= STORM_MAGE_COST_MANA;
    resources.mana_drain -= 2;

    let world_coords = map.world_coords(coords);
    info!("Spawning storm mage at {coords}");

    let Ok(parent_tx) = forges.get(parent_forge) else {
        warn!("Unable to find parent mana forge");
        return;
    };

    let mut mage = StormMage::default();
    mage.apply_to_map(coords, config.1, &mut map);

    commands.spawn((
        BuildingLocation(coords),
        BuildingType::StormMage,
        mage,
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
            image: buildings.storm_mage.clone(),
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
pub struct StormMage {
    /// The cells impacted by this mage
    cells: Vec<IVec2>,
    /// The angle of the wind from this mage
    wind: Vec2,
    /// The range that the storm mage works in
    range: i32,
}

impl Default for StormMage {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            wind: Vec2::X * 100.0,
            range: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Reflect)]
pub enum MageRotation {
    Left,
    Down,
    #[default]
    Right,
    Up,
}

impl MageRotation {
    /// Gets the next rotation clockwise
    pub fn next(self) -> Self {
        match self {
            MageRotation::Left => MageRotation::Up,
            MageRotation::Down => MageRotation::Left,
            MageRotation::Right => MageRotation::Down,
            MageRotation::Up => MageRotation::Right,
        }
    }
}

///The different cells to use depending on the rotation of the mage
impl StormMage {
    /// Gets the cells that the mage handles based on its rotation
    fn get_relevant_cells(rotation: MageRotation, range: i32) -> impl Iterator<Item = IVec2> {
        let (x_range, y_range) = match rotation {
            MageRotation::Left => (-5..=-1, -range..=range),
            MageRotation::Down => (-range..=range, 1..=5),
            MageRotation::Right => (1..=5, -range..=range),
            MageRotation::Up => (-range..=range, 1..=5),
        };

        x_range.flat_map(move |x| y_range.clone().map(move |y| IVec2::new(x, y)))
    }

    /// applies the effects of the storm mage to the map
    pub fn apply_to_map(&mut self, mage_cell: IVec2, rotation: MageRotation, map: &mut GameMap) {
        for cell in Self::get_relevant_cells(rotation, self.range) {
            let Some(map_cell) = map.get_mut(cell + mage_cell) else {
                warn!("Unable to locate cell in map, skipping wind from storm mage");
                continue;
            };

            map_cell.wind += self.wind;
            self.cells.push(cell);
        }
    }
}
