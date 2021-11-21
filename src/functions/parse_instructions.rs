use crate::structures::{Error, Instruction};
use crate::traits::AsString;
use log::error;


pub(crate) fn parse_instructions(instructions: Box<String>) -> Result<Vec<Instruction>, Error> {
    let instructions_data = match json::parse(&instructions) {
    Ok(result) => result,
    Err(e) => return Err(Error::InvalidJson("instructions.json".to_string(), *instructions))
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
        full_vcdiff_size:     instruction["FullReplaceSize"].as_usize().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["FullReplaceSize"])))?,
        delta_vcdiff_size:    instruction["DeltaSize"].as_usize().ok_or_else(|| Error::None(format!("retrieve_instructions.rs: Could not cast JSON version_number as a usize, input was {}", instruction["DeltaSize"])))?,
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


mod myTests {
    const instructions_raw : &'static str = include_str!("../../tests/instructions.json");
    use super::parse_instructions;
    use crate::structures::Instruction;
    use super::Error;
    use std::{cmp::Eq, collections::HashMap, hash::Hash};

    #[test]
    pub fn myTest() -> Result<(),Error> {
        let instructions = parse_instructions(instructions_raw.to_string())?;
        let groups = instructions.group_by_key(|a| match a.newest_hash.as_ref() { Some(a) => a.to_owned(), None => String::new()} );
        let newest_hash_groups : Vec<&Vec<Instruction>> = groups.iter().filter(|array| array.len() > 1).collect();
        //println!("{:#?}", newest_hash_groups);

        let groups = instructions.group_by_key(|a| match a.previous_hash.as_ref() { Some(a) => a.to_owned(), None => String::new()} );
        let previous_hash_groups : Vec<&Vec<Instruction>> = groups.iter().filter(|array| array.len() > 1).collect();
        //println!("{:#?}", previous_hash_groups);

        let new_files = instructions.iter().filter(|instruction| instruction.previous_hash.is_none()).cloned().collect::<Vec<Instruction>>();
        let deleted_files = instructions.iter().filter(|instruction| instruction.newest_hash.is_none()).cloned().collect::<Vec<Instruction>>();
        let moved_files = new_files.iter().filter(|instruction| {
          let hash = instruction.newest_hash.as_ref().unwrap();
          deleted_files.iter().any(|deleted_file| deleted_file.previous_hash.as_ref().unwrap().eq(hash))
        }).cloned().collect::<Vec<Instruction>>();



        let new_previous_hash_groups : Vec<&[Instruction]> = instructions.group_by(|a, b| (a.newest_hash.is_some() && b.previous_hash.is_some() && a.newest_hash.as_ref().unwrap() == b.previous_hash.as_ref().unwrap()) || (a.previous_hash.is_some() && b.newest_hash.is_some() && a.previous_hash.as_ref().unwrap() == b.newest_hash.as_ref().unwrap()) ).filter(|array| array.len() > 1).collect();
        let delta_hash_groups : Vec<&[Instruction]> = instructions.group_by(|a, b| a.delta_vcdiff_hash.is_some() && b.delta_vcdiff_hash.is_some() && a.delta_vcdiff_hash.as_ref().unwrap() == b.delta_vcdiff_hash.as_ref().unwrap() ).filter(|array| array.len() > 1).collect();
        let full_hash_groups : Vec<&[Instruction]> = instructions.group_by(|a, b| a.full_vcdiff_hash.is_some() && b.full_vcdiff_hash.is_some() && a.full_vcdiff_hash.as_ref().unwrap() == b.full_vcdiff_hash.as_ref().unwrap() ).filter(|array| array.len() > 1).collect();
        //instructions.
        Ok(())
    }

    trait GroupByKey {
      fn group_by_key<'a, F, T>(self: &Self, pred: F) -> Vec<Vec<Instruction>> where F: FnMut(&Instruction) -> T, T: Eq + Hash;
    }

  impl GroupByKey for Vec<Instruction> {
    fn group_by_key<F, T>(self: &Vec<Instruction>, mut pred: F) -> Vec<Vec<Instruction>> where F: FnMut(&Instruction) -> T, T: Eq + Hash {
      let mut m : HashMap<T, Vec<Instruction>> = HashMap::new();
      for item in self.iter() {
        m.entry(pred(item)).or_insert_with(Vec::new).push(item.to_owned())
      }
      m.values().cloned().collect::<Vec<Vec<Instruction>>>()
    }
  }
}