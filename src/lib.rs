use std::sync::Arc;

use bevy_ecs::{
    component::{ComponentHooks, StorageType},
    prelude::*,
    world::Command,
};

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

    pub fn add<B: Bundle + Clone>(&self, bundle: B) -> DynBundle {
        DynBundle {
            bundle: Arc::new(move |entity: &mut EntityWorldMut| {
                entity.insert(bundle.clone());
            }),
            parent: Some(Arc::new(self.clone())),
        }
    }

    pub fn del<B: Bundle + Clone>(&self) -> DynBundle {
        DynBundle {
            bundle: Arc::new(move |entity: &mut EntityWorldMut| {
                entity.remove::<B>();
            }),
            parent: Some(Arc::new(self.clone())),
        }
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
            world.commands().add(DynBundleCommand(entity));
        });
    }
}

struct DynBundleCommand(Entity);

impl Command for DynBundleCommand {
    fn apply(self, world: &mut World) {
        let Some(mut entity_mut) = world.get_entity_mut(self.0) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Component)]
    struct A;

    #[derive(Debug, Clone, PartialEq, Component)]
    struct B;

    #[derive(Debug, Clone, PartialEq, Component)]
    struct C;

    #[derive(Clone, Bundle)]
    struct BundleAB {
        a: A,
        b: B,
    }

    #[test]
    fn add_bundle() {
        let mut world = World::default();

        let bundle = DynBundle::new(BundleAB { a: A, b: B });

        let entity = world.spawn(bundle.clone()).id();

        world.flush();

        assert!(world.entity(entity).contains::<A>());
        assert!(world.entity(entity).contains::<B>());

        let bundle_with_c = bundle.add(C);

        let new_entity = world.spawn(bundle_with_c).id();

        world.flush();

        assert!(world.entity(new_entity).contains::<A>());
        assert!(world.entity(new_entity).contains::<B>());
        assert!(world.entity(new_entity).contains::<C>());
    }

    #[test]
    fn delete_bundle() {
        let mut world = World::default();

        let bundle = DynBundle::new(BundleAB { a: A, b: B });

        let entity = world.spawn(bundle.clone()).id();

        world.flush();

        assert!(world.entity(entity).contains::<A>());
        assert!(world.entity(entity).contains::<B>());

        let bundle_without_a = bundle.del::<A>();

        let new_entity = world.spawn(bundle_without_a).id();

        world.flush();

        assert!(!world.entity(new_entity).contains::<A>());
        assert!(world.entity(new_entity).contains::<B>());
    }

    #[test]
    fn add_and_delete_bundle() {
        let mut world = World::default();

        let bundle = DynBundle::new(A);

        let entity = world.spawn(bundle.clone()).id();

        world.flush();

        assert!(world.entity(entity).contains::<A>());
        assert!(world.entity(entity).contains::<B>());

        let bundle_with_c = bundle.add(C);

        let new_entity = world.spawn(bundle_with_c).id();

        world.flush();

        assert!(world.entity(new_entity).contains::<A>());
        assert!(world.entity(new_entity).contains::<B>());
        assert!(world.entity(new_entity).contains::<C>());
    }

    #[test]
    fn inherit_bundle() {
        let mut world = World::default();

        let parent_bundle = DynBundle::new(A);

        let child_bundle = parent_bundle.add(B);

        let parent = world.spawn(parent_bundle).id();
        let child = world.spawn(child_bundle).id();

        world.flush();

        assert!(world.entity(parent).contains::<A>());

        assert!(world.entity(child).contains::<A>());
        assert!(world.entity(child).contains::<B>());
    }

    #[test]
    fn shared_parent_bundle() {
        let mut world = World::default();

        let parent_bundle = DynBundle::new(A);

        let child_bundle_1 = parent_bundle.add(B);
        let child_bundle_2 = parent_bundle.add(C);

        let child_1 = world.spawn(child_bundle_1).id();
        let child_2 = world.spawn(child_bundle_2).id();

        world.flush();

        assert!(world.entity(child_1).contains::<A>());
        assert!(world.entity(child_1).contains::<B>());

        assert!(world.entity(child_2).contains::<A>());
        assert!(world.entity(child_2).contains::<C>());
    }

    #[test]
    fn bundle_inside_bundle() {
        let mut world = World::default();

        let a_bundle = DynBundle::new(A);

        let abc_bundle = DynBundle::new((a_bundle, B, C));

        let entity = world.spawn(abc_bundle).id();

        world.flush();

        assert!(world.entity(entity).contains::<A>());
        assert!(world.entity(entity).contains::<B>());
        assert!(world.entity(entity).contains::<C>());
    }
}
