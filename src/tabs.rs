//! The tab bar: which tabs exist and which one is currently selected.

/// Holds the tab titles and the index of the selected one.
pub struct TabManager {
    selection: usize,
    titles: Vec<String>,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            selection: 0,
            titles: vec![],
        }
    }

    pub fn titles(&self) -> &[String] {
        &self.titles
    }

    pub fn get_selection(&self) -> usize {
        self.selection
    }

    pub fn add_tab(&mut self, title: &str) {
        self.titles.push(title.to_string());
    }

    /// Select the next tab, wrapping around to the first.
    pub fn next_tab(&mut self) {
        if !self.titles.is_empty() {
            self.selection = (self.selection + 1) % self.titles.len();
        }
    }

    /// Select the previous tab, wrapping around to the last.
    pub fn prev_tab(&mut self) {
        if !self.titles.is_empty() {
            if self.selection == 0 {
                self.selection = self.titles.len() - 1;
            } else {
                self.selection -= 1;
            }
        }
    }

    /// Select the tab at `index`, ignoring out-of-range values.
    pub fn select(&mut self, index: usize) {
        if index < self.titles.len() {
            self.selection = index;
        }
    }
}
