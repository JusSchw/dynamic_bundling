use std::sync::Arc;

use bevy_ecs::{
    component::{ComponentHooks, StorageType},
    prelude::*,
    world::Command,
};
use bevy_hierarchy::{BuildChildren, ChildBuild};

type BundleFn = Arc<dyn Fn(&mut EntityWorldMut) + Send + Sync>;

#[derive(Clone)]
pub struct DynBundle {
    bundle: BundleFn,
    parent: Option<Arc<DynBundle>>,
}

impl DynBundle {
    pub fn new<B: Bundle + Clone>(bundle: B) -> Self {
        DynBundle {
            bundle: Arc::new(move |entity: &mut EntityWorldMut| {
                entity.insert(bundle.clone());
            }),
            parent: None,
        }
    }

    pub fn insert<B: Bundle + Clone>(&self, bundle: B) -> Self {
        DynBundle {
            bundle: Arc::new(move |entity: &mut EntityWorldMut| {
                entity.insert(bundle.clone());
            }),
            parent: Some(Arc::new(self.clone())),
        }
    }

    pub fn remove<B: Bundle + Clone>(&self) -> Self {
        DynBundle {
            bundle: Arc::new(move |entity: &mut EntityWorldMut| {
                entity.remove::<B>();
            }),
            parent: Some(Arc::new(self.clone())),
        }
    }

    pub fn append(&self, dyn_bundle: DynBundle) -> Self {
        DynBundle {
            bundle: dyn_bundle.bundle.clone(),
            parent: match dyn_bundle.parent {
                Some(parent) => Some(Arc::new((*parent).append(self.clone()))),
                None => Some(Arc::new(self.clone())),
            },
        }
    }

    pub fn append_some(&self, opt_bundle: Option<DynBundle>) -> Self {
        if let Some(bundle) = opt_bundle {
            self.append(bundle);
        }
        self.clone()
    }

    fn apply(&self, entity_mut: &mut EntityWorldMut) {
        if let Some(ref parent) = self.parent {
            parent.apply(entity_mut);
        }
        (self.bundle)(entity_mut);
    }
}

impl Default for DynBundle {
    fn default() -> Self {
        DynBundle {
            bundle: Arc::new(|_| ()),
            parent: None,
        }
    }
}

impl Component for DynBundle {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            world.commands().queue(DynBundleCommand(entity));
        });
    }
}

struct DynBundleCommand(Entity);

impl Command for DynBundleCommand {
    fn apply(self, world: &mut World) {
        let Ok(mut entity_mut) = world.get_entity_mut(self.0) else {
            #[cfg(debug_assertions)]
            panic!("Entity with DynBundle component not found");

            #[cfg(not(debug_assertions))]
            return;
        };
        let Some(dyn_bundle) = entity_mut.take::<DynBundle>() else {
            #[cfg(debug_assertions)]
            panic!("DynBundle component not found");

            #[cfg(not(debug_assertions))]
            return;
        };
        dyn_bundle.apply(&mut entity_mut);
    }
}

#[derive(Clone, Default)]
pub struct Child(DynBundle);

impl Child {
    pub fn new(dyn_bundle: DynBundle) -> Self {
        Self(dyn_bundle)
    }
}

impl Component for Child {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            world.commands().queue(ChildCommand(entity));
        });
    }
}

struct ChildCommand(Entity);

impl Command for ChildCommand {
    fn apply(self, world: &mut World) {
        let Ok(mut entity_mut) = world.get_entity_mut(self.0) else {
            #[cfg(debug_assertions)]
            panic!("Entity with Child component not found");

            #[cfg(not(debug_assertions))]
            return;
        };
        let Some(child) = entity_mut.take::<Child>() else {
            #[cfg(debug_assertions)]
            panic!("Child component not found");

            #[cfg(not(debug_assertions))]
            return;
        };
        entity_mut.with_child(child.0);
    }
}

#[derive(Clone, Default)]
pub struct Children(Vec<DynBundle>);

impl Children {
    pub fn new(dyn_bundles: impl IntoIterator<Item = DynBundle>) -> Self {
        Self(dyn_bundles.into_iter().collect())
    }
}

impl Component for Children {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, entity, _| {
            world.commands().queue(ChildrenCommand(entity));
        });
    }
}

struct ChildrenCommand(Entity);

