#[derive(Clone)]
pub struct FilePart {
    pub file: String,
    pub part_num: usize,
    pub from: u64,
    pub to: u64,
}

impl FilePart {
    pub fn new(file: String, part_num: usize, from: u64, to: u64) -> Self {
        Self {
            file,
            part_num,
            from,
            to,
        }
    }
}