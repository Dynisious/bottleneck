//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-15

use super::*;
use crate::pollicy::*;
use sync_stack::*;
use core::marker::PhantomData;
use alloc::vec::Vec;

/// Stores a multiple resource instances and provides mutual exclusion to all of them.
pub struct MultiResource<R, Pollicy = Reuse,> {
  /// The collection of resources to use.
  resources: Vec<R>,
  /// The indexes of the resources not currently in use.
  available_resources: SingleResource<Vec<usize>, Reuse,>,
  /// A stack of threads waiting to access a resource.
  sync_stack: SyncStack,
  _data: PhantomData<Pollicy>,
}

impl<R, P,> MultiResource<R, P,>
  where R: Resource, {
  /// An empty resource pool.
  pub const INIT: Self = Self {
    resources: Vec::new(),
    available_resources: SingleResource::INIT,
    sync_stack: SyncStack::new(),
    _data: PhantomData,
  };

  /// Creates a new resource pool of `count` new elements.
  pub fn new_resources(count: usize,) -> Self {
    Self::with_resources(
      (0..count).map(|_,| R::new(),).collect(),
    )
  }
}

impl<R,> MultiResource<R, Reuse,>
  where R: Resource, {
  /// Attempts to aquire the resource and run the closure.
  /// 
  /// If the resource could not be locked the closure is returned.
  fn attempt_aquire<F,>(&self, f: F,) -> Option<F>
    where F: FnOnce(usize, &mut R,), {
    struct Finish<'pool, R,>
      where R: Resource, {
      resource: usize,
      pool: &'pool MultiResource<R, Reuse,>,
    }

    impl<R,> Drop for Finish<'_, R,>
      where R: Resource, {
      fn drop(&mut self,) {
        let release = |_, resources: &mut Vec<usize>,| resources.push(self.resource,);

        //Release the resource.
        while self.pool.available_resources.try_get_resource::<_,>(release,) {
          core::sync::atomic::spin_loop_hint();
        };
      }
    }

    let mut resource = None;
    //Aquire a resource.
    self.available_resources.try_get_resource(
      |_, resources,| resource = resources.pop(),
    );

    match resource {
      None => Some(f),
      Some(resource) => {
        let _ = Finish::<R,> { resource, pool: self, };

        f(resource, unsafe { &mut self.resources_mut()[resource] },);

        None
      },
    }
  }
}

impl<R,> MultiResource<R, NoReuse,>
  where R: Resource, {
  /// Attempts to aquire the resource and run the closure.
  /// 
  /// If the resource could not be locked the closure is returned.
  fn attempt_aquire<F,>(&self, f: F,) -> Option<F>
    where F: FnOnce(usize, &mut R,), {
    struct Finish<'pool, R,>
      where R: Resource, {
      resource: usize,
      pool: &'pool MultiResource<R, NoReuse,>,
    }

    impl<R,> Drop for Finish<'_, R,>
      where R: Resource, {
      fn drop(&mut self,) {
        //Replace the resource.
        unsafe { self.pool.resources_mut()[self.resource] = R::new(); }

        let release = |_, resources: &mut Vec<usize>,| resources.push(self.resource,);

        //Release the resource.
        while self.pool.available_resources.try_get_resource(release,) {
          core::sync::atomic::spin_loop_hint();
        };
      }
    }

    let mut resource = None;
    //Aquire a resource.
    self.available_resources.try_get_resource(
      |_, resources,| resource = resources.pop(),
    );

    match resource {
      None => Some(f),
      Some(resource) => {
        let _ = Finish::<R,> { resource, pool: self, };

        f(resource, unsafe { &mut self.resources_mut()[resource] },);

        None
      },
    }
  }
}

impl<R, P,> MultiResource<R, Pollicy<P,>,>
  where R: Resource,
    P: ReusePollicy<R,>, {
  /// Attempts to aquire the resource and run the closure.
  /// 
  /// If the resource could not be locked the closure is returned.
  fn attempt_aquire<F,>(&self, f: F,) -> Option<F>
    where F: FnOnce(usize, &mut R,), {
    struct Finish<'pool, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      resource: usize,
      pool: &'pool MultiResource<R, Pollicy<P,>,>,
    }

    impl<R, P,> Drop for Finish<'_, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      fn drop(&mut self,) {
        let resource = unsafe { &mut self.pool.resources_mut()[self.resource] };

        //Check if we reuse the resource.
        if !P::reuse(resource,) { *resource = R::new() }

        let release = |_, resources: &mut Vec<usize>,| resources.push(self.resource,);

        //Release the resource.
        while self.pool.available_resources.try_get_resource(release,) {
          core::sync::atomic::spin_loop_hint();
        }
      }
    }

    let mut resource = None;
    //Aquire a resource.
    self.available_resources.try_get_resource(
      |_, resources,| resource = resources.pop(),
    );

    match resource {
      None => Some(f),
      Some(resource) => {
        let _ = Finish::<R, P,> { resource, pool: self, };

        f(resource, unsafe { &mut self.resources_mut()[resource] },);

        None
      },
    }
  }
}

