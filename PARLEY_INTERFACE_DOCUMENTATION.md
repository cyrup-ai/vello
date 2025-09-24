# Complete Parley Interface Documentation

This document comprehensively documents ALL parley interfaces, types, methods, and data flow that must be replaced with cosmic-text + glyphon.

## 1. Core Context Types

### 1.1 FontContext
**Location**: Used in keyboard.rs:7, element.rs:6
**Purpose**: Font management and loading
**Methods Used**:
- Constructor: Implicitly created (exact constructor unknown)
- Font loading: Used with `load_font_data()` (via document font_system)
- Passed to TreeBuilder factory and PlainEditor operations

### 1.2 LayoutContext<TextBrush>
**Location**: Used in keyboard.rs:7, element.rs:6, construct.rs
**Purpose**: Text layout creation and management
**Methods Used**:
```rust
// Factory method for creating text layout builders
fn tree_builder(
    &mut self,
    font_ctx: &mut FontContext,
    scale: f32,
    is_main_thread: bool,
    style: &TextStyle
) -> TreeBuilder<TextBrush>
```

## 2. Text Layout Builder (TreeBuilder)

### 2.1 TreeBuilder<TextBrush>
**Location**: construct.rs:544, 873, 969
**Purpose**: Builds text layouts with styled spans and inline boxes
**Methods**:

```rust
// Text content methods
fn push_text(&mut self, text: &str)
// Adds raw text to current span

// Style management
fn push_style_span(&mut self, style: TextStyle)
// Starts new styled text region

fn pop_style_span(&mut self)
// Ends current styled region

fn push_style_modification_span(&mut self, modifications: &[])
// Temporary style override (used for <br> tags)

// Whitespace control
fn set_white_space_mode(&mut self, mode: WhiteSpaceCollapse)
// Controls text wrapping and whitespace collapsing

// Inline elements
fn push_inline_box(&mut self, inline_box: InlineBox)
// Adds placeholder for replaced elements (img, canvas, input)

// Build final layout
fn build(self) -> (Layout, String)
// Returns (layout object, full text content)
```

## 3. Text Editor Types

### 3.1 PlainEditor<TextBrush>
**Location**: element.rs:329, 343
**Purpose**: Text editing for input/textarea elements
**Constructor**:
```rust
PlainEditor::new(font_size: f32) -> PlainEditor<TextBrush>
```

**Methods**:
```rust
fn set_text(&mut self, text: &str)
fn set_scale(&mut self, scale: f32)
fn set_width(&mut self, width: Option<f32>)
fn edit_styles(&mut self) -> &mut StyleBuilder
fn refresh_layout(&mut self, font_ctx: &mut FontContext, layout_ctx: &mut LayoutContext)
fn text(&self) -> &str
fn raw_text(&self) -> &str
fn selected_text(&self) -> Option<&str>
fn driver<'a>(&'a mut self, font_ctx: &'a mut FontContext, layout_ctx: &'a mut LayoutContext) 
    -> PlainEditorDriver<'a, TextBrush>
```

### 3.2 PlainEditorDriver<'a, TextBrush>
**Location**: keyboard.rs:92-213
**Purpose**: Handles keyboard/mouse input for text editor
**Methods**:

```rust
// Cursor movement
fn move_left(&mut self)
fn move_right(&mut self)
fn move_up(&mut self)
fn move_down(&mut self)
fn move_word_left(&mut self)
fn move_word_right(&mut self)
fn move_to_line_start(&mut self)
fn move_to_line_end(&mut self)
fn move_to_text_start(&mut self)
fn move_to_text_end(&mut self)
fn move_to_point(&mut self, x: f32, y: f32)

// Selection
fn select_left(&mut self)
fn select_right(&mut self)
fn select_up(&mut self)
fn select_down(&mut self)
fn select_word_left(&mut self)
fn select_word_right(&mut self)
fn select_to_line_start(&mut self)
fn select_to_line_end(&mut self)
fn select_to_text_start(&mut self)
fn select_to_text_end(&mut self)
fn select_all(&mut self)
fn collapse_selection(&mut self)
fn extend_selection_to_point(&mut self, x: f32, y: f32)

// Text modification
fn insert_or_replace_selection(&mut self, text: &str)
fn delete(&mut self)
fn backdelete(&mut self)
fn delete_word(&mut self)
fn backdelete_word(&mut self)
fn delete_selection(&mut self)

// IME support
fn set_compose(&mut self, text: &str, cursor: Option<(usize, usize)>)
fn clear_compose(&mut self)
```

