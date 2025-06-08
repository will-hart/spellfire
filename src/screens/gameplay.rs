//! The screen state for the main gameplay.

use bevy::{input::common_conditions::input_just_pressed, prelude::*, ui::Val::*};

use crate::{
    Pause,
    input::MousePosition,
    menus::Menu,
    screens::{
        Screen,
        gameplay::building::{
            ManaLine, ParentBuilding, SpawnCityHall, SpawnLumberMill, SpawnManaForge, SpawnMinotaur,
        },
    },
    wildfire::{GameMap, OnLightningStrike},
};

mod building;
pub mod story_mode;
mod toolbar;
mod victory;

pub use building::{BuildingType, CityHall, RequiresCityHall};
pub use toolbar::OnRedrawToolbar;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<BuildingMode>();
    app.register_type::<CursorModeItem>();
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
            handle_mouse_click_input.run_if(input_just_pressed(MouseButton::Left)),
            handle_build_mode_changing
                .run_if(resource_changed::<BuildingMode>)
                .after(cancel_cursor_mode),
        )
            .chain()
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
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
            lumber: 100,
        }
    }
}

#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource)]
pub enum BuildingMode {
    #[default]
    None,
    Lightning,
    PlaceCityHall,
    PlaceLumberMill,
    PlaceManaForge,
    PlaceMinotaur,
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
        BuildingMode::Lightning => {
            if let Some(map) = maybe_map {
                let coords = map.tile_coords(mouse.world_pos);
                commands.trigger(OnLightningStrike(coords));
            } else {
                warn!("Skipping lightning strike input as there is no map yet");
            }
        }
        BuildingMode::PlaceManaForge => {
            commands.queue(SpawnManaForge(mouse.world_pos));
        }
        BuildingMode::PlaceMinotaur => {
            commands.queue(SpawnMinotaur(mouse.world_pos));
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
    forge_placements: Query<Entity, With<ParentBuilding>>,
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

fn handle_build_mode_changing(
    mut commands: Commands,
    mode: Res<BuildingMode>,
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
        }
        BuildingMode::Lightning | BuildingMode::PlaceCityHall | BuildingMode::PlaceLumberMill => {}
        BuildingMode::PlaceManaForge => {
            info!("Spawning building mode items for mana forge placement");
            commands.spawn((
                ParentBuilding::new(BuildingType::ManaForge),
                CursorModeItem,
                ManaLine {
                    from: Vec3::ZERO,
                    to: Vec3::ZERO,
                    disabled: true,
                },
            ));
        }
        BuildingMode::PlaceMinotaur => {
            info!("Spawning building mode items for minotaur placement");
            commands.spawn((
                ParentBuilding::new(BuildingType::Minotaur),
                CursorModeItem,
                ManaLine {
                    from: Vec3::ZERO,
                    to: Vec3::ZERO,
                    disabled: true,
                },
            ));
        }
    }
}
