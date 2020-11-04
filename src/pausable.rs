use std::task::{ Context, Poll };
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, atomic::AtomicBool};
use std::sync::atomic::Ordering;
use futures::task::AtomicWaker;

pub struct FutureContext {
  pub paused: AtomicBool,
  pub waker: AtomicWaker,
  pub cancelled: AtomicBool,
}

impl FutureContext {
  pub fn pause(&self) -> Result<(), ()> {
    if self.paused.load(Ordering::Relaxed) {
      return Err(());
    }
    self.paused.swap(true, Ordering::Relaxed);
    Ok(())
  }
  pub fn resume(&self) -> Result<(), ()> {
    if !self.paused.load(Ordering::Relaxed) {
      return Err(());
    }
    self.paused.swap(false,Ordering::Relaxed);
    self.waker.wake();
    Ok(())
  }
  pub fn cancel(&self) -> Result<(), ()> {
    if self.cancelled.load(Ordering::Relaxed) {
      return Err(());
    }
    self.cancelled.swap(true, Ordering::Relaxed);
    Ok(())
  }
}

pub trait PausableTrait<B, A: Future<Output = B>> {
  fn pausable(self, future_context: Arc<FutureContext>) -> Pausable<B, A>;
}

impl<B, A: Future<Output = B>> PausableTrait<B, A> for A {
  fn pausable(self, future_context: Arc<FutureContext>) -> Pausable<B, A> {
    Pausable {
      future: self,
      future_context
    }
  }
}

pub struct Pausable<B, A: Future<Output = B>> {
  future: A,
  future_context: Arc<FutureContext>,
}

impl<B, A: Future<Output = B>> Future for Pausable<B, A> {
  type Output = B;

  fn poll(mut self: Pin<&mut Self>, wake: &mut Context<'_>) -> Poll<Self::Output> {
    if self.future_context.paused.load(Ordering::Relaxed) {
      self.future_context.waker.register(wake.waker());
      return Poll::Pending;
    }
    // I copied this code from Stack Overflow without reading the text that
    // told me how to verify that this code uses `unsafe` correctly.
    if let Poll::Ready(b) = unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.future) }.poll(wake) {
      Poll::Ready(b)
    } else {
      Poll::Pending
    }
  }
}