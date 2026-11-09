use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::error::LayoutError;

/// How the panes of a window are arranged.
///
/// The vocabulary follows the editor convention the layout is written for:
/// `vertical` places panes side by side (NeoVim left, dev server right) and
/// `horizontal` stacks them (terminal below). The tmux adapter translates
/// `vertical` into `split-window -h` and `horizontal` into `split-window -v`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Split {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pane {
    #[serde(default)]
    pub cmd: String,
}

impl Pane {
    pub fn new(cmd: impl Into<String>) -> Self {
        Self { cmd: cmd.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<Split>,
    #[serde(default)]
    pub panes: Vec<Pane>,
}

impl Window {
    pub fn new(name: impl Into<String>, split: Option<Split>, panes: Vec<Pane>) -> Self {
        Self {
            name: name.into(),
            split,
            panes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub windows: Vec<Window>,
}

impl Layout {
    pub fn new(windows: Vec<Window>) -> Self {
        Self { windows }
    }

    /// Reject layouts that could not be materialized faithfully.
    pub fn validate(&self, name: &str) -> Result<(), LayoutError> {
        if name.trim().is_empty() {
            return Err(LayoutError::EmptyName);
        }
        if name.chars().any(char::is_whitespace) {
            return Err(LayoutError::WhitespaceInName(name.to_string()));
        }
        if self.windows.is_empty() {
            return Err(LayoutError::NoWindows(name.to_string()));
        }
        for window in &self.windows {
            if window.name.trim().is_empty() {
                return Err(LayoutError::EmptyWindowName(name.to_string()));
            }
            if window.split.is_some() && window.panes.len() < 2 {
                return Err(LayoutError::SplitWithOnePane {
                    layout: name.to_string(),
                    window: window.name.clone(),
                });
            }
        }
        Ok(())
    }
}

pub type Layouts = BTreeMap<String, Layout>;

#[cfg(test)]
mod tests {
    use super::*;

    fn dev_layout() -> Layout {
        Layout::new(vec![
            Window::new("editor", None, vec![Pane::new("nvim .")]),
            Window::new(
                "run",
                Some(Split::Vertical),
                vec![Pane::new(""), Pane::new("")],
            ),
        ])
    }

    #[test]
    fn a_layout_with_named_windows_and_panes_is_valid() {
        assert_eq!(dev_layout().validate("dev"), Ok(()));
    }

    #[test]
    fn a_layout_without_windows_is_rejected() {
        let layout = Layout::new(Vec::new());
        assert_eq!(
            layout.validate("dev"),
            Err(LayoutError::NoWindows("dev".to_string()))
        );
    }

    #[test]
    fn a_split_needs_at_least_two_panes() {
        let layout = Layout::new(vec![Window::new(
            "run",
            Some(Split::Horizontal),
            vec![Pane::new("")],
        )]);
        assert!(matches!(
            layout.validate("dev"),
            Err(LayoutError::SplitWithOnePane { .. })
        ));
    }

    #[test]
    fn a_layout_name_with_whitespace_is_rejected() {
        assert!(matches!(
            dev_layout().validate("my dev"),
            Err(LayoutError::WhitespaceInName(_))
        ));
    }

    #[test]
    fn a_window_may_be_declared_without_panes() {
        let layout = Layout::new(vec![Window::new("shell", None, Vec::new())]);
        assert_eq!(layout.validate("scratch"), Ok(()));
    }
}
