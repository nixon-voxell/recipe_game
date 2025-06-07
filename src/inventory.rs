use crate::physics::GameLayer;
use crate::{
    character_controller::CharacterController,
    machine::recipe::RecipeMeta,
};
use avian3d::prelude::*;
use bevy::{platform::collections::HashMap, prelude::*};
use item::{ItemRegistry, ItemType};

mod inventory_input;
pub mod item;

pub(super) struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            inventory_input::InventoryInputPlugin,
            item::ItemPlugin,
        ))
        .add_observer(handle_item_collection)
        .add_systems(Update, detect_item_collisions);

        app.register_type::<Inventory>().register_type::<Item>();
    }
}

/// Detect item collection
fn detect_item_collisions(
    mut collision_events: EventReader<CollisionStarted>,
    q_players: Query<Entity, With<CharacterController>>,
    q_items: Query<&Item>,
    q_collider_of: Query<&ColliderOf>,
    item_registry: ItemRegistry,
    mut commands: Commands,
) {
    let Some(item_meta_asset) = item_registry.get() else {
        return;
    };

    for CollisionStarted(collider1, collider2) in
        collision_events.read()
    {
        // Get the entities that own these colliders
        let entity1 =
            if let Ok(collider_of) = q_collider_of.get(*collider1) {
                collider_of.body
            } else {
                *collider1
            };

        let entity2 =
            if let Ok(collider_of) = q_collider_of.get(*collider2) {
                collider_of.body
            } else {
                *collider2
            };

        // Check if one entity is a player and the other is an item
        let (player_entity, item_entity) = if q_players
            .contains(entity1)
            && q_items.contains(entity2)
        {
            (entity1, entity2)
        } else if q_players.contains(entity2)
            && q_items.contains(entity1)
        {
            (entity2, entity1)
        } else {
            continue;
        };

        if let Ok(item) = q_items.get(item_entity) {
            if let Some(item_meta) = item_meta_asset.get(&item.id) {
                // Only auto-collect ingredients
                if item_meta.item_type == ItemType::Ingredient {
                    info!(
                        "Player {:?} collecting item {:?} ('{}') via collision event",
                        player_entity, item_entity, item.id
                    );

                    // Trigger collection event
                    commands.trigger_targets(
                        ItemCollectionEvent { item: item_entity },
                        player_entity,
                    );
                }
            }
        }
    }
}

/// Observer that handles item collection
fn handle_item_collection(
    trigger: Trigger<ItemCollectionEvent>,
    mut commands: Commands,
    mut q_inventories: Query<&mut Inventory>,
    q_items: Query<&Item>,
    q_players: Query<Entity, With<CharacterController>>,
    item_registry: ItemRegistry,
) {
    let Some(item_meta_asset) = item_registry.get() else {
        return;
    };

    let player_entity = trigger.target();
    let item_entity = trigger.event().item;

    if q_players.get(player_entity).is_err() {
        warn!(
            "Attempted to collect item for non-player entity: {}",
            player_entity
        );
        return;
    }

    // Get the item being collected
    let Ok(world_item) = q_items.get(item_entity) else {
        warn!(
            "Attempted to collect non-existent item: {}",
            item_entity
        );
        return;
    };

    let Some(item_meta) = item_meta_asset.get(&world_item.id) else {
        warn!("Item {} not found in registry", world_item.id);
        return;
    };

    // Ensure player has an inventory
    let mut inventory_just_created = false;
    if q_inventories.get(player_entity).is_err() {
        commands.entity(player_entity).insert(Inventory::default());
        inventory_just_created = true;
        info!("Created new inventory for player {:?}", player_entity);
    }

    if inventory_just_created {
        commands.trigger_targets(
            ItemCollectionEvent { item: item_entity },
            player_entity,
        );
        return;
    }

    let Ok(mut inventory) = q_inventories.get_mut(player_entity)
    else {
        warn!("Player {:?} has no inventory", player_entity);
        return;
    };

    let item_id = &world_item.id;
    let collected_quantity = world_item.quantity;

    // Add to inventory based on item type
    let success = match item_meta.item_type {
        ItemType::Ingredient => inventory.add_ingredient(
            item_id.clone(),
            collected_quantity,
            item_meta.max_stack_size,
        ),
        ItemType::Tower => inventory.add_tower(
            item_id.clone(),
            collected_quantity,
            item_meta.max_stack_size,
        ),
    };

    if success {
        info!(
            "Player {:?} collected {}x {} ({})",
            player_entity,
            collected_quantity,
            item_id,
            match item_meta.item_type {
                ItemType::Ingredient => "ingredient",
                ItemType::Tower => "tower",
            }
        );

        // Remove the item from the world
        commands.entity(item_entity).despawn();
    } else {
        // TODO: Handle stack overflow
        // For now, just log a warning
        warn!(
            "Could not collect {}x {}: would exceed max stack size ({})",
            collected_quantity, item_id, item_meta.max_stack_size
        );
    }
}

