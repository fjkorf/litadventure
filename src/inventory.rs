use bevy::prelude::*;

use crate::game_data::{GameDataHandles, GameDataReady, RecipeBook};
use crate::interaction::FeedbackText;

/// A collected item in the player's inventory.
#[derive(Debug, Clone)]
pub struct Item {
    pub item_id: String,
    pub name: String,
    pub description: String,
    pub qty: u32,
}

/// The player's inventory.
#[derive(Resource, Default, Debug)]
pub struct Inventory {
    pub items: Vec<Item>,
    pub combination_recipes: Vec<CombinationRecipe>,
}

/// Defines what happens when two items are combined.
#[derive(Debug, Clone)]
pub struct CombinationRecipe {
    pub item_a: String,
    pub item_b: String,
    pub result_id: String,
    pub result_name: String,
    pub result_description: String,
}

impl Inventory {
    pub fn add_item(&mut self, item_id: &str, name: &str, description: &str) {
        if let Some(existing) = self.items.iter_mut().find(|i| i.item_id == item_id) {
            existing.qty += 1;
        } else {
            self.items.push(Item {
                item_id: item_id.to_string(),
                name: name.to_string(),
                description: description.to_string(),
                qty: 1,
            });
        }
    }

    pub fn remove_item(&mut self, item_id: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|i| i.item_id == item_id) {
            if self.items[pos].qty > 1 {
                self.items[pos].qty -= 1;
            } else {
                self.items.remove(pos);
            }
            true
        } else {
            false
        }
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.items.iter().any(|i| i.item_id == item_id)
    }

    pub fn combine(&mut self, item_a: &str, item_b: &str) -> Option<String> {
        let recipe = self.combination_recipes.iter().find(|r| {
            (r.item_a == item_a && r.item_b == item_b)
                || (r.item_a == item_b && r.item_b == item_a)
        });

        let recipe = recipe?.clone();

        // Consume ingredients
        self.remove_item(item_a);
        self.remove_item(item_b);

        // Add result
        self.add_item(&recipe.result_id, &recipe.result_name, &recipe.result_description);

        Some(recipe.result_name)
    }
}

/// Message fired when the player picks up an item.
#[derive(Message)]
pub struct ItemPickedUp {
    pub item_id: String,
    pub name: String,
}

/// System: when an InventoryItem entity is clicked and collected,
/// add it to the inventory and despawn the world entity.
pub fn collect_item(
    mut events: MessageReader<ItemPickedUp>,
    mut inventory: ResMut<Inventory>,
    mut feedback: ResMut<FeedbackText>,
) {
    for ev in events.read() {
        inventory.add_item(&ev.item_id, &ev.name, "");
        feedback.0 = format!("You pick up the {}.", ev.name);
    }
}

/// Load recipes from RON data once assets are ready.
fn load_recipes_from_data(
    ready: Res<GameDataReady>,
    handles: Res<GameDataHandles>,
    assets: Res<Assets<RecipeBook>>,
    mut inventory: ResMut<Inventory>,
    mut loaded: Local<bool>,
) {
    if *loaded || !ready.0 {
        return;
    }

    let Some(handle) = &handles.recipes else {
        return;
    };
    let Some(data) = assets.get(handle) else {
        return;
    };

    for def in &data.recipes {
        inventory.combination_recipes.push(CombinationRecipe {
            item_a: def.item_a.clone(),
            item_b: def.item_b.clone(),
            result_id: def.result_id.clone(),
            result_name: def.result_name.clone(),
            result_description: def.result_description.clone(),
        });
    }

    *loaded = true;
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .add_message::<ItemPickedUp>()
            .add_systems(Update, (load_recipes_from_data, collect_item));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_item_and_stack() {
        let mut inv = Inventory::default();
        inv.add_item("key", "Key", "A brass key.");
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].qty, 1);

        inv.add_item("key", "Key", "A brass key.");
        assert_eq!(inv.items.len(), 1);
        assert_eq!(inv.items[0].qty, 2);

        inv.add_item("flashlight", "Flashlight", "A small flashlight.");
        assert_eq!(inv.items.len(), 2);
    }

    #[test]
    fn remove_item_decrements_and_removes() {
        let mut inv = Inventory::default();
        inv.add_item("potion", "Potion", "Heals you.");
        inv.add_item("potion", "Potion", "Heals you.");
        assert_eq!(inv.items[0].qty, 2);

        assert!(inv.remove_item("potion"));
        assert_eq!(inv.items[0].qty, 1);

        assert!(inv.remove_item("potion"));
        assert!(inv.items.is_empty());

        assert!(!inv.remove_item("potion"));
    }

    #[test]
    fn has_item_works() {
        let mut inv = Inventory::default();
        assert!(!inv.has_item("key"));
        inv.add_item("key", "Key", "");
        assert!(inv.has_item("key"));
    }

    #[test]
    fn combine_items_with_recipe() {
        let mut inv = Inventory::default();
        inv.add_item("lens", "Lens", "A glass lens.");
        inv.add_item("tube", "Tube", "A metal tube.");
        inv.combination_recipes.push(CombinationRecipe {
            item_a: "lens".into(),
            item_b: "tube".into(),
            result_id: "telescope".into(),
            result_name: "Telescope".into(),
            result_description: "A small telescope.".into(),
        });

        let result = inv.combine("lens", "tube");
        assert_eq!(result.as_deref(), Some("Telescope"));
        assert!(inv.has_item("telescope"));
        assert!(!inv.has_item("lens"));
        assert!(!inv.has_item("tube"));
    }

    #[test]
    fn combine_without_recipe_returns_none() {
        let mut inv = Inventory::default();
        inv.add_item("rock", "Rock", "");
        inv.add_item("stick", "Stick", "");

        let result = inv.combine("rock", "stick");
        assert!(result.is_none());
        // Items should still be there (not consumed)
        assert!(inv.has_item("rock"));
        assert!(inv.has_item("stick"));
    }

    #[test]
    fn combine_is_order_independent() {
        let mut inv = Inventory::default();
        inv.add_item("a", "A", "");
        inv.add_item("b", "B", "");
        inv.combination_recipes.push(CombinationRecipe {
            item_a: "a".into(),
            item_b: "b".into(),
            result_id: "c".into(),
            result_name: "C".into(),
            result_description: "".into(),
        });

        // Reversed order should still work
        let result = inv.combine("b", "a");
        assert_eq!(result.as_deref(), Some("C"));
    }
}
