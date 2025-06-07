use core::time::Duration;

use bevy::animation::AnimationTarget;
use bevy::prelude::*;

use crate::asset_pipeline::animation_pipeline::{
    AnimationGraphMap, NodeMap,
};
use crate::asset_pipeline::{AssetState, PrefabAssets};
use crate::player::PlayerType;

use super::{CharacterController, IsGrounded, IsMoving};

pub(super) struct CharacterAnimationPlugin;

impl Plugin for CharacterAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (setup_animation_graph, movement_animation)
                .run_if(in_state(AssetState::Loaded)),
        );
    }
}

fn movement_animation(
    q_characters: Query<
        (
            &NodeMap,
            &IsMoving,
            &IsGrounded,
            &AnimationTarget,
            &PlayerType,
        ),
        With<CharacterController>,
    >,
    mut q_animation_players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
    )>,
) -> Result {
    for (
        node_map,
        is_moving,
        is_grounded,
        animation_target,
        player_type,
    ) in q_characters.iter()
    {
        let (mut anim_player, mut anim_transitions) =
            q_animation_players.get_mut(animation_target.player)?;

        if is_grounded.0 == false {
            let jump_node =
                *node_map.get("JumpUp").ok_or(format!(
                    "No idle animation found for {:?}!",
                    player_type
                ))?;

            if anim_player.is_playing_animation(jump_node) == false {
                anim_transitions
                    .play(
                        &mut anim_player,
                        jump_node,
                        Duration::from_millis(100),
                    )
                    .set_speed(2.0);
            }

            continue;
        }

        if is_moving.0 {
            let walking_node =
                *node_map.get("Walking").ok_or(format!(
                    "No walking animation found for {:?}!",
                    player_type
                ))?;

            if anim_player.is_playing_animation(walking_node) == false
            {
                anim_transitions
                    .play(
                        &mut anim_player,
                        walking_node,
                        Duration::from_millis(200),
                    )
                    .set_speed(1.5)
                    .repeat();
            }
        } else {
            let idle_node = *node_map.get("Idle").ok_or(format!(
                "No idle animation found for {:?}!",
                player_type
            ))?;

            if anim_player.is_playing_animation(idle_node) == false {
                anim_transitions
                    .play(
                        &mut anim_player,
                        idle_node,
                        Duration::from_millis(200),
                    )
                    .repeat();
            }
        }
    }

    Ok(())
}

fn setup_animation_graph(
    mut commands: Commands,
    q_characters: Query<
        (&PlayerType, &AnimationTarget, Entity),
        (With<CharacterController>, Without<NodeMap>),
    >,
    prefabs: Res<PrefabAssets>,
) -> Result {
    for (player_type, animation_target, entity) in q_characters.iter()
    {
        let AnimationGraphMap { graph, node_map } = prefabs
            .get_animation(player_type.prefab_name())
            .ok_or(format!(
                "Unable to get animation for {player_type:?}!"
            ))?;

        commands.entity(entity).insert(node_map.clone());
        commands.entity(animation_target.player).insert((
            AnimationGraphHandle(graph.clone()),
            AnimationTransitions::new(),
        ));

        info!("Setup animation graph for {player_type:?}.");
    }

    Ok(())
}
