//! Cell and grid management for zero-allocation terminal rendering

/// Style flags for cell styling
pub mod style {
    pub const BOLD: u8 = 1 << 0;
    pub const ITALIC: u8 = 1 << 1;
    pub const UNDERLINE: u8 = 1 << 2;
    pub const DIM: u8 = 1 << 3;
    pub const INVERSE: u8 = 1 << 4;
    pub const STRIKETHROUGH: u8 = 1 << 5;
}

/// Cell data representing a single terminal character
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub character: char,
    pub foreground: u8,  // Index into color palette
    pub background: u8,  // Index into color palette
    pub style_flags: u8, // Bold, italic, underline, etc.
}

impl Default for Cell {
    #[inline]
    fn default() -> Self {
        Self {
            character: ' ',
            foreground: 7, // Default white
            background: 0, // Default black
            style_flags: 0,
        }
    }
}

impl Cell {
    /// Create a new cell with the given character
    #[inline]
    pub const fn new(character: char) -> Self {
        Self {
            character,
            foreground: 7,
            background: 0,
            style_flags: 0,
        }
    }

    /// Create a styled cell
    #[inline]
    pub const fn styled(character: char, foreground: u8, background: u8, style_flags: u8) -> Self {
        Self {
            character,
            foreground,
            background,
            style_flags,
        }
    }

    /// Check if cell has a specific style flag
    #[inline]
    pub const fn has_style(&self, flag: u8) -> bool {
        (self.style_flags & flag) != 0
    }

    /// Add a style flag
    #[inline]
    pub fn add_style(&mut self, flag: u8) {
        self.style_flags |= flag;
    }

    /// Remove a style flag
    #[inline]
    pub fn remove_style(&mut self, flag: u8) {
        self.style_flags &= !flag;
    }

    /// Toggle a style flag
    #[inline]
    pub fn toggle_style(&mut self, flag: u8) {
        self.style_flags ^= flag;
    }

    /// Check if cell is empty (space with default colors)
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.character == ' '
            && self.foreground == 7
            && self.background == 0
            && self.style_flags == 0
    }
}

/// Fixed-size terminal grid with zero-allocation updates
#[repr(align(64))] // Cache line alignment for better performance
pub struct CellGrid<const COLS: usize, const ROWS: usize> {
    cells: [[Cell; COLS]; ROWS],
    dirty_rows: [bool; ROWS],
    change_count: u64,
}

impl<const COLS: usize, const ROWS: usize> CellGrid<COLS, ROWS> {
    /// Create a new empty grid
    #[inline]
    pub const fn new() -> Self {
        let empty_cell = Cell {
            character: ' ',
            foreground: 7,
            background: 0,
            style_flags: 0,
        };

        Self {
            cells: [[empty_cell; COLS]; ROWS],
            dirty_rows: [false; ROWS],
            change_count: 0,
        }
    }

    /// Set a cell at the given position
    #[inline]
    pub fn set_cell(&mut self, col: usize, row: usize, cell: Cell) {
        if col < COLS && row < ROWS {
            self.cells[row][col] = cell;
            self.dirty_rows[row] = true;
            self.change_count = self.change_count.wrapping_add(1);
        }
    }

    /// Get a cell at the given position
    #[inline]
    pub fn get_cell(&self, col: usize, row: usize) -> Option<Cell> {
        if col < COLS && row < ROWS {
            Some(self.cells[row][col])
        } else {
            None
        }
    }

