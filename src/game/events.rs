use bevy::prelude::*;

/// Message fired when an enemy dies
#[derive(Message)]
pub struct EnemyDeathEvent {
    pub enemy_entity: Entity,
    pub position: Vec2,
}

/// Message fired when loot should be dropped (typically when an enemy dies)
#[derive(Message)]
pub struct LootDropEvent {
    pub position: Vec2,
}

/// Event fired when player collides with an enemy
#[derive(Message)]
pub struct PlayerEnemyCollisionEvent {
    pub player_entity: Entity,
    pub enemy_entity: Entity,
}

/// Event fired when a bullet collides with an enemy
#[derive(Message)]
pub struct BulletEnemyCollisionEvent {
    pub bullet_entity: Entity,
    pub enemy_entity: Entity,
}