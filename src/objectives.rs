use bevy::prelude::*;

use crate::game_data::{GameDataHandles, GameDataReady, ObjectiveSet};
use crate::interaction::PlayerNavigated;
use crate::inventory::Inventory;

/// A single objective.
#[derive(Debug, Clone)]
pub struct Objective {
    pub id: String,
    pub text: String,
    pub visible: bool,
    pub completed: bool,
    /// Objectives that this one conflicts with (completing this removes them).
    pub conflicts_with: Vec<String>,
    /// Objectives that activate when this one completes.
    pub unlocks: Vec<String>,
    /// Item required to auto-complete this objective.
    pub requires_item: Option<String>,
    /// Camera spot navigation that completes this objective.
    pub requires_navigation: Option<String>,
}

/// Manages all game objectives.
#[derive(Resource, Default, Debug)]
pub struct Objectives {
    pub all: Vec<Objective>,
}

/// Message fired when an objective completes.
#[derive(Message)]
pub struct ObjectiveCompleted {
    pub id: String,
}

impl Objectives {
    pub fn add(&mut self, id: &str, text: &str, visible: bool) {
        if self.all.iter().any(|o| o.id == id) {
            return;
        }
        self.all.push(Objective {
            id: id.to_string(),
            text: text.to_string(),
            visible,
            completed: false,
            conflicts_with: Vec::new(),
            unlocks: Vec::new(),
            requires_item: None,
            requires_navigation: None,
        });
    }

    pub fn complete(&mut self, id: &str) -> bool {
        let Some(obj) = self.all.iter_mut().find(|o| o.id == id && !o.completed) else {
            return false;
        };
        obj.completed = true;

        let conflicts: Vec<String> = obj.conflicts_with.clone();
        let unlocks: Vec<String> = obj.unlocks.clone();

        // Remove conflicting objectives
        for cid in &conflicts {
            if let Some(c) = self.all.iter_mut().find(|o| o.id == *cid) {
                c.completed = true;
                c.visible = false;
            }
        }

        // Reveal unlocked objectives
        for uid in &unlocks {
            if let Some(u) = self.all.iter_mut().find(|o| o.id == *uid) {
                u.visible = true;
            }
        }

        true
    }

    pub fn active_visible(&self) -> Vec<&Objective> {
        self.all
            .iter()
            .filter(|o| o.visible && !o.completed)
            .collect()
    }

    pub fn current_text(&self) -> &str {
        self.active_visible()
            .first()
            .map(|o| o.text.as_str())
            .unwrap_or("Look around.")
    }

    pub fn is_completed(&self, id: &str) -> bool {
        self.all
            .iter()
            .any(|o| o.id == id && o.completed)
    }
}

/// Check if any objective's required item has been collected.
fn check_item_objectives(
    inv: Res<Inventory>,
    objectives: Res<Objectives>,
    mut completed: MessageWriter<ObjectiveCompleted>,
) {
    for obj in &objectives.all {
        if obj.completed {
            continue;
        }
        if let Some(ref required) = obj.requires_item {
            if inv.has_item(required) {
                completed.write(ObjectiveCompleted {
                    id: obj.id.clone(),
                });
            }
        }
    }
}

/// Check if any objective's navigation requirement has been met.
fn check_navigation_objectives(
    mut reader: MessageReader<PlayerNavigated>,
    objectives: Res<Objectives>,
    mut completed: MessageWriter<ObjectiveCompleted>,
) {
    for ev in reader.read() {
        for obj in &objectives.all {
            if obj.completed {
                continue;
            }
            if let Some(ref required) = obj.requires_navigation {
                if *required == ev.spot_name {
                    completed.write(ObjectiveCompleted {
                        id: obj.id.clone(),
                    });
                }
            }
        }
    }
}

/// Process objective completion messages.
fn process_completions(
    mut reader: MessageReader<ObjectiveCompleted>,
    mut objectives: ResMut<Objectives>,
) {
    for ev in reader.read() {
        objectives.complete(&ev.id);
    }
}

/// Load objectives from RON data once assets are ready.
fn load_objectives_from_data(
    ready: Res<GameDataReady>,
    handles: Res<GameDataHandles>,
    assets: Res<Assets<ObjectiveSet>>,
    mut objectives: ResMut<Objectives>,
    mut loaded: Local<bool>,
) {
    if *loaded || !ready.0 {
        return;
    }

    let Some(handle) = &handles.objectives else {
        return;
    };
    let Some(data) = assets.get(handle) else {
        return;
    };

    for def in &data.objectives {
        objectives.all.push(Objective {
            id: def.id.clone(),
            text: def.text.clone(),
            visible: def.visible,
            completed: false,
            conflicts_with: def.conflicts_with.clone(),
            unlocks: def.unlocks.clone(),
            requires_item: def.requires_item.clone(),
            requires_navigation: def.requires_navigation.clone(),
        });
    }

    *loaded = true;
}

pub struct ObjectivesPlugin;

impl Plugin for ObjectivesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Objectives>()
            .add_message::<ObjectiveCompleted>()
            .add_systems(Update, load_objectives_from_data)
            .add_systems(
                Update,
                (
                    check_item_objectives,
                    check_navigation_objectives,
                    process_completions,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_query_objectives() {
        let mut obj = Objectives::default();
        obj.add("a", "Do A", true);
        obj.add("b", "Do B", false);

        assert_eq!(obj.active_visible().len(), 1);
        assert_eq!(obj.current_text(), "Do A");
    }

    #[test]
    fn complete_objective() {
        let mut obj = Objectives::default();
        obj.add("a", "Do A", true);

        assert!(obj.complete("a"));
        assert!(obj.is_completed("a"));
        assert!(obj.active_visible().is_empty());
    }

    #[test]
    fn complete_unlocks_next() {
        let mut obj = Objectives::default();
        obj.add("a", "Do A", true);
        obj.add("b", "Do B", false);
        obj.all[0].unlocks.push("b".into());

        obj.complete("a");
        assert_eq!(obj.active_visible().len(), 1);
        assert_eq!(obj.current_text(), "Do B");
    }

    #[test]
    fn complete_removes_conflicts() {
        let mut obj = Objectives::default();
        obj.add("a", "Path A", true);
        obj.add("b", "Path B", true);
        obj.all[0].conflicts_with.push("b".into());

        obj.complete("a");
        assert!(obj.is_completed("a"));
        assert!(obj.is_completed("b")); // Conflict removed
    }

    #[test]
    fn duplicate_add_ignored() {
        let mut obj = Objectives::default();
        obj.add("a", "Do A", true);
        obj.add("a", "Do A again", true);
        assert_eq!(obj.all.len(), 1);
    }
}
