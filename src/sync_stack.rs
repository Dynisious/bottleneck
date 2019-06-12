//! Author --- daniel.bechaz@gmail.com  
//! Last Moddified --- 2019-06-13

use std::{
  thread::{self, Thread,},
  sync::atomic::{AtomicPtr, Ordering,},
};

/// A stack of blocked threads.
pub(super) struct SyncStack(AtomicPtr<SyncStackNode>,);

impl SyncStack {
  /// Creates a new empty SyncStack.
  pub const fn new() -> Self { SyncStack(AtomicPtr::new(std::ptr::null_mut(),),) }
  /// Blocks this thread until another thread pops it from the `SyncStack`.
  /// 
  /// Returns `true` if this thread was blocked and then unblocked.
  pub fn push(&self,) -> bool {
    //The node for this thread on the sync stack.
    let mut node = SyncStackNode {
      thread: thread::current(),
      rest: self.0.load(Ordering::Acquire,),
    };
    let node_ptr = &mut node as *mut _;
    
    //Attempt to update the current pointer. 
    if self.0.compare_and_swap(node.rest, node_ptr, Ordering::Release,) == node.rest {
      //Pointer updated, park.
      thread::park();

      //Unparked, return
      true
    } else { false }
  }
  /// Unblocks a thread from the `SyncStack`.
  /// 
  /// Returns `true` if a thread was unblocked.
  pub fn pop(&self,) -> bool {
    //Get the node on the top of the stack.
    let mut node = self.0.load(Ordering::Acquire,);

    loop {
      //Confirm that the stack is not empty.
      if node == std::ptr::null_mut() { return false }

      let rest = unsafe { &mut *node }.rest;
      let new_node = self.0.compare_and_swap(node, rest, Ordering::AcqRel,);

      //Pop the node off the stack.
      if new_node == node {
        //Unpark the thread.
        unsafe { &mut *node }.thread.unpark();
        return true;
      } else {
        node = new_node;
      }
    }
  }
}

/// A node in a `SyncStack`
pub(super) struct SyncStackNode {
  /// The thread to wake.
  thread: Thread,
  /// The rest of the `SyncStack`.
  rest: *mut Self,
}
