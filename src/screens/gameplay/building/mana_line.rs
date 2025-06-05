//! Code for drawing mana lines

use bevy::{color::palettes::tailwind::SKY_500, prelude::*};
#[cfg(target_os = "macos")]
use bevy_simple_subsecond_system::hot;
use bevy_vector_shapes::prelude::*;

use crate::{
    Pause,
    screens::{Screen, gameplay::building::ManaLine},
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        draw_mana_lines.run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
}

#[cfg_attr(target_os = "macos", hot)]
fn draw_mana_lines(time: Res<Time>, mut painter: ShapePainter, mut lines: Query<&mut ManaLine>) {
    for mut line in &mut lines {
        let mana_colour = SKY_500.lighter((3. * time.elapsed_secs()).cos() / 12.0);
        painter.thickness = 2.0;
        painter.set_color(mana_colour);
        painter.cap = Cap::Round;
        painter.line(line.from, line.to);

        // work out where the mana circle goes
        if line.mana_dot_distance > 0.0 {
            let tf = painter.transform;
            painter.translate(
                line.from + (line.to - line.from).normalize_or_zero() * line.mana_dot_distance,
            );
            painter.set_color(mana_colour);
            painter.circle(2.0);
            painter.transform = tf;
        }

        // move the line
        line.mana_dot_distance =
            (line.mana_dot_distance + time.delta_secs() * 80.0) % (line.to - line.from).length();
    }
}
