use std::task::{ Context, Poll };
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use futures::task::AtomicWaker;

pub static FUTURE_CONTEXT : FutureContext = FutureContext { paused: AtomicBool::new(false), waker: AtomicWaker::new(), cancelled: AtomicBool::new(false) };

pub trait BackgroundService {
  fn pause(&self) -> Result<(), ()>;
  fn resume(&self) -> Result<(), ()>;
  fn stop(&self) -> Result<(), ()>;
}

pub struct FutureContext {
  pub paused: AtomicBool,
  pub waker: AtomicWaker,
  pub cancelled: AtomicBool,
}

impl BackgroundService for FutureContext {
  fn pause(&self) -> Result<(), ()> {
    if self.paused.load(Ordering::Relaxed) {
      return Err(());
    }
    self.paused.swap(true, Ordering::Relaxed);
    Ok(())
  }
  fn resume(&self) -> Result<(), ()> {
    if !self.paused.load(Ordering::Relaxed) {
      return Err(());
    }
    self.paused.swap(false,Ordering::Relaxed);
    self.waker.wake();
    Ok(())
  }
  fn stop(&self) -> Result<(), ()> {
    if self.cancelled.load(Ordering::Relaxed) {
      return Err(());
    }
    self.cancelled.swap(true, Ordering::Relaxed);
    Ok(())
  }
}

pub trait PausableTrait<B, A: Future<Output = B>> {
  fn pausable(self) -> Pausable<B, A>;
}

impl<B, A: Future<Output = B>> PausableTrait<B, A> for A {
  fn pausable(self) -> Pausable<B, A> {
    Pausable {
      future: self,
    }
  }
}

pub struct Pausable<B, A: Future<Output = B>> {
  future: A,
}

impl<B, A: Future<Output = B>> Future for Pausable<B, A> {
  type Output = B;

  fn poll(mut self: Pin<&mut Self>, wake: &mut Context<'_>) -> Poll<Self::Output> {
    if FUTURE_CONTEXT.paused.load(Ordering::Relaxed) {
      FUTURE_CONTEXT.waker.register(wake.waker());
      return Poll::Pending;
    }
    // I copied this code from Stack Overflow without reading the text that told me how to verify that this code uses `unsafe` correctly.
    // https://stackoverflow.com/questions/57369123/no-method-named-poll-found-for-a-type-that-implements-future
    if let Poll::Ready(b) = unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.future) }.poll(wake) {
      Poll::Ready(b)
    } else {
      Poll::Pending
    }
  }
}