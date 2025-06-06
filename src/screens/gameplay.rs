//! The screen state for the main gameplay.

use std::time::Duration;

use bevy::{
    color::palettes::tailwind::{SLATE_700, SLATE_800},
    ecs::relationship::RelatedSpawnerCommands,
    input::common_conditions::input_just_pressed,
    math::CompassOctant,
    prelude::*,
    time::common_conditions::on_timer,
    ui::Val::*,
};

use crate::{
    Pause,
    demo::level::spawn_level,
    input::MousePosition,
    menus::Menu,
    screens::{
        Screen,
        gameplay::building::{
            BuildingAssets, ManaLine, ParentBuilding, ResourceAssets, SpawnCityHall,
            SpawnLumberMill, SpawnManaForge, SpawnMinotaur,
        },
    },
    theme::node_builder::NodeBuilder,
    wildfire::{GameMap, OnLightningStrike, WindDirection},
};

mod building;
pub mod story_mode;
mod victory;

pub use building::{BuildingType, CityHall, RequiresCityHall};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<ToolbarUi>();
    app.register_type::<EnergyTextMarker>();
    app.register_type::<LumberTextMarker>();
    app.register_type::<BuildingMode>();
    app.register_type::<CursorModeItem>();
    app.register_type::<PlayerResources>();
    app.register_type::<BuildTextHint>();
    app.register_type::<BuildTextMarker>();
    app.register_type::<EndlessMode>();
    app.register_type::<BuildingHintToolbar>();

    app.init_resource::<BuildingMode>();
    app.init_resource::<BuildTextHint>();

    app.add_plugins((building::plugin, story_mode::plugin, victory::plugin));

    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_level, spawn_toolbar.after(spawn_level)),
    );

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
            update_toolbar.run_if(
                resource_exists::<PlayerResources>.and(on_timer(Duration::from_millis(300))),
            ),
            update_build_hint_ui,
            handle_mouse_click_input.run_if(input_just_pressed(MouseButton::Left)),
            handle_build_mode_changing
                .run_if(resource_changed::<BuildingMode>)
                .after(cancel_cursor_mode),
        )
            .chain()
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );

    app.add_observer(handle_on_redraw_toolbar);
}

#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource, Default)]
pub struct EndlessMode;

#[derive(Event, Debug, Clone, Default)]
pub struct OnRedrawToolbar;

fn handle_on_redraw_toolbar(_trigger: Trigger<OnRedrawToolbar>, mut commands: Commands) {
    commands.run_system_cached(spawn_toolbar);
}

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

#[derive(Resource, Reflect, Debug, Default)]
#[reflect(Resource)]
pub struct BuildTextHint(pub Option<String>);

impl BuildTextHint {
    /// Clears the text
    pub fn clear(&mut self) {
        self.0 = None;
    }

