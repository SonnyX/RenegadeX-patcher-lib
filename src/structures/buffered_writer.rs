pub struct BufWriter<W: std::io::Write, F: FnMut(&mut W, &mut u64)> {
    pub inner: Option<W>,
    pub buf: Vec<u8>,
    pub written: u64,
    pub panicked: bool,
    pub callback: F,
}