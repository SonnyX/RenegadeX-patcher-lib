pub struct FilePart {
    pub file: String,
    pub part_num: usize,
    pub from: usize,
    pub to: usize,
}

impl FilePart {
    pub fn new(file: String, part_num: usize, from: usize, to: usize) -> Self {
        Self {
            file,
            part_num,
            from,
            to,
        }
    }
}