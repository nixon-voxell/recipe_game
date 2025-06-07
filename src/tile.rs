use bevy::prelude::*;
use pathfinding::prelude::*;

pub(super) struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileMap>()
            .add_systems(
                PostUpdate,
                setup_tile.after(TransformSystem::TransformPropagate),
            )
            .add_observer(on_placed)
            .add_observer(on_freed);

        app.register_type::<Tile>();

        #[cfg(feature = "dev")]
        app.register_type::<TileMap>();
    }
}

/// The half size of the map, this should be
/// greater or equal to the half size of the map.
const HALF_MAP_SIZE: usize = 20;

/// Setup tile inside the [`TileMap`].
fn setup_tile(
    q_tiles: Query<
        (&GlobalTransform, Entity),
        (Or<(Added<Tile>, Added<GlobalTransform>)>, With<Tile>),
    >,
    mut tile_map: ResMut<TileMap>,
) -> Result {
    for (transform, entity) in q_tiles.iter() {
        let translation = transform.translation();

        *tile_map.get_mut(&translation).ok_or(format!(
            "Unable to get tile for {entity}, {translation}"
        ))? = Some(TileMeta::new(entity));
    }

    Ok(())
}

fn on_placed(
    trigger: Trigger<OnAdd, PlacedBy>,
    q_transforms: Query<&GlobalTransform>,
    mut tile_map: ResMut<TileMap>,
) -> Result {
    let entity = trigger.target();

    let transform = q_transforms.get(entity)?;

    if let Some(tile) = tile_map
        .get_mut(&transform.translation())
        .ok_or(format!(
            "Unable to get tile for {entity}, {transform:?}"
        ))?
        .as_mut()
    {
        tile.occupied = true;
    }

    Ok(())
}

fn on_freed(
    trigger: Trigger<OnRemove, PlacedBy>,
    q_transforms: Query<&GlobalTransform>,
    mut tile_map: ResMut<TileMap>,
) -> Result {
    let entity = trigger.target();

    let transform = q_transforms.get(entity)?;

    if let Some(tile) = tile_map
        .get_mut(&transform.translation())
        .ok_or(format!(
            "Unable to get tile for {entity}, {transform:?}"
        ))?
        .as_mut()
    {
        tile.occupied = false;
    }

    Ok(())
}

#[derive(Resource, Deref)]
#[cfg_attr(feature = "dev", derive(Reflect))]
#[cfg_attr(feature = "dev", reflect(Resource))]
pub struct TileMap(Vec<Option<TileMeta>>);

impl TileMap {
    pub const KNIGHT: &[IVec2] = &[
        // Top.
        IVec2::new(0, 1),
        // Bottom.
        IVec2::new(0, -1),
        // Left.
        IVec2::new(-1, 0),
        // Right.
        IVec2::new(1, 0),
    ];

    pub fn within_map_range(coordinate: &IVec2) -> bool {
        const MAP_SIZE: i32 = HALF_MAP_SIZE as i32 * 2;

        if coordinate.x < 0 || coordinate.y < 0 {
            warn!("Attempt to obtain negative coordinate!");
            return false;
        } else if coordinate.x >= MAP_SIZE || coordinate.y >= MAP_SIZE
        {
            warn!("Attempt to obtain out of bounds coordinate!");
            return false;
        }

        true
    }

    /// Get the closest tile coordinate.
    pub fn translation_to_tile_coord(
        translation: &Vec3,
    ) -> Option<UVec2> {
        // Prevent going negative.
        let coordinate =
            ((translation.xz() * 0.5).round().as_ivec2())
                + HALF_MAP_SIZE as i32;

        if TileMap::within_map_range(&coordinate) == false {
            return None;
        }

        Some(coordinate.as_uvec2())
    }

    pub fn tile_coord_to_tile_idx(coordinate: &UVec2) -> usize {
        let map_size = HALF_MAP_SIZE as u32 * 2;
        (coordinate.x + coordinate.y * map_size) as usize
    }

    pub fn translation_to_tile_idx(
        translation: &Vec3,
    ) -> Option<usize> {
        TileMap::translation_to_tile_coord(translation)
            .map(|coord| TileMap::tile_coord_to_tile_idx(&coord))
    }

