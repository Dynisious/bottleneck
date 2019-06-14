//! A crate which defines types and traits for [ResourcePool]s which provide mutal
//! exclusion to a pool of resource instances.
//! 
//! # Example
//! 
//! ```rust
//! use bottleneck::*;
//! use std::{thread::{self, Thread}, time::Duration};
//! 
//! static RESOURCE: SingleResource<i32> = SingleResource::INIT;
//! 
//! thread::spawn(move || {
//!   RESOURCE.get_resource::<Thread, _>(|_, resource,| {
//!     assert_eq!(*resource, 0);
//!     *resource = 1;
//!   });
//! });
//! 
//! thread::sleep(Duration::from_secs(1));
//! 
//! RESOURCE.get_resource::<Thread, _>(|_, resource,| {
//!   assert_eq!(*resource, 1);
//! });
//! ```
//! 
//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-15

#![deny(missing_docs,)]
#![no_std]
#![feature(const_fn, const_raw_ptr_deref, const_vec_new, vec_remove_item,)]

extern crate alloc;
#[cfg(test,)]
extern crate std;

use sync_stack::Park;

mod resource;
pub mod pollicy;
mod single_resource;
mod multi_resource;

pub use self::{
  single_resource::*,
  multi_resource::*,
};

/// Defines the behaviour of a resource pool.
pub unsafe trait ResourcePool {
  /// Defines the type of resource in this resource pool.
  type Resource;

  /// Gets a resource from the resource pool.
  /// 
  /// The `ResourcePool` will enforce mutal exculsion to each resource.
  /// 
  /// The `usize` returned with the resource indicates its `Id` in the resource pool.
  /// 
  /// # Params
  /// 
  /// f --- The closure to run once a resource is aquired.  
  fn get_resource<P, F,>(&self, f: F,)
    where P: Park,
      F: FnOnce(usize, &mut Self::Resource,),;
  /// A non blocking equivelant of `get_resource`.
  /// 
  /// Returns `true` if the closure was executed.
  /// 
  /// # Params
  /// 
  /// f --- The closure to run once a resource is aquired.  
  fn try_get_resource<F,>(&self, f: F,) -> bool
    where F: FnOnce(usize, &mut Self::Resource,),;
}

/// Tags a type as a valid resource.
pub trait Resource: Sized {
  /// Creates a new instance of the resource.
  fn new() -> Self;
}

/// A resource with a constant intial value.
pub trait ConstResource: Resource {
  /// The inital value.
  const INIT: Self;
}
