use bevy::{
    ecs::{
        component::StorageType,
        query::{QueryData, QueryFilter},
        system::{SystemParam, SystemParamItem},
        world::DeferredWorld,
    },
    prelude::*,
};
use std::{
    marker::PhantomData,
    mem,
    sync::{Arc, Mutex},
};

pub trait ReactiveQueryData<F: QueryFilter>: QueryData + Sized {
    type State: Send + Sync + 'static;

    fn init(world: &mut World) -> <Self as ReactiveQueryData<F>>::State;

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveQueryData<F>>::State) -> bool;

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData<F>>::State,
    ) -> Query<'w, 's, Self, F>;
}

impl<F, T> ReactiveQueryData<F> for &T
where
    F: QueryFilter + 'static,
    T: Component,
{
    type State = (QueryState<(), (Changed<T>, F)>, QueryState<&'static T, F>);

    fn init(world: &mut World) -> <Self as ReactiveQueryData<F>>::State {
        (QueryState::new(world), QueryState::new(world))
    }

    fn is_changed<'w>(
        mut world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
    ) -> bool {
        !world.reborrow().query(&mut state.0).is_empty()
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData<F>>::State,
    ) -> Query<'w, 's, Self, F> {
        // TODO verify safety
        unsafe { mem::transmute(world.query(&mut state.1)) }
    }
}

pub trait ReactiveSystemParam: SystemParam {
    type State: Send + Sync + 'static;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State;

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveSystemParam>::State) -> bool;

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's>;
}

impl<R: Resource> ReactiveSystemParam for Res<'_, R> {
    type State = ();

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        let _ = world;
    }

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveSystemParam>::State) -> bool {
        let _ = state;
        world.resource_ref::<R>().is_changed()
    }

    fn get<'w>(
        world: &'w mut DeferredWorld<'w>,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 'w> {
        let _ = state;
        world.resource_ref::<R>()
    }
}

impl<D, F> ReactiveSystemParam for Query<'_, '_, D, F>
where
    D: ReactiveQueryData<F> + QueryData + 'static,
    F: QueryFilter + 'static,
{
    type State = <D as ReactiveQueryData<F>>::State;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        <D as ReactiveQueryData<F>>::init(world)
    }

    fn is_changed<'a>(
        world: DeferredWorld,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> bool {
        <D as ReactiveQueryData<F>>::is_changed(world, state)
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        <D as ReactiveQueryData<F>>::get(world, state)
    }
}

impl<T: ReactiveSystemParam> ReactiveSystemParam for (T,) {
    type State = <T as ReactiveSystemParam>::State;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        T::init(world)
    }

    fn is_changed<'a>(
        world: DeferredWorld,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> bool {
        T::is_changed(world, state)
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        (T::get(world, state),)
    }
}

pub trait ReactiveSystemParamFunction<Marker> {
    type Param: ReactiveSystemParam;

    fn run(&mut self, param: SystemParamItem<Self::Param>, entity: Entity);
}

impl<Marker, F> ReactiveSystemParamFunction<Marker> for F
where
    F: SystemParamFunction<Marker, In = Entity, Out = ()>,
    F::Param: ReactiveSystemParam,
{
    type Param = F::Param;

    fn run(&mut self, param: SystemParamItem<Self::Param>, entity: Entity) {
        SystemParamFunction::run(self, entity, param)
    }
}

pub trait ReactiveSystem: Send + Sync {
    fn init(&mut self, world: &mut World);

    fn is_changed(&mut self, world: DeferredWorld) -> bool;

    fn run(&mut self, world: DeferredWorld, entity: Entity);
}

pub struct FunctionReactiveSystem<F, S, Marker> {
    f: F,
    state: Option<S>,
    _marker: PhantomData<Marker>,
}

impl<F, S, Marker> ReactiveSystem for FunctionReactiveSystem<F, S, Marker>
where
    F: ReactiveSystemParamFunction<Marker> + Send + Sync,
    F::Param: ReactiveSystemParam<State = S>,
    S: Send + Sync,
    Marker: Send + Sync,
{
    fn init(&mut self, world: &mut World) {
        self.state = Some(F::Param::init(world));
    }

    fn is_changed(&mut self, world: DeferredWorld) -> bool {
        F::Param::is_changed(world, self.state.as_mut().unwrap())
    }

    fn run(&mut self, mut world: DeferredWorld, entity: Entity) {
        self.f.run(
            F::Param::get(&mut world.reborrow(), self.state.as_mut().unwrap()),
            entity,
        );
    }
}

#[derive(Clone)]
pub struct Reaction {
    system: Arc<Mutex<Box<dyn ReactiveSystem>>>,
}

impl Component for Reaction {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_insert(|mut world, entity, _| {
            world.commands().add(move |world: &mut World| {
                let me = world
                    .query::<&Reaction>()
                    .get(world, entity)
                    .unwrap()
                    .clone();
                me.system.lock().unwrap().init(world);
            });
        });
    }
}

impl Reaction {
    pub fn new<Marker>(
        system: impl ReactiveSystemParamFunction<Marker> + Send + Sync + 'static,
    ) -> Self
    where
        Marker: Send + Sync + 'static,
    {
        Self {
            system: Arc::new(Mutex::new(Box::new(FunctionReactiveSystem {
                f: system,
                state: None,
                _marker: PhantomData,
            }))),
        }
    }
}

pub fn react(mut world: DeferredWorld, reaction_query: Query<(Entity, &Reaction)>) {
    for (entity, reaction) in &reaction_query {
        let mut system = reaction.system.lock().unwrap();

        if system.is_changed(world.reborrow()) {
            system.run(world.reborrow(), entity);
        }
    }
}
