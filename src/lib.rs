use bevy::{
    ecs::{
        component::StorageType,
        query::{QueryData, QueryFilter, ReadOnlyQueryData, WorldQuery},
        system::{SystemParam, SystemParamItem, SystemState},
        world::DeferredWorld,
    },
    prelude::*,
    utils::HashSet,
};
use std::{
    error::Error,
    marker::PhantomData,
    mem,
    ops::Deref,
    sync::{Arc, Mutex},
};

pub trait ReactiveQueryData<F: QueryFilter>: QueryData + Sized {
    type State: Send + Sync + 'static;

    fn init(world: &mut World) -> <Self as ReactiveQueryData<F>>::State;

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveQueryData<F>>::State) -> bool;

    fn is_changed_with_entity(
        world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
        entity: Entity,
    ) -> bool;

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
    type State = SystemState<(
        Query<'static, 'static, (), (Changed<T>, F)>,
        Query<'static, 'static, &'static T, F>,
    )>;

    fn init(world: &mut World) -> <Self as ReactiveQueryData<F>>::State {
        SystemState::new(world)
    }

    fn is_changed<'w>(
        world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
    ) -> bool {
        !state.get(&world).0.is_empty()
    }

    fn is_changed_with_entity(
        world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
        entity: Entity,
    ) -> bool {
        state.get(&world).0.get(entity).is_ok()
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData<F>>::State,
    ) -> Query<'w, 's, Self, F> {
        // TODO verify safety
        unsafe { mem::transmute(state.get(world).1) }
    }
}

pub trait ReactiveSystemParam: SystemParam {
    type State: Send + Sync + 'static;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State;

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveSystemParam>::State) -> bool;

    /// Get the system parameter.
    ///
    /// # Safety
    /// `world` must not be mutated during this function call.
    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's>;
}

impl ReactiveSystemParam for Commands<'_, '_> {
    type State = ();

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        let _ = world;
    }

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveSystemParam>::State) -> bool {
        let _ = world;
        let _ = state;

        false
    }

    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        let _ = state;

        world.commands()
    }
}

impl<T: FromWorld + Send> ReactiveSystemParam for Local<'_, T> {
    type State = SystemState<Local<'static, T>>;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        SystemState::new(world)
    }

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveSystemParam>::State) -> bool {
        let _ = world;
        let _ = state;

        false
    }

    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        state.get(world)
    }
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

    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
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

    unsafe fn get<'w: 's, 's>(
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

    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        (T::get(world, state),)
    }
}

impl<T1: ReactiveSystemParam, T2: ReactiveSystemParam> ReactiveSystemParam for (T1, T2) {
    type State = (
        <T1 as ReactiveSystemParam>::State,
        <T2 as ReactiveSystemParam>::State,
    );

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        (T1::init(world), T2::init(world))
    }

    fn is_changed<'a>(
        mut world: DeferredWorld,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> bool {
        T1::is_changed(world.reborrow(), &mut state.0) || T2::is_changed(world, &mut state.1)
    }

    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        let world_ptr = world as *mut _;
        (
            T1::get(unsafe { &mut *world_ptr }, &mut state.0),
            T2::get(unsafe { &mut *world_ptr }, &mut state.1),
        )
    }
}

pub struct ReactiveQueryState<D: QueryData + 'static, F: QueryFilter + 'static, S> {
    query: SystemState<Query<'static, 'static, D, F>>,
    query_state: S,
    entities: HashSet<Entity>,
}

pub struct ReactiveQuery<'w, 's, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static = ()> {
    query: Query<'w, 's, D, F>,
    entities: &'s mut HashSet<Entity>,
}

impl<'w, 's, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> ReactiveQuery<'w, 's, D, F> {
    pub fn get(&mut self, entity: Entity) -> Result<<D as WorldQuery>::Item<'_>, Box<dyn Error>> {
        self.entities.insert(entity);

        self.query
            .get(entity)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

unsafe impl<D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> SystemParam
    for ReactiveQuery<'_, '_, D, F>
{
    type State = ();

    type Item<'world, 'state> = ReactiveQuery<'world, 'state, D, F>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        let _ = world;
        let _ = system_meta;
        todo!()
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: bevy::ecs::component::Tick,
    ) -> Self::Item<'world, 'state> {
        let _ = state;
        let _ = system_meta;
        let _ = world;
        let _ = change_tick;
        todo!()
    }
}

