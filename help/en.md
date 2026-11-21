# Welcome
ScreenCapture captures the screen, annotates images and records videos. Almost every action can be done in several ways: keyboard shortcut, Home button, notification-area menu or mouse. This help covers them all.
![accueil]
- **Home**: start a capture and browse the history.
- **Settings**: folder, format, shortcuts, retention, language, updates.
- **Help**: this page. Press `F1` from the Home page.
> ScreenCapture stays available in the notification area (near the clock): the window's close button hides it without quitting.
> On Mac, `Ctrl` means `⌘`, and capture shortcuts are chosen in Settings.

# Taking a capture
Six kinds of capture:
- **Area**: part of the screen, chosen with the mouse.
- **Full screen**: all screens at once.
- **Window**: the window in the foreground.
- **Last area**: the same area as the previous capture, without drawing it again.
- **Delayed**: an area, after a countdown (to open a menu, for example).
- **Video**: recording of an area, see "Recording a video".
## To start a capture
- The **keyboard shortcut**, from any application: `Print Screen` for an area, `Ctrl + Print Screen` for full screen, `Alt + Print Screen` for a window, `Shift + Print Screen` for the last area, `Ctrl + Shift + Print Screen` for video. They can be changed in Settings.
- The **Home cards** (Area, Full screen, Window…).
- The **notification-area menu**: right-click the ScreenCapture icon.
- The **left bar button** (at the top), for an area.
- The **command line**: `ScreenCapture.exe --cmd region` (see "Going further").
## Choosing the area
![selection]
- **Drag** to draw the area; releasing takes the capture.
- **Extend downwards** (Windows): without releasing the button, turn the **wheel**: the page or document under the area scrolls and the area grows accordingly (the total height is shown with ↓). Release the button to capture.
- **Click** a window to capture all of it (it is outlined on hover).
- `Enter` captures the whole screen, `Esc` cancels.
- `C` copies the color under the cursor (hex code); the magnifier shows enlarged pixels.
- The help bar moves out of the way if it covers the area you are selecting.
## After the capture
Depending on the "After capture" setting:
- **Thumbnail**: a thumbnail appears for a few seconds. Clicking it opens the editor.
- **Editor**: the image opens directly for annotation.
- **Nothing**: the capture is only saved and/or copied.
The capture can also be copied to the clipboard automatically and saved to the captures folder (Settings).
## Scrolling capture
To capture a page taller than the screen (web page, document…): **Scrolling** card on the home page, or the notification area menu.
- Select the area that scrolls, preferably without the scrollbar.
- The page scrolls down by itself and the images are stitched together; the capture stops at the bottom of the page, or with **Finish** / `Esc`.
- To scroll yourself, move the mouse: automatic scrolling stops and the stitching follows your wheel (not too fast), until **Finish**.
- Fixed headers and footers are kept only once.
> On Mac, automatic scrolling needs the **Accessibility** permission (System Settings > Privacy & Security). Without it, scroll the page yourself with the wheel or trackpad, then **Finish**.

# Annotating in the editor
![editeur]
The editor opens after a capture, when you double-click a thumbnail in the history, with the "Open an image" button, or when you drag an image file onto the window.
## The tools
Each tool has a key (without Ctrl):
- `V` Select, `A` Arrow, `L` Line, `R` Rectangle, `E` Ellipse, `P` Freehand.
- `H` Highlighter, `B` Blur / pixelate, `M` Magnifier.
- `T` Text, `U` Speech bubble, `N` Numbering, `S` Stamp.
- `C` Crop, `X` Cut out a strip, `D` Move an area, `I` Color picker.
Holding `Shift` while drawing gives 45° angles (arrows, lines) or regular shapes (square, circle).
## Editing a shape
- **Select**: Select tool, then click the shape. The handles resize it.
- **Move**: drag the shape, or use the arrow keys (`Shift` + arrow: 10 pixels).
- **Move around a zoomed image**: hold the **right button** (or the middle button) and move the mouse; the wheel also scrolls.
- **Bubbles**: four types (rounded, rectangular, oval, thought) in the right panel. Drag the **tail handle** to point or lengthen the tail without moving the bubble; "Default tail" puts it back.
- **Duplicate**: `Ctrl + D`, or the "Duplicate" button in the right panel. A duplicated number takes the next number in the series.
- **Edit the content**: **double-click** a text, bubble, number or stamp (or the "Edit content" button). For a number, type the new value (5, e, IV… depending on the style) then `Enter`.
- **Color, thickness, text size, fill, shadow, dashes**: right panel, applied to the selected shape, or to the next shapes when nothing is selected.
- **Bring to front / send to back** and **delete** (`Del`): right panel buttons.
- **Undo / redo**: `Ctrl + Z` / `Ctrl + Y`.
## Numbers and stamps
- **Numbering**: each click places the next number. The right panel chooses the style (1, 2, 3 · a, b, c · I, II, III…), the next value, and can renumber everything in order.
- **Stamps**: APPROVED, CONFIDENTIAL, DRAFT… or your own text in the **Custom stamp** field (it is remembered). Clicking the image places the stamp; double-clicking a placed stamp edits its text.
## Transforming the image
- **Crop** (`C`): drag the area to keep, then `Enter`.
- **Cut out a strip** (`X`): removes a horizontal or vertical strip and joins the two pieces (straight or zigzag seam).
- **Move an area** (`D`): select part of the image and drag it; the original spot is filled with the background color.
- **Rotate**, **resize** and **extract text** (character recognition): buttons in the top bar.
## Saving and sharing
- `Ctrl + S` saves, `Ctrl + Shift + S` saves under another name or format.
- `Ctrl + C` copies the annotated image to the clipboard.
- **Export effects** (border, drop shadow): added to the saved or copied image.
- `Ctrl + wheel` zooms, `Ctrl + 0` fits to the window; the middle mouse button pans the view.

