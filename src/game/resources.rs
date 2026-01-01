use bevy::prelude::*;

/// Tracks whether the next InGame entry should reset game state.
/// Set to true when starting a fresh game (from Intro/GameOver).
/// Set to false when continuing from LevelComplete.
#[derive(Resource, Default)]
pub struct FreshGameStart(pub bool);

impl FreshGameStart {
    pub fn new() -> Self {
        Self(true) // Default to fresh start
    }
}

/// Configuration for game level progression
#[derive(Debug, Clone)]
pub struct LevelConfig {
    /// Base number of kills needed to advance from level 1
    pub base_kills: u32,
    /// Multiplier applied each level (kills_needed = base_kills * multiplier^(level-1))
    pub kill_multiplier: f32,
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            base_kills: 10,
            kill_multiplier: 1.5,
        }
    }
}

/// Tracks game level progression
#[derive(Resource, Debug)]
pub struct GameLevel {
    /// Current game level (starts at 1)
    pub level: u32,
    /// Number of enemies killed in current level
    pub kills_this_level: u32,
    /// Total enemies killed this game
    pub total_kills: u32,
    /// Configuration for progression
    pub config: LevelConfig,
}

impl Default for GameLevel {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLevel {
    pub fn new() -> Self {
        Self {
            level: 1,
            kills_this_level: 0,
            total_kills: 0,
            config: LevelConfig::default(),
        }
    }

    /// Calculate kills needed to advance from current level
    pub fn kills_to_advance(&self) -> u32 {
        let multiplier = self.config.kill_multiplier.powi(self.level as i32 - 1);
        (self.config.base_kills as f32 * multiplier).ceil() as u32
    }

    /// Register a kill and return true if level advanced
    pub fn register_kill(&mut self) -> bool {
        self.kills_this_level += 1;
        self.total_kills += 1;

        if self.kills_this_level >= self.kills_to_advance() {
            self.level += 1;
            self.kills_this_level = 0;
            true
        } else {
            false
        }
    }

    /// Progress percentage toward next level (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        self.kills_this_level as f32 / self.kills_to_advance() as f32
    }
}

#[derive(Resource, Default)]
pub struct PlayerPosition(pub Vec2);

#[derive(Resource)]
pub struct EnemySpawnState {
    pub time_since_last_spawn: f32,
}

impl Default for EnemySpawnState {
    fn default() -> Self {
        Self {
            time_since_last_spawn: 0.0,
        }
    }
}

impl EnemySpawnState {
    /// Calculate spawn rate based on game level.
    /// Level 1: 0.6 enemies/second, then 1.5x per level.
    pub fn spawn_rate_for_level(game_level: u32) -> f32 {
        0.6 * 1.5_f32.powi(game_level.saturating_sub(1) as i32)
    }
}

#[derive(Resource, Default)]
pub struct PlayerDamageTimer {
    pub time_since_last_damage: f32,
    pub has_taken_damage: bool,
}

#[derive(Resource, Default)]
pub struct ScreenTintEffect {
    pub remaining_duration: f32,
    pub color: Color,
}

/// Tracks how long the player has survived in the current game session
#[derive(Resource, Default)]
pub struct SurvivalTime(pub f32);

/// Tracks whether the camera is in free-look mode (right mouse button held)
#[derive(Resource, Default)]
pub struct FreeCameraState {
    /// Whether free camera mode is currently active
    pub active: bool,
    /// Camera yaw (horizontal rotation) in radians
    pub yaw: f32,
    /// Camera pitch (vertical rotation) in radians
    pub pitch: f32,
}

impl FreeCameraState {
    /// Resets the camera rotation to match the isometric view angles
    pub fn reset_to_isometric(&mut self) {
        // Isometric camera looks at origin from (0, 20, 15)
        // Pitch is the angle down from horizontal: atan2(20, 15) ≈ 0.93 rad (53°)
        self.yaw = 0.0;
        self.pitch = -std::f32::consts::FRAC_PI_4; // -45 degrees (looking down)
        self.active = false;
    }
}

/// Shared mesh handles for all game entities to avoid recreating meshes
#[derive(Resource)]
pub struct GameMeshes {
    /// Player mesh (1.0 x 1.0 x 1.0 cube)
    pub player: Handle<Mesh>,
    /// Enemy mesh (0.75 x 1.5 x 0.75 rectangular prism - double height)
    pub enemy: Handle<Mesh>,
    /// Bullet mesh (0.3 x 0.3 x 0.3 cube)
    pub bullet: Handle<Mesh>,
    /// Laser beam mesh (thin elongated cube: 0.1 x 0.1 x 1.0, scaled by length)
    pub laser: Handle<Mesh>,
    /// Rocket mesh (elongated cube: 0.25 x 0.25 x 0.6)
    pub rocket: Handle<Mesh>,
    /// Explosion mesh (sphere with radius 1.0, scaled by explosion radius)
    pub explosion: Handle<Mesh>,
    /// Target marker mesh (small flat cube: 0.3 x 0.05 x 0.3)
    pub target_marker: Handle<Mesh>,
    /// Small loot mesh for XP orbs (0.4 x 0.4 x 0.4 cube)
    pub loot_small: Handle<Mesh>,
    /// Medium loot mesh for health packs and powerups (0.5 x 0.5 x 0.5 cube)
    pub loot_medium: Handle<Mesh>,
    /// Large loot mesh for weapons (0.6 x 0.6 x 0.6 cube)
    pub loot_large: Handle<Mesh>,
    /// Rock mesh (1.0 x 0.5 x 1.0 flat cube)
    pub rock: Handle<Mesh>,
    /// Whisper core mesh (small sphere for the glowing center)
    pub whisper_core: Handle<Mesh>,
    /// Lightning segment mesh (thin elongated cube for lightning bolts, 1x1 unit scaled per segment)
    pub lightning_segment: Handle<Mesh>,
    /// Whisper arc mesh (small rectangle for lightning arc effects)
    pub whisper_arc: Handle<Mesh>,
    /// Orbital particle mesh (small sphere for trail particles)
    pub orbital_particle: Handle<Mesh>,
    /// Powerup mesh (medium cube for powerup items)
    pub powerup: Handle<Mesh>,
}

