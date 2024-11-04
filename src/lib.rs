use bevy::{
    ecs::system::{SystemParam, SystemParamItem},
    prelude::*,
};
use std::{marker::PhantomData, sync::Mutex};

pub trait ReactiveSystemParam: SystemParam {
    fn is_changed(world: &World) -> bool;

    fn get(world: &World) -> Self::Item<'_, '_>;
}

impl<R: Resource> ReactiveSystemParam for Res<'_, R> {
    fn is_changed(world: &World) -> bool {
        world.resource_ref::<R>().is_changed()
    }

    fn get(world: &World) -> Self::Item<'_, '_> {
        world.resource_ref::<R>()
    }
}

impl<T: ReactiveSystemParam> ReactiveSystemParam for (T,) {
    fn is_changed(world: &World) -> bool {
        T::is_changed(world)
    }

    fn get(world: &World) -> Self::Item<'_, '_> {
        (T::get(world),)
    }
}

pub trait ReactiveSystemParamFunction<Marker> {
    type Param: ReactiveSystemParam;

    fn is_changed(&self, world: &World) -> bool;

    fn run(&mut self, param: SystemParamItem<Self::Param>);
}

impl<Marker, F> ReactiveSystemParamFunction<Marker> for F
where
    F: SystemParamFunction<Marker, In = (), Out = ()>,
    F::Param: ReactiveSystemParam,
{
    type Param = F::Param;

    fn is_changed(&self, world: &World) -> bool {
        <F::Param as ReactiveSystemParam>::is_changed(world)
    }

    fn run(&mut self, param: SystemParamItem<Self::Param>) {
        SystemParamFunction::run(self, (), param)
    }
}

pub trait ReactiveSystem: Send + Sync {
    fn is_changed(&self, world: &World) -> bool;

    fn run(&mut self, world: &World);
}

pub struct FunctionReactiveSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<F, Marker> ReactiveSystem for FunctionReactiveSystem<F, Marker>
where
    F: ReactiveSystemParamFunction<Marker> + Send + Sync,
    Marker: Send + Sync,
{
    fn is_changed(&self, world: &World) -> bool {
        self.f.is_changed(world)
    }

    fn run(&mut self, world: &World) {
        self.f.run(F::Param::get(world));
    }
}

#[derive(Component)]
pub struct Reaction {
    system: Mutex<Box<dyn ReactiveSystem>>,
}

impl Reaction {
    pub fn new<Marker>(
        system: impl ReactiveSystemParamFunction<Marker> + Send + Sync + 'static,
    ) -> Self
    where
        Marker: Send + Sync + 'static,
    {
        Self {
            system: Mutex::new(Box::new(FunctionReactiveSystem {
                f: system,
                _marker: PhantomData,
            })),
        }
    }
}

pub fn react(world: &World, reaction_query: Query<&Reaction>) {
    for reaction in &reaction_query {
        let mut system = reaction.system.lock().unwrap();
        if system.is_changed(world) {
            system.run(world);
        }
    }
}
