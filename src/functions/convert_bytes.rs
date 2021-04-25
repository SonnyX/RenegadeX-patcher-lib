use log::error;

/// Convert a raw bytesize into a network speed
pub fn convert(num: f64) -> String {
  let negative = if num.is_sign_positive() { "" } else { "-" };
  let num = num.abs();
  let units = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
  if num < 1_f64 {
    return format!("{}{} {}", negative, num, "B");
  }
  let delimiter = 1000_f64;
  let exponent = std::cmp::min((num.ln() / delimiter.ln()).floor() as i32, (units.len() - 1) as i32);
  let pretty_bytes = format!("{:.2}", num / delimiter.powi(exponent)).parse::<f64>().unwrap_or_else(|error| {
    error!("{}:{}:{} has encountered an parsing issue: {}", module_path!(),file!(),line!(), error);
    panic!("{}:{}:{} has encountered an parsing issue: {}", module_path!(),file!(),line!(), error)
  }) * 1_f64;
  let unit = units[exponent as usize];
  format!("{}{} {}", negative, pretty_bytes, unit)
}