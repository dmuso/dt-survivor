use bevy::prelude::*;

/// Type of entity that died (for death handling)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Player,
    Enemy,
    Projectile,
}

/// Message fired when an entity takes damage
#[derive(Message, Debug, Clone)]
pub struct DamageEvent {
    /// The entity that took damage
    pub target: Entity,
    /// Amount of damage dealt
    pub amount: f32,
    /// Source of the damage (if any)
    pub source: Option<Entity>,
}

impl DamageEvent {
    pub fn new(target: Entity, amount: f32) -> Self {
        Self {
            target,
            amount,
            source: None,
        }
    }

    pub fn with_source(target: Entity, amount: f32, source: Entity) -> Self {
        Self {
            target,
            amount,
            source: Some(source),
        }
    }
}

/// Message fired when an entity dies
#[derive(Message, Debug, Clone)]
pub struct DeathEvent {
    /// The entity that died
    pub entity: Entity,
    /// Position where the entity died
    pub position: Vec3,
    /// Type of entity that died
    pub entity_type: EntityType,
}

impl DeathEvent {
    pub fn new(entity: Entity, position: Vec3, entity_type: EntityType) -> Self {
        Self {
            entity,
            position,
            entity_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod damage_event_tests {
        use super::*;

        #[test]
        fn test_damage_event_new() {
            let mut world = World::new();
            let target = world.spawn_empty().id();
            let event = DamageEvent::new(target, 25.0);

            assert_eq!(event.target, target);
            assert_eq!(event.amount, 25.0);
            assert!(event.source.is_none());
        }

        #[test]
        fn test_damage_event_with_source() {
            let mut world = World::new();
            let target = world.spawn_empty().id();
            let source = world.spawn_empty().id();
            let event = DamageEvent::with_source(target, 50.0, source);

            assert_eq!(event.target, target);
            assert_eq!(event.amount, 50.0);
            assert_eq!(event.source, Some(source));
        }
    }

    mod death_event_tests {
        use super::*;

        #[test]
        fn test_death_event_new() {
            let mut world = World::new();
            let entity = world.spawn_empty().id();
            let position = Vec3::new(100.0, 200.0, 0.0);
            let event = DeathEvent::new(entity, position, EntityType::Enemy);

            assert_eq!(event.entity, entity);
            assert_eq!(event.position, position);
            assert_eq!(event.entity_type, EntityType::Enemy);
        }

        #[test]
        fn test_entity_type_equality() {
            assert_eq!(EntityType::Player, EntityType::Player);
            assert_eq!(EntityType::Enemy, EntityType::Enemy);
            assert_eq!(EntityType::Projectile, EntityType::Projectile);
            assert_ne!(EntityType::Player, EntityType::Enemy);
        }
    }
}
