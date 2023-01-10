use tracing::error;

/// Convert a raw bytesize into a human readable string, e.g. 4_248_578 returns 4.25 MB
pub fn human_readable_bytesize(num: i64) -> String {
  let negative = if num.is_positive() { "" } else { "-" };
  let num = num.abs() as f64;
  if num < 1000.0 {
    return format!("{}{:.2} {}", negative, num, "B");
  }
  const UNITS : [&str; 9] = ["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
  const DELIMITER : f64 = 1000_f64;

  let exponent = std::cmp::min((num.ln() / DELIMITER.ln()).floor() as i32, (UNITS.len() - 1) as i32);
  let pretty_bytes = format!("{:.2}", num / DELIMITER.powi(exponent)).parse::<f64>().unwrap_or_else(|error| {
    error!("{}:{}:{} has encountered an parsing issue: {}", module_path!(),file!(),line!(), error);
    panic!("{}:{}:{} has encountered an parsing issue: {}", module_path!(),file!(),line!(), error)
  });
  let unit = UNITS[exponent as usize];
  format!("{}{} {}", negative, pretty_bytes, unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(human_readable_bytesize(4_248_578_547), "4.25 GB");
    }
}