//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-14

use super::*;
use alloc::vec::Vec;

macro_rules! int_resource {
  ($tp:ty,) => {
    impl Resource for $tp {
      #[inline]
      fn new() -> Self { Self::INIT }
    }

    impl ConstResource for $tp {
      const INIT: Self = 0;
    }
  };
  ($tp:ty, $($rest:ty,)+) => {
    int_resource!($tp,);

    int_resource!($($rest,)+);
  };
  ($tp:ty $(, $rest:ty)*) => {
    int_resource!($tp,);

    int_resource!($($rest,)*);
  };
}

macro_rules! float_resource {
  ($tp:ty,) => {
    impl Resource for $tp {
      #[inline]
      fn new() -> Self { Self::INIT }
    }

    impl ConstResource for $tp {
      const INIT: Self = 0.;
    }
  };
  ($tp:ty, $($rest:ty,)+) => {
    float_resource!($tp,);

    float_resource!($($rest,)+);
  };
  ($tp:ty $(, $rest:ty)*) => {
    float_resource!($tp,);

    float_resource!($($rest,)*);
  };
}

int_resource!(usize, isize, u8, i8, u16, i16, u32, i32, u64, i64, u128, i128,);

float_resource!(f32, f64,);

impl<R,> Resource for Vec<R,> {
  #[inline]
  fn new() -> Self { Vec::new() }
}

impl<R,> ConstResource for Vec<R,> {
  const INIT: Self = Self::new();
}
