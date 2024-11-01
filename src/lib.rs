use std::sync::Arc;

use bevy_ecs::{
    component::{ComponentHooks, StorageType},
    prelude::*,
    world::{Command, DeferredWorld},
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
            .on_insert(|mut world: DeferredWorld<'_>, entity: Entity, _| {
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
            .on_replace(|mut world: DeferredWorld<'_>, entity: Entity, _| {
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
            .on_insert(|mut world: DeferredWorld<'_>, entity: Entity, _| {
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
            .on_replace(|mut world: DeferredWorld<'_>, entity: Entity, _| {
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
