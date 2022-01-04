#[derive(Clone, Debug)]
pub struct FilePart {
    pub file: String,
    pub part_byte: u64,
    pub from: u64,
    pub to: u64,
}

impl FilePart {
    pub fn new(file: String, part_byte: u64, from: u64, to: u64) -> Self {
        Self {
            file,
            part_byte,
            from,
            to,
        }
    }
}