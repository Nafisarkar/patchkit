pub struct Tab {
    pub tabnumber: usize,
    pub title: String,
}

impl Tab {
    pub fn new(tabnumber: usize, title: &str) -> Self {
        Self {
            tabnumber,
            title: title.to_string(),
        }
    }
}
