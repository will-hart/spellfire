//! The screen state for the main gameplay.

use bevy::{
    input::common_conditions::{input_just_pressed, input_pressed},
    prelude::*,
    ui::Val::*,
};

use crate::{
    Pause,
    input::MousePosition,
    menus::Menu,
    screens::{
        Screen,
        gameplay::building::{
            BuildingAssets, MageRotation, ManaLine, SpawnCityHall, SpawnLumberMill, SpawnManaForge,
            SpawnMinotaur, SpawnStormMage, SpawnWaterGolem, TrackParentBuildingWhilePlacing,
        },
    },
    wildfire::{GameMap, OnMeteorStrike},
};

mod building;
pub mod story_mode;
mod toolbar;
mod victory;

pub use building::{
    BuildingType, CityHall, LUMBER_MILL_COST_LUMBER, MANA_FORGE_COST_LUMBER, MINOTAUR_COST_MANA,
    RequiresCityHall, STORM_MAGE_COST_MANA, WATER_GOLEM_COST_MANA,
};
pub use toolbar::OnRedrawToolbar;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingMode>();
    app.register_type::<CursorModeItem>();
    app.register_type::<CursorModeFollower>();
    app.register_type::<PlayerResources>();
    app.register_type::<BuildTextHint>();
    app.register_type::<BuildTextMarker>();
    app.register_type::<EndlessMode>();

    app.init_resource::<BuildingMode>();
    app.init_resource::<BuildTextHint>();

    app.add_plugins((
        building::plugin,
        story_mode::plugin,
        toolbar::plugin,
        victory::plugin,
    ));

    // Toggle pause on key press.
    app.add_systems(
        Update,
        (
            (pause, spawn_pause_overlay, open_pause_menu).run_if(
                in_state(Screen::Gameplay)
                    .and(in_state(Menu::None))
                    .and(input_just_pressed(KeyCode::KeyP).or(input_just_pressed(KeyCode::Escape))),
            ),
            close_menu.run_if(
                in_state(Screen::Gameplay)
                    .and(not(in_state(Menu::None)))
                    .and(input_just_pressed(KeyCode::KeyP)),
            ),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), (close_menu, unpause));
    app.add_systems(
        OnEnter(Menu::None),
        unpause.run_if(in_state(Screen::Gameplay)),
    );

    app.add_systems(
        Update,
        cancel_cursor_mode
            .run_if(in_state(Screen::Gameplay).and(
                input_just_pressed(KeyCode::Space).or(input_just_pressed(MouseButton::Right)),
            )),
    );

    app.add_systems(
        Update,
        (
            cursor_mode_follower,
            handle_mouse_click_input.run_if(input_just_pressed(MouseButton::Left)),
            handle_build_mode_changing
                .run_if(resource_changed::<BuildingMode>)
                .after(cancel_cursor_mode),
        )
            .chain()
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );

    app.add_systems(
        Update,
        cheat.run_if(
            input_just_pressed(KeyCode::KeyC)
                .and(input_pressed(KeyCode::ControlLeft))
                .and(input_pressed(KeyCode::ShiftLeft))
                .and(resource_exists::<PlayerResources>),
        ),
    );
}

fn cheat(mut resources: ResMut<PlayerResources>) {
    resources.mana += 100;
    resources.lumber += 100;
}

#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource, Default)]
pub struct EndlessMode;

#[derive(Resource, Reflect, Debug, Clone)]
#[reflect(Resource)]
pub struct PlayerResources {
    /// The amount of mana in the bank
    pub mana: i32,
    /// The amount of mana being produced or drained per second
    pub mana_drain: i32,
    /// The amount of lumber in the bank
    pub lumber: i32,
}

impl Default for PlayerResources {
    fn default() -> Self {
        Self {
            mana: 0,
            mana_drain: 0,
            lumber: 80,
        }
    }
}

#[derive(Resource, Reflect, Debug, Clone, Copy, Default)]
#[reflect(Resource)]
pub struct StormMagePlacementRotation(pub MageRotation);

#[derive(Resource, Reflect, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[reflect(Resource)]
pub enum BuildingMode {
    #[default]
    None,
    Meteor,
    PlaceCityHall,
    PlaceLumberMill,
    PlaceManaForge,
    PlaceMinotaur,
    PlaceStormMage,
    PlaceWaterGolem,
}

impl From<BuildingMode> for BuildingType {
    fn from(value: BuildingMode) -> Self {
        match value {
            BuildingMode::PlaceMinotaur => BuildingType::Minotaur,
            BuildingMode::PlaceWaterGolem => BuildingType::WaterGolem,
            BuildingMode::PlaceStormMage => BuildingType::StormMage,
            BuildingMode::None
            | BuildingMode::Meteor
            | BuildingMode::PlaceCityHall
            | BuildingMode::PlaceLumberMill
            | BuildingMode::PlaceManaForge => unreachable!(),
        }
    }
}

fn unpause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(false));
}

fn pause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(true));
}