impl<R, Pol,> MultiResource<R, Pol,> {
  /// Gets the resources mutably.
  #[inline]
  unsafe fn resources_mut(&self,) -> &mut Vec<R> {
    &mut *(&self.resources as *const Vec<R> as *mut Vec<R>)
  }
  /// Creates a new resource pool.
  /// 
  /// # Param
  /// 
  /// resources --- The `Resource`s to use.  
  #[inline]
  pub fn with_resources(resources: Vec<R>,) -> Self {
    Self {
      available_resources: SingleResource::with_resource(
        (0..resources.len()).collect(),
      ),
      resources,
      sync_stack: SyncStack::new(),
      _data: PhantomData,
    }
  }
}

unsafe impl<R,> ResourcePool for MultiResource<R, Reuse,>
  where R: Resource, {
  type Resource = R;

  #[inline]
  fn try_get_resource<F,>(&self, f: F,) -> bool
    where F: FnOnce(usize, &mut Self::Resource,), {
    //Attempt to aquire the resource.
    let aquired = self.attempt_aquire(f,).is_none();

    //If we aquired the resource allow another a chance to.
    if aquired { self.sync_stack.pop(); }

    aquired
  }
  fn get_resource<P, F,>(&self, mut f: F,)
    where P: Park,
      F: FnOnce(usize, &mut Self::Resource,), {
    loop {
      //Attempt to aquire the resource.
      match self.attempt_aquire(f,) {
        //Failed to aquire the resource.
        Some(ret) => {
          //Return the resource for the next attempt.
          f = ret;
          //Wait for the resource to become available.
          self.sync_stack.park::<P,>();
        },
        //We aquired the resource.
        None => { self.sync_stack.pop(); break },
      }
    }
  }
}

unsafe impl<R,> ResourcePool for MultiResource<R, NoReuse,>
  where R: Resource, {
  type Resource = R;

  #[inline]
  fn try_get_resource<F,>(&self, f: F,) -> bool
    where F: FnOnce(usize, &mut Self::Resource,), {
    //Attempt to aquire the resource.
    let aquired = self.attempt_aquire(f,).is_none();

    //If we aquired the resource allow another a chance to.
    if aquired { self.sync_stack.pop(); }

    aquired
  }
  fn get_resource<P, F,>(&self, mut f: F,)
    where P: Park,
      F: FnOnce(usize, &mut Self::Resource,), {
    loop {
      //Attempt to aquire the resource.
      match self.attempt_aquire(f,) {
        //Failed to aquire the resource.
        Some(ret) => {
          //Return the resource for the next attempt.
          f = ret;
          //Wait for the resource to become available.
          self.sync_stack.park::<P,>();
        },
        //We aquired the resource.
        None => { self.sync_stack.pop(); break },
      }
    }
  }
}

unsafe impl<R, Pol,> ResourcePool for MultiResource<R, Pollicy<Pol,>,>
  where R: Resource,
    Pol: ReusePollicy<R,>, {
  type Resource = R;

  #[inline]
  fn try_get_resource<F,>(&self, f: F,) -> bool
    where F: FnOnce(usize, &mut Self::Resource,), {
    //Attempt to aquire the resource.
    let aquired = self.attempt_aquire(f,).is_none();

    //If we aquired the resource allow another a chance to.
    if aquired { self.sync_stack.pop(); }

    aquired
  }
  fn get_resource<P, F,>(&self, mut f: F,)
    where P: Park,
      F: FnOnce(usize, &mut Self::Resource,), {
    loop {
      //Attempt to aquire the resource.
      match self.attempt_aquire(f,) {
        //Failed to aquire the resource.
        Some(ret) => {
          //Return the resource for the next attempt.
          f = ret;
          //Wait for the resource to become available.
          self.sync_stack.park::<P,>();
        },
        //We aquired the resource.
        None => { self.sync_stack.pop(); break },
      }
    }
  }
}

#[cfg(test,)]
mod tests {
  use super::*;
  use std::{vec, thread::Thread,};

  #[test]
  fn test_multi_resource_multithread() {
    use std::{thread, time::Duration,};

    static mut RESOURCE: MultiResource<usize, Reuse,> = MultiResource::INIT;

    unsafe { RESOURCE = MultiResource::with_resources(vec![0; 5],); }
    
    for _ in 0..10 {
      thread::spawn(move || unsafe {
        RESOURCE.get_resource::<Thread, _,>(|i, r,| {
          thread::sleep(Duration::from_millis(2000,),); *r = i;
        },)
      },);
    }

    for _ in 0..10 {
      thread::spawn(move || unsafe {
        RESOURCE.get_resource::<Thread, _,>(|i, r,| {
          thread::sleep(Duration::from_millis(400,),); assert_eq!(*r, i,)
        },)
      },);
    }
  }
}
