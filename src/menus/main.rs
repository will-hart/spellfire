//! The main menu (seen on the title screen).

use crate::{
    asset_tracking::ResourceHandles,
    menus::Menu,
    screens::{EndlessMode, Screen},
    theme::{node_builder::NodeBuilder, widget},
};
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Main), spawn_main_menu);
}

fn spawn_main_menu(mut commands: Commands) {
    commands.remove_resource::<EndlessMode>();

    commands.spawn((
        Name::new("Main Menu"),
        NodeBuilder::new()
            .width(Val::Percent(100.0))
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
            widget::button_menu("Endless Mode", enter_loading_or_gameplay_screen),
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
