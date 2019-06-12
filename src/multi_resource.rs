//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-13

use super::*;
use crate::{pollicy::*, sync_stack::*,};
use parking_lot::RawMutex;
use lock_api::Mutex;
use std::{
  sync::atomic::{AtomicUsize, Ordering,},
  marker::PhantomData,
};

/// Stores a multiple resource instances and provides mutual exclusion to all of them.
pub struct MultiResource<R, Pollicy = NoReuse,> {
  resources: Vec<R>,
  available_count: AtomicUsize,
  available_resources: Mutex<RawMutex, Vec<usize>>,
  sync_stack: SyncStack,
  _data: PhantomData<Pollicy>,
}

impl<R, P,> MultiResource<R, P,> {
  /// A constant inital resource pool storing no resources.
  pub const INIT: Self = Self {
    resources: Vec::new(),
    available_count: AtomicUsize::new(0,),
    available_resources: Mutex::new(Vec::new(),),
    sync_stack: SyncStack::new(),
    _data: PhantomData,
  };
}

impl<R, P,> MultiResource<R, P,>
  where R: Resource, {
  /// Creates a new resource pool.
  pub fn new_resources(count: usize,) -> Self {
    Self::with_resources(
      (0..count).map(|_,| R::new(),).collect(),
    )
  }
}

impl<R, P,> MultiResource<R, P,> {
  #[inline]
  unsafe fn resource_mut(&self, resource: usize,) -> &mut R {
    &mut *(&self.resources[resource] as *const R as *mut R)
  }
  /// Creates a new resource pool.
  /// 
  /// # Param
  /// 
  /// resources --- The `Resource`s to use.  
  pub fn with_resources(resources: Vec<R>,) -> Self {
    Self {
      available_count: AtomicUsize::new(resources.len()),
      available_resources: Mutex::new((0..resources.len()).collect(),),
      resources,
      sync_stack: SyncStack::new(),
      _data: PhantomData,
    }
  }
  /// Replaces all the resources with `resources`.
  pub fn replace_resources(&self, resources: Vec<R,>,) {
    let mut to_own = self.resources.len();
    //Own all resources.
    while to_own > 0 {
      //Check if there are resources available.
      if self.available_count.load(Ordering::Acquire,) == 0 {
        //Wait for a resource to become available.
        self.sync_stack.push();
        continue;
      }

      //---Lock all available resources---

      let mut available = self.available_resources.lock();

      to_own -= available.len();
      available.clear();
      self.available_count.store(0, Ordering::Release,);
    }

    let resources_len = resources.len();

    //Replace the resource pool.
    core::mem::replace(unsafe { &mut *(&self.resources as *const _ as *mut _) }, resources,);
    //Tag all the new resources.
    self.available_resources.lock().extend(0..resources_len,);
    //Declare the number of available resources.
    self.available_count.store(resources_len, Ordering::Release,);
    
    //Unblock as many threads as there are resources.
    for _ in 0..resources_len {
      if !self.sync_stack.pop() { return }
    }
  }
}

unsafe impl<R, P,> ResourcePool for MultiResource<R, P,>
  where R: Resource,
    P: ReusePollicy<R,>, {
  type Resource = R;

  fn get_resource<F,>(&self, f: F,)
    where F: FnOnce(usize, &mut Self::Resource,), {
    struct Finish<'a, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      resource: usize,
      pool: &'a MultiResource<R, P,>,
    }

    impl<R, P,> Drop for Finish<'_, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      fn drop(&mut self,) {
        let resource = unsafe { self.pool.resource_mut(self.resource,) };

        if !P::reuse(resource,) {
          core::mem::replace(resource, R::new(),);
        }

        let mut available = self.pool.available_resources.lock();

        //Release the resource.
        available.push(self.resource,);
        //The number of available resources.
        let available_len = available.len();
        self.pool.available_count.store(available_len, Ordering::Release,);
        //Release the lock.
        core::mem::drop(available,);

        //Unpark as many threads as there are resources.
        for _ in 0..available_len {
          //Unpark a waiting thread.
          if !self.pool.sync_stack.pop() { break }
        }
      }
    }

    loop {
      if self.available_count.load(Ordering::Acquire,) == 0 {
        //Wait for a resource to become available.
        self.sync_stack.push();
      } else {
        let mut available = match self.available_resources.try_lock() {
          Some(lock) => lock,
          None => continue,
        };
        let available = match available.pop() {
          Some(resource) => resource,
          None => continue,
        };
        //Aquired the resource.
        let resource = unsafe { self.resource_mut(available,) };
        let _ = Finish { resource: available, pool: self, };
        
        return f(0, resource,);
      }
    }
  }
}

#[cfg(test,)]
mod tests {
  use super::*;

  #[test]
  fn test_multi_resource_multithread() {
    use std::{thread, time::Duration,};

    impl Resource for usize {
      fn new() -> Self { 0 }
    }

    static RESOURCE: MultiResource<usize, Reuse,> = MultiResource::INIT;

    RESOURCE.replace_resources(vec![0; 5],);
    
    for _ in 0..10 {
      thread::spawn(move || RESOURCE.get_resource(|i, r,| {
        thread::sleep(Duration::from_millis(2000,),); *r = i;
      },),);
    }

    for _ in 0..10 {
      thread::spawn(move || RESOURCE.get_resource(|i, r,| {
        thread::sleep(Duration::from_millis(400,),); assert_eq!(*r, i,)
      },),);
    }
  }
}