# The capture history
The Home page lists the captures in the folder, newest first.
- **Filters** All / Images / Videos and **search** by name.
- **Open**: double-click (editor for an image, player for a video).
- **Quick actions** when hovering a thumbnail: edit, copy, pin, show in Explorer, delete. Right-click: full menu.
## Selecting several captures
- **The round box** at the top left of a thumbnail adds it to the selection with a single click. After that, clicking another thumbnail adds or removes it, without holding any key.
- Or: `Ctrl` + click to add or remove, `Shift` + click for a range, `Ctrl + A` to select all.
- The **selection bar** shows how many captures are selected and lets you select all, pin, or **delete the whole selection** in one click (or `Del`). `Esc` clears the selection.
- Deleted captures go to the Recycle Bin, after confirmation.
## Adding images
- `Ctrl + V` pastes the clipboard image into the captures folder.
- Dragging image files from Explorer onto the window adds them.
- The "Open an image" button opens any image in the editor.

# Recording a video
- Start: `Ctrl + Shift + Print Screen`, the **Video** card on the Home page or the notification menu, then choose the area as for a capture.
- **Sound**: the `M` key (or a click on the chip) chooses with or without the microphone before starting. While recording, the microphone button of the floating bar mutes or restores the sound.
- **Stop**: floating bar button or the same shortcut. **Cancel**: `Esc`.
- MP4, WebM or GIF formats, frames per second, cursor and microphone: in Settings. Recording uses FFmpeg (offered by the installer).

# Capture retention
In Settings, ScreenCapture can delete old captures automatically:
- one **duration** for images and another for videos (7, 30 or 90 days, 1 year, custom duration or never);
- a **maximum folder size**: beyond it, the oldest captures go first;
- to the **Recycle Bin** (recoverable) or **permanently**.
**Pinned** captures (pin on the thumbnail) and the one open in the editor are never deleted; files that do not come from ScreenCapture are never touched. "Clean up now" shows what will be deleted before doing it.

# Settings
![parametres]
- **Capture**: folder, PNG or JPEG format, automatic saving and copying, what happens after a capture, cursor, delayed capture delay, magnifier.
- **Keyboard shortcuts**: type a shortcut (for example `ctrl+shift+KeyS`) then "Apply"; leave empty to disable it.
- **Editor colors**: edit, add or remove the proposed colors.
- **Updates**: automatic check and "Check now" button.
- **Appearance and startup**: language (French, English, Spanish, German or the system language), light or dark theme, accent color, closing to the notification area, starting minimized.
- **Launch when Windows starts** (or at login on Mac): turn it on or off at any time, even if it was chosen during installation. The switch shows the actual state.

# Updates and portable version
## Updates
When a new version is published, a banner appears at the top of the window. **Update** downloads it, checks its fingerprint, installs it and restarts ScreenCapture. **Later** hides the banner until the next start. On Mac, the disk image is downloaded and opened: just drag the application into Applications.
## Portable version
The portable version (`.zip` file) runs without installation, for example from a USB stick: unzip the folder and run `ScreenCapture.exe`. Settings and the log are stored in the `data` folder next to the program. Updates replace the program in the same place.

# Going further
## Command line
A running instance can be controlled with `ScreenCapture.exe --cmd` followed by: `region`, `full`, `window`, `last`, `delayed`, `video`, `show`, `hide`, `settings`, `cancel` or `quit`. Handy for a custom shortcut or a script.
## If something goes wrong
- If the application stops responding, launching it again from the Start menu replaces the frozen instance.
- A diagnostic log can be turned on by triple-clicking the version on the "About" page.
- ScreenCapture is free software (GPL-3.0 license): the source code and releases are on GitHub.
## On Mac: screen recording permission
ScreenCapture needs the **Screen Recording** permission for screenshots and videos (FFmpeg, started by the application, depends on it too). Without it, screenshots are **black** or show only the wallpaper.
- When the permission is missing, a red banner appears at the top of the window with **Open Settings** and **Restart**.
- In System Settings > Privacy & Security > Screen Recording, if ScreenCapture is already listed but screenshots stay black, remove the entry with the `−` button, add it again with `+` (or turn it on), then restart the application.
- System sound also needs the **System Audio Recording Only** permission, on the same page.
> Since macOS Sequoia, the system asks again about once a month for every application that records the screen. This is normal: just accept.
