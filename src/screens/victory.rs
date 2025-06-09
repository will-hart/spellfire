//! The victory screen

use bevy::prelude::*;

use crate::audio::sound_effect;
use crate::screens::{GameOverAssets, NextStoryLevel, Screen, StoryModeLevel, get_level_data};
use crate::theme::widget;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::LevelWon), spawn_level_victory_screen);
}

fn spawn_level_victory_screen(
    mut commands: Commands,
    story_level: Res<StoryModeLevel>,
    game_over_assets: Res<GameOverAssets>,
    mut next_level: ResMut<NextStoryLevel>,
) {
    next_level.0 = story_level.level_number + 1;
    let has_next = get_level_data(next_level.0).is_some();

    if has_next {
        commands.spawn(sound_effect(game_over_assets.you_won.clone()));
    } else {
        commands.spawn(sound_effect(game_over_assets.you_survived.clone()));
    }

    commands
        .spawn((
            widget::ui_root("Level Victory Menu"),
            StateScoped(Screen::LevelWon),
            children![
                widget::header("VICTORY!"),
                (
                    Text::new("Your have successfully defended your City Hall!"),
                    TextFont::from_font_size(24.0),
                ),
            ],
        ))
        .with_children(|parent| {
            if has_next {
                parent.spawn((widget::button(
                    "Next Level",
                    |_trigger: Trigger<Pointer<Click>>, mut next: ResMut<NextState<Screen>>| {
                        next.set(Screen::Gameplay);
                    },
                ),));
            }

            parent.spawn(widget::button(
                if has_next {
                    "Flee to the menu"
                } else {
                    "Retire with honour!"
                },
                |_trigger: Trigger<Pointer<Click>>, mut next: ResMut<NextState<Screen>>| {
                    next.set(Screen::Title);
                },
            ));
        });
}
