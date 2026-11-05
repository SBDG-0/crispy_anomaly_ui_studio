# Editor capability review and proposed next milestones

Inspected 2026-10-27. Current application: Anomaly UI Studio v0.6.

## Evidence inspected

- Current Rust source, including document parsing, patch export, properties, atlas rendering and descriptor indexing.
- Local reference executable: `F:\.other.games\.stalker-anomaly-modding\stalker_tools_plugins\X-Ray_UI_Editor_RC1\X-Ray UI Editor RC1\X-Ray UI Editor RC1.exe`.
- Reference main window and Texture Descriptor Editor. This was a UI inspection, not a save/export or correctness test of that application. No game assets were edited.
- Base Anomaly unpacked XML and video files.
- Reference release description: https://www.moddb.com/downloads/x-ray-ui-editor
- PhotoCraft primary project and control documentation: https://github.com/storytold/photocraft and https://github.com/storytold/photocraft/blob/main/docs/control-protocol.md

## Video feasibility

FFmpeg and ffprobe are already available on this PC. ffprobe identified:

| File under unpacked textures | Container | Video | Dimensions | Duration |
| --- | --- | --- | --- | --- |
| ui/video_crow.ogm | Ogg | Theora | 32 x 32 | 13.36 seconds |
| ui/ui_background_new.ogm | Ogg | Theora | 1920 x 1080 | About 7.04 seconds |

FFmpeg successfully decoded one frame of video_crow.ogm to a null output. This establishes decoder feasibility, not implemented canvas playback.

A suitable implementation would resolve the video through enabled source layers, decode on a worker, retain a bounded queue of recent frames, and upload visible frames to the existing canvas. Add play/pause, looping, seeking and a preview timeline. Keep audio off initially. Preserve original XML references; preview decoding does not require converting the mod's source video.

Do not decode whole videos into a long list of RGBA images. Several full-resolution videos could otherwise use excessive memory and freeze the editor. Start with visible videos only and cancellation when documents/layers change.

Before promising engine-equivalent output, check asset precedence when both a DDS and OGM share a name, transparency/alpha conventions, timing, and any engine-specific crop/scaling rules.

Video and animation are different systems. XML light_anim, xform_anim and sprite-sheet/animated-static settings do not become correct simply by supporting OGM. Each needs its own preview behavior.

## Capability comparison

| Area | Reference editor: observed controls / documented capability | Our v0.6 |
| --- | --- | --- |
| Geometry | Position and sizing | Implemented, plus modifiers, axis guides and output pixel zoom |
| Properties | ID, texture, font, text, colors, mirror/stretch and bar parameters | Existing geometry editable; other attributes currently read-only |
| Descriptors | Create/open descriptors; add/delete regions; draw selection; ID and rectangle fields | Descriptor files are browsable, but no dedicated authoring workflow |
| Diagnostics | Image/ID/header inspection and button-piece checks | DDS decoding errors, path diagnostics and partial state/frame previews |
| Saving | Single/batch XML saving and backup option | One current document's edits exported to a separate mod |
| Video | No video playback control confirmed during inspection | Not implemented; decoder feasibility verified |

## General property editing: recommended design

Provide a complete attribute editor for the selected node, not just a fixed list of familiar attributes. Known fields should get numeric, boolean, color or choice controls; unknown attributes should remain editable strings. Support adding/removing attributes and editing text values. Distinguish an element's attributes from its texture/text child attributes.

Expose texture references, text, font, alignment, tint/opacity, stretch/mirror, animation names and progress parameters. Preserve comments, includes, original encoding and unrelated attributes. Apply XML escaping and validate before accepting/exporting changes. Not every property can have an accurate visual preview immediately; property editing and preview support must be stated separately.

The current patch model assumes existing attribute-value spans. Adding attributes, changing node text and editing multiple documents require a richer edit model and a stable source-preserving serialization strategy before expanding the UI.

## Descriptor editing: recommended design

- A dedicated descriptor mode with atlas selection and region handles.
- Create/edit/delete region definitions and change their IDs, atlas paths and pixel rectangles.
- Search/select a texture ID when editing a layout's texture reference.
- Validate duplicates, missing images, out-of-bounds rectangles and composite button/frame pieces.
- Show references before renaming a shared ID. Script-generated references cannot always be found statically.
- Preview unsaved descriptor edits immediately through an in-memory source overlay.
- Keep separate undo state per document and export all modified layout/descriptor files together.

Layout coordinates and descriptor coordinates must stay clearly distinct: layout x/y places an element on screen; descriptor x/y selects pixels from an atlas. Editing a shared region can affect several elements.

## Canvas ideas worth evaluating

PhotoCraft's documentation describes temporary Hand/Zoom tools, configurable shortcuts, grouped keyboard nudges and view-only navigation. Those patterns are useful references, rather than a reason to copy an entire photo editor's control scheme.

For this tool, useful candidate additions are Space-to-pan, keyboard nudging, an overlap-selection menu, multi-selection, alignment/distribution, edge/center snapping and configurable shortcuts. Preserve the current Ctrl axis lock, Shift fine movement and Alt aspect lock unless the user requests a change. The user expressed uncertainty about adapting PhotoCraft controls, so this remains exploratory.

## Suggested implementation order

1. Multi-document/source-preserving edit model, complete property/text editing and undo.
2. Dedicated descriptor authoring with live preview and multi-file mod export.
3. Optional canvas conveniences, chosen individually.
4. OGM preview playback with play/pause, looping and seeking.
5. Separate XML animation preview support:
   - Blinking and color/opacity animation (light_anim and related flags).
   - Rotation and transform animation (heading, xform_anim and related settings).
   - Sprite-sheet/animated-static playback with frame timing and looping.

## Continuation note

### Additional requested export feature: DXML-style patches

Add **DXML patch export** as an alternative to full XML file export. Generate Anomaly-compatible `modxml_<name>.script` patches targeting changed XML nodes/attributes/text through the supported DXML API, so compatible edits can be applied without replacing entire UI XML files.

Verify the target Modded Exes/DXML version and API before implementation. Preserve source filenames and aspect variants, use selectors that distinguish duplicate sibling elements, and show a reviewable patch before export. Clearly identify changes that cannot be represented reliably as DXML rather than silently producing a broken patch. Keep ordinary XML mod export available. This is a requested roadmap item, not an implemented feature.

### Additional requested canvas feature: aspect-ratio conversion

Add explicit **Adjust to 16:9** and **Adjust to 21:9** actions for the main canvas layout. These must convert the actual layout geometry/scaling between source and target aspect ratios, rather than only changing the preview viewport. The user specifically requested conversion in both directions: 21:9 to 16:9 and 16:9 to 21:9.

Determine the source layout ratio explicitly (do not infer it from the current preview selector alone), use the actual target resolution/aspect preset, and preserve deliberate alignment/anchoring where supported. The conversion should be reviewable, undoable as one operation, and exported through the existing separate-mod workflow. Define how this interacts with authored _16/_21 variants and runtime ratio correction before implementing automatic scaling. This was added to tomorrow's list; no conversion is implemented yet.

The user explicitly added all three XML animation preview categories on 2026-10-27 and plans to continue on 2026-10-28. They are roadmap items, not implemented features. Resume with the property/descriptor editing foundation, keeping video and each XML animation system as separate preview work. No reminder or scheduled automation was requested.

No application build was changed by this review.
