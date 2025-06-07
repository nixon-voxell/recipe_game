use bevy::prelude::*;

pub(super) struct PlayerMarkPlugin;

impl Plugin for PlayerMarkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMark(3));

        app.add_systems(
            Update,
            check_player_mark.run_if(resource_changed::<PlayerMark>),
        );
    }
}

fn check_player_mark(_player_mark: Res<PlayerMark>) {}

#[derive(Resource, Deref, DerefMut)]
pub struct PlayerMark(pub u32);