## 4. Style Types

### 4.1 TextStyle
**Source**: stylo_to_parley::style() return type
**Location**: construct.rs:537, 796, 865, 1051
**Fields**:
```rust
struct TextStyle {
    font_stack: FontStack<'static>,  // Font family chain
    font_size: f32,                  // Size in pixels
    line_height: LineHeight,         // Line spacing
    brush: TextBrush,                // Text color
    // Additional properties (exact list unknown)
}
```

### 4.2 FontStack<'static>
**Location**: construct.rs:598
**Purpose**: Font family fallback chain
**Format**: String like "Arial, sans-serif" or "Bullet, monospace, sans-serif"

### 4.3 LineHeight
**Location**: construct.rs:48-52, 1062
**Purpose**: Line spacing specification
**Variants**:
```rust
enum LineHeight {
    FontSizeRelative(f32),  // Multiplier of font size
    Absolute(f32),          // Absolute pixel value
    MetricsRelative(_),     // Not used
}
```

### 4.4 WhiteSpaceCollapse
**Location**: construct.rs:881, 973, 1044
**Purpose**: Text whitespace handling mode
**Variants**:
```rust
enum WhiteSpaceCollapse {
    Collapse,  // Normal HTML whitespace collapsing
    Preserve,  // Preserve all whitespace (like <pre>)
}
```

### 4.5 TextBrush
**Location**: Throughout as generic parameter
**Purpose**: Text color/fill
**Implementation**: Maps to peniko::Brush (color representation)

## 5. Layout Structures

### 5.1 Layout
**Location**: Return from TreeBuilder::build()
**Purpose**: Computed text layout
**Methods**:
```rust
fn inline_boxes(&self) -> impl Iterator<Item = &InlineBox>
fn calculate_content_widths(&mut self) -> ContentWidths { max: f32, min: f32 }
fn break_all_lines(&mut self, max_width: Option<f32>)
fn size(&self) -> (f32, f32)  // (width, height)
fn lines(&self) -> impl Iterator<Item = Line>
```

### 5.2 InlineBox
**Location**: construct.rs:1033, 1109
**Purpose**: Placeholder for replaced elements in text flow
**Fields**:
```rust
struct InlineBox {
    id: u64,        // Node ID (cast from usize)
    index: usize,   // Position in layout (set by push_inline_box)
    width: f32,     // Set during layout
    height: f32,    // Set during layout
    x: f32,         // Position (set by layout engine)
    y: f32,         // Position (set by layout engine)
}
```

### 5.3 TextLayout
**Location**: node/mod.rs (exact definition not shown)
**Purpose**: Container for text layout result
**Fields**:
```rust
struct TextLayout {
    text: String,        // Full text content
    layout: Layout,      // Parley layout object
}
```

### 5.4 Cluster
**Location**: node.rs:10
**Purpose**: Text cluster for hit testing
**Usage**: Used in text selection and cursor positioning

### 5.5 PositionedLayoutItem
**Location**: debug.rs:1, inline.rs:181
**Purpose**: Items in a laid-out line
**Variants**:
```rust
enum PositionedLayoutItem {
    GlyphRun(/* ... */),  // Text run
    InlineBox(InlineBox), // Replaced element
}
```

## 6. Layout Context Methods

### 6.1 AlignmentOptions
**Location**: inline.rs:1
**Purpose**: Text alignment control
**Usage**: Used with layout.align() for text justification

## 7. Style Conversion (stylo_to_parley)

