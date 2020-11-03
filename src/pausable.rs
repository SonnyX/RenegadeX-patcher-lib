use std::task::Poll;
use std::future::Future;

pub struct Pausable<B, A: Future<Output = B> + Unpin> {
    a: A,
}

impl<B, A: std::future::Future<Output = B> + Unpin> Future for Pausable<B, A> where A: Future<Output = B> {
    type Output = B;

    fn poll(mut self: std::pin::Pin<&mut Self>, wake: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let paused = true;
        if paused {
            return Poll::Pending;
        }
        
        if let Poll::Ready(b) = std::pin::Pin::new(&mut self.a).poll(wake) {
            Poll::Ready(b)
        } else {
            Poll::Pending
        }
    }
}