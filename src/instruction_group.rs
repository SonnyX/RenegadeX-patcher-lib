use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::instructions::Instruction;

pub struct InstructionGroup {
	future: Pin<Box<dyn Future<Output = ()>>>,
	/// SHA256 hash of this file during current patch, None if the file is to be deleted
	hash: Option<String>,
	instructions: Vec<Instruction>,
}

impl Future for InstructionGroup {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, wake: &mut Context<'_>) -> Poll<Self::Output> {
		if let Poll::Ready(output) = (*self).future.as_mut().poll(wake)
		{
			Poll::Ready(output)
		} else {
			Poll::Pending
		}
	}
}

impl InstructionGroup {
	pub fn new() -> Self {
		Self {
			future: download(),
			hash: None,
			instructions: Vec::new()
		}
	}
}

fn download() -> Pin<Box<dyn Future<Output = ()>>> {
	Box::pin(async {

	})
}
