use bevy::prelude::*;

#[derive(Component)]
pub struct Rock;

/// Marker component for the ground plane in 3D space
#[derive(Component)]
pub struct GroundPlane;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rock_component_can_be_created() {
        let _rock = Rock;
    }

    #[test]
    fn test_ground_plane_component_can_be_created() {
        let _ground = GroundPlane;
    }
}