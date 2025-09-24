# TODO: Complete Parley Interface Documentation and Replacement

## Phase 1: Document Parley Interfaces

- [ ] **Document TreeBuilder Interface Contract** `/Volumes/samsung_t9/zeroshot/forks/blitz/packages/blitz-dom/src/layout/construct.rs:544-929`
  - Document tree_builder() factory method signature and parameters
  - Document push_text(str) - adds text content to current span
  - Document push_style_span(TextStyle) - starts new styled text span  
  - Document pop_style_span() - ends current styled span
  - Document push_inline_box(InlineBox) - adds replaced element placeholder
  - Document set_white_space_mode(WhiteSpaceCollapse) - controls text wrapping
  - Document push_style_modification_span(&[]) - temporary style override
  - Document build() -> (Layout, String) return values and their structure
  - Document how Layout contains inline_boxes() iterator
  - Document how Layout measures text with calculate_content_widths()
  - Document how Layout breaks lines with break_all_lines()

- [ ] **Document FontContext and LayoutContext Interfaces** `/Volumes/samsung_t9/zeroshot/forks/blitz/packages/blitz-dom/src/document.rs`
  - Document FontContext methods used for font loading and management
  - Document LayoutContext creation and initialization
  - Document LayoutContext.tree_builder() factory method
  - Document how contexts interact with each other
  - Document font registration with load_font_data()

- [ ] **Document PlainEditor Interface** `/Volumes/samsung_t9/zeroshot/forks/blitz/packages/blitz-dom/src/node/element.rs:329-358`
  - Document PlainEditor::new(font_size) constructor
  - Document set_text(), set_scale(), set_width() methods
  - Document edit_styles() -> StyleBuilder pattern
  - Document refresh_layout() method signature
  - Document text() and raw_text() accessors
  - Document selected_text() for getting selection
  - Document driver() method returning PlainEditorDriver

- [ ] **Document PlainEditorDriver Interface** `/Volumes/samsung_t9/zeroshot/forks/blitz/packages/blitz-dom/src/events/keyboard.rs:92-213`
  - Document all cursor movement methods
  - Document selection methods
  - Document text modification methods
  - Document IME methods

## Phase 2: Document Type Definitions and Data Structures

- [ ] **Document TextStyle Structure**
- [ ] **Document WhiteSpaceCollapse Enum**
- [ ] **Document InlineBox Structure**
- [ ] **Document TextLayout Structure**

## Phase 3: Document Style Conversion Pipeline

- [ ] **Document stylo_to_parley::style() Conversion**
- [ ] **Document FontStack Type**

## Phase 4: Document Text Rendering Pipeline

- [ ] **Document parley::Layout to Rendering Path**
- [ ] **Document Glyph Positioning and Metrics**

## Phase 5: Create Cosmic-Text Interface Mapping

- [ ] **Map TreeBuilder to cosmic-text Buffer Builder**
- [ ] **Map PlainEditor to cosmic_text::Editor**
- [ ] **Map Style Properties to cosmic_text::Attrs**