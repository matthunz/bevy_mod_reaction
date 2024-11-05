use crate::{IntoReactiveSystem, ReactiveSystem, ReactiveSystemParamFunction, Scope};
use bevy_ecs::{
    component::{ComponentHooks, StorageType},
    prelude::*,
    world::DeferredWorld,
};
use std::sync::{Arc, Mutex};

pub(crate) struct Inner {
    system: Box<dyn ReactiveSystem<In = (), Out = ()>>,
    entities: Vec<Entity>,
}

#[derive(Clone)]
pub struct Reaction {
    inner: Arc<Mutex<Inner>>,
}

impl Component for Reaction {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world, entity, _| {
            world.commands().add(move |world: &mut World| {
                let me = world
                    .query::<&Reaction>()
                    .get(world, entity)
                    .unwrap()
                    .clone();
                me.inner.lock().unwrap().system.init(world);
            });
        });
    }
}

impl Reaction {
    pub fn new<Marker, S>(system: impl IntoReactiveSystem<Marker, System = S>) -> Self
    where
        Marker: Send + Sync + 'static,
        S: ReactiveSystem<In = (), Out = ()> + 'static,
    {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                system: Box::new(system.into_reactive_system()),
                entities: Vec::new(),
            })),
        }
    }

    pub fn derive<Marker, B>(
        system: impl ReactiveSystemParamFunction<Marker, In = (), Out = B> + Send + Sync + 'static,
    ) -> Self
    where
        Marker: Send + Sync + 'static,
        B: Bundle,
    {
        Self::new(system.map(|scope: In<Scope<B>>, mut commands: Commands| {
            commands.entity(scope.entity).insert(scope.0.input);
        }))
    }

    pub fn switch<Marker, A, B>(
        system: impl ReactiveSystemParamFunction<Marker, In = (), Out = bool> + Send + Sync + 'static,
        mut f: impl FnMut() -> A + Send + Sync + 'static,
        mut g: impl FnMut() -> B + Send + Sync + 'static,
    ) -> Self
    where
        Marker: Send + Sync + 'static,
        A: Bundle,
        B: Bundle,
    {
        Self::new(system.map(
            move |scope: In<Scope<bool>>, mut commands: Commands, mut local: Local<bool>| {
                if scope.input {
                    if !*local {
                        commands.entity(scope.entity).remove::<B>();
                        commands.entity(scope.entity).insert(f());
                        *local = true;
                    }
                } else if *local {
                    commands.entity(scope.entity).remove::<A>();
                    commands.entity(scope.entity).insert(g());
                    *local = false;
                }
            },
        ))
    }

    pub fn add_target(&mut self, entity: Entity) -> &mut Self {
        self.inner.lock().unwrap().entities.push(entity);
        self
    }

    pub fn run(&self, mut world: DeferredWorld, entity: Entity) {
        let inner = &mut *self.inner.lock().unwrap();

        if inner.system.is_changed(world.reborrow()) {
            if inner.entities.is_empty() {
                inner.system.run((), world.reborrow(), entity);
            } else {
                for entity in &inner.entities {
                    inner.system.run((), world.reborrow(), *entity);
                }
            }
        }
    }
}
