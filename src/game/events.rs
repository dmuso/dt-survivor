use bevy::prelude::*;

/// Message fired when an enemy dies
#[derive(Message)]
pub struct EnemyDeathEvent {
    pub enemy_entity: Entity,
    pub position: Vec2,
}