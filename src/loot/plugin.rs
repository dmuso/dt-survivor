use bevy::prelude::*;
use crate::states::*;
use crate::loot::systems::*;
use crate::loot::events::*;
use crate::game::events::LootDropEvent;

/// Resource to debounce loot pickup sounds.
/// Only one sound plays within a random 100-250ms window to prevent audio spam.
#[derive(Resource)]
pub struct LootSoundCooldown {
    pub timer: Timer,
}

impl Default for LootSoundCooldown {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.1, TimerMode::Once); // Start with min duration
        // Start finished so first sound plays immediately
        timer.tick(std::time::Duration::from_secs_f32(0.1));
        Self { timer }
    }
}

impl LootSoundCooldown {
    /// Reset the cooldown with a random duration between 100-250ms
    pub fn reset_random(&mut self) {
        use rand::Rng;
        let duration = rand::thread_rng().gen_range(0.1..=0.25);
        self.timer.set_duration(std::time::Duration::from_secs_f32(duration));
        self.timer.reset();
    }
}

/// System to tick the loot sound cooldown timer
pub fn tick_loot_sound_cooldown(mut cooldown: ResMut<LootSoundCooldown>, time: Res<Time>) {
    cooldown.timer.tick(time.delta());
}

pub fn plugin(app: &mut App) {
    app
        .add_message::<LootDropEvent>()
        .add_message::<PickupEvent>()
        .add_message::<ItemEffectEvent>()
        .init_resource::<LootSoundCooldown>()
        .add_systems(Update, (
            // Tick the sound cooldown timer
            tick_loot_sound_cooldown.run_if(in_state(GameState::InGame)),
            // Loot drop system spawns DroppedItem entities from enemy deaths
            loot_drop_system.run_if(in_state(GameState::InGame)),

            // ECS-based pickup systems - ordered pipeline
            // 1. Detect when items enter pickup radius
            detect_pickup_collisions.run_if(in_state(GameState::InGame)),
            // 2. Start pop-up animation when pickup event received
            start_popup_animation.run_if(in_state(GameState::InGame)),
            // 3. Animate the pop-up (fly up, then fall)
            animate_popup.run_if(in_state(GameState::InGame)),
            // 4. Attract items toward player
            update_item_attraction.run_if(in_state(GameState::InGame)),
            // 5. Move items based on velocity
            update_item_movement.run_if(in_state(GameState::InGame)),
            // 6. Complete pickup when items reach player
            complete_pickup_when_close.run_if(in_state(GameState::InGame)),
            // 7. Apply pickup effects
            apply_item_effects.run_if(in_state(GameState::InGame)),
            // 8. Cleanup consumed items
            cleanup_consumed_items.run_if(in_state(GameState::InGame)),
        ));
}