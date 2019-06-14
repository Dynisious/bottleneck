//! Defines the different kinds of reuse policies for resource pools.
//! 
//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-14

use core::marker::PhantomData;

/// A trait which defines a reuse pollicy.
pub trait ReusePollicy<R,> {
  /// Returns `true` if `resource` should be reused.
  /// 
  /// This function is called after a `resource` has been released or the thread holding
  /// it panicked. If any cleanup is required before a `resource` instance can be reused
  /// it should be performed by this function.
  /// 
  /// # Params
  /// 
  /// resource --- The resource instance which was just released.
  fn reuse(resource: &mut R,) -> bool;
}

/// A pollicy which makes the decision to reuse or discard a resource instance at runtime.
pub struct Pollicy<P,>(PhantomData<P>,);

/// A flag to indicate that a resource pool should reuse resource instances.
/// 
/// This is the strictest pollicy which will spend the minimum amount of time producing
/// resource instances.
pub struct Reuse;

/// A flag to indicate that a resource pool should not reuse resource instances.
/// 
/// This is the loosest pollicy which also provides the highest concurrency; since no
/// resource instance will be reused twice, a new one is generated for every caller.
/// 
/// As an example of the consequences of this pollicy, a `SingleResource` pool using this
/// pollicy provides higher concurrency than a `MultiResource` pool using the same
/// pollicy by producing new resource instances as needed while a `MultiResource` will
/// always limit the number of threads which can access the pool at once.
pub struct NoReuse;
