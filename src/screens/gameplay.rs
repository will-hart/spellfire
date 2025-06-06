//! The screen state for the main gameplay.

use bevy::{
    color::palettes::tailwind::{SLATE_700, SLATE_800},
    ecs::relationship::RelatedSpawnerCommands,
    input::common_conditions::input_just_pressed,
    math::CompassOctant,
    prelude::*,
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
            BuildingAssets, ManaLine, ParentManaForge, SpawnManaForge, SpawnMinotaur,
        },
    },
    theme::node_builder::NodeBuilder,
    wildfire::{GameMap, OnLightningStrike, WindDirection},
};

mod building;
pub use building::BuildingType;

pub(super) fn plugin(app: &mut App) {
    app.register_type::<EnergyTextMarker>();
    app.register_type::<BuildingMode>();
    app.register_type::<CursorModeItem>();
    app.register_type::<PlayerResources>();
    app.register_type::<BuildTextHint>();
    app.register_type::<BuildTextMarker>();
    app.register_type::<EndlessMode>();
    app.register_type::<BuildingHintToolbar>();

    app.init_resource::<BuildingMode>();
    app.init_resource::<BuildTextHint>();

    app.add_plugins(building::plugin);

    app.add_systems(OnEnter(Screen::Gameplay), (spawn_level, spawn_toolbar));

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
            update_toolbar.run_if(resource_exists::<PlayerResources>),
            update_build_hint_ui,
            handle_mouse_click_input.run_if(input_just_pressed(MouseButton::Left)),
            handle_build_mode_change
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
}

impl Default for PlayerResources {
    fn default() -> Self {
        Self { mana: 50 }
    }
}

#[derive(Resource, Reflect, Debug, Clone, Default)]
#[reflect(Resource)]
pub enum BuildingMode {
    #[default]
    None,
    Lightning,
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
    mode: Res<BuildingMode>,
    mouse: Res<MousePosition>,
    maybe_map: Option<Res<GameMap>>,
) {
    match *mode {
        BuildingMode::None => {}
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
    mode: BuildingMode,
    image: Handle<Image>,
    hover_text: impl Into<String>,
    selected_text: impl Into<String>,
) {
    let mode = mode.clone();
    let selected = selected_text.into();
    let hover = hover_text.into();

    toolbar
        .spawn((
            NodeBuilder::new()
                // .width(Val::Px(200.0))
                .height(Val::Px(32.0))
                .center_content()
                .margin(UiRect::right(Val::Px(10.0)))
                .build(),
            Button,
            children![ImageNode { image, ..default() }],
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

fn spawn_toolbar(mut commands: Commands, building_assets: Res<BuildingAssets>) {
    commands
        .spawn((
            toolbar_node()
                .top(0.0)
                .justify(JustifyContent::SpaceBetween)
                .align_content(AlignContent::SpaceBetween)
                .build(),
            StateScoped(Screen::Gameplay),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Resource Toolbar"),
                NodeBuilder::new().height(Val::Px(35.0)).center_content().build(),
                children![
                    (
                        EnergyTextMarker,
                        Text::new("ENERGY: 10"),
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

                    toolbar_button(
                        toolbar,
                        BuildingMode::Lightning,
                        building_assets.lightning.clone(),
                        "Lightning Bolt. Be a pyro and start some fires :D",
                        "Click to trigger a lightning bolt, press <space> to stop."
                    );

                    toolbar_button(toolbar,
                         BuildingMode::PlaceManaForge,
                         building_assets.mana_forge.clone(),
                          "MANA FORGE. Cost: 50 Mana. Produces Mana (3/sec), powers other buildings.",
                           "Click the map to place a forge. Press <space> to cancel placement."
                    );

                    toolbar_button(toolbar,
                        BuildingMode::PlaceMinotaur,
                        building_assets.minotaur.clone(),
                        "MINOTAUR HUTCH. Cost: 40 Mana. The minotaur inside consumes 1 mana / sec and turns trees into grass into dirt.",
                        "Click the map to place a minotaur camp (close to a mana forge). Press <space> to cancel placement."
                    );
                });
        });

    commands.spawn((
        Name::new("Build text toolbar"),
        BuildingHintToolbar,
        StateScoped(Screen::Gameplay),
        toolbar_node()
            .top(35.0)
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
    mut energy_text: Single<&mut Text, (Without<WindTextMarker>, With<EnergyTextMarker>)>,
    mut wind_text: Single<&mut Text, (Without<EnergyTextMarker>, With<WindTextMarker>)>,
) {
    let cell_state = if let Some(cell) = map.tile_at_world_pos(mouse.world_pos) {
        format!("{}", *cell)
    } else {
        String::new()
    };

    energy_text.0 = format!("MANA: {}", player_resource.mana);
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
    build_text: Res<BuildTextHint>,
    mut toolbar: Single<&mut Visibility, With<BuildingHintToolbar>>,
    mut hint_text: Single<&mut Text, With<BuildTextMarker>>,
) {
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
    forge_placements: Query<Entity, With<ParentManaForge>>,
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

fn handle_build_mode_change(
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
        BuildingMode::Lightning | BuildingMode::PlaceManaForge => {}
        BuildingMode::PlaceMinotaur => {
            info!("Spawning building mode items for minotaur placement");
            commands.spawn((
                ParentManaForge(None),
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
