//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-13

use super::*;
use crate::{pollicy::*, sync_stack::*,};
use std::{
  sync::atomic::{AtomicBool, Ordering,},
  marker::PhantomData,
};

/// Stores a single resource and forces all threads to access it one at a time.
pub struct SingleResource<R, Pollicy = NoReuse,> {
  resource: R,
  in_use: AtomicBool,
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
  #[inline]
  unsafe fn resource_mut(&self,) -> &mut R { &mut *(&self.resource as *const R as *mut R) }
  /// Creates a new resource pool.
  /// 
  /// # Param
  /// 
  /// resource --- The `Resource` to use.  
  pub fn with_resource(resource: R,) -> Self {
    Self {
      resource: resource.into(),
      in_use: AtomicBool::new(false,),
      sync_stack: SyncStack::new(),
      _data: PhantomData,
    }
  }
}

unsafe impl<R, P,> ResourcePool for SingleResource<R, P,>
  where R: Resource,
    P: ReusePollicy<R,>, {
  type Resource = R;

  fn get_resource<F,>(&self, f: F,)
    where F: FnOnce(usize, &mut Self::Resource,), {
    struct Finish<'a, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      pool: &'a SingleResource<R, P,>,
    }

    impl<R, P,> Drop for Finish<'_, R, P,>
      where R: Resource,
        P: ReusePollicy<R,>, {
      fn drop(&mut self,) {
        if !P::reuse(unsafe { self.pool.resource_mut() },) {
          core::mem::replace(unsafe { self.pool.resource_mut() }, R::new(),);
        }
        
        //Release the resource.
        self.pool.in_use.store(false, Ordering::Release,);
        //Unpark a waiting thread.
        self.pool.sync_stack.pop();
      }
    }

    loop {
      if self.in_use.compare_and_swap(false, true, Ordering::AcqRel,) {
        //Wait for the resource.
        self.sync_stack.push();
      } else {
        //Aquired the resource.
        let resource = unsafe { self.resource_mut() };
        let _ = Finish { pool: self, };
        
        return f(0, resource,);
      }
    }
  }
}

#[cfg(test,)]
mod tests {
  use super::*;

  impl Resource for i32 {
    #[inline]
    fn new() -> Self { 0 }
  }

  impl ConstResource for i32 {
    const INIT: Self = 0;
  }

  #[test]
  fn test_single_resource() {
    let resource = SingleResource::<i32, Reuse,>::new();

    resource.get_resource(|_, r,| *r = 1,);
    resource.get_resource(|_, r,| assert_eq!(*r, 1,),);

    let resource = SingleResource::<i32, NoReuse,>::new();

    resource.get_resource(|_, r,| *r = 1,);
    resource.get_resource(|_, r,| assert_eq!(*r, 0,),);
  }
  #[test]
  fn test_single_resource_multithread() {
    use std::{thread, time::Duration,};

    static RESOURCE: SingleResource<i32, Reuse,> = SingleResource::INIT;

    thread::spawn(move || RESOURCE.get_resource(|_, r,| {
      thread::sleep(Duration::from_millis(600,),); *r = 1;
    },),);
    thread::spawn(move || {
      thread::sleep(Duration::from_millis(300,),);
      RESOURCE.get_resource(|_, r,| assert_eq!(*r, 1,),)
    },);

    let resource = SingleResource::<i32, NoReuse,>::new();

    resource.get_resource(|_, r,| *r = 1,);
    resource.get_resource(|_, r,| assert_eq!(*r, 0,),);
  }
}
