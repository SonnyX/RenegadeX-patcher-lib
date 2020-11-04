use std::task::{ Context, Poll };
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, atomic::AtomicBool};
use std::sync::atomic::Ordering;

pub trait PausableTrait<B, A: Future<Output = B>> {
  fn pausable(self, paused: Arc<AtomicBool>) -> Pausable<B, A>;
}

impl<B, A: Future<Output = B>> PausableTrait<B, A> for A {
  fn pausable(self, paused: Arc<AtomicBool>) -> Pausable<B, A> {
    Pausable {
        a: self,
        paused,
    }
  }
}

pub struct Pausable<B, A: Future<Output = B>> {
    a: A,
    paused: Arc<AtomicBool>,
}

impl<B, A: Future<Output = B>> Future for Pausable<B, A> {
    type Output = B;

    fn poll(self: Pin<&mut Self>, wake: &mut Context<'_>) -> Poll<Self::Output> {
        if self.paused.load(Ordering::Relaxed) {
            return Poll::Pending;
        }
        
        if let Poll::Ready(b) = unsafe { self.map_unchecked_mut(|s| &mut s.a) }.poll(wake) {
            Poll::Ready(b)
        } else {
            Poll::Pending
        }
    }
}