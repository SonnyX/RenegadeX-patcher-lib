use crate::structures::instructions::Instruction;

pub(crate) struct InstructionGroup {
	/// SHA256 hash of this file during current patch, None if the file is to be deleted
	pub hash: Option<String>,
	pub instructions: Vec<Instruction>,
}