impl GameMeshes {
    pub fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            player: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            enemy: meshes.add(Cuboid::new(0.75, 1.5, 0.75)),
            bullet: meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
            laser: meshes.add(Cuboid::new(0.1, 0.1, 1.0)),
            rocket: meshes.add(Cuboid::new(0.25, 0.25, 0.6)),
            explosion: meshes.add(Sphere::new(1.0)),
            target_marker: meshes.add(Cuboid::new(0.3, 0.05, 0.3)),
            loot_small: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
            loot_medium: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
            loot_large: meshes.add(Cuboid::new(0.6, 0.6, 0.6)),
            rock: meshes.add(Cuboid::new(1.0, 0.5, 1.0)),
            whisper_core: meshes.add(Sphere::new(0.15)),
            lightning_segment: meshes.add(Cuboid::new(1.0, 0.02, 0.02)),
            whisper_arc: meshes.add(Cuboid::new(0.1, 0.02, 0.02)),
            orbital_particle: meshes.add(Sphere::new(0.05)),
            powerup: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
        }
    }
}

/// Calculate enemy scale based on level
/// Base scale is 0.75, increases by 15% per level above 1
pub fn enemy_scale_for_level(level: u8) -> f32 {
    let base_scale = 0.75;
    let scale_per_level = 0.15;
    base_scale + (level.saturating_sub(1) as f32 * scale_per_level)
}

/// Materials for each enemy rarity level (1-5)
#[derive(Resource)]
pub struct EnemyLevelMaterials {
    /// Level 1 - Common (Grey)
    pub common: Handle<StandardMaterial>,
    /// Level 2 - Uncommon (Green)
    pub uncommon: Handle<StandardMaterial>,
    /// Level 3 - Rare (Blue)
    pub rare: Handle<StandardMaterial>,
    /// Level 4 - Epic (Purple)
    pub epic: Handle<StandardMaterial>,
    /// Level 5 - Legendary (Gold with emissive glow)
    pub legendary: Handle<StandardMaterial>,
}

impl EnemyLevelMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            common: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.6, 0.6),
                ..default()
            }),
            uncommon: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.8, 0.2),
                ..default()
            }),
            rare: materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.4, 1.0),
                ..default()
            }),
            epic: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.2, 0.8),
                ..default()
            }),
            legendary: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.84, 0.0),
                emissive: bevy::color::LinearRgba::rgb(2.0, 1.68, 0.0),
                ..default()
            }),
        }
    }

    /// Get material handle for a given enemy level (1-5)
    pub fn for_level(&self, level: u8) -> Handle<StandardMaterial> {
        match level {
            1 => self.common.clone(),
            2 => self.uncommon.clone(),
            3 => self.rare.clone(),
            4 => self.epic.clone(),
            _ => self.legendary.clone(),
        }
    }
}

/// Materials for each XP orb rarity level (1-5)
/// Higher level orbs have emissive glow for visual distinction
#[derive(Resource)]
pub struct XpOrbMaterials {
    /// Level 1 - Common (Grey)
    pub common: Handle<StandardMaterial>,
    /// Level 2 - Uncommon (Green)
    pub uncommon: Handle<StandardMaterial>,
    /// Level 3 - Rare (Blue)
    pub rare: Handle<StandardMaterial>,
    /// Level 4 - Epic (Purple)
    pub epic: Handle<StandardMaterial>,
    /// Level 5 - Legendary (Gold with strong emissive glow)
    pub legendary: Handle<StandardMaterial>,
}

impl XpOrbMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        // Emissive values provide glow via bloom (no PointLights on loot)
        Self {
            common: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.6, 0.6),
                emissive: bevy::color::LinearRgba::rgb(0.75, 0.75, 0.75),
                ..default()
            }),
            uncommon: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.8, 0.2),
                emissive: bevy::color::LinearRgba::rgb(0.0, 1.0, 0.25),
                ..default()
            }),
            rare: materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.4, 1.0),
                emissive: bevy::color::LinearRgba::rgb(0.25, 0.5, 1.5),
                ..default()
            }),
            epic: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.2, 0.8),
                emissive: bevy::color::LinearRgba::rgb(1.0, 0.35, 1.25),
                ..default()
            }),
            legendary: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.84, 0.0),
                emissive: bevy::color::LinearRgba::rgb(2.5, 2.1, 0.0),
                ..default()
            }),
        }
    }

    /// Get material handle for a given XP orb level (1-5)
    pub fn for_level(&self, level: u8) -> Handle<StandardMaterial> {
        match level {
            1 => self.common.clone(),
            2 => self.uncommon.clone(),
            3 => self.rare.clone(),
            4 => self.epic.clone(),
            _ => self.legendary.clone(),
        }
    }
}

/// Shared material for the damage flash effect - bright white emissive
#[derive(Resource)]
pub struct DamageFlashMaterial(pub Handle<StandardMaterial>);

/// Statistics for the current level
/// Tracks time elapsed, enemies killed, and XP gained during a game level
#[derive(Resource, Debug, Default)]
pub struct LevelStats {
    /// Time elapsed in current level (seconds)
    pub time_elapsed: f32,
    /// Enemies killed in current level
    pub enemies_killed: u32,
    /// XP gained in current level
    pub xp_gained: u32,
}

impl LevelStats {
    /// Create a new LevelStats with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset stats for a new level
    pub fn reset(&mut self) {
        self.time_elapsed = 0.0;
        self.enemies_killed = 0;
        self.xp_gained = 0;
    }

    /// Record an enemy kill
    pub fn record_kill(&mut self) {
        self.enemies_killed += 1;
    }

    /// Record XP gained
    pub fn record_xp(&mut self, amount: u32) {
        self.xp_gained += amount;
    }