    /// Sets the text
    pub fn set(&mut self, text: impl Into<String>) {
        self.0 = Some(text.into());
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

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct EnergyTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct LumberTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct WindTextMarker;

#[derive(Component, Reflect, Debug, Clone, Copy)]
#[reflect(Component)]
pub struct BuildingHintToolbar;

fn toolbar_node() -> NodeBuilder {
    NodeBuilder::new()
        .position(PositionType::Absolute)
        .width(Val::Percent(100.0))
        .height(Val::Px(35.0))
        .padding(UiRect::horizontal(Val::Px(10.0)))
        .left(0.0)
        .background(SLATE_800)
        .flex_direction(FlexDirection::Row)
}

fn toolbar_button(
    toolbar: &mut RelatedSpawnerCommands<ChildOf>,
    button_label: impl Into<String>,
    mode: BuildingMode,
    image: Handle<Image>,
    hover_text: impl Into<String>,
    selected_text: impl Into<String>,
) {
    let mode = mode.clone();
    let label = button_label.into();
    let selected = selected_text.into();
    let hover = hover_text.into();

    toolbar
        .spawn((
            NodeBuilder::new()
                // .width(Val::Px(200.0))
                .height(Val::Px(32.0))
                .center_content()
                .background(SLATE_800)
                .margin(UiRect::right(Val::Px(10.0)))
                .build(),
            Button,
            children![
                (
                    NodeBuilder::new()
                        .margin(UiRect::horizontal(Val::Px(5.0)))
                        .build(),
                    ImageNode { image, ..default() }
                ),
                (Text::new(label), TextFont::from_font_size(12.0),)
            ],
        ))
        .observe(
            move |_trigger: Trigger<Pointer<Over>>,
                  mode: Res<BuildingMode>,
                  mut hint: ResMut<BuildTextHint>| {
                if !matches!(*mode, BuildingMode::None) {
                    return;
                }

                hint.set(hover.clone());
            },
        )
        .observe(
            move |_trigger: Trigger<Pointer<Click>>,
                  mut new_mode: ResMut<BuildingMode>,
                  mut hint: ResMut<BuildTextHint>| {
                info!("Setting building mode to {mode:?}");
                *new_mode = mode.clone();
                hint.set(selected.clone());
            },
        )
        .observe(
            |_trigger: Trigger<Pointer<Out>>,
             mode: Res<BuildingMode>,
             mut hint: ResMut<BuildTextHint>| {
                if matches!(*mode, BuildingMode::None) {
                    hint.clear();
                }
            },
        );
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct ToolbarUi;

fn spawn_toolbar(
    mut commands: Commands,
    requires_city_hall: Option<Res<RequiresCityHall>>,
    maybe_endless_mode: Option<Res<EndlessMode>>,
    resource_assets: Res<ResourceAssets>,
    building_assets: Res<BuildingAssets>,
    previous_toolbars: Query<Entity, With<ToolbarUi>>,
) {
    for previous in &previous_toolbars {
        commands.entity(previous).despawn();
    }

    let requires_city_hall = requires_city_hall.is_some();

    commands
        .spawn((
            ToolbarUi,
            toolbar_node()
                .top(0.0)
                .justify(JustifyContent::SpaceBetween)
                .align_content(AlignContent::SpaceBetween)
                .build(),
            StateScoped(Screen::Gameplay),
        ))
        .with_children(|parent| {
            if requires_city_hall {
                return;
            }

            parent.spawn((
                Name::new("Resource Toolbar"),
                NodeBuilder::new().height(Val::Px(35.0)).center_content().build(),
                children![
                    (
                        Node{
                          margin: UiRect::right(Val::Px(5.0)),
                          ..default()
                        },
                        ImageNode {
                            image: resource_assets.resource_icons.clone(),
                            rect: Some(Rect::from_corners(Vec2::ZERO, Vec2::splat(16.0))),
                            ..default()
                        }
                    ),
                    (
                        EnergyTextMarker,
                        NodeBuilder::new().background(SLATE_800).center_content().margin(UiRect::horizontal(Val::Px(5.0))).build(),
                        Text::new("0"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        }
                    ),
                    (
                        Node {
                            margin: UiRect::right(Val::Px(5.0)),
                            ..default()
                        },
                        ImageNode {
                            image: resource_assets.resource_icons.clone(),
                            rect: Some(Rect::from_corners(Vec2::new(16.0, 0.0), Vec2::new(32.0, 16.0))),
                            ..default()
                        },
                    ),
                    (
                        LumberTextMarker,
                        NodeBuilder::new().background(SLATE_800).center_content().margin(UiRect::horizontal(Val::Px(5.0))).build(),
                        Text::new("0"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        }
                    ),
                    (
                        WindTextMarker,
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        }
                    ),
                ],
            ));

            parent
                .spawn((
                    Name::new("Building button toolbar"),
                    NodeBuilder::new().center_content().build(),
                ))
                .with_children(|toolbar| {
                    toolbar.spawn((
                        NodeBuilder::new().margin(UiRect::right(Val::Px(10.0))).build(),
                        Text::new("Buildings: "),
                        TextFont::from_font_size(12.)
                    ));

                    #[cfg(debug_assertions)]
                    let show_bolt_in_story = true;
                    #[cfg(not(debug_assertions))]
                    let show_bolt_in_story = false;

                    if maybe_endless_mode.is_some() || show_bolt_in_story{
                        toolbar_button(
                            toolbar,
                            "Lightning",
                            BuildingMode::Lightning,
                            building_assets.lightning.clone(),
                            "Lightning Bolt. Be a pyro and start some fires :D",
                            "Click to trigger a lightning bolt, press <space> to stop."
                        );
                    }

                    toolbar_button(toolbar,
                        "Mill",
                         BuildingMode::PlaceLumberMill,
                         building_assets.lumber_mill.clone(),
                          "LUMBER MILL. Cost: 30 Lumber. Produces Lumber from nearby trees every (0.5 sec). Doesn't require a Mana Forge nearby.",
                           "Click the map to place a lumber mill. Press <space> to cancel placement."
                    );

                    toolbar_button(toolbar,
                        "Mana Forge",
                         BuildingMode::PlaceManaForge,
                         building_assets.mana_forge.clone(),
                          "MANA FORGE. Cost: 50 Lumber. Produces Mana (3/sec), required for most other buildings.",
                           "Click the map to place a forge. Press <space> to cancel placement."
                    );

                    toolbar_button(toolbar,
                        "Minotaur",
                        BuildingMode::PlaceMinotaur,
                        building_assets.minotaur.clone(),
                        "MINOTAUR HUTCH. Cost: 40 Mana. The minotaur inside consumes 1 mana / sec and turns trees into grass into dirt. Requires Mana Forge nearby.",
                        "Click the map to place a minotaur camp (close to a mana forge). Press <space> to cancel placement."
                    );
                });
        });

    commands.spawn((
        Name::new("Build text toolbar"),
        ToolbarUi,
        BuildingHintToolbar,
        StateScoped(Screen::Gameplay),
        toolbar_node()
            .top(if requires_city_hall { 0.0 } else { 35.0 })
            .center_content()
            .background(SLATE_700)
            .build(),
        children![(
            BuildTextMarker,
            Text::new(""),
            TextFont::from_font_size(12.0),
        )],
    ));
}

fn update_toolbar(
    player_resource: Res<PlayerResources>,
    wind: Res<WindDirection>,
    mouse: Res<MousePosition>,
    map: Res<GameMap>,
    mut energy_text: Single<
        &mut Text,
        (
            Without<LumberTextMarker>,
            Without<WindTextMarker>,
            With<EnergyTextMarker>,
        ),
    >,
    mut wind_text: Single<
        &mut Text,
        (
            Without<LumberTextMarker>,
            Without<EnergyTextMarker>,
            With<WindTextMarker>,
        ),
    >,
    mut lumber_text: Single<
        &mut Text,
        (
            With<LumberTextMarker>,
            Without<EnergyTextMarker>,
            Without<WindTextMarker>,
        ),
    >,
) {
    let cell_state = if let Some(cell) = map.tile_at_world_pos(mouse.world_pos) {
        format!("{}", *cell)
    } else {
        String::new()
    };

    energy_text.0 = format!(
        "{} ({:+})",
        player_resource.mana, player_resource.mana_drain
    );
    lumber_text.0 = format!("{}", player_resource.lumber);
    wind_text.0 = format!(
        " | WIND: From {} / {} | {cell_state}",
        match CompassOctant::from(Dir2::new(wind.0).expect("to dir")) {
            CompassOctant::North => "N",
            CompassOctant::NorthEast => "NE",
            CompassOctant::East => "E",
            CompassOctant::SouthEast => "SE",
            CompassOctant::South => "S",
            CompassOctant::SouthWest => "SW",
            CompassOctant::West => "W",
            CompassOctant::NorthWest => "NW",
        },
        wind.0
    );
}

fn update_build_hint_ui(
    maybe_requires_city_hall: Option<Res<RequiresCityHall>>,
    build_text: Res<BuildTextHint>,
    mut toolbar: Single<&mut Visibility, With<BuildingHintToolbar>>,
    mut hint_text: Single<&mut Text, With<BuildTextMarker>>,
) {
    if maybe_requires_city_hall.is_some() {
        **toolbar = Visibility::Visible;
        hint_text.0 = "Click to place your city hall on grass or trees. Take care of this building, if you lose it everything is lost!".into();
        return;
    }

    if let Some(text) = &build_text.0 {
        **toolbar = Visibility::Visible;
        if hint_text.0 != *text {
            hint_text.0 = text.clone();
        }
    } else {
        **toolbar = Visibility::Hidden;
    }
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
