use crate::element::Element;

/// All 64 spell types across 8 elements (8 spells per element).
/// Each variant represents a unique spell with its own mechanics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpellType {
    // Fire spells (8)
    Fireball,
    FlameLance,
    Ashfall,
    MeteorShower,
    PhoenixFlare,
    Combustion,
    Immolate,
    Hellfire,

    // Frost spells (8)
    IceShard,
    FrostNova,
    Blizzard,
    FrozenRay,
    GlacialSpike,
    IceBarrier,
    Shatter,
    AbsoluteZero,

    // Poison spells (8)
    VenomBolt,
    PlagueCloud,
    ToxicSpray,
    Miasma,
    CorrosivePool,
    Pandemic,
    Blight,
    Necrosis,

    // Lightning spells (8)
    Spark,
    ChainLightning,
    ThunderStrike,
    StaticField,
    Flashstep,
    Overcharge,
    Electrocute,
    StormCall,

    // Light spells (8)
    HolyBeam,
    RadiantBeam,
    Radiance,
    Smite,
    DivineLight,
    Consecration,
    Purify,
    Judgment,

    // Dark spells (8)
    ShadowBolt,
    VoidRift,
    DarkPulse,
    Corruption,
    SoulDrain,
    Nightmare,
    Eclipse,
    Oblivion,

    // Chaos spells (8)
    WildMagic,
    Entropy,
    ChaosBolt,
    Randomize,
    Unstable,
    Paradox,
    Mayhem,
    Cataclysm,

    // Psychic spells (8)
    MindBlast,
    Telekinesis,
    PsychicWave,
    Confusion,
    MentalSpike,
    Hallucination,
    Dominate,
    PsychicShatter,
}

impl SpellType {
    /// Returns a unique numeric identifier for this spell (0-63).
    pub fn id(&self) -> u32 {
        match self {
            // Fire (0-7)
            SpellType::Fireball => 0,
            SpellType::FlameLance => 1,
            SpellType::Ashfall => 2,
            SpellType::MeteorShower => 3,
            SpellType::PhoenixFlare => 4,
            SpellType::Combustion => 5,
            SpellType::Immolate => 6,
            SpellType::Hellfire => 7,

            // Frost (8-15)
            SpellType::IceShard => 8,
            SpellType::FrostNova => 9,
            SpellType::Blizzard => 10,
            SpellType::FrozenRay => 11,
            SpellType::GlacialSpike => 12,
            SpellType::IceBarrier => 13,
            SpellType::Shatter => 14,
            SpellType::AbsoluteZero => 15,

            // Poison (16-23)
            SpellType::VenomBolt => 16,
            SpellType::PlagueCloud => 17,
            SpellType::ToxicSpray => 18,
            SpellType::Miasma => 19,
            SpellType::CorrosivePool => 20,
            SpellType::Pandemic => 21,
            SpellType::Blight => 22,
            SpellType::Necrosis => 23,

            // Lightning (24-31)
            SpellType::Spark => 24,
            SpellType::ChainLightning => 25,
            SpellType::ThunderStrike => 26,
            SpellType::StaticField => 27,
            SpellType::Flashstep => 28,
            SpellType::Overcharge => 29,
            SpellType::Electrocute => 30,
            SpellType::StormCall => 31,

            // Light (32-39)
            SpellType::HolyBeam => 32,
            SpellType::RadiantBeam => 33,
            SpellType::Radiance => 34,
            SpellType::Smite => 35,
            SpellType::DivineLight => 36,
            SpellType::Consecration => 37,
            SpellType::Purify => 38,
            SpellType::Judgment => 39,

            // Dark (40-47)
            SpellType::ShadowBolt => 40,
            SpellType::VoidRift => 41,
            SpellType::DarkPulse => 42,
            SpellType::Corruption => 43,
            SpellType::SoulDrain => 44,
            SpellType::Nightmare => 45,
            SpellType::Eclipse => 46,
            SpellType::Oblivion => 47,

            // Chaos (48-55)
            SpellType::WildMagic => 48,
            SpellType::Entropy => 49,
            SpellType::ChaosBolt => 50,
            SpellType::Randomize => 51,
            SpellType::Unstable => 52,
            SpellType::Paradox => 53,
            SpellType::Mayhem => 54,
            SpellType::Cataclysm => 55,

            // Psychic (56-63)
            SpellType::MindBlast => 56,
            SpellType::Telekinesis => 57,
            SpellType::PsychicWave => 58,
            SpellType::Confusion => 59,
            SpellType::MentalSpike => 60,
            SpellType::Hallucination => 61,
            SpellType::Dominate => 62,
            SpellType::PsychicShatter => 63,
        }
    }

