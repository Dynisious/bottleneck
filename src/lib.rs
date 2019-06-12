//! A crate which defines types and traits for [ResourcePool]s which provide mutal
//! exclusion to a pool of resource instances.
//! 
//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-13

#![deny(missing_docs,)]
#![feature(const_fn, const_raw_ptr_deref, const_vec_new,)]

mod sync_stack;
pub mod pollicy;
mod single_resource;
mod multi_resource;

pub use self::{single_resource::*, multi_resource::*,};

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

/// Defines the behaviour of a resource pool.
pub unsafe trait ResourcePool {
  /// Defines the type of resource in this resource pool.
  type Resource;

  /// Gets a resource from the resource pool.
  /// 
  /// The `ResourcePool` must enforce mutal exculsion to each resource.
  fn get_resource<F,>(&self, f: F,)
    where F: FnOnce(usize, &mut Self::Resource,);
}
