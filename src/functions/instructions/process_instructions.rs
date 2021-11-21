

pub fn process_instruction(instruction: &instruction) {
    //lets start off by trying to open the file.
    match OpenOptions::new().read(true).open(&instruction.path) {
      Ok(_file) => {
        // Check whether the instruction says whether to keep, update, or delete the file
        if instruction.newest_hash.is_some() {
          // Keep or update the file
          add_file_to_hash_queue(instruction);
        } else {
          // Remove the file
          info!("Found entry {} that needs deleting.", instruction.path);
          //TODO: DeletionQueue, delete it straight away?
        }
      },
      Err(_e) => {
        if let Some(key) = &instruction.newest_hash {
          let delta_path = format!("{}patcher/{}", self.renegadex_location.borrow(), &key);
          let mut download_hashmap = self.download_hashmap.lock().unexpected("");

          // Check if a download for this hash already exists
          if !download_hashmap.contains_key(key) {
            let download_entry = DownloadEntry {
              file_path: delta_path.clone(),
              file_size: instruction.full_vcdiff_size,
              file_hash: instruction.full_vcdiff_hash.clone().unexpected(""),
              patch_entries: Vec::new(),
            };
            download_hashmap.insert(key.clone(), download_entry);
            let mut state = self.state.lock().unexpected("");
            state.download_size.1 += instruction.full_vcdiff_size as u64;
            drop(state);
          }

          // Add a new patch_entry for this instruction to the after-download actions
          let patch_entry = PatchEntry {
            target_path: instruction.path.clone(),
            delta_path,
            has_source: false,
            target_hash: key.clone(),
          };
          download_hashmap.get_mut(key).unexpected("").patch_entries.push(patch_entry); //should we add it to a downloadQueue??
          drop(download_hashmap);
          let mut state = self.state.lock().unexpected("");
          state.patch_files.1 += 1;
          drop(state);
        }
      }
    };
}

fn add_file_to_hash_queue(instruction: Instruction) {
  let mut hash_queue = self.hash_queue.lock().unexpected("");
  hash_queue.push(instruction.clone());
  drop(hash_queue);
  let mut state = self.state.lock().unexpected("");
  state.hashes_checked.1 += 1;
  drop(state);
}