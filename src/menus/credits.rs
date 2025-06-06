//! The credits menu.

use bevy::{
    ecs::spawn::SpawnIter, input::common_conditions::input_just_pressed, prelude::*, ui::Val::*,
};

use crate::{
    asset_tracking::LoadResource, audio::music, menus::Menu, screens::Screen, theme::prelude::*,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Credits), spawn_credits_menu);
    app.add_systems(
        Update,
        go_back.run_if(in_state(Menu::Credits).and(input_just_pressed(KeyCode::Escape))),
    );

    app.register_type::<CreditsAssets>();
    app.load_resource::<CreditsAssets>();
    app.add_systems(OnEnter(Screen::Title), start_credits_music);
}

fn spawn_credits_menu(mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Credits Menu"),
        GlobalZIndex(2),
        StateScoped(Menu::Credits),
        children![
            widget::header("Created by"),
            created_by(),
            widget::header("Assets"),
            assets(),
            widget::button("Back", go_back_on_click),
        ],
    ));
}

fn created_by() -> impl Bundle {
    grid(vec![["Will Hart", "Created for bevy jam 6"]])
}

fn assets() -> impl Bundle {
    grid(vec![[
        "Bevy logo",
        "All rights reserved by the Bevy Foundation, permission granted for splash screen use when unmodified",
    ]])
}

fn grid(content: Vec<[&'static str; 2]>) -> impl Bundle {
    (
        Name::new("Grid"),
        Node {
            display: Display::Grid,
            row_gap: Px(10.0),
            column_gap: Px(30.0),
            grid_template_columns: RepeatedGridTrack::px(2, 400.0),
            ..default()
        },
        Children::spawn(SpawnIter(content.into_iter().flatten().enumerate().map(
            |(i, text)| {
                (
                    widget::label(text),
                    Node {
                        justify_self: if i % 2 == 0 {
                            JustifySelf::End
                        } else {
                            JustifySelf::Start
                        },
                        ..default()
                    },
                )
            },
        ))),
    )
}

fn go_back_on_click(_: Trigger<Pointer<Click>>, mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}

fn go_back(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Main);
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct CreditsAssets {
    #[dependency]
    menu_music: Handle<AudioSource>,
}

impl FromWorld for CreditsAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            menu_music: assets.load("audio/music/spellfire_menu_theme.ogg"),
        }
    }
}

fn start_credits_music(mut commands: Commands, credits_music: Res<CreditsAssets>) {
    commands.spawn((
        Name::new("Credits Music"),
        StateScoped(Screen::Title),
        music(credits_music.menu_music.clone()),
    ));
}