    pub fn tile_coord_to_world_space(coordinate: &IVec2) -> Vec2 {
        (coordinate - HALF_MAP_SIZE as i32).as_vec2() * 2.0
    }

    fn get_mut(
        &mut self,
        translation: &Vec3,
    ) -> Option<&mut Option<TileMeta>> {
        TileMap::translation_to_tile_idx(translation)
            .and_then(|index| self.0.get_mut(index))
    }

    /// Find a path from start to end from the tile map.
    ///
    /// If a path is found, a vector of world space [`IVec2`]
    /// will be returned.
    ///
    /// None will be returned if there is no valid path.
    pub fn pathfind_to(
        &self,
        start_translation: &Vec3,
        end_translation: &Vec3,
        to_tower: bool,
    ) -> Option<Vec<IVec2>> {
        let start =
            TileMap::translation_to_tile_coord(start_translation)?
                .as_ivec2();
        let end =
            TileMap::translation_to_tile_coord(end_translation)?
                .as_ivec2();

        Some(
            astar(
                &start,
                |&current| {
                    TileMap::KNIGHT
                        .iter()
                        .map(move |m| current + m)
                        .filter(|coord| {
                            // Must be a valid coordinate.
                            if TileMap::within_map_range(coord)
                                == false
                            {
                                return false;
                            }
                            let index =
                                TileMap::tile_coord_to_tile_idx(
                                    &coord.as_uvec2(),
                                );
                            let tile_meta = self[index];

                            // Must not be occupied.
                            tile_meta.is_some_and(|t| {
                                t.occupied() == false
                            })
                        })
                        .map(|p| (p, 1))
                },
                // Always find the closest to the target.
                |potential| potential.distance_squared(end),
                |&current| {
                    if to_tower {
                        // The surroundings needs to have a tower.
                        TileMap::KNIGHT
                            .iter()
                            .map(move |m| current + m)
                            .any(|coord| {
                                // Must be a valid coordinate.
                                if TileMap::within_map_range(&coord)
                                    == false
                                {
                                    return false;
                                }

                                let index =
                                    TileMap::tile_coord_to_tile_idx(
                                        &coord.as_uvec2(),
                                    );
                                let tile_meta = self[index];

                                // Allow pathfinding towards tower.
                                tile_meta
                                    .is_some_and(|t| t.occupied())
                            })
                    } else {
                        current == end
                    }
                },
            )?
            .0
            .into_iter()
            .collect(),
        )
    }
}

impl Default for TileMap {
    fn default() -> Self {
        const MAP_SIZE: usize = HALF_MAP_SIZE * 2;
        Self(vec![None; MAP_SIZE * MAP_SIZE])
    }
}

#[derive(Reflect, Debug, Clone, Copy)]
pub struct TileMeta {
    #[allow(dead_code)]
    target: Entity,
    occupied: bool,
}

impl TileMeta {
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            occupied: false,
        }
    }

    pub fn occupied(&self) -> bool {
        self.occupied
    }

    pub fn target(&self) -> Entity {
        self.target
    }
}

/// Tag component for tiles that can be placed on.
#[derive(Component, Reflect, Clone, Debug)]
#[reflect(Component)]
pub struct Tile;

/// Attached to a [`Tile`] when it's being placed on.
#[derive(Component, Deref, Default, Debug)]
#[relationship_target(relationship = PlacedOn)]
pub struct PlacedBy(Vec<Entity>);

/// Attached to the item that is being placed on a [`Tile`].
#[derive(Component, Deref, Debug)]
#[relationship(relationship_target = PlacedBy)]
pub struct PlacedOn(pub Entity);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_within_range() {
        let tiles = [
            IVec2::new(-1, 0),
            IVec2::new(0, -1),
            IVec2::new(HALF_MAP_SIZE as i32 * 2, 0),
            IVec2::new(0, HALF_MAP_SIZE as i32 * 2),
        ];

        for tile in tiles {
            assert!(TileMap::within_map_range(&tile) == false);
        }
    }

    #[test]
    fn test_coordinate_spaces() {
        let translation = Vec3::new(2.0, 0.0, 4.0);

        let coord = TileMap::translation_to_tile_coord(&translation)
            .expect("Should be in range.");
        let world_space =
            TileMap::tile_coord_to_world_space(&coord.as_ivec2());

        assert_eq!(world_space, translation.xz());
    }
}
