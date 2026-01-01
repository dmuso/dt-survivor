use bevy::prelude::*;
use crate::element::Element;

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
    /// Element type of the damage (for debuff application)
    pub element: Option<Element>,
}

impl DamageEvent {
    pub fn new(target: Entity, amount: f32) -> Self {
        Self {
            target,
            amount,
            source: None,
            element: None,
        }
    }

    pub fn with_source(target: Entity, amount: f32, source: Entity) -> Self {
        Self {
            target,
            amount,
            source: Some(source),
            element: None,
        }
    }

    pub fn with_element(target: Entity, amount: f32, element: Element) -> Self {
        Self {
            target,
            amount,
            source: None,
            element: Some(element),
        }
    }

    pub fn with_source_and_element(
        target: Entity,
        amount: f32,
        source: Entity,
        element: Element,
    ) -> Self {
        Self {
            target,
            amount,
            source: Some(source),
            element: Some(element),
        }
    }

    /// Check if this damage is of the specified element type
    pub fn is_element(&self, element: Element) -> bool {
        self.element == Some(element)
    }

    /// Check if this is poison damage
    pub fn is_poison(&self) -> bool {
        self.is_element(Element::Poison)
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
            assert!(event.element.is_none());
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
            assert!(event.element.is_none());
        }

        #[test]
        fn test_damage_event_with_element() {
            let mut world = World::new();
            let target = world.spawn_empty().id();
            let event = DamageEvent::with_element(target, 30.0, Element::Poison);

            assert_eq!(event.target, target);
            assert_eq!(event.amount, 30.0);
            assert!(event.source.is_none());
            assert_eq!(event.element, Some(Element::Poison));
        }

        #[test]
        fn test_damage_event_with_source_and_element() {
            let mut world = World::new();
            let target = world.spawn_empty().id();
            let source = world.spawn_empty().id();
            let event = DamageEvent::with_source_and_element(target, 40.0, source, Element::Fire);

            assert_eq!(event.target, target);
            assert_eq!(event.amount, 40.0);
            assert_eq!(event.source, Some(source));
            assert_eq!(event.element, Some(Element::Fire));
        }

        #[test]
        fn test_damage_event_is_element() {
            let mut world = World::new();
            let target = world.spawn_empty().id();
            let event = DamageEvent::with_element(target, 25.0, Element::Poison);

            assert!(event.is_element(Element::Poison));
            assert!(!event.is_element(Element::Fire));
        }

        #[test]
        fn test_damage_event_is_poison() {
            let mut world = World::new();
            let target = world.spawn_empty().id();

            let poison_event = DamageEvent::with_element(target, 25.0, Element::Poison);
            assert!(poison_event.is_poison());

            let fire_event = DamageEvent::with_element(target, 25.0, Element::Fire);
            assert!(!fire_event.is_poison());

            let no_element_event = DamageEvent::new(target, 25.0);
            assert!(!no_element_event.is_poison());
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
