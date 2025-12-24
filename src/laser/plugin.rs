use bevy::prelude::*;
use crate::states::*;
use crate::laser::systems::*;

pub fn plugin(app: &mut App) {
    app
        .add_systems(Update, (
            update_laser_beams,
            laser_beam_collision_system,
            render_laser_beams,
        ).run_if(in_state(GameState::InGame)));
}