impl<D, F> ReactiveSystemParam for ReactiveQuery<'_, '_, D, F>
where
    D: ReactiveQueryData<F> + ReadOnlyQueryData + 'static,
    F: QueryFilter + 'static,
{
    type State = ReactiveQueryState<D, F, <D as ReactiveQueryData<F>>::State>;

    fn init(world: &mut World) -> <Self as ReactiveSystemParam>::State {
        ReactiveQueryState {
            query: SystemState::new(world),
            query_state: D::init(world),
            entities: HashSet::new(),
        }
    }

    fn is_changed(
        mut world: DeferredWorld,
        state: &mut <Self as ReactiveSystemParam>::State,
    ) -> bool {
        if state.entities.is_empty() {
            return true;
        }

        for entity in state.entities.iter() {
            if D::is_changed_with_entity(world.reborrow(), &mut state.query_state, *entity) {
                return true;
            }
        }

        false
    }

    unsafe fn get<'w: 's, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveSystemParam>::State,
    ) -> Self::Item<'w, 's> {
        ReactiveQuery {
            query: state.query.get(world),
            entities: &mut state.entities,
        }
    }
}

pub struct Scope<T = ()> {
    pub entity: Entity,
    pub input: T,
}

impl<T> Deref for Scope<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

pub trait ReactiveSystemParamFunction<Marker> {
    type Param: ReactiveSystemParam;

    type In;

    type Out;

    fn run(
        &mut self,
        param: SystemParamItem<Self::Param>,
        input: Self::In,
        entity: Entity,
    ) -> Self::Out;
}

impl<Marker, F, T> ReactiveSystemParamFunction<Marker> for F
where
    F: SystemParamFunction<Marker, In = Scope<T>>,
    F::Param: ReactiveSystemParam,
{
    type Param = F::Param;

    type In = T;

    type Out = F::Out;

    fn run(
        &mut self,
        param: SystemParamItem<Self::Param>,
        input: Self::In,
        entity: Entity,
    ) -> Self::Out {
        SystemParamFunction::run(self, Scope { entity, input }, param)
    }
}

pub trait ReactiveSystem: Send + Sync {
    type In;

    type Out;

    fn init(&mut self, world: &mut World);

    fn is_changed(&mut self, world: DeferredWorld) -> bool;

    fn run(&mut self, input: Self::In, world: DeferredWorld, entity: Entity) -> Self::Out;
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
    type In = F::In;
    type Out = F::Out;

    fn init(&mut self, world: &mut World) {
        self.state = Some(F::Param::init(world));
    }

    fn is_changed(&mut self, world: DeferredWorld) -> bool {
        F::Param::is_changed(world, self.state.as_mut().unwrap())
    }

    fn run(&mut self, input: Self::In, mut world: DeferredWorld, entity: Entity) -> Self::Out {
        // TODO check for overlapping params
        let mut world = world.reborrow();
        let params = unsafe { F::Param::get(&mut world, self.state.as_mut().unwrap()) };

        self.f.run(params, input, entity)
    }
}

pub trait IntoReactiveSystem<Marker> {
    type System: ReactiveSystem;

    fn into_reactive_system(self) -> Self::System;

    fn map<SMarker, S>(
        self,
        system: impl IntoReactiveSystem<SMarker, System = S>,
    ) -> Map<Self::System, S>
    where
        Self: Sized,
    {
        Map {
            a: self.into_reactive_system(),
            b: system.into_reactive_system(),
        }
    }
}

impl<S: ReactiveSystem> IntoReactiveSystem<()> for S {
    type System = Self;

    fn into_reactive_system(self) -> Self::System {
        self
    }
}

impl<Marker, F> IntoReactiveSystem<fn(Marker)> for F
where
    Marker: Send + Sync,
    F: ReactiveSystemParamFunction<Marker> + Send + Sync,
{
    type System = FunctionReactiveSystem<F, <F::Param as ReactiveSystemParam>::State, Marker>;

    fn into_reactive_system(self) -> Self::System {
        FunctionReactiveSystem {
            f: self,
            state: None,
            _marker: PhantomData,
        }
    }
}

pub struct Map<A, B> {
    a: A,
    b: B,
}

impl<A, B> ReactiveSystem for Map<A, B>
where
    A: ReactiveSystem,
    B: ReactiveSystem<In = A::Out>,
{
    type In = A::In;

    type Out = B::Out;

    fn init(&mut self, world: &mut World) {
        self.a.init(world);
        self.b.init(world);
    }

    fn is_changed(&mut self, mut world: DeferredWorld) -> bool {
        self.a.is_changed(world.reborrow()) || self.b.is_changed(world)
    }

    fn run(&mut self, input: Self::In, mut world: DeferredWorld, entity: Entity) -> Self::Out {
        let out = self.a.run(input, world.reborrow(), entity);
        self.b.run(out, world, entity)
    }
}

struct Inner {
    system: Box<dyn ReactiveSystem<In = (), Out = ()>>,
    entities: Vec<Entity>,
}

#[derive(Clone)]
pub struct Reaction {
    inner: Arc<Mutex<Inner>>,
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
}

pub fn react(mut world: DeferredWorld, reaction_query: Query<(Entity, &Reaction)>) {
    for (entity, reaction) in &reaction_query {
        let inner = &mut *reaction.inner.lock().unwrap();

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
