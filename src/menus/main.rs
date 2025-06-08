//! The main menu (seen on the title screen).

use crate::{
    asset_tracking::ResourceHandles,
    menus::Menu,
    screens::{EndlessMode, NextStoryLevel, Screen, StoryModeLevel},
    theme::{node_builder::NodeBuilder, widget},
};
use bevy::{
    audio::Volume,
    color::palettes::{css::WHITE, tailwind::SLATE_300},
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Main), spawn_main_menu);
    app.add_systems(Startup, lower_volume_you_psychos);
}

fn lower_volume_you_psychos(mut vol: ResMut<GlobalVolume>) {
    *vol = GlobalVolume::from(Volume::Linear(0.5));
}

fn spawn_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.remove_resource::<EndlessMode>();
    commands.remove_resource::<StoryModeLevel>();
    commands.insert_resource(NextStoryLevel::default());

    commands.spawn((
        Name::new("Main Menu Hints"),
        GlobalZIndex(2),
        StateScoped(Menu::Main),
        NodeBuilder::new()
            .width(Val::Percent(100.0))
            .height(Val::Percent(80.0))
            .position(PositionType::Absolute)
            .top(0.0)
            .padding(UiRect::all(Val::Px(20.0)))
            .center_content()
            .flex_direction(FlexDirection::Row)
            .build(),
        children![
            (
                NodeBuilder::new()
                    .width(Val::Percent(50.0))
                    .flex_direction(FlexDirection::Column)
                    .row_gap(Val::Px(20.0))
                    .center_content()
                    .build(),
                children![
                    ImageNode::new(asset_server.load_with_settings(
                        // This should be an embedded asset for instant loading, but that is
                        // currently [broken on Windows Wasm builds](https://github.com/bevyengine/bevy/issues/14246).
                        "images/logo.png",
                        |settings: &mut ImageLoaderSettings| {
                            // Make an exception for the splash image in case
                            // `ImagePlugin::default_nearest()` is used for pixel art.
                            settings.sampler = ImageSampler::linear();
                        },
                    )),
                    (
                        Text::new("A spell-slinging, wildfire-fighting strategy game"),
                        TextFont::from_font_size(24.0),
                    ),
                    (
                        Text::new("Made for Bevy Jam 6 by Will Hart"),
                        TextFont::from_font_size(24.0),
                    )
                ],
            ),
            (
                NodeBuilder::new()
                    .width(Val::Percent(50.0))
                    .flex_direction(FlexDirection::Column)
                    .padding(UiRect::horizontal(Val::Px(20.0)))
                    .row_gap(Val::Px(20.0))
                    .build(),
                children![
                    (Text::new("Instructions"), TextFont::from_font_size(36.0),),
                    (
                        Text::new("You raided their dungeon one too many times and now the Goblin Shaman are out for revenge! They've vowed to use the elemental powers of wind and fire to try to destroy your city. But we aren't defenceless - we can use our own magical powers to save our homes!"),
                        TextFont::from_font_size(20.0),
                        TextColor(SLATE_300.into()),
                    ),
                    (
                        Text::new("Protect your City Hall at all costs. Place buildings on the map (see the tooltips) to produce mana and harvest lumber. Summon creatures to defend your City Hall against the raging wildfire!"),
                        TextFont::from_font_size(20.0),
                        TextColor(SLATE_300.into()),
                    )
,
                    (
                        Text::new("Be careful not to let the flames too close to your buildings - if one burns, the magical backlash may trigger some kind of reaction!"),
                        TextFont::from_font_size(20.0),
                        TextColor(WHITE.into()),
)
                ]
            )
        ],
    ));

    commands.spawn((
        Name::new("Main Menu Buttons"),
        NodeBuilder::new()
            .width(Val::Percent(100.0))
            .height(Val::Percent(20.0))
            .position(PositionType::Absolute)
            .bottom(0.0)
            .flex_direction(FlexDirection::Row)
            .padding(UiRect::all(Val::Px(40.0)))
            .center_content()
            .build(),
        GlobalZIndex(2),
        StateScoped(Menu::Main),
        #[cfg(not(target_family = "wasm"))]
        children![
            widget::button_menu("Story Mode", enter_loading_or_gameplay_screen),
            widget::button_menu("Endless Mode", enter_loading_or_gameplay_screen_endless),
            widget::button_menu("Settings", open_settings_menu),
            widget::button_menu("Credits", open_credits_menu),
            widget::button_menu("Exit", exit_app),
        ],
        #[cfg(target_family = "wasm")]
        children![
            widget::button_menu("Story Mode", enter_loading_or_gameplay_screen),
            widget::button_menu("Endless Mode", enter_loading_or_gameplay_screen_endless),
            widget::button_menu("Settings", open_settings_menu),
            widget::button_menu("Credits", open_credits_menu),
        ],
    ));
}

fn enter_loading_or_gameplay_screen(
    _: Trigger<Pointer<Click>>,
    resource_handles: Res<ResourceHandles>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    if resource_handles.is_all_done() {
        next_screen.set(Screen::Gameplay);
    } else {
        next_screen.set(Screen::Loading);
    }
}

fn enter_loading_or_gameplay_screen_endless(
    _: Trigger<Pointer<Click>>,
    mut commands: Commands,
    resource_handles: Res<ResourceHandles>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    commands.init_resource::<EndlessMode>();

    if resource_handles.is_all_done() {
        next_screen.set(Screen::Gameplay);
    } else {
        next_screen.set(Screen::Loading);
    }
}

fn open_settings_menu(_: Trigger<Pointer<Click>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Settings);
}

fn open_credits_menu(_: Trigger<Pointer<Click>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Credits);
}

#[cfg(not(target_family = "wasm"))]
fn exit_app(_: Trigger<Pointer<Click>>, mut app_exit: EventWriter<AppExit>) {
    app_exit.write(AppExit::Success);
}