    /// Get a mutable reference to a cell
    #[inline]
    pub fn get_cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        if col < COLS && row < ROWS {
            self.dirty_rows[row] = true;
            self.change_count = self.change_count.wrapping_add(1);
            Some(&mut self.cells[row][col])
        } else {
            None
        }
    }

    /// Get an entire row
    #[inline]
    pub fn get_row(&self, row: usize) -> Option<&[Cell; COLS]> {
        if row < ROWS {
            Some(&self.cells[row])
        } else {
            None
        }
    }

    /// Get a mutable row (marks it as dirty)
    #[inline]
    pub fn get_row_mut(&mut self, row: usize) -> Option<&mut [Cell; COLS]> {
        if row < ROWS {
            self.dirty_rows[row] = true;
            self.change_count = self.change_count.wrapping_add(1);
            Some(&mut self.cells[row])
        } else {
            None
        }
    }

    /// Check if a row is dirty
    #[inline]
    pub fn is_row_dirty(&self, row: usize) -> bool {
        row < ROWS && self.dirty_rows[row]
    }

    /// Mark a row as dirty
    #[inline]
    pub fn mark_row_dirty(&mut self, row: usize) {
        if row < ROWS {
            self.dirty_rows[row] = true;
            self.change_count = self.change_count.wrapping_add(1);
        }
    }

    /// Clear dirty flags for all rows
    #[inline]
    pub fn clear_dirty_flags(&mut self) {
        self.dirty_rows = [false; ROWS];
    }

    /// Clear the entire grid
    #[inline]
    pub fn clear(&mut self) {
        for row in 0..ROWS {
            for col in 0..COLS {
                self.cells[row][col] = Cell::default();
            }
            self.dirty_rows[row] = true;
        }
        self.change_count = self.change_count.wrapping_add(1);
    }

    /// Fill the grid with a specific cell
    #[inline]
    pub fn fill(&mut self, cell: Cell) {
        for row in 0..ROWS {
            for col in 0..COLS {
                self.cells[row][col] = cell;
            }
            self.dirty_rows[row] = true;
        }
        self.change_count = self.change_count.wrapping_add(1);
    }

    /// Get the change count for detecting modifications
    #[inline]
    pub fn change_count(&self) -> u64 {
        self.change_count
    }

    /// Get dimensions
    #[inline]
    pub const fn dimensions(&self) -> (usize, usize) {
        (COLS, ROWS)
    }

    /// Get number of dirty rows
    #[inline]
    pub fn dirty_row_count(&self) -> usize {
        self.dirty_rows.iter().filter(|&&dirty| dirty).count()
    }

    /// Iterate over dirty rows with their indices
    pub fn dirty_rows_iter(&self) -> impl Iterator<Item = (usize, &[Cell; COLS])> + '_ {
        self.dirty_rows
            .iter()
            .enumerate()
            .filter_map(|(idx, &dirty)| {
                if dirty {
                    Some((idx, &self.cells[idx]))
                } else {
                    None
                }
            })
    }

    /// Clear a specific row
    #[inline]
    pub fn clear_row(&mut self, row: usize) {
        if row < ROWS {
            for col in 0..COLS {
                self.cells[row][col] = Cell::default();
            }
            self.dirty_rows[row] = true;
            self.change_count = self.change_count.wrapping_add(1);
        }
    }

    /// Copy a row from source to destination
    #[inline]
    pub fn copy_row(&mut self, src_row: usize, dst_row: usize) {
        if src_row < ROWS && dst_row < ROWS && src_row != dst_row {
            self.cells[dst_row] = self.cells[src_row];
            self.dirty_rows[dst_row] = true;
            self.change_count = self.change_count.wrapping_add(1);
        }
    }

    /// Scroll the grid up by n rows
    pub fn scroll_up(&mut self, n: usize) {
        if n == 0 || n >= ROWS {
            return;
        }

        // Move rows up
        for row in 0..(ROWS - n) {
            self.cells[row] = self.cells[row + n];
        }

        // Clear bottom rows
        for row in (ROWS - n)..ROWS {
            for col in 0..COLS {
                self.cells[row][col] = Cell::default();
            }
        }

        // Mark all as dirty
        self.dirty_rows = [true; ROWS];
        self.change_count = self.change_count.wrapping_add(1);
    }

    /// Scroll the grid down by n rows
    pub fn scroll_down(&mut self, n: usize) {
        if n == 0 || n >= ROWS {
            return;
        }

        // Move rows down
        for row in (n..ROWS).rev() {
            self.cells[row] = self.cells[row - n];
        }

        // Clear top rows
        for row in 0..n {
            for col in 0..COLS {
                self.cells[row][col] = Cell::default();
            }
        }

        // Mark all as dirty
        self.dirty_rows = [true; ROWS];
        self.change_count = self.change_count.wrapping_add(1);
    }
}

impl<const COLS: usize, const ROWS: usize> Default for CellGrid<COLS, ROWS> {
    fn default() -> Self {
        Self::new()
    }
}
