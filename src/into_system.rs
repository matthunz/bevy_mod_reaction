use crate::{
    FunctionReactiveSystem, ReactiveSystem, ReactiveSystemParam, ReactiveSystemParamFunction,
};
use bevy_ecs::{prelude::*, world::DeferredWorld};
use std::marker::PhantomData;

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
