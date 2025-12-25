use bevy::prelude::*;

/// Event fired when player collects the Whisper drop
#[derive(Message)]
pub struct WhisperCollectedEvent {
    pub player_entity: Entity,
    pub whisper_drop_entity: Entity,
    pub position: Vec2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_collected_event_creation() {
        let event = WhisperCollectedEvent {
            player_entity: Entity::from_bits(1),
            whisper_drop_entity: Entity::from_bits(2),
            position: Vec2::new(100.0, 200.0),
        };
        assert_eq!(event.position, Vec2::new(100.0, 200.0));
    }
}
