#[cfg(test)]
mod tests {
    use crate::traits::ExpectUnwrap;
    use crate::patcher::Downloader;
    use crate::patcher::Patcher;
    use crate::pausable::PausableTrait;
    use tokio::time::Duration;
    use tokio::time::sleep;

  #[test]
   fn downloader() {

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();
    let result = rt.spawn(async {
      let mut patcher : Downloader = Downloader::new();
      patcher.set_location("C:/RenegadeX/".to_string());
      patcher.set_version_url("https://static.renegade-x.com/launcher_data/version/release.json".to_string());    
      patcher.retrieve_mirrors().await.unexpected("");
      patcher.rank_mirrors().await.unexpected("");
      patcher.remove_unversioned().await.unexpected("");
      //patcher.download().await.unexpected("");
    });
    rt.block_on(result).unexpected("downloader.rs: Couldn't do first unwrap on rt.block_on().");
  }


  #[test]
   fn patcher() -> Result<(), ()> {

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();
    let result = rt.spawn(async {
      println!("Executing Patcher::start");
      let patcher = Patcher::start_patching("renegadex_location: String".to_string(), "".to_string()).await;
      println!("Executed Patcher::start");
      sleep(Duration::from_secs(2)).await;

      patcher.pause()?;
      println!("paused");

      sleep(Duration::from_secs(15)).await;
      
      patcher.resume()?;
      println!("resumed");
      sleep(Duration::from_secs(5)).await;

      println!("Waited for 15 seconds");
      Ok::<(), ()>(())
    });
    rt.block_on(result).unexpected("downloader.rs: Couldn't do first unwrap on rt.block_on().")
  }
  /*
  #[test]
  fn test_hash() {
    let mut mirrors = Mirrors::new();
    mirrors.get_mirrors("https://static.renegade-x.com/launcher_data/version/release.json").unexpected("");
    let mirror : Mirror = mirrors.get_mirror();
    let file = OpenOptions::new().read(true).write(true).create(true).open("10kb_file").unexpected("");
    file.set_len(10004).unexpected("");

    let replace_from = mirror.address.rfind('/').unwrap_or_else(|| mirror.address.len());
    let mut download_url = format!("{}", mirror.address);
    download_url.replace_range(replace_from.., "/10kb_file");
    println!("{}", download_url);
    let resume_part = 0;
    let part_size = 10u64.pow(6) as usize;
    let download_entry = DownloadEntry {
      file_path: r"10kb_file".to_string(),
      file_size: 10000,
      file_hash: r"".to_string(),
      patch_entries: Vec::new()
    };

    let unlocked_state = Arc::new(Mutex::new(Progress::new()));
    let result : Result<(), traits::Error> = get_download_file(unlocked_state, &mirror, file, &download_url, resume_part, part_size, &download_entry);
    assert!(result.is_ok());

    let hash = get_hash("10kb_file");
    assert!(hash == "57E4EA27346F82C265C5081ED51E137A6F0DD61F51655775E83BFFCC52E48A2A")
  }
*/

  #[test]
  fn download_file_from_mirror() {
    let mut patcher : Downloader = Downloader::new();
    patcher.set_location("C:/RenegadeX/".to_string());
    patcher.set_version_url("https://static.renegade-x.com/launcher_data/version/release.json".to_string());
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    let _guard = rt.enter();
    let result = rt.spawn(async move {
      patcher.retrieve_mirrors().await.unexpected("");
      patcher.rank_mirrors().await.unexpected("");
      patcher
    }.pausable());
    let patcher = rt.block_on(result).unexpected("downloader.rs: Couldn't do first unwrap on rt.block_on().");
    println!("{:#?}", patcher);
    let file : Vec<u8> = vec![];
    let result = patcher.download_file_from_mirrors("/redists/UE3Redist.exe", file);
    println!("Download result: {:#?}", result);
  }

  /*
  #[test]
  fn download_https_file() {
    let mut mirrors = Mirrors::new();
    let mut rt = tokio::runtime::Builder::new().basic_scheduler().enable_time().enable_io().build().unwrap();
    let result = rt.enter(|| {
      rt.spawn(async move {
        mirrors.get_mirrors("https://static.renegade-x.com/launcher_data/version/release.json").await.unexpected("");
        mirrors.test_mirrors().await.unexpected("");
        mirrors
      })
    });
    let mirrors = rt.block_on(result).unexpected("downloader.rs: Couldn't do first unwrap on rt.block_on().");

    let mut mirrors_vec : Vec<Mirror> = Vec::new();
    let mut mirror : Mirror = mirrors.get_mirror();
    while !mirror.address.as_str().contains("https://") {
      mirrors_vec.insert(0, mirror);
      mirror = mirrors.get_mirror();
    }

    let file = OpenOptions::new().read(true).write(true).create(true).open("10kb_file").unexpected("");
    file.set_len(10004).unexpected("");

    let replace_from = mirror.address.rfind('/').unwrap_or_else(|| mirror.address.len());
    let mut download_url = format!("{}", mirror.address);
    download_url.replace_range(replace_from.., "/10kb_file");
    println!("{}", download_url);

    let resume_part = 0;
    let part_size = 10u64.pow(6) as usize;
    let download_entry = DownloadEntry {
      file_path: r"10kb_file".to_string(),
      file_size: 10000,
      file_hash: r"".to_string(),
      patch_entries: Vec::new()
    };

    let unlocked_state = Arc::new(Mutex::new(Progress::new()));
    let result = rt.enter(|| {
      rt.spawn(async move {
        get_download_file(unlocked_state, &mirror, file, &download_url, resume_part, part_size, &download_entry).await
      })
    });
    let result = rt.block_on(result).unexpected("downloader.rs: Couldn't do first unwrap on rt.block_on().");
    assert!(result.is_ok());

    let hash = get_hash("10kb_file").expect("Shouldn't fail");
    assert!(hash == "57E4EA27346F82C265C5081ED51E137A6F0DD61F51655775E83BFFCC52E48A2A")
  }
  */
}