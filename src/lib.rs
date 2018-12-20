extern crate reqwest;
extern crate json;

use std::process;
use std::io;
use std::fs::File;

pub fn update_game() -> Result<(), reqwest::Error> {
	let release_json_response = reqwest::get("https://static.renegade-x.com/launcher_data/version/release.json")?.text()?;
	let release_data = json::parse(&release_json_response).unwrap();
	let instructions_hash = &release_data["game"]["instructions_hash"];
	//TODO: check if instuctions_hash has changed since last time the game was started and if the previous update was succesfully completed.
	
	
	let mirrors = &release_data["game"]["mirrors"];
	let mirror_url = format!("{}{}/", &mirrors[0]["url"], &release_data["game"]["patch_path"]);
  let instructions_url = format!("{}instructions.json", &mirror_url);
	println!("Downloading instructions.json:");
	let mirror_response = reqwest::get(&instructions_url)?.text()?;
	println!("Downloading complete! Rustifying!");
  let mirror_data = json::parse(&mirror_response).unwrap();
	println!("Rustifying complete! Showing first entry:");
	println!("{}", &mirror_data[0]);
  //probably the part where tokio should kick in!

  let first_file_download_url = format!("{}full/{}",&mirror_url,&mirror_data[0]["NewHash"]);
  let mut first_file_download_response = reqwest::get(&first_file_download_url)?;
  println!("Downloaded first file into memory!");
  let mut file_delta: Vec<u8> = vec![];
  let file_delta_size = match first_file_download_response.copy_to(&mut file_delta) {
    Ok(result) => result,
    Err(e) => panic!("Copy failed: {}", e)
  };
  if file_delta_size != mirror_data[0]["FullReplaceSize"].as_u64().unwrap() {
    panic!("delta file does not match the correct size.");
  }
  
  let mut slice: &[u8] = &file_delta;
  let mut dest = {
    let fname = "/home/sonny/eclipse-workspace/renegade_x_launcher/delta";
        match File::create(&fname) {
          Ok(file) => file,
          Err(e) => panic!("Error!")
        }
  };
  match io::copy(&mut slice, &mut dest) {
    Ok(o) => o,
    Err(e) => panic!("Error!")
  };

  /*
   * Using command-line interface to decode files, nasty solution as it is not cross-platform compatible out of the box, might create a vcdiff library which is able to decompress this.
   */
  let mut xdelta = process::Command::new(Some("xdelta3").unwrap())
      .arg("-d")
      .arg("/home/sonny/eclipse-workspace/renegade_x_launcher/delta")
      .arg("/home/sonny/eclipse-workspace/renegade_x_launcher/output")
      .stdout(process::Stdio::piped())
      .stderr(process::Stdio::inherit())
      .spawn().expect("failed to execute child");
  if !xdelta.wait().expect("failed to wait on child").success() {
    println!("Failed to decompile");
  }
	Ok(())
}
