use std::{io, iter::repeat, ops::Rem};

use crate::theme::{SimpleTheme, TermThemeRenderer, Theme};

use console::{Key, Term};

/// Renders a multi select prompt.
///
/// ## Example usage
/// ```rust,no_run
/// # fn test() -> Result<(), Box<dyn std::error::Error>> {
/// use dialoguer::MultiSelect;
///
/// let items = vec!["Option 1", "Option 2"];
/// let chosen : Vec<usize> = MultiSelect::new()
///     .items(&items)
///     .interact()?;
/// # Ok(())
/// # }
/// ```
pub struct MultiSelect<'a> {
    defaults: Vec<bool>,
    items: Vec<String>,
    prompt: Option<String>,
    clear: bool,
    theme: &'a dyn Theme,
    paged: bool,
    page_size: u32,
}

impl<'a> Default for MultiSelect<'a> {
    fn default() -> MultiSelect<'a> {
        MultiSelect::new()
    }
}

impl<'a> MultiSelect<'a> {
    /// Creates a multi select prompt.
    pub fn new() -> MultiSelect<'static> {
        MultiSelect::with_theme(&SimpleTheme)
    }

    /// Creates a multi select prompt with a specific theme.
    pub fn with_theme(theme: &'a dyn Theme) -> MultiSelect<'a> {
        MultiSelect {
            items: vec![],
            defaults: vec![],
            clear: true,
            prompt: None,
            theme,
            paged: false,
            page_size: 10,
        }
    }

    /// Enables or disables paging
    pub fn paged(&mut self, val: bool) -> &mut MultiSelect<'a> {
        self.paged = val;
        self
    }

    /// Declares the page size for the element
    pub fn page_size(&mut self, val: u32) -> &mut MultiSelect<'a> {
        self.page_size = if val <= 0 { 10 } else { val };
        self
    }

    /// Sets the clear behavior of the menu.
    ///
    /// The default is to clear the menu.
    pub fn clear(&mut self, val: bool) -> &mut MultiSelect<'a> {
        self.clear = val;
        self
    }

    /// Sets a defaults for the menu.
    pub fn defaults(&mut self, val: &[bool]) -> &mut MultiSelect<'a> {
        self.defaults = val
            .to_vec()
            .iter()
            .cloned()
            .chain(repeat(false))
            .take(self.items.len())
            .collect();
        self
    }

    /// Add a single item to the selector.
    #[inline]
    pub fn item<T: ToString>(&mut self, item: T) -> &mut MultiSelect<'a> {
        self.item_checked(item, false)
    }

    /// Add a single item to the selector with a default checked state.
    pub fn item_checked<T: ToString>(&mut self, item: T, checked: bool) -> &mut MultiSelect<'a> {
        self.items.push(item.to_string());
        self.defaults.push(checked);
        self
    }

    /// Adds multiple items to the selector.
    pub fn items<T: ToString>(&mut self, items: &[T]) -> &mut MultiSelect<'a> {
        for item in items {
            self.items.push(item.to_string());
            self.defaults.push(false);
        }
        self
    }

    /// Adds multiple items to the selector with checked state
    pub fn items_checked<T: ToString>(&mut self, items: &[(T, bool)]) -> &mut MultiSelect<'a> {
        for &(ref item, checked) in items {
            self.items.push(item.to_string());
            self.defaults.push(checked);
        }
        self
    }

    /// Prefaces the menu with a prompt.
    ///
    /// When a prompt is set the system also prints out a confirmation after
    /// the selection.
    pub fn with_prompt<S: Into<String>>(&mut self, prompt: S) -> &mut MultiSelect<'a> {
        self.prompt = Some(prompt.into());
        self
    }

    /// Enables user interaction and returns the result.
    ///
    /// The user can select the items with the space bar and on enter
    /// the selected items will be returned.
    pub fn interact(&self) -> io::Result<Vec<usize>> {
        self.interact_on(&Term::stderr())
    }

    /// Like [interact](#method.interact) but allows a specific terminal to be set.
    pub fn interact_on(&self, term: &Term) -> io::Result<Vec<usize>> {
        let mut page = 0;

        if self.items.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Empty list of items given to `MultiSelect`",
            ));
        }

        let capacity = if self.paged {
            if self.page_size > 0 {
                self.page_size as usize
            } else {
                10 as usize
            }
        } else {
            self.items.len()
        };

        let pages = (self.items.len() as f64 / capacity as f64).ceil() as usize;

        let mut render = TermThemeRenderer::new(term, self.theme);
        let mut sel = 0;
        let mut prompt_string: String = String::from("");

        if let Some(ref prompt) = self.prompt {
            prompt_string = String::from(prompt);
            // render.multi_select_prompt(prompt)?;
        }

        let mut size_vec = Vec::new();

        for items in self
            .items
            .iter()
            .flat_map(|i| i.split('\n'))
            .collect::<Vec<_>>()
        {
            let size = &items.len();
            size_vec.push(*size);
        }

        let mut checked: Vec<bool> = self.defaults.clone();
        let mut search_string: String = String::from("");
        let original_items = self.items.clone();

        loop {
            let render_prompt_str = format!("{} {}", prompt_string, search_string);
            render.clear()?;
            render.multi_select_prompt(&render_prompt_str)?;
            let filtered_indexed_items: Vec<_> = original_items
                .iter()
                .enumerate()
                .filter(|&(_, item)| {
                    search_string.len() == 0
                        || item.to_lowercase().contains(&search_string.to_lowercase())
                })
                .map(|(idx, item)| (item, idx))
                .collect();

            let filtered_items: Vec<_> = filtered_indexed_items
                .iter()
                .map(|(item, _)| item)
                .collect();

            for (idx, item) in filtered_items
                .iter()
                .enumerate()
                .skip(page * capacity)
                .take(capacity)
            {
                // Render the prompt and selected text if it exists
                let (_, orig_idx) = filtered_indexed_items[idx];
                render.multi_select_prompt_item(item, checked[orig_idx], sel == idx)?;
            }

            term.hide_cursor()?;
            term.flush()?;

            match term.read_key()? {
                Key::ArrowDown => {
                    if sel == !0 {
                        sel = 0;
                    } else {
                        sel = (sel as u64 + 1).rem(filtered_items.len() as u64) as usize;
                    }
                }
                Key::ArrowUp => {
                    if sel == !0 {
                        sel = filtered_items.len() - 1;
                    } else {
                        sel = ((sel as i64 - 1 + filtered_items.len() as i64)
                            % (filtered_items.len() as i64)) as usize;
                    }
                }
                Key::ArrowLeft => {
                    if self.paged {
                        if page == 0 {
                            page = pages - 1;
                        } else {
                            page -= 1;
                        }

                        sel = page * capacity;
                    }
                }
                Key::ArrowRight => {
                    if self.paged {
                        if page == pages - 1 {
                            page = 0;
                        } else {
                            page += 1;
                        }

                        sel = page * capacity;
                    }
                }
                Key::Char(' ') => {
                    // TODO: Fetch the original index from the items list
                    // and add update the checked array entries
                    let (_, orig_idx) = filtered_indexed_items[sel];
                    checked[orig_idx] = !checked[orig_idx];
                }
                Key::Escape => {
                    if self.clear {
                        render.clear()?;
                    }

                    if let Some(ref prompt) = self.prompt {
                        render.multi_select_prompt_selection(prompt, &[][..])?;
                    }

                    term.show_cursor()?;
                    term.flush()?;

                    return Ok(self
                        .defaults
                        .clone()
                        .into_iter()
                        .enumerate()
                        .filter_map(|(idx, checked)| if checked { Some(idx) } else { None })
                        .collect());
                }
                Key::Enter => {
                    if self.clear {
                        render.clear()?;
                    }

                    if let Some(ref prompt) = self.prompt {
                        let selections: Vec<_> = checked
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, &checked)| {
                                if checked {
                                    Some(self.items[idx].as_str())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        render.multi_select_prompt_selection(prompt, &selections[..])?;
                    }

                    term.show_cursor()?;
                    term.flush()?;

                    return Ok(checked
                        .into_iter()
                        .enumerate()
                        .filter_map(|(idx, checked)| if checked { Some(idx) } else { None })
                        .collect());
                }
                Key::Char(x) => {
                    search_string.push(x);
                }
                Key::Backspace => {
                    if search_string.len() > 0 {
                        search_string.pop();
                    }
                }
                _ => {}
            }

            if sel < page * capacity || sel >= (page + 1) * capacity {
                page = sel / capacity;
            }

            render.clear_preserve_prompt(&size_vec)?;
        }
    }
}