#[derive(Event)]
pub struct ItemCollectionEvent {
    pub item: Entity,
}

/// Marks an entity as having an inventory for both towers and ingredients
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Inventory {
    /// Map of tower ID to quantity available (can be selected and placed)
    towers: HashMap<String, u32>,
    /// Map of ingredient ID to quantity collected (display only, cannot be selected)
    ingredients: HashMap<String, u32>,
    /// Currently selected tower for placement (if any)
    pub selected_tower: Option<String>,
}

impl Inventory {
    /// Add towers to the inventory.
    pub fn add_tower(
        &mut self,
        tower_id: String,
        quantity: u32,
        max_stack_size: u32,
    ) -> bool {
        let current_count =
            self.towers.get(&tower_id).copied().unwrap_or(0);
        let new_total = current_count + quantity;

        if new_total <= max_stack_size {
            self.towers.insert(tower_id, new_total);
            true
        } else {
            false
        }
    }

    /// Remove towers from the inventory
    pub fn remove_tower(
        &mut self,
        tower_id: &str,
        quantity: u32,
    ) -> bool {
        let current_count =
            self.towers.get(tower_id).copied().unwrap_or(0);
        if current_count >= quantity {
            let new_count = current_count - quantity;
            if new_count == 0 {
                self.towers.remove(tower_id);
            } else {
                self.towers.insert(tower_id.to_string(), new_count);
            }
            true
        } else {
            false
        }
    }

    /// Add ingredients to the inventory with stack limit checking
    pub fn add_ingredient(
        &mut self,
        ingredient_id: String,
        quantity: u32,
        max_stack_size: u32,
    ) -> bool {
        let current_count = self
            .ingredients()
            .get(&ingredient_id)
            .copied()
            .unwrap_or(0);
        let new_total = current_count + quantity;

        if new_total <= max_stack_size {
            self.ingredients.insert(ingredient_id, new_total);
            true
        } else {
            false
        }
    }

    pub fn has_recipe(&self, recipe: &RecipeMeta) -> bool {
        for ingredient in recipe.ingredients.iter() {
            let available_quantity = self
                .ingredients
                .get(&ingredient.item_id)
                .copied()
                .unwrap_or(0);

            if available_quantity < ingredient.quantity {
                return false;
            }
        }

        true
    }

    /// Check if the inventory has the required ingredients and use it.
    ///
    /// This will call [`Self::has_recipe()`] first.
    pub fn check_and_use_recipe(
        &mut self,
        recipe: &RecipeMeta,
    ) -> bool {
        if self.has_recipe(recipe) == false {
            return false;
        }

        for ingredient in recipe.ingredients.iter() {
            // SAFETY: We already made sure that the ingredients above.
            // In caes the quantity is actually zero!
            let mut default_0 = 0;
            let available_quantity = self
                .ingredients
                .get_mut(&ingredient.item_id)
                .unwrap_or(&mut default_0);

            *available_quantity -= ingredient.quantity;
        }

        true
    }
}

impl Inventory {
    pub fn ingredients(&self) -> &HashMap<String, u32> {
        &self.ingredients
    }

    pub fn towers(&self) -> &HashMap<String, u32> {
        &self.towers
    }
}

/// Core data for any item (both towers and ingredients).
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
    CollisionEventsEnabled,
    CollisionLayers::new(GameLayer::InventoryItem, LayerMask::ALL,)
)]
pub struct Item {
    /// A unique identifier that corresponds to [`item::ItemMeta`]
    pub id: String,
    /// How many are in this stack.
    pub quantity: u32,
}
