use bevy::animation::graph::AnimationNodeIndex;
use bevy::prelude::*;

use crate::element::Element;

/// Resource holding handles to the whisper's animation clips and scene
#[derive(Resource)]
pub struct WhisperAnimations {
    /// Handle to the whisper GLTF scene
    pub scene: Handle<Scene>,
    /// Animation graph for the whisper
    pub graph: Handle<AnimationGraph>,
    /// Node indices for all animations (one per mesh)
    pub animation_nodes: Vec<AnimationNodeIndex>,
}

/// Resource tracking the spell origin position.
/// When Whisper is not collected, spells are disabled.
/// When Whisper is collected, this contains Whisper's 3D position.
#[derive(Resource, Default)]
pub struct SpellOrigin {
    /// None = spells disabled, Some(pos) = cast from this 3D position
    pub position: Option<Vec3>,
}

impl SpellOrigin {
    pub fn is_active(&self) -> bool {
        self.position.is_some()
    }
}


/// Resource tracking whether Whisper has been collected this game
#[derive(Resource, Default)]
pub struct WhisperState {
    pub collected: bool,
}

/// Resource representing the player's elemental attunement chosen at game start.
/// When Whisper is collected, the player selects an element for a 10% damage bonus
/// to spells matching that element.
#[derive(Resource, Default, Debug, Clone)]
pub struct WhisperAttunement {
    element: Option<Element>,
}

impl WhisperAttunement {
    /// Create new attunement with no element selected.
    pub fn new() -> Self {
        Self { element: None }
    }

    /// Create attunement with a specific element.
    pub fn with_element(element: Element) -> Self {
        Self {
            element: Some(element),
        }
    }

    /// Set the attuned element.
    pub fn set_element(&mut self, element: Element) {
        self.element = Some(element);
    }

    /// Clear the attunement.
    pub fn clear(&mut self) {
        self.element = None;
    }

    /// Get current attuned element.
    pub fn element(&self) -> Option<Element> {
        self.element
    }

    /// Calculate damage multiplier for a spell's element.
    /// Returns 1.1 (10% bonus) for matching element, 1.0 otherwise.
    pub fn damage_multiplier(&self, spell_element: Element) -> f32 {
        match self.element {
            Some(attuned) if attuned == spell_element => 1.1,
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spell_origin_default() {
        let origin = SpellOrigin::default();
        assert!(origin.position.is_none());
        assert!(!origin.is_active());
    }

    #[test]
    fn test_spell_origin_active() {
        let origin = SpellOrigin {
            position: Some(Vec3::new(10.0, 3.0, 20.0)),
        };
        assert!(origin.is_active());
        assert_eq!(origin.position.unwrap(), Vec3::new(10.0, 3.0, 20.0));
    }


    #[test]
    fn test_whisper_state_default() {
        let state = WhisperState::default();
        assert!(!state.collected);
    }

    #[test]
    fn test_whisper_animations_type_check() {
        // WhisperAnimations holds scene, graph, and animation node handles
        // This test verifies the struct exists and type signatures are correct
        // Full integration testing requires asset loading plugins
        use bevy::animation::graph::AnimationNodeIndex;
        use bevy::asset::Handle;

        // Verify the type is correctly defined
        fn _type_check(
            scene: Handle<Scene>,
            graph: Handle<AnimationGraph>,
            animation_nodes: Vec<AnimationNodeIndex>,
        ) -> WhisperAnimations {
            WhisperAnimations {
                scene,
                graph,
                animation_nodes,
            }
        }

        // Just verify the type compiles - actual handle creation is tested in integration
    }

    mod whisper_attunement_new_tests {
        use super::*;

        #[test]
        fn new_has_no_element() {
            let attunement = WhisperAttunement::new();
            assert!(attunement.element().is_none());
        }

        #[test]
        fn default_has_no_element() {
            let attunement = WhisperAttunement::default();
            assert!(attunement.element().is_none());
        }
    }

    mod whisper_attunement_with_element_tests {
        use super::*;

        #[test]
        fn with_element_has_correct_element() {
            let attunement = WhisperAttunement::with_element(Element::Fire);
            assert_eq!(attunement.element(), Some(Element::Fire));
        }

        #[test]
        fn with_element_works_for_all_elements() {
            for &element in Element::all() {
                let attunement = WhisperAttunement::with_element(element);
                assert_eq!(attunement.element(), Some(element));
            }
        }
    }

    mod whisper_attunement_set_element_tests {
        use super::*;

        #[test]
        fn set_element_changes_element() {
            let mut attunement = WhisperAttunement::new();
            attunement.set_element(Element::Lightning);
            assert_eq!(attunement.element(), Some(Element::Lightning));
        }

        #[test]
        fn set_element_overwrites_existing() {
            let mut attunement = WhisperAttunement::with_element(Element::Fire);
            attunement.set_element(Element::Frost);
            assert_eq!(attunement.element(), Some(Element::Frost));
        }
    }

    mod whisper_attunement_clear_tests {
        use super::*;

        #[test]
        fn clear_removes_element() {
            let mut attunement = WhisperAttunement::with_element(Element::Dark);
            attunement.clear();
            assert!(attunement.element().is_none());
        }

        #[test]
        fn clear_on_empty_remains_empty() {
            let mut attunement = WhisperAttunement::new();
            attunement.clear();
            assert!(attunement.element().is_none());
        }
    }

    mod whisper_attunement_damage_multiplier_tests {
        use super::*;

        #[test]
        fn damage_multiplier_no_attunement_returns_1() {
            let attunement = WhisperAttunement::new();
            assert_eq!(attunement.damage_multiplier(Element::Fire), 1.0);
        }

        #[test]
        fn damage_multiplier_matching_element_returns_1_1() {
            let attunement = WhisperAttunement::with_element(Element::Fire);
            assert_eq!(attunement.damage_multiplier(Element::Fire), 1.1);
        }

        #[test]
        fn damage_multiplier_non_matching_element_returns_1() {
            let attunement = WhisperAttunement::with_element(Element::Fire);
            assert_eq!(attunement.damage_multiplier(Element::Frost), 1.0);
        }

        #[test]
        fn damage_multiplier_each_element_matches_correctly() {
            for &attuned_element in Element::all() {
                let attunement = WhisperAttunement::with_element(attuned_element);
                for &spell_element in Element::all() {
                    let multiplier = attunement.damage_multiplier(spell_element);
                    if attuned_element == spell_element {
                        assert_eq!(
                            multiplier, 1.1,
                            "Expected 1.1 for matching {:?} == {:?}",
                            attuned_element, spell_element
                        );
                    } else {
                        assert_eq!(
                            multiplier, 1.0,
                            "Expected 1.0 for non-matching {:?} != {:?}",
                            attuned_element, spell_element
                        );
                    }
                }
            }
        }
    }

    mod whisper_attunement_trait_tests {
        use super::*;

        #[test]
        fn attunement_is_clone() {
            let original = WhisperAttunement::with_element(Element::Chaos);
            let cloned = original.clone();
            assert_eq!(cloned.element(), Some(Element::Chaos));
        }

        #[test]
        fn attunement_is_debug() {
            let attunement = WhisperAttunement::with_element(Element::Psychic);
            let debug_str = format!("{:?}", attunement);
            assert!(debug_str.contains("WhisperAttunement"));
        }
    }
}
