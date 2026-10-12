use crate::tab::Tab;

pub struct TabManager {
    selection: usize,
    tabs: Vec<Tab>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            selection: 0,
            tabs: vec![],
        }
    }

    pub fn get_tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn get_tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn get_selection(&self) -> usize {
        self.selection
    }

    pub fn add_tab(&mut self, title: &str) {
        let tab = Tab::new(self.get_tab_count(), title);
        self.tabs.push(tab);
    }

    pub fn next_tab(&mut self) {
        let total = self.get_tab_count();
        if total == 0 {
            return;
        }
        self.selection = (self.selection + 1) % total;
    }

    pub fn prev_tab(&mut self) {
        let total = self.get_tab_count();
        if total == 0 {
            return;
        }
        if self.selection == 0 {
            self.selection = total - 1;
        } else {
            self.selection -= 1;
        }
    }
}
