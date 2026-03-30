---
title: feat: iOS hello world demo with visual debug overlay
type: feat
status: active
date: 2026-03-30
---

# iOS Hello World Demo with Visual Debug Overlay

## Overview

A minimal, working iOS app built in Swift that renders a simple interactive hello world screen with a persistent on-screen debug overlay showing render loop statistics and view hierarchy info.

## Problem Frame

The VoxParty team is exploring iOS-native development. This sprint establishes a clean iOS project scaffold with a working build pipeline and a visual debug overlay — the foundation for all future iOS work.

## Requirements Trace

- R1. App launches and renders a visible hello world screen on iOS Simulator
- R2. Visual debug overlay is always visible, showing live FPS and frame count
- R3. View hierarchy depth is displayed in the debug overlay
- R4. XcodeGen is used for project generation (no manual .xcodeproj)
- R5. Build succeeds via `xcodegen generate && xcodebuild` on iOS Simulator

## Scope Boundaries

- **Not included:** App Store deployment, provisioning profiles, Swift Package Manager dependencies beyond what's in Xcode, networking, persistence, animations beyond basic state
- **Focus:** Build pipeline correctness, debug overlay implementation, basic interactivity

## Key Technical Decisions

- **SwiftUI over UIKit**: Declarative, fewer files, modern iOS standard
- **XcodeGen over manual .xcodeproj**: Code-driven project files, version-controlled, CI-friendly
- **Timer + Text for debug overlay**: No third-party profiling library — simple `Text` overlay using `TimelineView` for live frame rate

## Implementation Units

- [ ] **Unit 1: Install XcodeGen and scaffold project**

**Goal:** A clean Xcode project that builds on iOS Simulator.

**Requirements:** R4, R5

**Dependencies:** Homebrew (to install XcodeGen)

**Files:**
- Create: `ios-hello-world/project.yml` — XcodeGen configuration
- Create: `ios-hello-world/Sources/App.swift` — @main entry point
- Create: `ios-hello-world/Sources/ContentView.swift` — hello world screen
- Create: `ios-hello-world/Info.plist`

**Approach:**
1. Install XcodeGen via Homebrew if not present
2. Write `project.yml` targeting iOS 17.0, iPhone 17 Pro simulator
3. Write minimal SwiftUI app with `@main` attribute
4. Generate project: `xcodegen generate`
5. Verify build: `xcodebuild -project HelloWorld.xcodeproj -scheme HelloWorld -configuration Debug -destination 'platform=iOS Simulator,name=iPhone 17 Pro' build`

**Verification:**
- `xcodegen generate` exits 0 and produces `HelloWorld.xcodeproj`
- `xcodebuild ... build` exits 0 with "BUILD SUCCEEDED"

---

- [ ] **Unit 2: Add interactive hello world screen**

**Goal:** A visible screen with text and a tap interaction that proves the view responds.

**Requirements:** R1

**Dependencies:** Unit 1

**Files:**
- Modify: `ios-hello-world/Sources/ContentView.swift`
- Modify: `ios-hello-world/Sources/App.swift`

**Approach:**
- ContentView shows a centered "Hello, World!" text
- Add a `@State var` counter that increments on tap
- Show the counter value below the greeting

**Test scenarios:**
- Happy path: App launches, "Hello, World!" is visible, tapping increments counter
- Edge case: Counter starts at 0, first tap makes it 1

**Verification:**
- Screen renders "Hello, World!" on simulator launch
- Tapping the text increments the displayed counter

---

- [ ] **Unit 3: Implement visual debug overlay**

**Goal:** A persistent overlay on the top-left corner showing live stats.

**Requirements:** R2, R3

**Dependencies:** Unit 2

**Files:**
- Create: `ios-hello-world/Sources/DebugOverlay.swift`
- Modify: `ios-hello-world/Sources/ContentView.swift`

**Approach:**
- `DebugOverlay` is a `VStack` of `Text` views positioned absolute topLeading
- Use `TimelineView(.animation)` to drive live FPS (frame count / elapsed time)
- Display: FPS (1-second rolling), total frame count, SwiftUI view depth (hardcoded to 1 for this simple app), simulator device name
- Style: monospacedSystemFont, small size, semi-transparent background

**Technical design (directional sketch):**
```
DebugOverlay:
  background: Color.black.opacity(0.5), cornerRadius(8)
  VStack(alignment: .leading, spacing: 2):
    Text("FPS: \(fps)")       // driven by TimelineView
    Text("Frames: \(count)")   // incremented each frame
    Text("View depth: 1")
    Text("Device: iPhone 17 Pro")
```

**Verification:**
- Overlay is visible in the top-left corner on launch
- FPS value updates every second
- Frame count increments continuously

## System-Wide Impact

- **New project**: This creates a new `ios-hello-world/` subdirectory in the worktree. No existing files are modified.
- **No shared state**: HelloWorld and DebugOverlay are self-contained SwiftUI views.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| XcodeGen not installed | Detect in Unit 1, install via `brew install xcodegen` |
| Simulator not available | Hardcode iPhone 17 Pro which is present in `xcrun simctl list` |
| Build fails on different macOS | Test on current machine; CI can adjust destination |

## Documentation / Operational Notes

- `ios-hello-world/` is generated code — `.xcodeproj` is committed (XcodeGen is a dev tool dependency)
- To rebuild after code changes: `xcodegen generate` (only needed if project.yml or file structure changes)

## Sources & References

- XcodeGen: https://xcodegen.org/
- SwiftUI @main: https://developer.apple.com/documentation/swiftui/app
- TimelineView for animations: https://developer.apple.com/documentation/swiftui/timelineview