fn handle_mouse_click_input(
    mut commands: Commands,
    mut mode: ResMut<BuildingMode>,
    mouse: Res<MousePosition>,
    maybe_mage_rotation: Option<Res<StormMagePlacementRotation>>,
    maybe_requires_city_hall: Option<Res<RequiresCityHall>>,
    maybe_map: Option<Res<GameMap>>,
) {
    if maybe_requires_city_hall.is_some() && !matches!(*mode, BuildingMode::PlaceCityHall) {
        warn!(
            "Cannot handle - {mode:?}. Requires city hall before any other buildings can be placed"
        );
        *mode = BuildingMode::PlaceCityHall;
        return;
    }

    match *mode {
        BuildingMode::None => {}
        BuildingMode::PlaceCityHall => {
            commands.queue(SpawnCityHall(mouse.world_pos));
        }
        BuildingMode::PlaceLumberMill => {
            commands.queue(SpawnLumberMill(mouse.world_pos));
        }
        BuildingMode::Meteor => {
            if let Some(map) = maybe_map {
                let coords = map.tile_coords(mouse.world_pos);
                commands.trigger(OnMeteorStrike(coords));
            } else {
                warn!("Skipping meteor strike input as there is no map yet");
            }
        }
        BuildingMode::PlaceManaForge => {
            commands.queue(SpawnManaForge(mouse.world_pos));
        }
        BuildingMode::PlaceMinotaur => {
            commands.queue(SpawnMinotaur(mouse.world_pos));
        }
        BuildingMode::PlaceStormMage => {
            commands.queue(SpawnStormMage(
                mouse.world_pos,
                maybe_mage_rotation.map(|r| r.0).unwrap_or_default(),
            ));
        }
        BuildingMode::PlaceWaterGolem => {
            commands.queue(SpawnWaterGolem(mouse.world_pos));
        }
    }
}

#[derive(Reflect, Debug, Default, Clone)]
pub enum HintMessage {
    #[default]
    None,
    Text(String),
    BuildingData {
        name: String,
        cost: String,
        details: String,
    },
}

impl From<&str> for HintMessage {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

#[derive(Resource, Reflect, Debug, Default)]
#[reflect(Resource)]
pub struct BuildTextHint(HintMessage);

impl BuildTextHint {
    /// Clears the text
    pub fn clear(&mut self) {
        self.0 = HintMessage::None;
    }

    /// Sets the hint as text
    pub fn set(&mut self, text: impl Into<String>) {
        self.0 = HintMessage::Text(text.into());
    }
}

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct BuildTextMarker;

fn spawn_pause_overlay(mut commands: Commands) {
    commands.spawn((
        Name::new("Pause Overlay"),
        Node {
            width: Percent(100.0),
            height: Percent(100.0),
            ..default()
        },
        GlobalZIndex(1),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        StateScoped(Pause(true)),
    ));
}

fn open_pause_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Pause);
}

fn close_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::None);
}

fn cancel_cursor_mode(
    mut commands: Commands,
    mut mode: ResMut<BuildingMode>,
    forge_placements: Query<Entity, With<TrackParentBuildingWhilePlacing>>,
) {
    info!("Resetting cursor mode");
    *mode = BuildingMode::None;

    for parent in forge_placements {
        commands.entity(parent).despawn();
    }
}

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
pub struct CursorModeItem;

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
pub struct CursorModeFollower;

fn cursor_mode_follower(
    mouse: Res<MousePosition>,
    mut cursor_items: Query<&mut Transform, With<CursorModeFollower>>,
) {
    for mut cursor_tx in &mut cursor_items {
        cursor_tx.translation = mouse.world_pos.extend(1.0);
    }
}

fn handle_build_mode_changing(
    mut commands: Commands,
    mode: Res<BuildingMode>,
    building_assets: Res<BuildingAssets>,
    mut hint: ResMut<BuildTextHint>,
    previous_items: Query<Entity, With<CursorModeItem>>,
) {
    // despawn previous entities
    for entity in &previous_items {
        commands.entity(entity).despawn();
    }

    match *mode {
        BuildingMode::None => {
            hint.clear();
            commands.remove_resource::<StormMagePlacementRotation>();
        }
        BuildingMode::Meteor => {}
        BuildingMode::PlaceCityHall => {
            commands.spawn((
                CursorModeFollower,
                CursorModeItem,
                Sprite {
                    image: building_assets.city_hall.clone(),
                    ..default()
                },
            ));
        }
        BuildingMode::PlaceLumberMill => {
            commands.spawn((
                CursorModeFollower,
                CursorModeItem,
                Sprite {
                    image: building_assets.lumber_mill.clone(),
                    ..default()
                },
            ));
        }

        BuildingMode::PlaceManaForge => {
            info!("Spawning building mode items for mana forge placement");
            commands.spawn((
                TrackParentBuildingWhilePlacing::new(BuildingType::ManaForge),
                CursorModeItem,
                CursorModeFollower,
                ManaLine::new(Vec3::ZERO, Vec3::ZERO),
                Sprite {
                    image: building_assets.mana_forge.clone(),
                    ..default()
                },
            ));
        }
        next_mode @ BuildingMode::PlaceMinotaur
        | next_mode @ BuildingMode::PlaceWaterGolem
        | next_mode @ BuildingMode::PlaceStormMage => {
            info!("Spawning building mode items for {mode:?} placement");
            commands.spawn((
                TrackParentBuildingWhilePlacing::new(next_mode.into()),
                CursorModeItem,
                CursorModeFollower,
                ManaLine::new(Vec3::ZERO, Vec3::ZERO),
                Sprite {
                    image: match next_mode {
                        BuildingMode::PlaceMinotaur => building_assets.minotaur.clone(),
                        BuildingMode::PlaceWaterGolem => building_assets.water_golem.clone(),
                        BuildingMode::PlaceStormMage => building_assets.storm_mage.clone(),
                        BuildingMode::None
                        | BuildingMode::Meteor
                        | BuildingMode::PlaceCityHall
                        | BuildingMode::PlaceLumberMill
                        | BuildingMode::PlaceManaForge => {
                            unreachable!();
                        }
                    },
                    ..default()
                },
            ));
        }
    }
}