    /// Returns the element this spell belongs to.
    pub fn element(&self) -> Element {
        match self.id() {
            0..=7 => Element::Fire,
            8..=15 => Element::Frost,
            16..=23 => Element::Poison,
            24..=31 => Element::Lightning,
            32..=39 => Element::Light,
            40..=47 => Element::Dark,
            48..=55 => Element::Chaos,
            56..=63 => Element::Psychic,
            _ => unreachable!("Invalid spell ID"),
        }
    }

    /// Returns the display name for this spell.
    pub fn name(&self) -> &'static str {
        match self {
            // Fire
            SpellType::Fireball => "Fireball",
            SpellType::FlameLance => "Flame Lance",
            SpellType::Ashfall => "Ashfall",
            SpellType::MeteorShower => "Meteor Shower",
            SpellType::PhoenixFlare => "Phoenix Flare",
            SpellType::Combustion => "Combustion",
            SpellType::Immolate => "Immolate",
            SpellType::Hellfire => "Hellfire",

            // Frost
            SpellType::IceShard => "Ice Shard",
            SpellType::FrostNova => "Frost Nova",
            SpellType::Blizzard => "Blizzard",
            SpellType::FrozenRay => "Frozen Ray",
            SpellType::GlacialSpike => "Glacial Spike",
            SpellType::IceBarrier => "Ice Barrier",
            SpellType::Shatter => "Shatter",
            SpellType::AbsoluteZero => "Absolute Zero",

            // Poison
            SpellType::VenomBolt => "Venom Bolt",
            SpellType::PlagueCloud => "Plague Cloud",
            SpellType::ToxicSpray => "Toxic Spray",
            SpellType::Miasma => "Miasma",
            SpellType::CorrosivePool => "Corrosive Pool",
            SpellType::Pandemic => "Pandemic",
            SpellType::Blight => "Blight",
            SpellType::Necrosis => "Necrosis",

            // Lightning
            SpellType::Spark => "Spark",
            SpellType::ChainLightning => "Chain Lightning",
            SpellType::ThunderStrike => "Thunder Strike",
            SpellType::StaticField => "Static Field",
            SpellType::Flashstep => "Flashstep",
            SpellType::Overcharge => "Overcharge",
            SpellType::Electrocute => "Electrocute",
            SpellType::StormCall => "Storm Call",

            // Light
            SpellType::HolyBeam => "Holy Beam",
            SpellType::RadiantBeam => "Radiant Beam",
            SpellType::Radiance => "Radiance",
            SpellType::Smite => "Smite",
            SpellType::DivineLight => "Divine Light",
            SpellType::Consecration => "Consecration",
            SpellType::Purify => "Purify",
            SpellType::Judgment => "Judgment",

            // Dark
            SpellType::ShadowBolt => "Shadow Bolt",
            SpellType::VoidRift => "Void Rift",
            SpellType::DarkPulse => "Dark Pulse",
            SpellType::Corruption => "Corruption",
            SpellType::SoulDrain => "Soul Drain",
            SpellType::Nightmare => "Nightmare",
            SpellType::Eclipse => "Eclipse",
            SpellType::Oblivion => "Oblivion",

            // Chaos
            SpellType::WildMagic => "Wild Magic",
            SpellType::Entropy => "Entropy",
            SpellType::ChaosBolt => "Chaos Bolt",
            SpellType::Randomize => "Randomize",
            SpellType::Unstable => "Unstable",
            SpellType::Paradox => "Paradox",
            SpellType::Mayhem => "Mayhem",
            SpellType::Cataclysm => "Cataclysm",

            // Psychic
            SpellType::MindBlast => "Mind Blast",
            SpellType::Telekinesis => "Telekinesis",
            SpellType::PsychicWave => "Psychic Wave",
            SpellType::Confusion => "Confusion",
            SpellType::MentalSpike => "Mental Spike",
            SpellType::Hallucination => "Hallucination",
            SpellType::Dominate => "Dominate",
            SpellType::PsychicShatter => "Psychic Shatter",
        }
    }

    /// Returns the flavor description for this spell.
    pub fn description(&self) -> &'static str {
        match self {
            // Fire
            SpellType::Fireball => "A blazing projectile that ignites enemies on impact.",
            SpellType::FlameLance => "A piercing lance of concentrated fire.",
            SpellType::Ashfall => "Embers rain down over an area dealing sustained damage.",
            SpellType::MeteorShower => "Summons falling meteors to devastate an area.",
            SpellType::PhoenixFlare => "Releases a burst of phoenix fire that heals allies.",
            SpellType::Combustion => "Causes enemies to spontaneously combust.",
            SpellType::Immolate => "Sets the target ablaze with lingering flames.",
            SpellType::Hellfire => "Calls upon infernal flames from the depths.",

            // Frost
            SpellType::IceShard => "Launches a razor-sharp shard of ice.",
            SpellType::FrostNova => "An icy explosion that freezes nearby enemies.",
            SpellType::Blizzard => "A fierce snowstorm that slows and damages.",
            SpellType::FrozenRay => "A continuous beam of freezing cold.",
            SpellType::GlacialSpike => "A massive spike of ice erupts from the ground.",
            SpellType::IceBarrier => "Creates a protective barrier of ice.",
            SpellType::Shatter => "Shatters frozen enemies for massive damage.",
            SpellType::AbsoluteZero => "Drops temperature to lethal levels.",

            // Poison
            SpellType::VenomBolt => "A toxic projectile that poisons on contact.",
            SpellType::PlagueCloud => "Creates a lingering cloud of deadly plague.",
            SpellType::ToxicSpray => "Sprays a cone of corrosive poison.",
            SpellType::Miasma => "A creeping mist that weakens all within.",
            SpellType::CorrosivePool => "Creates a pool of acid on the ground.",
            SpellType::Pandemic => "Spreads infection between nearby enemies.",
            SpellType::Blight => "Withers the life force of the target.",
            SpellType::Necrosis => "Causes flesh to decay and rot.",

            // Lightning
            SpellType::Spark => "A quick jolt of electricity.",
            SpellType::ChainLightning => "Lightning that arcs between multiple targets.",
            SpellType::ThunderStrike => "Lightning strikes from above, dealing area damage.",
            SpellType::StaticField => "Creates a field that shocks nearby enemies.",
            SpellType::Flashstep => "Brief teleport releasing lightning at origin and destination.",
            SpellType::Overcharge => "Supercharges the caster with electric power.",
            SpellType::Electrocute => "Channels continuous lightning into a target.",
            SpellType::StormCall => "Summons a devastating electrical storm.",

            // Light
            SpellType::HolyBeam => "A beam of purifying holy light.",
            SpellType::RadiantBeam => "A focused beam of pure light energy.",
            SpellType::Radiance => "Emits blinding light in all directions.",
            SpellType::Smite => "Calls down divine judgment on a target.",
            SpellType::DivineLight => "Bathes an area in healing light.",
            SpellType::Consecration => "Sanctifies the ground, damaging evil.",
            SpellType::Purify => "Cleanses corruption and heals wounds.",
            SpellType::Judgment => "Delivers ultimate divine punishment.",

            // Dark
            SpellType::ShadowBolt => "A bolt of concentrated darkness.",
            SpellType::VoidRift => "Opens a rift to the void that pulls enemies in.",
            SpellType::DarkPulse => "Releases a wave of dark energy.",
            SpellType::Corruption => "Infects the target with creeping darkness.",
            SpellType::SoulDrain => "Steals life force from enemies.",
            SpellType::Nightmare => "Traps enemies in terrifying visions.",
            SpellType::Eclipse => "Blocks all light, empowering dark attacks.",
            SpellType::Oblivion => "Erases targets from existence.",

            // Chaos
            SpellType::WildMagic => "Unpredictable magical energy with random effects.",
            SpellType::Entropy => "Accelerates decay and disorder.",
            SpellType::ChaosBolt => "A projectile with randomly changing properties.",
            SpellType::Randomize => "Scrambles the properties of affected targets.",
            SpellType::Unstable => "Creates volatile energy that may explode.",
            SpellType::Paradox => "Warps reality in impossible ways.",
            SpellType::Mayhem => "Causes widespread chaotic destruction.",
            SpellType::Cataclysm => "Unleashes ultimate chaotic devastation.",

            // Psychic
            SpellType::MindBlast => "A devastating psychic attack on the mind.",
            SpellType::Telekinesis => "Moves objects with mental power.",
            SpellType::PsychicWave => "A wave of mental energy that stuns.",
            SpellType::Confusion => "Befuddles enemy minds.",
            SpellType::MentalSpike => "Pierces mental defenses with focused thought.",
            SpellType::Hallucination => "Makes enemies see things that aren't there.",
            SpellType::Dominate => "Takes control of an enemy's mind.",
            SpellType::PsychicShatter => "Breaks the mind completely.",
        }
    }

    /// Returns the base damage for this spell (before level scaling).
    pub fn base_damage(&self) -> f32 {
        match self {
            // Fire - generally high damage
            SpellType::Fireball => 15.0,
            SpellType::FlameLance => 25.0,
            SpellType::Ashfall => 18.0, // Area DOT, moderate base for multiple ember hits
            SpellType::MeteorShower => 30.0,
            SpellType::PhoenixFlare => 18.0,
            SpellType::Combustion => 22.0,
            SpellType::Immolate => 12.0, // DOT spell, lower initial
            SpellType::Hellfire => 35.0,

            // Frost - moderate damage with control
            SpellType::IceShard => 12.0,
            SpellType::FrostNova => 15.0,
            SpellType::Blizzard => 18.0,
            SpellType::FrozenRay => 20.0,
            SpellType::GlacialSpike => 28.0,
            SpellType::IceBarrier => 5.0, // Defensive, low damage
            SpellType::Shatter => 40.0,   // Bonus vs frozen
            SpellType::AbsoluteZero => 35.0,

            // Poison - DOT focused
            SpellType::VenomBolt => 10.0,
            SpellType::PlagueCloud => 8.0,
            SpellType::ToxicSpray => 14.0,
            SpellType::Miasma => 6.0,
            SpellType::CorrosivePool => 12.0,
            SpellType::Pandemic => 10.0,
            SpellType::Blight => 16.0,
            SpellType::Necrosis => 25.0,

            // Lightning - fast, chain damage
            SpellType::Spark => 8.0,
            SpellType::ChainLightning => 15.0,
            SpellType::ThunderStrike => 30.0,
            SpellType::StaticField => 10.0,
            SpellType::Flashstep => 20.0, // Mobility spell with AoE damage at origin/destination
            SpellType::Overcharge => 5.0, // Buff, low direct damage
            SpellType::Electrocute => 20.0,
            SpellType::StormCall => 35.0,

            // Light - healing/support focused
            SpellType::HolyBeam => 18.0,
            SpellType::RadiantBeam => 22.0,
            SpellType::Radiance => 15.0,
            SpellType::Smite => 28.0,
            SpellType::DivineLight => 10.0,
            SpellType::Consecration => 12.0,
            SpellType::Purify => 8.0,
            SpellType::Judgment => 40.0,

            // Dark - life steal, debuffs
            SpellType::ShadowBolt => 16.0,
            SpellType::VoidRift => 25.0,
            SpellType::DarkPulse => 20.0,
            SpellType::Corruption => 10.0,
            SpellType::SoulDrain => 15.0,
            SpellType::Nightmare => 12.0,
            SpellType::Eclipse => 18.0,
            SpellType::Oblivion => 45.0,

            // Chaos - unpredictable
            SpellType::WildMagic => 20.0,
            SpellType::Entropy => 15.0,
            SpellType::ChaosBolt => 22.0,
            SpellType::Randomize => 10.0,
            SpellType::Unstable => 25.0,
            SpellType::Paradox => 20.0,
            SpellType::Mayhem => 28.0,
            SpellType::Cataclysm => 50.0,

            // Psychic - crowd control
            SpellType::MindBlast => 20.0,
            SpellType::Telekinesis => 12.0,
            SpellType::PsychicWave => 15.0,
            SpellType::Confusion => 8.0,
            SpellType::MentalSpike => 25.0,
            SpellType::Hallucination => 10.0,
            SpellType::Dominate => 5.0,
            SpellType::PsychicShatter => 35.0,
        }
    }

    /// Returns the fire rate in shots per second.
    pub fn fire_rate(&self) -> f32 {
        match self {
            // Fire - balanced fire rates
            SpellType::Fireball => 2.0,
            SpellType::FlameLance => 1.5,
            SpellType::Ashfall => 0.25, // Long duration zone spell, slow cast rate
            SpellType::MeteorShower => 0.3,
            SpellType::PhoenixFlare => 1.0,
            SpellType::Combustion => 1.2,
            SpellType::Immolate => 2.5,
            SpellType::Hellfire => 0.25,

            // Frost - slower but impactful
            SpellType::IceShard => 3.0,
            SpellType::FrostNova => 0.5,
            SpellType::Blizzard => 0.4,
            SpellType::FrozenRay => 4.0, // Continuous
            SpellType::GlacialSpike => 0.8,
            SpellType::IceBarrier => 0.2,
            SpellType::Shatter => 0.6,
            SpellType::AbsoluteZero => 0.15,

            // Poison - sustained damage
            SpellType::VenomBolt => 2.5,
            SpellType::PlagueCloud => 0.4,
            SpellType::ToxicSpray => 1.5,
            SpellType::Miasma => 0.3,
            SpellType::CorrosivePool => 0.5,
            SpellType::Pandemic => 0.6,
            SpellType::Blight => 1.0,
            SpellType::Necrosis => 0.5,

            // Lightning - fast attacks
            SpellType::Spark => 5.0,
            SpellType::ChainLightning => 1.2,
            SpellType::ThunderStrike => 0.4,
            SpellType::StaticField => 0.5,
            SpellType::Flashstep => 0.3, // Short cooldown mobility spell
            SpellType::Overcharge => 0.2,
            SpellType::Electrocute => 3.0,
            SpellType::StormCall => 0.2,

            // Light - moderate
            SpellType::HolyBeam => 2.0,
            SpellType::RadiantBeam => 4.0,
            SpellType::Radiance => 0.8,
            SpellType::Smite => 0.6,
            SpellType::DivineLight => 0.5,
            SpellType::Consecration => 0.3,
            SpellType::Purify => 0.4,
            SpellType::Judgment => 0.2,

            // Dark - medium speed
            SpellType::ShadowBolt => 2.0,
            SpellType::VoidRift => 0.4,
            SpellType::DarkPulse => 0.8,
            SpellType::Corruption => 1.5,
            SpellType::SoulDrain => 2.0,
            SpellType::Nightmare => 0.5,
            SpellType::Eclipse => 0.2,
            SpellType::Oblivion => 0.1,

            // Chaos - varied
            SpellType::WildMagic => 1.5,
            SpellType::Entropy => 1.0,
            SpellType::ChaosBolt => 1.8,
            SpellType::Randomize => 0.6,
            SpellType::Unstable => 0.8,
            SpellType::Paradox => 0.4,
            SpellType::Mayhem => 0.5,
            SpellType::Cataclysm => 0.1,

            // Psychic - control focused
            SpellType::MindBlast => 1.2,
            SpellType::Telekinesis => 2.0,
            SpellType::PsychicWave => 0.6,
            SpellType::Confusion => 0.8,
            SpellType::MentalSpike => 1.5,
            SpellType::Hallucination => 0.4,
            SpellType::Dominate => 0.2,
            SpellType::PsychicShatter => 0.3,
        }
    }

    /// Returns all 64 spell type variants.
    pub fn all() -> &'static [SpellType; 64] {
        &[
            // Fire
            SpellType::Fireball,
            SpellType::FlameLance,
            SpellType::Ashfall,
            SpellType::MeteorShower,
            SpellType::PhoenixFlare,
            SpellType::Combustion,
            SpellType::Immolate,
            SpellType::Hellfire,
            // Frost
            SpellType::IceShard,
            SpellType::FrostNova,
            SpellType::Blizzard,
            SpellType::FrozenRay,
            SpellType::GlacialSpike,
            SpellType::IceBarrier,
            SpellType::Shatter,
            SpellType::AbsoluteZero,
            // Poison
            SpellType::VenomBolt,
            SpellType::PlagueCloud,
            SpellType::ToxicSpray,
            SpellType::Miasma,
            SpellType::CorrosivePool,
            SpellType::Pandemic,
            SpellType::Blight,
            SpellType::Necrosis,
            // Lightning
            SpellType::Spark,
            SpellType::ChainLightning,
            SpellType::ThunderStrike,
            SpellType::StaticField,
            SpellType::Flashstep,
            SpellType::Overcharge,
            SpellType::Electrocute,
            SpellType::StormCall,
            // Light
            SpellType::HolyBeam,
            SpellType::RadiantBeam,
            SpellType::Radiance,
            SpellType::Smite,
            SpellType::DivineLight,
            SpellType::Consecration,
            SpellType::Purify,
            SpellType::Judgment,
            // Dark
            SpellType::ShadowBolt,
            SpellType::VoidRift,
            SpellType::DarkPulse,
            SpellType::Corruption,
            SpellType::SoulDrain,
            SpellType::Nightmare,
            SpellType::Eclipse,
            SpellType::Oblivion,
            // Chaos
            SpellType::WildMagic,
            SpellType::Entropy,
            SpellType::ChaosBolt,
            SpellType::Randomize,
            SpellType::Unstable,
            SpellType::Paradox,
            SpellType::Mayhem,
            SpellType::Cataclysm,
            // Psychic
            SpellType::MindBlast,
            SpellType::Telekinesis,
            SpellType::PsychicWave,
            SpellType::Confusion,
            SpellType::MentalSpike,
            SpellType::Hallucination,
            SpellType::Dominate,
            SpellType::PsychicShatter,
        ]
    }

    /// Returns all spells for a given element.
    pub fn by_element(element: Element) -> &'static [SpellType] {
        match element {
            Element::Fire => &[
                SpellType::Fireball,
                SpellType::FlameLance,
                SpellType::Ashfall,
                SpellType::MeteorShower,
                SpellType::PhoenixFlare,
                SpellType::Combustion,
                SpellType::Immolate,
                SpellType::Hellfire,
            ],
            Element::Frost => &[
                SpellType::IceShard,
                SpellType::FrostNova,
                SpellType::Blizzard,
                SpellType::FrozenRay,
                SpellType::GlacialSpike,
                SpellType::IceBarrier,
                SpellType::Shatter,
                SpellType::AbsoluteZero,
            ],
            Element::Poison => &[
                SpellType::VenomBolt,
                SpellType::PlagueCloud,
                SpellType::ToxicSpray,
                SpellType::Miasma,
                SpellType::CorrosivePool,
                SpellType::Pandemic,
                SpellType::Blight,
                SpellType::Necrosis,
            ],
            Element::Lightning => &[
                SpellType::Spark,
                SpellType::ChainLightning,
                SpellType::ThunderStrike,
                SpellType::StaticField,
                SpellType::Flashstep,
                SpellType::Overcharge,
                SpellType::Electrocute,
                SpellType::StormCall,
            ],
            Element::Light => &[
                SpellType::HolyBeam,
                SpellType::RadiantBeam,
                SpellType::Radiance,
                SpellType::Smite,
                SpellType::DivineLight,
                SpellType::Consecration,
                SpellType::Purify,
                SpellType::Judgment,
            ],
            Element::Dark => &[
                SpellType::ShadowBolt,
                SpellType::VoidRift,
                SpellType::DarkPulse,
                SpellType::Corruption,
                SpellType::SoulDrain,
                SpellType::Nightmare,
                SpellType::Eclipse,
                SpellType::Oblivion,
            ],
            Element::Chaos => &[
                SpellType::WildMagic,
                SpellType::Entropy,
                SpellType::ChaosBolt,
                SpellType::Randomize,
                SpellType::Unstable,
                SpellType::Paradox,
                SpellType::Mayhem,
                SpellType::Cataclysm,
            ],
            Element::Psychic => &[
                SpellType::MindBlast,
                SpellType::Telekinesis,
                SpellType::PsychicWave,
                SpellType::Confusion,
                SpellType::MentalSpike,
                SpellType::Hallucination,
                SpellType::Dominate,
                SpellType::PsychicShatter,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    mod spell_count_tests {
        use super::*;

        #[test]
        fn all_returns_exactly_64_variants() {
            assert_eq!(SpellType::all().len(), 64);
        }

        #[test]
        fn each_element_has_8_spells() {
            for element in Element::all() {
                let spells = SpellType::by_element(*element);
                assert_eq!(
                    spells.len(),
                    8,
                    "Element {:?} should have 8 spells, got {}",
                    element,
                    spells.len()
                );
            }
        }

        #[test]
        fn by_element_returns_correct_total() {
            let total: usize = Element::all()
                .iter()
                .map(|e| SpellType::by_element(*e).len())
                .sum();
            assert_eq!(total, 64);
        }
    }

    mod id_tests {
        use super::*;

        #[test]
        fn all_ids_are_unique() {
            let mut seen = HashSet::new();
            for spell in SpellType::all() {
                let id = spell.id();
                assert!(
                    seen.insert(id),
                    "Duplicate ID found: {} for {:?}",
                    id,
                    spell
                );
            }
            assert_eq!(seen.len(), 64);
        }

        #[test]
        fn ids_are_sequential_0_to_63() {
            let mut ids: Vec<u32> = SpellType::all().iter().map(|s| s.id()).collect();
            ids.sort();
            for (i, id) in ids.iter().enumerate() {
                assert_eq!(
                    *id, i as u32,
                    "Expected ID {} at position {}, got {}",
                    i, i, id
                );
            }
        }

        #[test]
        fn fire_spells_have_ids_0_to_7() {
            for spell in SpellType::by_element(Element::Fire) {
                let id = spell.id();
                assert!(id <= 7, "Fire spell {:?} has ID {} (expected 0-7)", spell, id);
            }
        }

        #[test]
        fn frost_spells_have_ids_8_to_15() {
            for spell in SpellType::by_element(Element::Frost) {
                let id = spell.id();
                assert!(
                    (8..=15).contains(&id),
                    "Frost spell {:?} has ID {} (expected 8-15)",
                    spell,
                    id
                );
            }
        }

        #[test]
        fn psychic_spells_have_ids_56_to_63() {
            for spell in SpellType::by_element(Element::Psychic) {
                let id = spell.id();
                assert!(
                    (56..=63).contains(&id),
                    "Psychic spell {:?} has ID {} (expected 56-63)",
                    spell,
                    id
                );
            }
        }
    }

    mod element_tests {
        use super::*;

        #[test]
        fn each_spell_returns_correct_element() {
            for element in Element::all() {
                for spell in SpellType::by_element(*element) {
                    assert_eq!(
                        spell.element(),
                        *element,
                        "Spell {:?} should return element {:?}",
                        spell,
                        element
                    );
                }
            }
        }

        #[test]
        fn fireball_is_fire_element() {
            assert_eq!(SpellType::Fireball.element(), Element::Fire);
        }

        #[test]
        fn frost_nova_is_frost_element() {
            assert_eq!(SpellType::FrostNova.element(), Element::Frost);
        }

        #[test]
        fn venom_bolt_is_poison_element() {
            assert_eq!(SpellType::VenomBolt.element(), Element::Poison);
        }

        #[test]
        fn thunder_strike_is_lightning_element() {
            assert_eq!(SpellType::ThunderStrike.element(), Element::Lightning);
        }

        #[test]
        fn radiant_beam_is_light_element() {
            assert_eq!(SpellType::RadiantBeam.element(), Element::Light);
        }

        #[test]
        fn shadow_bolt_is_dark_element() {
            assert_eq!(SpellType::ShadowBolt.element(), Element::Dark);
        }

        #[test]
        fn wild_magic_is_chaos_element() {
            assert_eq!(SpellType::WildMagic.element(), Element::Chaos);
        }

        #[test]
        fn mind_blast_is_psychic_element() {
            assert_eq!(SpellType::MindBlast.element(), Element::Psychic);
        }
    }

    mod name_tests {
        use super::*;

        #[test]
        fn all_spells_have_non_empty_names() {
            for spell in SpellType::all() {
                assert!(
                    !spell.name().is_empty(),
                    "Spell {:?} has empty name",
                    spell
                );
            }
        }

        #[test]
        fn fireball_name_is_fireball() {
            assert_eq!(SpellType::Fireball.name(), "Fireball");
        }

        #[test]
        fn chain_lightning_name_has_space() {
            assert_eq!(SpellType::ChainLightning.name(), "Chain Lightning");
        }
    }

    mod description_tests {
        use super::*;

        #[test]
        fn all_spells_have_non_empty_descriptions() {
            for spell in SpellType::all() {
                assert!(
                    !spell.description().is_empty(),
                    "Spell {:?} has empty description",
                    spell
                );
            }
        }

        #[test]
        fn descriptions_end_with_period() {
            for spell in SpellType::all() {
                let desc = spell.description();
                assert!(
                    desc.ends_with('.'),
                    "Spell {:?} description should end with period: {}",
                    spell,
                    desc
                );
            }
        }
    }

    mod base_damage_tests {
        use super::*;

        #[test]
        fn all_spells_have_positive_damage() {
            for spell in SpellType::all() {
                assert!(
                    spell.base_damage() > 0.0,
                    "Spell {:?} should have positive base damage",
                    spell
                );
            }
        }

        #[test]
        fn fireball_base_damage_is_15() {
            assert_eq!(SpellType::Fireball.base_damage(), 15.0);
        }

        #[test]
        fn cataclysm_has_highest_chaos_damage() {
            let chaos_spells = SpellType::by_element(Element::Chaos);
            let max_damage = chaos_spells
                .iter()
                .map(|s| s.base_damage())
                .fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(SpellType::Cataclysm.base_damage(), max_damage);
        }
    }

    mod fire_rate_tests {
        use super::*;

        #[test]
        fn all_spells_have_positive_fire_rate() {
            for spell in SpellType::all() {
                assert!(
                    spell.fire_rate() > 0.0,
                    "Spell {:?} should have positive fire rate",
                    spell
                );
            }
        }

        #[test]
        fn spark_has_high_fire_rate() {
            // Spark should be a fast, quick spell
            assert!(
                SpellType::Spark.fire_rate() >= 4.0,
                "Spark should have high fire rate"
            );
        }

        #[test]
        fn cataclysm_has_low_fire_rate() {
            // Ultimate spells should be slow
            assert!(
                SpellType::Cataclysm.fire_rate() <= 0.2,
                "Cataclysm should have low fire rate"
            );
        }
    }

    mod trait_tests {
        use super::*;

        #[test]
        fn spell_type_is_clone() {
            let spell = SpellType::Fireball;
            let cloned = spell.clone();
            assert_eq!(spell, cloned);
        }

        #[test]
        fn spell_type_is_copy() {
            let spell = SpellType::Fireball;
            let copied = spell;
            assert_eq!(spell, copied);
        }

        #[test]
        fn spell_type_is_eq() {
            assert_eq!(SpellType::Fireball, SpellType::Fireball);
            assert_ne!(SpellType::Fireball, SpellType::IceShard);
        }

        #[test]
        fn spell_type_is_hashable() {
            let mut set = HashSet::new();
            set.insert(SpellType::Fireball);
            set.insert(SpellType::IceShard);
            assert!(set.contains(&SpellType::Fireball));
            assert!(set.contains(&SpellType::IceShard));
            assert!(!set.contains(&SpellType::VenomBolt));
        }

        #[test]
        fn spell_type_is_debug() {
            let debug_str = format!("{:?}", SpellType::Fireball);
            assert_eq!(debug_str, "Fireball");
        }
    }
}
