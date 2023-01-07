use std::sync::Arc;
use std::task::{ Context, Poll };
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use futures::task::AtomicWaker;

pub trait BackgroundService {
  fn pause(&self) -> Result<(), ()>;
  fn resume(&self) -> Result<(), ()>;
  fn stop(&self) -> Result<(), ()>;
}

#[derive(Clone, Debug)]
pub struct FutureContext {
  pub paused: Arc<AtomicBool>,
  pub waker: Arc<AtomicWaker>,
  pub cancelled: Arc<AtomicBool>,
}

impl FutureContext {
  pub(crate) fn new() -> Self {
    FutureContext {
      paused: Arc::new(AtomicBool::new(false)),
      waker: Arc::new(AtomicWaker::new()),
      cancelled: Arc::new(AtomicBool::new(false))
    }
  }
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
  fn pausable(self, context: Arc<FutureContext>) -> Pausable<B, A>;
}

impl<B, A: Future<Output = B>> PausableTrait<B, A> for A {
  fn pausable(self, context: Arc<FutureContext>) -> Pausable<B, A> {
    Pausable {
      future: self,
      context
    }
  }
}

pub struct Pausable<B, A: Future<Output = B>> {
  future: A,
  context: Arc<FutureContext>
}

impl<B, A: Future<Output = B>> Future for Pausable<B, A> {
  type Output = B;

  fn poll(mut self: Pin<&mut Self>, wake: &mut Context<'_>) -> Poll<Self::Output> {
    if self.context.cancelled.load(Ordering::Relaxed) {
      drop(wake);
      panic!("Cancelled futures!");
    }
    if self.context.paused.load(Ordering::Relaxed) {
      self.context.waker.register(wake.waker());
      return Poll::Pending;
    }

    if let Poll::Ready(b) = unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.future) }.poll(wake) {
      Poll::Ready(b)
    } else {
      Poll::Pending
    }
  }
}