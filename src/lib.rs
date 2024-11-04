#![feature(specialization)]
#![allow(incomplete_features)]

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

    pub fn append(&self, dyn_bundle: impl IntoDynBundle) -> Self {
        let dyn_bundle = dyn_bundle.into_dyn_bundle();
        DynBundle {
            bundle: dyn_bundle.bundle.clone(),
            parent: match dyn_bundle.parent {
                Some(parent) => Some(Arc::new((*parent).append(self.clone()))),
                None => Some(Arc::new(self.clone())),
            },
        }
    }

    pub fn append_some(&self, opt_bundle: Option<impl IntoDynBundle>) -> Self {
        if let Some(bundle) = opt_bundle {
            self.append(bundle.into_dyn_bundle());
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

pub trait IntoDynBundle {
    fn into_dyn_bundle(self) -> DynBundle;
}

impl<B: Bundle + Clone> IntoDynBundle for B {
    default fn into_dyn_bundle(self) -> DynBundle {
        DynBundle::new(self.clone())
    }
}

impl IntoDynBundle for DynBundle {
    fn into_dyn_bundle(self) -> DynBundle {
        self
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
