use crate::{IntoReactiveSystem, ReactiveSystem, Scope};
use bevy_app::PostUpdate;
use bevy_ecs::{
    component::{ComponentHooks, StorageType},
    prelude::*,
    schedule::ScheduleLabel,
    world::DeferredWorld,
};
use std::sync::{Arc, Mutex};

pub(crate) struct Inner {
    system: Box<dyn ReactiveSystem<In = (), Out = ()>>,
    entities: Vec<Entity>,
}

#[derive(Clone)]
pub struct Reaction<L = PostUpdate> {
    inner: Arc<Mutex<Inner>>,
    _label: L,
}

impl<L: ScheduleLabel> Component for Reaction<L> {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(|mut world, entity, _| {
            world.commands().add(move |world: &mut World| {
                let inner = world
                    .query::<&Reaction<L>>()
                    .get(world, entity)
                    .unwrap()
                    .inner
                    .clone();
                inner.lock().unwrap().system.init(world);
            });
        });
    }
}

impl<L: ScheduleLabel> Reaction<L> {
    pub fn from_label<Marker, S>(
        label: L,
        system: impl IntoReactiveSystem<Marker, System = S>,
    ) -> Self
    where
        Marker: Send + Sync + 'static,
        S: ReactiveSystem<In = (), Out = ()> + 'static,
    {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                system: Box::new(system.into_reactive_system()),
                entities: Vec::new(),
            })),
            _label: label,
        }
    }

    pub fn with_label<L2>(&self, label: L2) -> Reaction<L2> {
        let inner = self.inner.clone();
        Reaction {
            inner,
            _label: label,
        }
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

impl Reaction {
    pub fn new<Marker, S>(system: impl IntoReactiveSystem<Marker, System = S>) -> Self
    where
        Marker: Send + Sync + 'static,
        S: ReactiveSystem<In = (), Out = ()> + 'static,
    {
        Self::from_label(PostUpdate, system)
    }

    /// Create a new [`Reaction`] that derives a [`Bundle`] from a [`ReactiveSystem`].
    pub fn derive<Marker, S, B>(system: impl IntoReactiveSystem<Marker, System = S>) -> Self
    where
        Marker: Send + Sync + 'static,
        B: Bundle,
        S: ReactiveSystem<In = (), Out = B> + 'static,
    {
        Self::new(system.map(|scope: In<Scope<B>>, mut commands: Commands| {
            commands.entity(scope.entity).insert(scope.0.input);
        }))
    }

    /// Create a new [`Reaction`] that switches between two [`Bundle`]s.
    ///
    /// When `system` returns `true`, the `make_if` closure is called to create a new [`Bundle`] and replace the current [`Bundle`].
    /// Otherwise, the `make_else` closure is called to replace the original [`Bundle`].
    pub fn switch<Marker, S, A, B>(
        system: impl IntoReactiveSystem<Marker, System = S>,
        mut make_if: impl FnMut() -> A + Send + Sync + 'static,
        mut make_else: impl FnMut() -> B + Send + Sync + 'static,
    ) -> Self
    where
        Marker: Send + Sync + 'static,
        S: ReactiveSystem<In = (), Out = bool> + 'static,
        A: Bundle,
        B: Bundle,
    {
        Self::new(system.map(
            move |scope: In<Scope<bool>>, mut commands: Commands, mut local: Local<bool>| {
                if scope.input {
                    if !*local {
                        commands.entity(scope.entity).remove::<B>();
                        commands.entity(scope.entity).insert(make_if());
                        *local = true;
                    }
                } else if *local {
                    commands.entity(scope.entity).remove::<A>();
                    commands.entity(scope.entity).insert(make_else());
                    *local = false;
                }
            },
        ))
    }

    /// Create a new [`Reaction`] that spawns [`Bundle`]s from an iterator.
    pub fn from_iter<Marker, S, I>(system: impl IntoReactiveSystem<Marker, System = S>) -> Self
    where
        Marker: Send + Sync + 'static,
        S: ReactiveSystem<In = (), Out = I> + 'static,
        I: IntoIterator + 'static,
        I::Item: Bundle,
    {
        Self::new(system.map(
            move |scope: In<Scope<I>>, mut commands: Commands, mut local: Local<Vec<Entity>>| {
                for entity in &local {
                    commands.entity(*entity).despawn();
                }
                local.clear();

                for item in scope.0.input {
                    let entity = commands.spawn(item).id();
                    local.push(entity);
                }
            },
        ))
    }
}
