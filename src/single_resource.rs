//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-15

use super::*;
use crate::pollicy::*;
use sync_stack::*;
use core::{
  sync::atomic::{AtomicBool, Ordering,},
  marker::PhantomData,
};

/// Stores a single resource and forces all threads to access it one at a time.
pub struct SingleResource<R, Pollicy = Reuse,> {
  /// The resource instance to use.
  resource: R,
  /// A flag indicating if the resource is currently in use.
  in_use: AtomicBool,
  /// A stack of threads waiting to access this resource.
  sync_stack: SyncStack,
  _data: PhantomData<Pollicy>,
}

impl<R, P,> SingleResource<R, P,>
  where R: ConstResource, {
  /// A constant inital resource pool.
  pub const INIT: Self = Self {
    resource: R::INIT,
    in_use: AtomicBool::new(false,),
    sync_stack: SyncStack::new(),
    _data: PhantomData,
  };
}

impl<R, P,> SingleResource<R, P,>
  where R: Resource, {
  /// Creates a new resource pool.
  pub fn new() -> Self { Self::with_resource(R::new(),) }
}

impl<R, P,> SingleResource<R, P,> {
  /// Creates a new resource pool.
  /// 
  /// # Param
  /// 
  /// resource --- The `Resource` to use.  
  pub const fn with_resource(resource: R,) -> Self {
    Self {
      resource,
      in_use: AtomicBool::new(false,),
      sync_stack: SyncStack::new(),
      _data: PhantomData,
    }
  }
  /// Gets the resource instance mutably.
  #[inline]
  unsafe fn resource_mut(&self,) -> &mut R { &mut *(&self.resource as *const R as *mut R) }
}

impl<R,> SingleResource<R, Reuse,>
  where R: Resource, {
  /// Attempts to aquire the resource and run the closure.
  /// 
  /// If the resource could not be locked the closure is returned.
  fn attempt_aquire<F,>(&self, f: F,) -> Option<F>
    where F: FnOnce(usize, &mut R,), {
    struct Finish<'pool, R,> {
      pool: &'pool SingleResource<R, Reuse,>,
    }

    impl<R,> Drop for Finish<'_, R,> {
      fn drop(&mut self,) {
        //Release the lock.
        self.pool.in_use.store(false, Ordering::Relaxed,);
        //Pop a waiting thread from the stack.
        self.pool.sync_stack.pop();
      }
    }

    //Attempt to lock the resource.
    if self.in_use.compare_and_swap(false, true, Ordering::Acquire,) { return Some(f) }

    let _ = Finish { pool: self, };

    //Aquired the resource.
    f(0, unsafe { self.resource_mut() },);

    None
  }
}

impl<R,> SingleResource<R, NoReuse,>
  where R: Resource, {
  /// Attempts to aquire the resource and run the closure.
  /// 
  /// If the resource could not be locked the closure is returned.
  fn attempt_aquire<F,>(&self, f: F,) -> Option<F>
    where F: FnOnce(usize, &mut R,), {
    //Attempt to lock the resource.
    if self.in_use.compare_and_swap(false, true, Ordering::Acquire,) { return Some(f) }

    //Aquired the resource.
    let mut resource = core::mem::replace(unsafe { self.resource_mut() }, R::new(),);
    //Release the lock.
    self.in_use.store(false, Ordering::Relaxed,);
    //Pop a waiting thread from the stack.
    self.sync_stack.pop();
    
    f(0, &mut resource,);

    None
  }
}

impl<R, P,> SingleResource<R, Pollicy<P,>,>
  where R: Resource,
    P: ReusePollicy<R,>, {
  /// Attempts to aquire the resource and run the closure.
  /// 
  /// If the resource could not be locked the closure is returned.
  fn attempt_aquire<F,>(&self, f: F,) -> Option<F>
    where F: FnOnce(usize, &mut R,), {
    struct Finish<'a, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      pool: &'a SingleResource<R, Pollicy<P,>,>,
    }

    impl<R, P,> Drop for Finish<'_, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      fn drop(&mut self,) {
        //Check if the resource should be reused.
        let resource = unsafe { self.pool.resource_mut() };
        if !P::reuse(resource,) { *resource = R::new() }
        
        //Release the lock.
        self.pool.in_use.store(false, Ordering::Relaxed,);
        //Pop a waiting thread from the stack.
        self.pool.sync_stack.pop();
      }
    }

    //Attempt to lock the resource.
    if self.in_use.compare_and_swap(false, true, Ordering::Acquire,) { return Some(f) }

    let _ = Finish { pool: self, };
    //Aquired the resource.
    let resource = unsafe { self.resource_mut() };
    f(0, resource,);

    None
  }
}

unsafe impl<R,> ResourcePool for SingleResource<R, NoReuse,>
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

unsafe impl<R,> ResourcePool for SingleResource<R, Reuse,>
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

unsafe impl<R, Pol,> ResourcePool for SingleResource<R, Pollicy<Pol,>,>
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
  use std::thread::Thread;

  #[test]
  fn test_single_resource() {
    let resource = SingleResource::<i32, Reuse,>::new();

    resource.get_resource::<Thread, _,>(|_, r,| *r = 1,);
    resource.get_resource::<Thread, _,>(|_, r,| assert_eq!(*r, 1,),);

    let resource = SingleResource::<i32, NoReuse,>::new();

    resource.get_resource::<Thread, _,>(|_, r,| *r = 1,);
    resource.get_resource::<Thread, _,>(|_, r,| assert_eq!(*r, 0,),);
  }
  #[test]
  fn test_single_resource_multithread() {
    use std::{thread, time::Duration,};

    static RESOURCE: SingleResource<i32, Reuse,> = SingleResource::INIT;

    thread::spawn(move || RESOURCE.get_resource::<Thread, _,>(|_, r,| {
      thread::sleep(Duration::from_millis(2000,),); *r = 1;
    },),);
    thread::spawn(move || {
      thread::sleep(Duration::from_millis(2000,),);
      RESOURCE.get_resource::<Thread, _,>(|_, r,| assert_eq!(*r, 1,),)
    },);

    let resource = SingleResource::<i32, NoReuse,>::new();

    resource.get_resource::<Thread, _,>(|_, r,| *r = 1,);
    resource.get_resource::<Thread, _,>(|_, r,| assert_eq!(*r, 0,),);
  }
}
