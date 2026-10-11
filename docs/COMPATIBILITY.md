# Compatibility and acceptance

## Confirmed upstream blocker

Inspected revision: `a7c2999d04ebad613202a2fcfedaf258c924ce71` (2026-10-11).

- `components/script_bindings/webidls/HTMLMediaElement.webidl` has `MediaSource` commented out of MediaProvider.
- The inspected WebIDL directory does not define MediaSource or SourceBuffer.
- No Encrypted Media Extensions/Widevine integration is added by this project.
- The Servo media backend uses GStreamer. Installing codecs supplies native decoding, not missing DOM APIs.

Source: https://github.com/servo/servo/blob/a7c2999d04ebad613202a2fcfedaf258c924ce71/components/script_bindings/webidls/HTMLMediaElement.webidl

The expected failure is YouTube adaptive playback. This is a source-based compatibility finding, not a claimed live test against YouTube Music. Authentication, session persistence, service-worker behavior, IndexedDB usage, background playback, media keys, sleep inhibition and protected content all require separate validation. Do not advertise them as working features. The project does not spoof a Chromium user agent or silently substitute another browser engine.

## What would unblock working playback

Servo needs compatible MSE DOM APIs, source-buffer append/remove scheduling, stream demuxing, buffering/seeking, timestamp handling and a connection to its player backend. Passing the MSE web-platform tests and verifying the real player is necessary. More YouTube Music web-platform issues may remain after MSE is implemented. DRM-requiring content would require additional legitimate EME/CDM support.

If reliable playback is required immediately, the engine requirement would need reconsideration. No fallback engine is included here because Servo was explicitly requested.

## Manual acceptance on real hardware

1. Install the correct artifact on a clean machine; confirm the app opens and the address bar displays the real website origin.
2. Confirm the dedicated profile is used; test a normal HTTPS page and persistent browser data.
3. Open media diagnostics. Record OS, driver, GPU and GStreamer versions. A listed factory is not a passed playback test.
4. Play a local, unprotected media sample in Servo. Test software decoding first, then automatic mode; use GStreamer pipeline logs or driver counters to confirm the decoder actually selected.
5. Run the explicit hardware encoding test and auto/CPU test; play the resulting MP4. Explicit selection must fail if unavailable. Auto must reach CPU if hardware candidates fail.
6. Test invalid/unwritable output paths and existing-file rejection. Check pipeline shutdown after encoder failure.
7. Test YouTube Music page loading, sign-in, track start, seek, track changes, audio/video switching, pause/resume and account persistence only after the upstream blocker is resolved.
8. Repeat on Windows x86_64, Linux x86_64 and Linux ARM64. A hosted CI runner cannot validate consumer GPU capabilities.

Do not remove the experimental status based only on compilation or synthetic tests.
