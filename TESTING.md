# Picker and resize verification

Run `npm ci`, `npm run check`, and `npm run tauri dev`.

## Desktop picker

- Windows, macOS, X11: open the eyedropper or press Ctrl+Shift+C. A small live magnifier follows the cursor; other apps stay visible and animations continue. Click or press Enter to copy the pixel; Escape cancels. Scroll and + / - adjust zoom.
- Move between monitors, including a display to the left of the primary monitor and monitors with different scale factors. Pick known solid RGB colors at each display's corners. The center crosshair and copied hex must agree.
- Start while holding the opening click or shortcut; it must not immediately confirm. Cancel and reopen repeatedly. Quit the main app while picking; the panel must close.
- The selecting click must not activate a button or link in the underlying application.
- macOS requires Screen Recording and Accessibility permission for live capture and input interception. Missing permission must produce an error in the main app, without a stuck overlay. Restart after granting permission.
- Fedora GNOME Wayland: the native desktop PickColor portal handles live selection. This uses the desktop's picker UI, not the application's custom magnifier. Escape should cancel without adding history. A desktop without PickColor support should show an error, not silently fall back to a frozen screenshot.
- Disconnect a display while picking: the app must close with an error or continue safely. Opening two pickers at once must be prevented.

## Resize

- Drag each corner both smaller and larger. The content must scale uniformly and the window must settle to the original 400:744 ratio, without repeated expansion.
- Try a mostly vertical drag and a mostly horizontal drag. Both should control the scale.
- Test at 100%, 125%, 150%, and 200% OS scaling. Close immediately after a resize, reopen, and check that the saved logical window size returns.
- Reopen on a smaller display: the saved size should be limited to the available screen.

## Builds

The GitHub workflow builds Windows, macOS, and Linux; Linux's `all` bundle target includes DEB and RPM. These jobs need to run on GitHub before those installers can be considered verified.

The macOS release currently has no Developer ID signing/notarization configuration. A "damaged" or "corrupt" message on a downloaded Mac build cannot be diagnosed from the source alone. Verify the downloaded archive/DMG and configure Apple signing/notarization for distribution; do not treat this source change as resolving that release issue.

## Implementation references

- [Tauri: WebView2 window creation deadlocks in synchronous commands](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html#known-issues)
- [Wayland PickColor portal wrapper](https://docs.rs/ashpd/latest/ashpd/desktop/screenshot/index.html)
- [Native input interception](https://docs.rs/rdev/latest/rdev/)
