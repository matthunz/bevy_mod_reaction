use bevy::{
    ecs::{
        component::StorageType,
        query::QueryData,
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

pub trait ReactiveQueryData: QueryData + Sized {
    type State: Send + Sync + 'static;

    fn init(world: &mut World) -> <Self as ReactiveQueryData>::State;

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveQueryData>::State) -> bool;

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData>::State,
    ) -> Query<'w, 's, Self, ()>;
}

impl<T: Component> ReactiveQueryData for &T {
    type State = (QueryState<(), Changed<T>>, QueryState<&'static T>);

    fn init(world: &mut World) -> <Self as ReactiveQueryData>::State {
        (QueryState::new(world), QueryState::new(world))
    }

    fn is_changed<'w>(
        mut world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData>::State,
    ) -> bool {
        !world.reborrow().query(&mut state.0).is_empty()
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData>::State,
    ) -> Query<'w, 's, Self, ()> {
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

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {}

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveSystemParam>::State) -> bool {
        world.resource_ref::<R>().is_changed()
    }

    fn get<'w>(
        world: &'w mut DeferredWorld<'w>,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 'w> {
        world.resource_ref::<R>()
    }
}

impl<D: ReactiveQueryData + QueryData + 'static> ReactiveSystemParam for Query<'_, '_, D> {
    type State = <D as ReactiveQueryData>::State;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        <D as ReactiveQueryData>::init(world)
    }

    fn is_changed<'a>(
        world: DeferredWorld,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> bool {
        <D as ReactiveQueryData>::is_changed(world, state)
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        <D as ReactiveQueryData>::get(world, state)
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

    fn run(&mut self, param: SystemParamItem<Self::Param>);
}

impl<Marker, F> ReactiveSystemParamFunction<Marker> for F
where
    F: SystemParamFunction<Marker, In = (), Out = ()>,
    F::Param: ReactiveSystemParam,
{
    type Param = F::Param;

    fn run(&mut self, param: SystemParamItem<Self::Param>) {
        SystemParamFunction::run(self, (), param)
    }
}

pub trait ReactiveSystem: Send + Sync {
    fn init(&mut self, world: &mut World);

    fn is_changed(&mut self, world: DeferredWorld) -> bool;

    fn run(&mut self, world: DeferredWorld);
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

    fn run(&mut self, mut world: DeferredWorld) {
        self.f.run(F::Param::get(
            &mut world.reborrow(),
            self.state.as_mut().unwrap(),
        ));
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

pub fn react(mut world: DeferredWorld, reaction_query: Query<&Reaction>) {
    for reaction in &reaction_query {
        let mut system = reaction.system.lock().unwrap();

        if system.is_changed(world.reborrow()) {
            system.run(world.reborrow());
        }
    }
}
