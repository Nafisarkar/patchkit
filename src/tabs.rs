//! The tab bar: which tabs exist and which one is currently selected.

/// A single tab, identified by its title.
pub struct Tab {
    pub title: String,
}

impl Tab {
    /// Create a tab with the given title.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
        }
    }
}

/// Holds the list of tabs and the index of the selected one.
pub struct TabManager {
    selection: usize,
    tabs: Vec<Tab>,
}

impl TabManager {
    /// Create a manager with no tabs, selecting index 0.
    pub fn new() -> Self {
        Self {
            selection: 0,
            tabs: vec![],
        }
    }

    /// Number of tabs.
    pub fn get_tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// All tabs, in display order.
    pub fn get_tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Index of the currently selected tab.
    pub fn get_selection(&self) -> usize {
        self.selection
    }

    /// Append a tab with the given title.
    pub fn add_tab(&mut self, title: &str) {
        self.tabs.push(Tab::new(title));
    }

    /// Select the next tab, wrapping around to the first.
    pub fn next_tab(&mut self) {
        let total = self.get_tab_count();
        if total == 0 {
            return;
        }
        self.selection = (self.selection + 1) % total;
    }

    /// Select the previous tab, wrapping around to the last.
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
