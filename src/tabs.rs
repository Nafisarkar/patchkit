//! The tab bar: which tabs exist and which one is currently selected.

/// Holds the tab titles and the index of the selected one.
pub struct TabManager {
    selection: usize,
    titles: Vec<String>,
}

impl TabManager {
    /// Create a manager with no tabs, selecting index 0.
    pub fn new() -> Self {
        Self {
            selection: 0,
            titles: vec![],
        }
    }

    /// All tab titles, in display order.
    pub fn titles(&self) -> &[String] {
        &self.titles
    }

    /// Index of the currently selected tab.
    pub fn get_selection(&self) -> usize {
        self.selection
    }

    /// Append a tab with the given title.
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
}
