# Message Box

A Windows x86-64 protocol 2 plugin that shows messages on the computer, reads them aloud, or does both. Any approved device can trigger these actions; native plugins are not sandboxed. Requires an interactive Windows session, Windows PowerShell, Windows Forms and System.Speech. Speech uses an installed voice matching the selected language where available, otherwise the Windows default.

Run `python package.py` on Windows x86-64 with Rust and Python installed. Install the resulting ZIP in RCHub, start the plugin, and open the desktop or Android surface. The same surface supports testing on desktop. The last 10 sent messages are held only in plugin process memory and disappear when it stops. The history is visible to any client authorized to use the plugin.

Messages are limited to 2000 Unicode characters and titles to 100; control characters other than newline and tab are removed. Show windows are topmost and may auto-close after 1–300 seconds; 0 disables auto-close. Requests acknowledge starting the Windows processes, not successful display or completed speech; an error in Windows Forms or speech after startup cannot be reported to the client. Both actions may overlap and a partial result is possible. Do not automatically retry an uncertain request.

The host relays requests but does not push updates. Refresh history manually. Real-device and clean-install verification are required before release.
