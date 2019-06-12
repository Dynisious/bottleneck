//! Defines the different kinds of reuse policies for resource pools.
//! 
//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-13

/// A trait which defines a reuse pollicy.
pub trait ReusePollicy<R,> {
  /// Returns `true` if `resource` should be reused.
  fn reuse(resource: &mut R,) -> bool;
}

/// A flag to indicate that a resource pool should reuse resource instances.
#[derive(Default,)]
pub struct Reuse;

impl<R,> ReusePollicy<R,> for Reuse {
  #[inline]
  fn reuse(_: &mut R,) -> bool { true }
}

/// A flag to indicate that a resource pool should not reuse resource instances.
#[derive(Default,)]
pub struct NoReuse;

impl<R,> ReusePollicy<R,> for NoReuse {
  #[inline]
  fn reuse(_: &mut R,) -> bool { false }
}
