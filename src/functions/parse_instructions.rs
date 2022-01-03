use crate::structures::{Error, Instruction};
use crate::traits::AsString;
use log::error;


pub(crate) fn parse_instructions(instructions: Box<String>) -> Result<Vec<Instruction>, Error> {
    let instructions_data = match json::parse(&instructions) {
    Ok(result) => result,
    Err(e) => return Err(Error::InvalidJson(format!("instructions.json is invalid: {}", e), *instructions))
  };
  let mut instructions = Vec::with_capacity(instructions_data.len());
  instructions_data.into_inner().iter().for_each(|instruction| {
    let mut closure = || -> Result<(), Error> {
      instructions.push(Instruction {
        path:                 instruction["Path"].as_string().replace("\\", "/"),
        previous_hash:        instruction["OldHash"].as_string_option(),
        newest_hash:          instruction["NewHash"].as_string_option(),
        full_vcdiff_hash:     instruction["CompressedHash"].as_string_option(),
        delta_vcdiff_hash:    instruction["DeltaHash"].as_string_option(),
        full_vcdiff_size:     instruction["FullReplaceSize"].as_u64().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["FullReplaceSize"])))?,
        delta_vcdiff_size:    instruction["DeltaSize"].as_u64().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["DeltaSize"])))?,
        has_delta:            instruction["HasDelta"].as_bool().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["HasDelta"])))?
      });
      Ok(())
    };
    match closure() {
      Ok(()) => {},
      Err(e) => error!("Transforming instructions failed for instruction {}, with error: {}", instruction, e)
    };
  });
  Ok(instructions)
}