### 7.1 stylo_to_parley::style()
**Purpose**: Convert Stylo computed styles to parley TextStyle
**Signature**:
```rust
fn style(node_id: usize, computed: &ComputedValues) -> TextStyle
```
**Conversions**:
- Font family → FontStack
- Font size (px) → f32
- Font weight → (embedded in FontStack selection)
- Font style → (embedded in FontStack selection)
- Color → TextBrush
- Line height → LineHeight enum
- Text decoration → (unknown handling)

### 7.2 stylo_to_parley::white_space_collapse()
**Purpose**: Convert CSS white-space to WhiteSpaceCollapse
**Signature**:
```rust
fn white_space_collapse(css_value: style::WhiteSpaceCollapse) -> WhiteSpaceCollapse
```

## 8. Data Flow

### 8.1 Text Layout Pipeline
1. **Style Resolution**: Stylo ComputedValues → stylo_to_parley::style() → TextStyle
2. **Builder Creation**: LayoutContext.tree_builder() → TreeBuilder
3. **Content Addition**:
   - Text nodes → builder.push_text()
   - Styled spans → builder.push_style_span() / pop_style_span()
   - Replaced elements → builder.push_inline_box()
4. **Layout Generation**: builder.build() → (Layout, String)
5. **Line Breaking**: layout.break_all_lines(width)
6. **Positioning**: Layout iteration for glyph positions

### 8.2 Text Editor Pipeline
1. **Creation**: PlainEditor::new(font_size)
2. **Configuration**: set_text(), set_scale(), set_width()
3. **Style Setup**: edit_styles() → StyleBuilder
4. **Layout Refresh**: refresh_layout(font_ctx, layout_ctx)
5. **Input Handling**: driver() → PlainEditorDriver → keyboard/mouse events
6. **Text Access**: text(), raw_text(), selected_text()

### 8.3 Rendering Pipeline
1. **Layout Access**: TextLayout.layout → parley::Layout
2. **Line Iteration**: layout.lines() → Line iterator
3. **Item Iteration**: line.items() → PositionedLayoutItem
4. **Glyph Extraction**: GlyphRun processing
5. **GPU Submission**: (Currently broken in vello glyph_builder.rs)

## 9. Critical Integration Points

### 9.1 Font Loading
- Document stores fonts in font_system (now cosmic_text::FontSystem)
- Fonts loaded via load_font_data()
- Bullet font specifically loaded for list markers

### 9.2 Coordinate Systems
- Layout coordinates in logical pixels
- Scale factor applied for device pixels
- Baseline-relative positioning for text
- Inline boxes positioned within text flow

### 9.3 Text Measurement
- Content widths for sizing
- Line breaking for wrapping
- Hit testing for cursor positioning
- Cluster boundaries for selection

## 10. Required Cosmic-Text Mappings

### 10.1 Type Mappings
- FontContext → FontSystem (already done)
- LayoutContext → (custom TextLayoutSystem wrapper)
- TreeBuilder → (custom TextLayoutBuilder wrapping Buffer)
- PlainEditor → cosmic_text::Editor
- PlainEditorDriver → (custom driver using Editor::action())
- TextStyle → cosmic_text::Attrs
- FontStack → cosmic_text::Family
- LineHeight → cosmic_text::Metrics
- WhiteSpaceCollapse → cosmic_text::Wrap
- TextBrush → cosmic_text::Color
- Layout → cosmic_text::Buffer (with layout_runs())
- InlineBox → (custom tracking parallel to Buffer)

### 10.2 Method Mappings
- tree_builder() → TextLayoutBuilder::new()
- push_text() → accumulate text + attrs
- push_style_span() → track Attrs stack
- pop_style_span() → pop Attrs stack
- push_inline_box() → track inline objects
- set_white_space_mode() → set Wrap mode
- build() → Buffer with set_text() + shape_until_scroll()

### 10.3 Missing Functionality
- Inline box integration with cosmic-text Buffer
- Style spans with incremental text building
- Whitespace mode switching mid-layout
- Hit testing with clusters
- Custom glyph rendering for replaced elements

## END OF DOCUMENTATION