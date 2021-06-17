use crate::structures::{Error, Mirrors, GameState};
use ini::Ini;

pub fn get_game_state(mirrors: Mirrors, renegadex_location: String) -> Result<GameState, Error> {
  if mirrors.is_empty() {
    return Err(Error::NoMirrors());
  }
  let patch_dir_path = format!("{}/patcher/", renegadex_location).replace("//", "/");
  match std::fs::read_dir(patch_dir_path) {
    Ok(iter) => {
      if iter.count() != 0 {
        return Ok(GameState::Resume);
      }
    },
    Err(_e) => {}
  };

  let path = format!("{}/UDKGame/Config/DefaultRenegadeX.ini", renegadex_location);
  let conf = match Ini::load_from_file(&path) {
    Ok(file) => file,
    Err(_e) => {
      return Ok(GameState::Full);
    }
  };

  let section = conf.section(Some("RenX_Game.Rx_Game".to_owned())).ok_or_else(|| Error::None(format!("No Configuration section named \"RenX_Game.Rx_Game\"")))?;
  let game_version_number = section.get("GameVersionNumber").ok_or_else(|| Error::None(format!("No key in section \"RenX_Game.Rx_Game\"  named \"GameVersionNumber\"")))?;

  if mirrors.version_number.ok_or_else(|| Error::None(format!("Version Number in mirrors missing")))? != game_version_number {
    return Ok(GameState::Delta);
  }
  Ok(GameState::UpToDate)
}