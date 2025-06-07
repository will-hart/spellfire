//! Code for drawing mana lines

use bevy::{color::palettes::tailwind::SKY_500, prelude::*};
// use bevy_simple_subsecond_system::hot;
use bevy_vector_shapes::prelude::*;

use crate::{
    Pause,
    screens::{
        Screen,
        gameplay::building::{ManaLine, ManaLineBalls},
    },
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (draw_mana_lines, draw_mana_balls)
            .run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
}

// #[cfg_attr(target_os = "macos", hot)]
fn draw_mana_lines(time: Res<Time>, mut painter: ShapePainter, lines: Query<&ManaLine>) {
    for line in &lines {
        if line.disabled {
            continue;
        }

        let mana_colour = SKY_500.lighter((3. * time.elapsed_secs()).cos() / 12.0);
        painter.thickness = 2.0;
        painter.set_color(mana_colour);
        painter.cap = Cap::Round;
        painter.line(line.from, line.to);
    }
}

// #[cfg_attr(target_os = "macos", hot)]
fn draw_mana_balls(
    time: Res<Time>,
    mut painter: ShapePainter,
    mut balls: Query<(&mut ManaLineBalls, &ManaLine)>,
) {
    for (mut ball, line) in &mut balls {
        if line.disabled {
            continue;
        }

        let mana_colour = SKY_500.lighter((3. * time.elapsed_secs()).cos() / 12.0);

        // work out where the mana circle goes
        if ball.mana_dot_distance > 0.0 {
            let tf = painter.transform;
            painter.translate(
                line.from + (line.to - line.from).normalize_or_zero() * ball.mana_dot_distance,
            );
            painter.set_color(mana_colour);
            painter.circle(2.0);
            painter.transform = tf;
        }

        // move the line
        ball.mana_dot_distance =
            (ball.mana_dot_distance + time.delta_secs() * 80.0) % (line.to - line.from).length();
    }
}