    /// Format time as MM:SS
    pub fn formatted_time(&self) -> String {
        let minutes = (self.time_elapsed / 60.0) as u32;
        let seconds = (self.time_elapsed % 60.0) as u32;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Shared material handles for all game entities
#[derive(Resource)]
pub struct GameMaterials {
    /// Player material (green with slight emissive)
    pub player: Handle<StandardMaterial>,
    /// Enemy material (red)
    pub enemy: Handle<StandardMaterial>,
    /// Bullet material (yellow with emissive)
    pub bullet: Handle<StandardMaterial>,
    /// Laser beam material (cyan with strong emissive glow)
    pub laser: Handle<StandardMaterial>,
    /// Rocket pausing material (grey)
    pub rocket_pausing: Handle<StandardMaterial>,
    /// Rocket targeting material (yellow with emissive)
    pub rocket_targeting: Handle<StandardMaterial>,
    /// Rocket homing material (orange with emissive)
    pub rocket_homing: Handle<StandardMaterial>,
    /// Rocket exploding material (red with strong emissive)
    pub rocket_exploding: Handle<StandardMaterial>,
    /// Explosion material (red/orange with transparency and emissive)
    pub explosion: Handle<StandardMaterial>,
    /// Target marker material (red)
    pub target_marker: Handle<StandardMaterial>,
    /// XP orb material (light grey)
    pub xp_orb: Handle<StandardMaterial>,
    /// Health pack material (green)
    pub health_pack: Handle<StandardMaterial>,
    /// Pistol weapon loot material (yellow)
    pub weapon_pistol: Handle<StandardMaterial>,
    /// Laser weapon loot material (blue)
    pub weapon_laser: Handle<StandardMaterial>,
    /// Rocket launcher weapon loot material (orange)
    pub weapon_rocket: Handle<StandardMaterial>,
    /// Powerup material (magenta)
    pub powerup: Handle<StandardMaterial>,
    /// Rock obstacle material (grey)
    pub rock: Handle<StandardMaterial>,
    /// Whisper core material (red-orange with strong emissive glow)
    pub whisper_core: Handle<StandardMaterial>,
    /// Whisper drop material (dimmer red-orange)
    pub whisper_drop: Handle<StandardMaterial>,
    /// Lightning bolt/arc material (red-orange with HDR emissive)
    pub lightning: Handle<StandardMaterial>,
    /// Orbital particle/trail material (red-orange)
    pub orbital_particle: Handle<StandardMaterial>,
    /// Fireball projectile material (orange with emissive)
    pub fireball: Handle<StandardMaterial>,
    /// Radiant beam material (white/gold with strong emissive glow)
    pub radiant_beam: Handle<StandardMaterial>,
    /// Thunder strike effect material (yellow/electric with strong emissive glow)
    pub thunder_strike: Handle<StandardMaterial>,
    /// Thunder strike target marker material (yellow with transparency)
    pub thunder_strike_marker: Handle<StandardMaterial>,
    /// Fire nova (Inferno) material (orange-red with strong emissive glow)
    pub fire_nova: Handle<StandardMaterial>,
    /// Poison cloud projectile material (toxic green with emissive glow)
    pub poison_projectile: Handle<StandardMaterial>,
    /// Poison cloud zone material (translucent green toxic fog)
    pub poison_cloud: Handle<StandardMaterial>,
    /// Ice shard projectile material (ice blue with emissive glow)
    pub ice_shard: Handle<StandardMaterial>,
    /// Glacial pulse (Frost Nova) material (ice blue with strong emissive glow)
    pub glacial_pulse: Handle<StandardMaterial>,
}

impl GameMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            player: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.0, 0.2, 0.0),
                ..default()
            }),
            enemy: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                ..default()
            }),
            bullet: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.5, 0.5, 0.0),
                ..default()
            }),
            laser: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(0.0, 2.0, 2.0),
                unlit: true,
                ..default()
            }),
            rocket_pausing: materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.6, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.3, 0.2, 0.0),
                ..default()
            }),
            rocket_targeting: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.3, 0.3, 0.0),
                ..default()
            }),
            rocket_homing: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.3, 0.15, 0.0),
                ..default()
            }),
            rocket_exploding: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(1.0, 0.0, 0.0),
                ..default()
            }),
            explosion: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.3, 0.0, 0.5), // 50% transparency
                emissive: bevy::color::LinearRgba::rgb(1.0, 0.2, 0.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            target_marker: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.5, 0.0, 0.0),
                ..default()
            }),
            xp_orb: materials.add(StandardMaterial {
                base_color: Color::srgb(0.75, 0.75, 0.75),
                ..default()
            }),
            health_pack: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.0, 3.0, 0.5),
                ..default()
            }),
            weapon_pistol: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(3.0, 3.0, 0.5),
                ..default()
            }),
            weapon_laser: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(0.5, 1.0, 4.0),
                ..default()
            }),
            weapon_rocket: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: bevy::color::LinearRgba::rgb(3.0, 1.5, 0.0),
                ..default()
            }),
            powerup: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(3.0, 0.0, 3.0),
                ..default()
            }),
            rock: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            }),
            whisper_core: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(3.0, 3.0, 3.0),
                unlit: true,
                ..default()
            }),
            whisper_drop: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(1.5, 1.5, 1.5),
                unlit: true,
                ..default()
            }),
            lightning: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(3.0, 3.0, 3.0),
                unlit: true,
                ..default()
            }),
            orbital_particle: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(3.0, 3.0, 3.0),
                unlit: true,
                ..default()
            }),
            fireball: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0), // Orange (Fire element color)
                emissive: bevy::color::LinearRgba::rgb(2.0, 1.0, 0.0),
                ..default()
            }),
            radiant_beam: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.9), // White with slight gold tint
                emissive: bevy::color::LinearRgba::rgb(3.0, 3.0, 2.5), // Bright white/gold glow
                unlit: true,
                ..default()
            }),
            thunder_strike: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0), // Yellow (Lightning element color)
                emissive: bevy::color::LinearRgba::rgb(3.0, 3.0, 0.0), // Bright yellow glow
                unlit: true,
                ..default()
            }),
            thunder_strike_marker: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 0.0, 0.5), // Yellow with 50% transparency
                emissive: bevy::color::LinearRgba::rgb(1.0, 1.0, 0.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            fire_nova: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.3, 0.0, 0.7), // Orange-red with 70% opacity
                emissive: bevy::color::LinearRgba::rgb(2.5, 0.6, 0.0), // Bright orange glow
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            poison_projectile: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0), // Toxic green (Poison element color)
                emissive: bevy::color::LinearRgba::rgb(0.0, 2.0, 0.0), // Bright green glow
                ..default()
            }),
            poison_cloud: materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 0.8, 0.0, 0.5), // Translucent green
                emissive: bevy::color::LinearRgba::rgb(0.0, 1.0, 0.0), // Green glow
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            ice_shard: materials.add(StandardMaterial {
                base_color: Color::srgb(0.53, 0.81, 0.92), // Ice blue (Frost element color)
                emissive: bevy::color::LinearRgba::rgb(0.67, 1.0, 1.18), // Bright ice blue glow
                ..default()
            }),
            glacial_pulse: materials.add(StandardMaterial {
                base_color: Color::srgba(0.53, 0.81, 0.92, 0.7), // Ice blue with 70% opacity
                emissive: bevy::color::LinearRgba::rgb(1.0, 1.5, 1.8), // Bright ice blue glow
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod game_level_tests {
        use super::*;

        #[test]
        fn game_level_starts_at_one() {
            let level = GameLevel::new();
            assert_eq!(level.level, 1);
            assert_eq!(level.kills_this_level, 0);
            assert_eq!(level.total_kills, 0);
        }

        #[test]
        fn game_level_default_matches_new() {
            let level_new = GameLevel::new();
            let level_default = GameLevel::default();
            assert_eq!(level_new.level, level_default.level);
            assert_eq!(level_new.kills_this_level, level_default.kills_this_level);
            assert_eq!(level_new.total_kills, level_default.total_kills);
        }

        #[test]
        fn kills_to_advance_level_1() {
            let level = GameLevel::new();
            // Level 1: base_kills * 1.5^0 = 10 * 1 = 10
            assert_eq!(level.kills_to_advance(), 10);
        }

        #[test]
        fn kills_to_advance_increases_with_level() {
            let mut level = GameLevel::new();
            let kills_1 = level.kills_to_advance();
            level.level = 2;
            let kills_2 = level.kills_to_advance();
            level.level = 3;
            let kills_3 = level.kills_to_advance();
            assert!(kills_2 > kills_1);
            assert!(kills_3 > kills_2);
        }

        #[test]
        fn kills_to_advance_formula_is_correct() {
            let mut level = GameLevel::new();
            // Level 1: 10 * 1.5^0 = 10
            assert_eq!(level.kills_to_advance(), 10);
            // Level 2: 10 * 1.5^1 = 15
            level.level = 2;
            assert_eq!(level.kills_to_advance(), 15);
            // Level 3: 10 * 1.5^2 = 22.5 -> ceil = 23
            level.level = 3;
            assert_eq!(level.kills_to_advance(), 23);
            // Level 4: 10 * 1.5^3 = 33.75 -> ceil = 34
            level.level = 4;
            assert_eq!(level.kills_to_advance(), 34);
        }

        #[test]
        fn register_kill_increments_counters() {
            let mut level = GameLevel::new();
            level.register_kill();
            assert_eq!(level.kills_this_level, 1);
            assert_eq!(level.total_kills, 1);
        }

        #[test]
        fn register_kill_returns_false_before_threshold() {
            let mut level = GameLevel::new();
            for _ in 0..9 {
                assert!(!level.register_kill());
            }
            assert_eq!(level.kills_this_level, 9);
            assert_eq!(level.level, 1);
        }

        #[test]
        fn register_kill_advances_level_at_threshold() {
            let mut level = GameLevel::new();
            let threshold = level.kills_to_advance();
            for _ in 0..threshold - 1 {
                assert!(!level.register_kill());
            }
            // This should be the 10th kill, advancing the level
            assert!(level.register_kill());
            assert_eq!(level.level, 2);
            assert_eq!(level.kills_this_level, 0);
            assert_eq!(level.total_kills, 10);
        }

        #[test]
        fn register_kill_tracks_total_across_levels() {
            let mut level = GameLevel::new();
            // Advance to level 2 (10 kills)
            for _ in 0..10 {
                level.register_kill();
            }
            assert_eq!(level.level, 2);
            assert_eq!(level.total_kills, 10);

            // Add 5 more kills at level 2
            for _ in 0..5 {
                level.register_kill();
            }
            assert_eq!(level.total_kills, 15);
            assert_eq!(level.kills_this_level, 5);
        }

        #[test]
        fn progress_returns_correct_percentage() {
            let mut level = GameLevel::new();
            // 0/10 = 0.0
            assert!((level.progress() - 0.0).abs() < 0.01);

            level.kills_this_level = 5;
            // 5/10 = 0.5
            assert!((level.progress() - 0.5).abs() < 0.01);

            level.kills_this_level = 10;
            // 10/10 = 1.0
            assert!((level.progress() - 1.0).abs() < 0.01);
        }

        #[test]
        fn level_config_default_values() {
            let config = LevelConfig::default();
            assert_eq!(config.base_kills, 10);
            assert!((config.kill_multiplier - 1.5).abs() < 0.01);
        }

        #[test]
        fn custom_level_config() {
            let mut level = GameLevel::new();
            level.config = LevelConfig {
                base_kills: 20,
                kill_multiplier: 2.0,
            };
            // Level 1: 20 * 2^0 = 20
            assert_eq!(level.kills_to_advance(), 20);
            level.level = 2;
            // Level 2: 20 * 2^1 = 40
            assert_eq!(level.kills_to_advance(), 40);
        }
    }

    #[test]
    fn test_survival_time_default() {
        let time = SurvivalTime::default();
        assert_eq!(time.0, 0.0);
    }

    #[test]
    fn test_survival_time_increment() {
        let mut time = SurvivalTime::default();
        time.0 += 1.5;
        assert_eq!(time.0, 1.5);
    }

    mod spawn_rate_tests {
        use super::*;

        #[test]
        fn spawn_rate_at_level_1() {
            let rate = EnemySpawnState::spawn_rate_for_level(1);
            assert!((rate - 0.6).abs() < 0.001, "Level 1 should have 0.6 enemies/sec, got {}", rate);
        }

        #[test]
        fn spawn_rate_at_level_2() {
            let rate = EnemySpawnState::spawn_rate_for_level(2);
            // 0.6 * 1.5 = 0.9
            assert!((rate - 0.9).abs() < 0.001, "Level 2 should have 0.9 enemies/sec, got {}", rate);
        }

        #[test]
        fn spawn_rate_at_level_5() {
            let rate = EnemySpawnState::spawn_rate_for_level(5);
            // 0.6 * 1.5^4 = 0.6 * 5.0625 = 3.0375
            let expected = 0.6 * 1.5_f32.powi(4);
            assert!((rate - expected).abs() < 0.001, "Level 5 should have {} enemies/sec, got {}", expected, rate);
        }

        #[test]
        fn spawn_rate_at_level_10() {
            let rate = EnemySpawnState::spawn_rate_for_level(10);
            // 0.6 * 1.5^9 = ~23.1 enemies/sec
            let expected = 0.6 * 1.5_f32.powi(9);
            assert!((rate - expected).abs() < 0.001, "Level 10 should have {} enemies/sec, got {}", expected, rate);
        }

        #[test]
        fn spawn_rate_increases_with_level() {
            let rate_1 = EnemySpawnState::spawn_rate_for_level(1);
            let rate_5 = EnemySpawnState::spawn_rate_for_level(5);
            let rate_10 = EnemySpawnState::spawn_rate_for_level(10);

            assert!(rate_5 > rate_1, "Level 5 should have higher rate than level 1");
            assert!(rate_10 > rate_5, "Level 10 should have higher rate than level 5");
        }

        #[test]
        fn spawn_rate_at_level_0_equals_level_1() {
            // Edge case: level 0 should behave like level 1
            let rate_0 = EnemySpawnState::spawn_rate_for_level(0);
            let rate_1 = EnemySpawnState::spawn_rate_for_level(1);
            assert!((rate_0 - rate_1).abs() < 0.001, "Level 0 should equal level 1");
        }
    }

    #[test]
    fn test_free_camera_state_default() {
        let state = FreeCameraState::default();
        assert!(!state.active);
        assert_eq!(state.yaw, 0.0);
        assert_eq!(state.pitch, 0.0);
    }

    #[test]
    fn test_free_camera_state_reset_to_isometric() {
        let mut state = FreeCameraState {
            active: true,
            yaw: 1.5,
            pitch: 0.5,
        };
        state.reset_to_isometric();
        assert!(!state.active);
        assert_eq!(state.yaw, 0.0);
        assert_eq!(state.pitch, -std::f32::consts::FRAC_PI_4);
    }

    mod game_meshes_tests {
        use super::*;
        use bevy::asset::Assets;
        use bevy::pbr::StandardMaterial;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app
        }

        #[test]
        fn test_game_meshes_has_all_required_handles() {
            let mut app = setup_test_app();
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();

            let game_meshes = GameMeshes::new(&mut meshes);

            // Verify all handles can retrieve their assets (are strong handles)
            assert!(meshes.get(&game_meshes.player).is_some());
            assert!(meshes.get(&game_meshes.enemy).is_some());
            assert!(meshes.get(&game_meshes.bullet).is_some());
            assert!(meshes.get(&game_meshes.laser).is_some());
            assert!(meshes.get(&game_meshes.rocket).is_some());
            assert!(meshes.get(&game_meshes.explosion).is_some());
            assert!(meshes.get(&game_meshes.target_marker).is_some());
            assert!(meshes.get(&game_meshes.loot_small).is_some());
            assert!(meshes.get(&game_meshes.loot_medium).is_some());
            assert!(meshes.get(&game_meshes.loot_large).is_some());
            assert!(meshes.get(&game_meshes.rock).is_some());
            assert!(meshes.get(&game_meshes.whisper_core).is_some());
            assert!(meshes.get(&game_meshes.lightning_segment).is_some());
            assert!(meshes.get(&game_meshes.whisper_arc).is_some());
            assert!(meshes.get(&game_meshes.orbital_particle).is_some());
            assert!(meshes.get(&game_meshes.powerup).is_some());
        }

        #[test]
        fn test_game_materials_has_all_required_handles() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let game_materials = GameMaterials::new(&mut materials);

            // Verify all handles can retrieve their assets (are strong handles)
            assert!(materials.get(&game_materials.player).is_some());
            assert!(materials.get(&game_materials.enemy).is_some());
            assert!(materials.get(&game_materials.bullet).is_some());
            assert!(materials.get(&game_materials.laser).is_some());
            assert!(materials.get(&game_materials.rocket_pausing).is_some());
            assert!(materials.get(&game_materials.rocket_targeting).is_some());
            assert!(materials.get(&game_materials.rocket_homing).is_some());
            assert!(materials.get(&game_materials.rocket_exploding).is_some());
            assert!(materials.get(&game_materials.explosion).is_some());
            assert!(materials.get(&game_materials.target_marker).is_some());
            assert!(materials.get(&game_materials.xp_orb).is_some());
            assert!(materials.get(&game_materials.health_pack).is_some());
            assert!(materials.get(&game_materials.weapon_pistol).is_some());
            assert!(materials.get(&game_materials.weapon_laser).is_some());
            assert!(materials.get(&game_materials.weapon_rocket).is_some());
            assert!(materials.get(&game_materials.powerup).is_some());
            assert!(materials.get(&game_materials.rock).is_some());
            assert!(materials.get(&game_materials.whisper_core).is_some());
            assert!(materials.get(&game_materials.whisper_drop).is_some());
            assert!(materials.get(&game_materials.lightning).is_some());
            assert!(materials.get(&game_materials.orbital_particle).is_some());
            assert!(materials.get(&game_materials.fireball).is_some());
            assert!(materials.get(&game_materials.ice_shard).is_some());
        }

        #[test]
        fn test_game_materials_colors_match_expected_values() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let game_materials = GameMaterials::new(&mut materials);

            // Verify player is green
            let player_mat = materials.get(&game_materials.player).unwrap();
            assert_eq!(player_mat.base_color, Color::srgb(0.0, 1.0, 0.0));

            // Verify enemy is red
            let enemy_mat = materials.get(&game_materials.enemy).unwrap();
            assert_eq!(enemy_mat.base_color, Color::srgb(1.0, 0.0, 0.0));

            // Verify bullet is yellow with emissive
            let bullet_mat = materials.get(&game_materials.bullet).unwrap();
            assert_eq!(bullet_mat.base_color, Color::srgb(1.0, 1.0, 0.0));

            // Verify laser is cyan with emissive
            let laser_mat = materials.get(&game_materials.laser).unwrap();
            assert_eq!(laser_mat.base_color, Color::srgb(0.0, 1.0, 1.0));
            assert!(laser_mat.unlit);

            // Verify rocket_pausing is yellow-orange (visible during pause phase)
            let rocket_pausing_mat = materials.get(&game_materials.rocket_pausing).unwrap();
            assert_eq!(rocket_pausing_mat.base_color, Color::srgb(0.8, 0.6, 0.0));

            // Verify rocket_targeting is yellow
            let rocket_targeting_mat = materials.get(&game_materials.rocket_targeting).unwrap();
            assert_eq!(rocket_targeting_mat.base_color, Color::srgb(1.0, 1.0, 0.0));

            // Verify rocket_homing is orange
            let rocket_homing_mat = materials.get(&game_materials.rocket_homing).unwrap();
            assert_eq!(rocket_homing_mat.base_color, Color::srgb(1.0, 0.5, 0.0));

            // Verify rocket_exploding is red
            let rocket_exploding_mat = materials.get(&game_materials.rocket_exploding).unwrap();
            assert_eq!(rocket_exploding_mat.base_color, Color::srgb(1.0, 0.0, 0.0));

            // Verify explosion is orange with 50% transparency
            let explosion_mat = materials.get(&game_materials.explosion).unwrap();
            assert_eq!(explosion_mat.base_color, Color::srgba(1.0, 0.3, 0.0, 0.5));
            assert_eq!(explosion_mat.alpha_mode, AlphaMode::Blend);

            // Verify target_marker is red
            let target_marker_mat = materials.get(&game_materials.target_marker).unwrap();
            assert_eq!(target_marker_mat.base_color, Color::srgb(1.0, 0.0, 0.0));

            // Verify xp_orb is light grey
            let xp_mat = materials.get(&game_materials.xp_orb).unwrap();
            assert_eq!(xp_mat.base_color, Color::srgb(0.75, 0.75, 0.75));

            // Verify health_pack is green
            let health_mat = materials.get(&game_materials.health_pack).unwrap();
            assert_eq!(health_mat.base_color, Color::srgb(0.0, 1.0, 0.0));

            // Verify weapon_pistol is yellow
            let pistol_mat = materials.get(&game_materials.weapon_pistol).unwrap();
            assert_eq!(pistol_mat.base_color, Color::srgb(1.0, 1.0, 0.0));

            // Verify weapon_laser is blue
            let laser_mat = materials.get(&game_materials.weapon_laser).unwrap();
            assert_eq!(laser_mat.base_color, Color::srgb(0.0, 0.0, 1.0));

            // Verify weapon_rocket is orange
            let rocket_mat = materials.get(&game_materials.weapon_rocket).unwrap();
            assert_eq!(rocket_mat.base_color, Color::srgb(1.0, 0.5, 0.0));

            // Verify powerup is magenta
            let powerup_mat = materials.get(&game_materials.powerup).unwrap();
            assert_eq!(powerup_mat.base_color, Color::srgb(1.0, 0.0, 1.0));

            // Verify rock is grey
            let rock_mat = materials.get(&game_materials.rock).unwrap();
            assert_eq!(rock_mat.base_color, Color::srgb(0.5, 0.5, 0.5));

            // Verify whisper_core is white with emissive glow
            let whisper_core_mat = materials.get(&game_materials.whisper_core).unwrap();
            assert_eq!(whisper_core_mat.base_color, Color::srgb(1.0, 1.0, 1.0));
            assert!(whisper_core_mat.unlit);

            // Verify whisper_drop is white (dimmer emissive)
            let whisper_drop_mat = materials.get(&game_materials.whisper_drop).unwrap();
            assert_eq!(whisper_drop_mat.base_color, Color::srgb(1.0, 1.0, 1.0));
            assert!(whisper_drop_mat.unlit);

            // Verify lightning is white with HDR emissive
            let lightning_mat = materials.get(&game_materials.lightning).unwrap();
            assert_eq!(lightning_mat.base_color, Color::srgb(1.0, 1.0, 1.0));
            assert!(lightning_mat.unlit);

            // Verify orbital_particle is white
            let orbital_mat = materials.get(&game_materials.orbital_particle).unwrap();
            assert_eq!(orbital_mat.base_color, Color::srgb(1.0, 1.0, 1.0));
            assert!(orbital_mat.unlit);

            // Verify fireball is orange (Fire element color)
            let fireball_mat = materials.get(&game_materials.fireball).unwrap();
            assert_eq!(fireball_mat.base_color, Color::srgb(1.0, 0.5, 0.0));

            // Verify ice_shard is ice blue (Frost element color)
            let ice_shard_mat = materials.get(&game_materials.ice_shard).unwrap();
            assert_eq!(ice_shard_mat.base_color, Color::srgb(0.53, 0.81, 0.92));
        }

        #[test]
        fn test_setup_game_assets_inserts_resources() {
            use crate::states::GameState;
            use crate::game::systems::setup_game_assets;

            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::state::app::StatesPlugin,
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_state::<GameState>();
            app.add_systems(OnEnter(GameState::InGame), setup_game_assets);

            // Transition to InGame
            app.world_mut()
                .get_resource_mut::<bevy::state::state::NextState<GameState>>()
                .unwrap()
                .set(GameState::InGame);
            app.update();
            app.update();

            // Verify resources were inserted
            assert!(app.world().get_resource::<GameMeshes>().is_some());
            assert!(app.world().get_resource::<GameMaterials>().is_some());
            assert!(app.world().get_resource::<DamageFlashMaterial>().is_some());
        }
    }

    mod enemy_scale_tests {
        use super::*;

        #[test]
        fn enemy_scale_level_1_is_base_scale() {
            let scale = enemy_scale_for_level(1);
            assert!((scale - 0.75).abs() < 0.001);
        }

        #[test]
        fn enemy_scale_increases_with_level() {
            let scale_1 = enemy_scale_for_level(1);
            let scale_5 = enemy_scale_for_level(5);
            assert!(scale_5 > scale_1);
        }

        #[test]
        fn enemy_scale_values_are_correct() {
            // Level 1: 0.75 (base)
            assert!((enemy_scale_for_level(1) - 0.75).abs() < 0.001);
            // Level 2: 0.75 + 0.15 = 0.90
            assert!((enemy_scale_for_level(2) - 0.90).abs() < 0.001);
            // Level 3: 0.75 + 0.30 = 1.05
            assert!((enemy_scale_for_level(3) - 1.05).abs() < 0.001);
            // Level 4: 0.75 + 0.45 = 1.20
            assert!((enemy_scale_for_level(4) - 1.20).abs() < 0.001);
            // Level 5: 0.75 + 0.60 = 1.35
            assert!((enemy_scale_for_level(5) - 1.35).abs() < 0.001);
        }

        #[test]
        fn enemy_scale_is_reasonable_for_all_levels() {
            for level in 1..=5 {
                let scale = enemy_scale_for_level(level);
                assert!(scale >= 0.75 && scale <= 1.5,
                    "Scale {} for level {} should be between 0.75 and 1.5", scale, level);
            }
        }

        #[test]
        fn enemy_scale_level_0_same_as_level_1() {
            // Level 0 (invalid) should have same scale as level 1 due to saturating_sub
            let scale_0 = enemy_scale_for_level(0);
            let scale_1 = enemy_scale_for_level(1);
            assert!((scale_0 - scale_1).abs() < 0.001);
        }
    }

    mod enemy_level_materials_tests {
        use super::*;
        use bevy::asset::Assets;
        use bevy::pbr::StandardMaterial;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.init_asset::<StandardMaterial>();
            app
        }

        #[test]
        fn enemy_level_materials_has_all_handles() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let enemy_materials = EnemyLevelMaterials::new(&mut materials);

            // Verify all handles can retrieve their assets
            assert!(materials.get(&enemy_materials.common).is_some());
            assert!(materials.get(&enemy_materials.uncommon).is_some());
            assert!(materials.get(&enemy_materials.rare).is_some());
            assert!(materials.get(&enemy_materials.epic).is_some());
            assert!(materials.get(&enemy_materials.legendary).is_some());
        }

        #[test]
        fn enemy_level_materials_colors_are_correct() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let enemy_materials = EnemyLevelMaterials::new(&mut materials);

            // Level 1 - Grey
            let common_mat = materials.get(&enemy_materials.common).unwrap();
            assert_eq!(common_mat.base_color, Color::srgb(0.6, 0.6, 0.6));

            // Level 2 - Green
            let uncommon_mat = materials.get(&enemy_materials.uncommon).unwrap();
            assert_eq!(uncommon_mat.base_color, Color::srgb(0.0, 0.8, 0.2));

            // Level 3 - Blue
            let rare_mat = materials.get(&enemy_materials.rare).unwrap();
            assert_eq!(rare_mat.base_color, Color::srgb(0.2, 0.4, 1.0));

            // Level 4 - Purple
            let epic_mat = materials.get(&enemy_materials.epic).unwrap();
            assert_eq!(epic_mat.base_color, Color::srgb(0.6, 0.2, 0.8));

            // Level 5 - Gold
            let legendary_mat = materials.get(&enemy_materials.legendary).unwrap();
            assert_eq!(legendary_mat.base_color, Color::srgb(1.0, 0.84, 0.0));
        }

        #[test]
        fn enemy_level_materials_legendary_has_emissive() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let enemy_materials = EnemyLevelMaterials::new(&mut materials);

            let legendary_mat = materials.get(&enemy_materials.legendary).unwrap();
            // Verify emissive is set (non-zero)
            let emissive = legendary_mat.emissive;
            assert!(emissive.red > 0.0 || emissive.green > 0.0 || emissive.blue > 0.0,
                "Legendary material should have emissive glow");
        }

        #[test]
        fn for_level_returns_correct_material() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let enemy_materials = EnemyLevelMaterials::new(&mut materials);

            // Test each level returns expected material
            assert_eq!(enemy_materials.for_level(1), enemy_materials.common);
            assert_eq!(enemy_materials.for_level(2), enemy_materials.uncommon);
            assert_eq!(enemy_materials.for_level(3), enemy_materials.rare);
            assert_eq!(enemy_materials.for_level(4), enemy_materials.epic);
            assert_eq!(enemy_materials.for_level(5), enemy_materials.legendary);
        }

        #[test]
        fn for_level_handles_out_of_range() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let enemy_materials = EnemyLevelMaterials::new(&mut materials);

            // Levels > 5 should return legendary
            assert_eq!(enemy_materials.for_level(6), enemy_materials.legendary);
            assert_eq!(enemy_materials.for_level(10), enemy_materials.legendary);
            assert_eq!(enemy_materials.for_level(255), enemy_materials.legendary);
        }
    }

    mod xp_orb_materials_tests {
        use super::*;
        use bevy::asset::Assets;
        use bevy::pbr::StandardMaterial;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.init_asset::<StandardMaterial>();
            app
        }

        #[test]
        fn xp_orb_materials_has_all_handles() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let xp_materials = XpOrbMaterials::new(&mut materials);

            // Verify all handles can retrieve their assets
            assert!(materials.get(&xp_materials.common).is_some());
            assert!(materials.get(&xp_materials.uncommon).is_some());
            assert!(materials.get(&xp_materials.rare).is_some());
            assert!(materials.get(&xp_materials.epic).is_some());
            assert!(materials.get(&xp_materials.legendary).is_some());
        }

        #[test]
        fn xp_orb_materials_colors_are_correct() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let xp_materials = XpOrbMaterials::new(&mut materials);

            // Level 1 - Grey
            let common_mat = materials.get(&xp_materials.common).unwrap();
            assert_eq!(common_mat.base_color, Color::srgb(0.6, 0.6, 0.6));

            // Level 2 - Green
            let uncommon_mat = materials.get(&xp_materials.uncommon).unwrap();
            assert_eq!(uncommon_mat.base_color, Color::srgb(0.0, 0.8, 0.2));

            // Level 3 - Blue
            let rare_mat = materials.get(&xp_materials.rare).unwrap();
            assert_eq!(rare_mat.base_color, Color::srgb(0.2, 0.4, 1.0));

            // Level 4 - Purple
            let epic_mat = materials.get(&xp_materials.epic).unwrap();
            assert_eq!(epic_mat.base_color, Color::srgb(0.6, 0.2, 0.8));

            // Level 5 - Gold
            let legendary_mat = materials.get(&xp_materials.legendary).unwrap();
            assert_eq!(legendary_mat.base_color, Color::srgb(1.0, 0.84, 0.0));
        }

        #[test]
        fn xp_orb_materials_all_have_emissive() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let xp_materials = XpOrbMaterials::new(&mut materials);

            // All XP orb materials should have emissive for visibility
            for handle in [
                &xp_materials.common,
                &xp_materials.uncommon,
                &xp_materials.rare,
                &xp_materials.epic,
                &xp_materials.legendary,
            ] {
                let mat = materials.get(handle).unwrap();
                let emissive = mat.emissive;
                assert!(
                    emissive.red > 0.0 || emissive.green > 0.0 || emissive.blue > 0.0,
                    "XP orb material should have emissive glow"
                );
            }
        }

        #[test]
        fn xp_orb_legendary_has_strongest_emissive() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let xp_materials = XpOrbMaterials::new(&mut materials);

            let common_mat = materials.get(&xp_materials.common).unwrap();
            let legendary_mat = materials.get(&xp_materials.legendary).unwrap();

            // Legendary should have stronger emissive than common
            let common_brightness = common_mat.emissive.red + common_mat.emissive.green + common_mat.emissive.blue;
            let legendary_brightness = legendary_mat.emissive.red + legendary_mat.emissive.green + legendary_mat.emissive.blue;

            assert!(
                legendary_brightness > common_brightness,
                "Legendary XP orb should have stronger emissive than common"
            );
        }

        #[test]
        fn for_level_returns_correct_material() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let xp_materials = XpOrbMaterials::new(&mut materials);

            // Test each level returns expected material
            assert_eq!(xp_materials.for_level(1), xp_materials.common);
            assert_eq!(xp_materials.for_level(2), xp_materials.uncommon);
            assert_eq!(xp_materials.for_level(3), xp_materials.rare);
            assert_eq!(xp_materials.for_level(4), xp_materials.epic);
            assert_eq!(xp_materials.for_level(5), xp_materials.legendary);
        }

        #[test]
        fn for_level_handles_out_of_range() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let xp_materials = XpOrbMaterials::new(&mut materials);

            // Levels > 5 should return legendary
            assert_eq!(xp_materials.for_level(6), xp_materials.legendary);
            assert_eq!(xp_materials.for_level(10), xp_materials.legendary);
            assert_eq!(xp_materials.for_level(255), xp_materials.legendary);
        }
    }

    mod level_stats_tests {
        use super::*;

        #[test]
        fn level_stats_starts_at_zero() {
            let stats = LevelStats::new();
            assert_eq!(stats.time_elapsed, 0.0);
            assert_eq!(stats.enemies_killed, 0);
            assert_eq!(stats.xp_gained, 0);
        }

        #[test]
        fn level_stats_default_matches_new() {
            let stats_new = LevelStats::new();
            let stats_default = LevelStats::default();
            assert_eq!(stats_new.time_elapsed, stats_default.time_elapsed);
            assert_eq!(stats_new.enemies_killed, stats_default.enemies_killed);
            assert_eq!(stats_new.xp_gained, stats_default.xp_gained);
        }

        #[test]
        fn level_stats_records_kills() {
            let mut stats = LevelStats::new();
            stats.record_kill();
            stats.record_kill();
            assert_eq!(stats.enemies_killed, 2);
        }

        #[test]
        fn level_stats_records_xp() {
            let mut stats = LevelStats::new();
            stats.record_xp(50);
            stats.record_xp(25);
            assert_eq!(stats.xp_gained, 75);
        }

        #[test]
        fn level_stats_formats_time_correctly() {
            let mut stats = LevelStats::new();
            stats.time_elapsed = 125.0; // 2:05
            assert_eq!(stats.formatted_time(), "02:05");
        }

        #[test]
        fn level_stats_formats_zero_time() {
            let stats = LevelStats::new();
            assert_eq!(stats.formatted_time(), "00:00");
        }

        #[test]
        fn level_stats_formats_one_minute() {
            let mut stats = LevelStats::new();
            stats.time_elapsed = 60.0;
            assert_eq!(stats.formatted_time(), "01:00");
        }

        #[test]
        fn level_stats_formats_long_time() {
            let mut stats = LevelStats::new();
            stats.time_elapsed = 3661.5; // 61 minutes and 1.5 seconds
            assert_eq!(stats.formatted_time(), "61:01");
        }

        #[test]
        fn level_stats_resets_correctly() {
            let mut stats = LevelStats::new();
            stats.enemies_killed = 10;
            stats.xp_gained = 500;
            stats.time_elapsed = 60.0;
            stats.reset();
            assert_eq!(stats.enemies_killed, 0);
            assert_eq!(stats.xp_gained, 0);
            assert_eq!(stats.time_elapsed, 0.0);
        }

        #[test]
        fn level_stats_can_be_used_as_resource() {
            let mut app = App::new();
            app.init_resource::<LevelStats>();
            app.update();

            // Verify the resource exists with default values
            let stats = app.world().resource::<LevelStats>();
            assert_eq!(stats.time_elapsed, 0.0);
            assert_eq!(stats.enemies_killed, 0);
            assert_eq!(stats.xp_gained, 0);
        }

        #[test]
        fn level_stats_accumulates_multiple_kills() {
            let mut stats = LevelStats::new();
            for _ in 0..100 {
                stats.record_kill();
            }
            assert_eq!(stats.enemies_killed, 100);
        }

        #[test]
        fn level_stats_accumulates_multiple_xp() {
            let mut stats = LevelStats::new();
            stats.record_xp(5);   // Common
            stats.record_xp(15);  // Uncommon
            stats.record_xp(35);  // Rare
            stats.record_xp(75);  // Epic
            stats.record_xp(150); // Legendary
            assert_eq!(stats.xp_gained, 280);
        }
    }
}