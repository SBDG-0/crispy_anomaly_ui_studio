# Anomaly UI Studio — version 0.6

A local Rust/egui XML layout editor for S.T.A.L.K.E.R. Anomaly. This first version focuses on opening real files, visually editing rectangles, and exporting minimal source-preserving changes.

## Start

Download latest version from [releases](https://github.com/SBDG-0/crispy_anomaly_ui_studio/releases).

Unzip and open `Anomaly UI Studio v0.6.exe`. No installation or Rust runtime is required.

The known unpacked Anomaly folder is prefilled and scanned automatically if available. Otherwise browse to the folder containing `configs`, `scripts`, and `textures`, then click **Scan project**.

1. Search the XML filename list, then choose `maingame_16.xml`, `ui_mm_main_16.xml`, or `ui_options_16.xml`. The list shows approximately ten rows and scrolls independently. HUD/Main menu/Options shortcuts remain available.
2. Select an element in the list or on the canvas.
3. Drag to move it; drag its gold bottom-right handle to resize. Hold Ctrl/Strg to lock the drag to its initial dominant X or Y axis, Shift for ten-times-finer movement, and Alt while resizing to preserve proportions. Numeric properties are also editable.
4. Use Undo/Redo or Ctrl+Z/Ctrl+Y. Toggle **XML changes** to review changed attributes.
5. Click **Export mod…** and choose a separate output mod folder. The editor creates `gamedata/configs/ui/<original filename>` there. Existing output files require confirmation.

Source files are never saved in place. Export retains the current editing session; use **Discard edits** after exporting to switch files. Preview visibility does not change XML.

## Mod layers

Expand **Layers** above the file list to manage sources. The panel accepts a mod folder or its gamedata folder. Each mod has an enable checkbox, **Up / lower**, **Down / higher**, and **Remove** controls. Higher priority is lower in the list: the last enabled layer wins. Base Anomaly also has an enable checkbox; disabling it removes all base XML and texture fallbacks. Files missing from a mod fall back to earlier enabled layers. Changing the order or enabling a mod reloads the current XML from its winning source and rebuilds the texture index. Pending image loads from the old order are ignored.

Export and discard pending edits before changing layers; the controls are disabled while edits are pending. Layer paths, enabled states, priority, editor font size and atlas height are remembered in `studio-layers.txt` next to the executable. Older settings files remain compatible. This file contains local folder paths and is not required when sharing the application. This is an explicit editor layer order, not an automatic import of MO2 priority. DART's runtime transformations are not applied.

## Zoom, output resolution and editor font size

Choose a target resolution in the preview preset selector: 1920x1080, 2560x1440, 3840x2160, 2560x1080 or 3440x1440. At **1× pixels**, one output pixel equals one physical monitor pixel, accounting for Windows DPI scaling. The canvas can extend beyond the viewport; hold the middle mouse button and drag to pan. The ordinary mouse wheel also pans the canvas. XML still uses its 1024x768 virtual coordinate system.

**Ctrl+mouse wheel** zooms around the pointer while hovering over the canvas. This affects the drawing only, not application font size or XML. Middle-button dragging pans freely and does not move selected elements. Red horizontal **X locked** or blue vertical **Y locked** guides identify the active Ctrl/Strg drag constraint.

**Fit view** calculates a zoom once to fit the currently available canvas area. Resizing the window or atlas pane afterwards does not change that zoom or the size of the drawing. Click Fit view again when you want to refit. Aspect/output presets do not rewrite or convert the XML.

The **Font** numeric input at the bottom-right adjusts application text and control sizes from 10 to 24. Drag the value or type a size directly. It does not change game fonts, exported XML or canvas zoom.

Use the **HUD**, **Main menu** and **Options** shortcuts on the zoom toolbar to open the matching screen. They prefer the actual _16 or _21 file for the selected preview shape and fall back to _16 if necessary.

## XML structure and atlas inspection

The XML structure panel uses the same framed, full-width scroll area as the XML file browser. It follows the real XML hierarchy, including containers without geometry, textures and text nodes. Expand/collapse individual branches or use the global controls. Searching retains matching nodes and their ancestors.

Select a textured element or a texture node to see its full atlas with a gold region outline and a region preview below the canvas. Roll the mouse wheel over the atlas to zoom; the highlighted region becomes the focal center. Hold the middle mouse button to pan. **Fit atlas** resets this view. Selecting a different texture region resets the atlas camera. The panel shows the texture ID, image path, descriptor path, pixel coordinates and dimensions. The selected-region pane has a **1:1 pixels** checkbox: off fits the crop to available space, on shows one source pixel per physical display pixel with DPI correction. Large native crops are clipped to the pane and can be inspected by middle-button panning. The selected-region crop has a gold bounding outline in both fitted and 1:1 modes. Transparent areas use a checkerboard. The file source above the layers panel shows which mod supplied the open XML.

Drag the thin horizontal divider above the atlas to resize its pane; it uses the same line styling as the side-panel dividers. The canvas scale remains fixed when the atlas is resized. Atlas and selected-region images fit the actual available pane space, preserving their proportions, and grow with the pane until limited by its width.

## Texture diagnostics

Not every XML element has artwork of its own. Containers can render children; text uses fonts; templates can receive textures at runtime. For example, base Anomaly actor_status.script assigns indicator images to initially untextured ui_hud.xml templates with InitTexture.

The inspector distinguishes no declared texture, empty placeholders, unresolved IDs, and descriptors whose image is absent from the enabled sources. Missing resolution is not automatically classified as scripted. The default _e state of a button or _back piece of a frame may be shown as a clearly labeled partial preview. Complete state/composite rendering and arbitrary runtime texture assignments still require engine/script-specific support.

## What works

- Native GPU-rendered canvas, zoom, grid, bounds, selection and movement/resizing.
- Background texture indexing and decoding, cached DDS atlas previews, texture crop rectangles and tint. Uncompressed RGB/RGBA/BGRA, luminance and alpha DDS formats are supported alongside BC1–BC7 (including DXT1/3/5) and additional formats handled by the DDS library. Only the base mip is previewed. Unsupported or malformed formats report their specific decoding error and path.
- Base and mod-layer XML browsing; 16:9 and ultrawide output resolutions with fixed pixel zoom.
- Existing x/y/width/height attribute editing and undo/redo.
- XML export preserving comments, includes, whitespace, attribute order and original UTF-8/Windows-1251 encoding.
- Sample progress fills and placeholder text. Select a progress bar to access **Sample fill (preview only)** in Properties. This replaces the old toolbar **Bar value** slider; it only simulates health/stamina fill and never changes XML.

## Preview limits

This is an **XML layout prototype**, not an engine renderer. The output selector changes preview dimensions only. It does not generate or correct a _21 layout; open and edit the actual _21 file separately.

- Includes are preserved but not visually composed. An included background may therefore be absent.
- Runtime scripts, DXML, conditional visibility, engine parenting, special alignment and generated menu buttons/options rows are not reproduced.
- Nested XML coordinates are used as an approximate parent model. Some engine consumers attach those elements differently.
- All eligible rectangles are shown by default, including templates and elements the game may hide. Use **Visible in preview** to hide individual items temporarily.
- Text uses an editor font, and displayed strings may still be localization keys. Button state textures, font metrics and animation are not faithful game previews.
- DDS formats unsupported by the image decoder use placeholders. No texture artwork is modified or exported.
- Editing is limited to existing geometry attributes. Creating/deleting elements, saving projects, paired-layout synchronization and script editing are future work.

Proportional resizing uses a continuous screen-space calculation to avoid jumps when pointer directions cross. Toggling Alt during a resize rebases the operation to the current size, preserving the ratio present when the lock is engaged.

Always verify exported layouts in-game. Keep outputs in a dedicated MO2 mod rather than replacing unpacked reference files.

## Build

Run `./build.ps1 -Test -OutputName "Anomaly UI Studio v0.6.exe"` from PowerShell. The project-local toolchain is used when present; otherwise a normal Rust installation with a working Windows linker is required. Dependencies are pinned in Cargo.lock. The Windows resource compiler (windres for GNU builds or rc.exe for MSVC) embeds assets/app.ico into the executable; the same bundled icon is used for the window/taskbar.

Tests cover exact text preservation, duplicate nodes, includes, encoding, malformed nesting, DDS channel masks, row padding, alpha/luminance, truncated data, BC1–BC7 decoding, descriptor/image priority, optional base fallback, settings compatibility, pixel zoom/DPI, explicit fit, axis locking, fine movement, proportional resizing, rendered axis guides, pointer-anchored wheel zoom, middle-button panning without XML edits, atlas sizing, selection-focused atlas navigation, native-size region previews, texture diagnostics and proportional-resize crossover/modifier regressions. `anomaly-ui-editor.exe --validate-game` checks byte-for-byte unchanged export of the configured base HUD and menu files. It does not launch the GUI or modify game data.

`anomaly-ui-editor.exe --check-mod-assets <folder> ...` decodes DDS files recursively and writes `asset-validation.txt` in the current directory. This is a diagnostic mode and does not modify source textures.