impl Command for ChildrenCommand {
    fn apply(self, world: &mut World) {
        let Ok(mut entity_mut) = world.get_entity_mut(self.0) else {
            #[cfg(debug_assertions)]
            panic!("Entity with Children component not found");

            #[cfg(not(debug_assertions))]
            return;
        };
        let Some(children) = entity_mut.take::<Children>() else {
            #[cfg(debug_assertions)]
            panic!("Children component not found");

            #[cfg(not(debug_assertions))]
            return;
        };
        entity_mut.with_children(|builder| {
            for bundle in children.0 {
                builder.spawn(bundle);
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target(pub Entity);

impl Component for Target {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks
            .on_insert(|mut world, entity, _| {
                let Ok(entity_ref) = world.get_entity(entity) else {
                    #[cfg(debug_assertions)]
                    panic!("Entity with Target component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let Some(Target(target)) = entity_ref.get::<Target>() else {
                    #[cfg(debug_assertions)]
                    panic!("Target component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let target = *target;
                if !world
                    .entity(target)
                    .get::<TargetBy>()
                    .is_some_and(|t| t.0 == entity)
                {
                    world.commands().entity(target).insert(TargetBy(entity));
                }
            })
            .on_replace(|mut world, entity, _| {
                let Ok(entity_ref) = world.get_entity(entity) else {
                    #[cfg(debug_assertions)]
                    panic!("Entity with Target component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let Some(Target(target)) = entity_ref.get::<Target>() else {
                    #[cfg(debug_assertions)]
                    panic!("Target component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let target = *target;
                world.commands().entity(target).remove::<TargetBy>();
            });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TargetBy(pub Entity);

impl Component for TargetBy {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks
            .on_insert(|mut world, entity, _| {
                let Ok(entity_ref) = world.get_entity(entity) else {
                    #[cfg(debug_assertions)]
                    panic!("Entity with TargetBy component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let Some(TargetBy(targetby)) = entity_ref.get::<TargetBy>() else {
                    #[cfg(debug_assertions)]
                    panic!("TargetBy component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let targetby = *targetby;
                if !world
                    .entity(targetby)
                    .get::<Target>()
                    .is_some_and(|t| t.0 == entity)
                {
                    world.commands().entity(targetby).insert(Target(entity));
                }
            })
            .on_replace(|mut world, entity, _| {
                let Ok(entity_ref) = world.get_entity(entity) else {
                    #[cfg(debug_assertions)]
                    panic!("Entity with TargetBy component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let Some(TargetBy(targetby)) = entity_ref.get::<TargetBy>() else {
                    #[cfg(debug_assertions)]
                    panic!("TargetBy component not found");

                    #[cfg(not(debug_assertions))]
                    return;
                };
                let targetby = *targetby;
                world.commands().entity(targetby).remove::<Target>();
            });
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::*;

    use crate::*;

    /// Helper function to create a minimal ECS setup with necessary component hooks.
    fn setup_world() -> World {
        let mut world = World::new();

        // Register components with their hooks
        world.register_component::<Target>();
        world.register_component::<TargetBy>();
        world.register_component::<DynBundle>();
        world.register_component::<Child>();
        world.register_component::<Children>();

        world
    }

    #[test]
    fn insert_target_creates_targetby() {
        let mut world = setup_world();

        let target = world.spawn(()).id();
        let targeter = world.spawn(Target(target)).id();

        // Flush the commands to apply component hooks
        world.flush();

        // Verify that the target has a TargetBy component pointing back to the targeter
        let target_by = world
            .get::<TargetBy>(target)
            .expect("TargetBy should be present on target");
        assert_eq!(
            target_by.0, targeter,
            "TargetBy should reference the targeter"
        );
    }

    #[test]
    fn insert_targetby_creates_target() {
        let mut world = setup_world();

        let targeter = world.spawn(()).id();
        let targeted = world.spawn(TargetBy(targeter)).id();

        // Flush the commands to apply component hooks
        world.flush();

        // Verify that the targeter has a Target component pointing to the targeted
        let target = world
            .get::<Target>(targeter)
            .expect("Target should be present on targeter");
        assert_eq!(
            target.0, targeted,
            "Target should reference the targeted entity"
        );
    }

    #[test]
    fn replace_target_updates_targetby() {
        let mut world = setup_world();

        let original_target = world.spawn(()).id();
        let new_target = world.spawn(()).id();
        let targeter = world.spawn(Target(original_target)).id();

        world.flush();

        // Verify initial TargetBy on original_target
        let target_by = world
            .get::<TargetBy>(original_target)
            .expect("TargetBy should be present on original target");
        assert_eq!(
            target_by.0, targeter,
            "Initial TargetBy should reference the targeter"
        );

        // Replace Target with new_target
        world.entity_mut(targeter).insert(Target(new_target));

        world.flush();

        // Original target should no longer have TargetBy
        assert!(
            world.get::<TargetBy>(original_target).is_none(),
            "Original target should no longer have TargetBy"
        );

        // New target should have TargetBy pointing to targeter
        let new_target_by = world
            .get::<TargetBy>(new_target)
            .expect("TargetBy should be present on new target");
        assert_eq!(
            new_target_by.0, targeter,
            "New TargetBy should reference the targeter"
        );
    }

    #[test]
    fn replace_targetby_updates_target() {
        let mut world = setup_world();

        let targeter = world.spawn(()).id();
        let original_target = world.spawn(()).id();
        let new_target = world.spawn(()).id();
        let targetby = world.spawn(TargetBy(original_target)).id();

        // Initially associate targeter with original_target via Target
        world.entity_mut(targeter).insert(Target(original_target));

        world.flush();

        // Verify initial Target on targeter
        let target = world
            .get::<Target>(targeter)
            .expect("Target should be present on targeter");
        assert_eq!(
            target.0, original_target,
            "Initial Target should reference the original target"
        );

        // Replace TargetBy with new_target
        world.entity_mut(targetby).insert(TargetBy(new_target));

        world.flush();

        // Original target should no longer have TargetBy
        assert!(
            world.get::<Target>(original_target).is_none(),
            "Original target should no longer have TargetBy"
        );

        // New target should have TargetBy pointing to targeter
        let new_target_by = world
            .get::<TargetBy>(new_target)
            .expect("TargetBy should be present on new target");
        assert_eq!(
            new_target_by.0, targeter,
            "New TargetBy should reference the targeter"
        );

        // Targeter should now point to new_target
        let updated_target = world
            .get::<Target>(targeter)
            .expect("Target should be present on targeter");
        assert_eq!(
            updated_target.0, new_target,
            "Updated Target should reference the new target"
        );
    }

    #[test]
    fn remove_target_clears_targetby() {
        let mut world = setup_world();

        let target = world.spawn(()).id();
        let targeter = world.spawn(Target(target)).id();

        world.flush();

        // Ensure TargetBy is present
        assert!(
            world.get::<TargetBy>(target).is_some(),
            "TargetBy should be present on target after inserting Target"
        );

        // Remove Target component
        world.entity_mut(targeter).remove::<Target>();

        world.flush();

        // TargetBy should also be removed
        assert!(
            world.get::<TargetBy>(target).is_none(),
            "TargetBy should be removed from target after removing Target"
        );
    }

    #[test]
    fn remove_targetby_clears_target() {
        let mut world = setup_world();

        let targeter = world.spawn(()).id();
        let targeted = world.spawn(TargetBy(targeter)).id();

        world.flush();

        // Ensure Target is present
        assert!(
            world.get::<Target>(targeter).is_some(),
            "Target should be present on targeter after inserting TargetBy"
        );

        // Remove TargetBy component
        world.entity_mut(targeted).remove::<TargetBy>();

        world.flush();

        // Target should also be removed
        assert!(
            world.get::<Target>(targeter).is_none(),
            "Target should be removed from targeter after removing TargetBy"
        );
    }

    #[test]
    fn multiple_targets_and_targetbys() {
        let mut world = setup_world();

        let target1 = world.spawn(()).id();
        let target2 = world.spawn(()).id();
        let targeter1 = world.spawn(Target(target1)).id();
        let targeter2 = world.spawn(Target(target2)).id();

        world.flush();

        // Verify TargetBy for target1
        let target_by1 = world
            .get::<TargetBy>(target1)
            .expect("TargetBy should be present on target1");
        assert_eq!(
            target_by1.0, targeter1,
            "TargetBy1 should reference targeter1"
        );

        // Verify TargetBy for target2
        let target_by2 = world
            .get::<TargetBy>(target2)
            .expect("TargetBy should be present on target2");
        assert_eq!(
            target_by2.0, targeter2,
            "TargetBy2 should reference targeter2"
        );
    }

    #[test]
    fn cyclic_target_and_targetby_handling() {
        let mut world = setup_world();

        let entity_a = world.spawn(()).id();
        let entity_b = world.spawn(()).id();

        // Insert Target on entity_a pointing to entity_b
        world.entity_mut(entity_a).insert(Target(entity_b));

        // Insert TargetBy on entity_b pointing to entity_a
        world.entity_mut(entity_b).insert(TargetBy(entity_a));

        world.flush();

        // Verify TargetBy on entity_b points to entity_a
        let target_by_b = world
            .get::<TargetBy>(entity_b)
            .expect("TargetBy should be present on entity_b");
        assert_eq!(
            target_by_b.0, entity_a,
            "TargetBy on entity_b should reference entity_a"
        );

        // Verify Target on entity_a points to entity_b
        let target_a = world
            .get::<Target>(entity_a)
            .expect("Target should be present on entity_a");
        assert_eq!(
            target_a.0, entity_b,
            "Target on entity_a should reference entity_b"
        );

        // Ensure no additional Target or TargetBy components are created
        assert_eq!(
            world.query::<&Target>().iter(&world).count(),
            1,
            "There should only be one Target component"
        );
        assert_eq!(
            world.query::<&TargetBy>().iter(&world).count(),
            1,
            "There should only be one TargetBy component"
        );
    }

    #[test]
    fn inserting_target_twice_updates_targetby() {
        let mut world = setup_world();

        let target1 = world.spawn(()).id();
        let target2 = world.spawn(()).id();
        let targeter = world.spawn(Target(target1)).id();

        world.flush();

        // Verify TargetBy on target1
        let target_by1 = world
            .get::<TargetBy>(target1)
            .expect("TargetBy should be present on target1");
        assert_eq!(
            target_by1.0, targeter,
            "TargetBy on target1 should reference targeter"
        );

        // Insert another Target on the same targeter, pointing to target2
        world.entity_mut(targeter).insert(Target(target2));

        world.flush();

        // Verify that TargetBy on target1 is removed
        assert!(
            world.get::<TargetBy>(target1).is_none(),
            "TargetBy on target1 should be removed after updating Target"
        );

        // Verify that TargetBy on target2 is present
        let target_by2 = world
            .get::<TargetBy>(target2)
            .expect("TargetBy should be present on target2");
        assert_eq!(
            target_by2.0, targeter,
            "TargetBy on target2 should reference targeter"
        );
    }

    #[test]
    fn inserting_targetby_twice_updates_target() {
        let mut world = setup_world();

        let targeter1 = world.spawn(()).id();
        let targeter2 = world.spawn(()).id();
        let targeted = world.spawn(TargetBy(targeter1)).id();

        world.flush();

        // Verify Target on targeter1
        let target1 = world
            .get::<Target>(targeter1)
            .expect("Target should be present on targeter1");
        assert_eq!(
            target1.0, targeted,
            "Target on targeter1 should reference targeted"
        );

        // Insert another TargetBy on the same targeted, pointing to targeter2
        world.entity_mut(targeted).insert(TargetBy(targeter2));

        world.flush();

        // Verify that Target on targeter1 is removed
        assert!(
            world.get::<Target>(targeter1).is_none(),
            "Target on targeter1 should be removed after updating TargetBy"
        );

        // Verify that Target on targeter2 is present
        let target2 = world
            .get::<Target>(targeter2)
            .expect("Target should be present on targeter2");
        assert_eq!(
            target2.0, targeted,
            "Target on targeter2 should reference targeted"
        );
    }

    #[test]
    fn ensure_no_duplicate_targetby_on_multiple_insertions() {
        let mut world = setup_world();

        let target = world.spawn(()).id();
        let targeter = world.spawn(()).id();

        // Insert Target twice pointing to the same target
        world.entity_mut(targeter).insert(Target(target));
        world.entity_mut(targeter).insert(Target(target));

        world.flush();

        // Ensure only one TargetBy exists on target
        let target_bys: Vec<TargetBy> = world.query::<&TargetBy>().iter(&world).cloned().collect();
        assert_eq!(
            target_bys.len(),
            1,
            "There should only be one TargetBy component on the target"
        );

        let target_by = &target_bys[0];
        assert_eq!(
            target_by.0, targeter,
            "TargetBy should reference the targeter"
        );
    }

    #[test]
    fn ensure_no_duplicate_target_on_multiple_insertions() {
        let mut world = setup_world();

        let target = world.spawn(()).id();
        let targeter = world.spawn(()).id();

        // Insert TargetBy twice pointing to the same targeter
        world.entity_mut(targeter).insert(TargetBy(target));
        world.entity_mut(targeter).insert(TargetBy(target));

        world.flush();

        // Ensure only one Target exists on targeter
        let targets: Vec<Target> = world.query::<&Target>().iter(&world).cloned().collect();
        assert_eq!(
            targets.len(),
            1,
            "There should only be one Target component on the targeter"
        );

        let target_component = &targets[0];
        assert_eq!(
            target_component.0, target,
            "Target should reference the target"
        );
    }
}
