use crate::ReactiveQueryData;
use bevy_ecs::{
    component::Tick,
    prelude::*,
    query::{QueryData, QueryFilter, ReadOnlyQueryData, WorldQuery},
    system::{SystemMeta, SystemParam, SystemState},
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld},
};
use bevy_utils::HashSet;
use std::error::Error;

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

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let _ = world;
        let _ = system_meta;
        todo!()
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
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
