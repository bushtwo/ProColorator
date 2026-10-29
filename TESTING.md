# Color chooser verification

Run `npm ci`, `npm run check`, and `npm run tauri dev`.

## Choosing colors

- Switch between all eight color models. Sliders and numeric values should update to represent the selected color, within each model's conversion limits.
- Adjust sliders, type numeric values, and scrub numeric fields. The preview and HEX value should update immediately.
- In HEX mode, enter a valid HEX color and check that the preview matches it.
- Copy a color and paste into a text editor. The copied HEX should match the preview.
- Confirm that no eyedropper button or screen overlay remains and Ctrl+Shift+C no longer starts screen sampling.

## History

- Copy several colors, then inspect both list and grid views.
- Click a saved color and confirm that its HEX value is copied to the clipboard.
- Restart the desktop app. History and the selected history view should persist.

## Resizing

- Drag each corner smaller and larger. Content should scale uniformly and settle to the original 400:744 window ratio.
- Try mostly vertical and mostly horizontal drags.
- Test at 100%, 125%, 150%, and 200% OS scaling. Close immediately after resizing and confirm that the saved logical window size returns on launch.
- Reopen on a smaller display and check that the saved size fits the available screen.

## Releases and README

- Run the GitHub workflow on Windows, macOS, and Linux. Verify installers before publishing the draft release.
- Confirm that the repository homepage displays `docs/images/procolorator.jpg` and that the download link opens the repository's releases page.
- macOS signing/notarization is not configured. Investigate installation warnings separately from chooser behavior.
