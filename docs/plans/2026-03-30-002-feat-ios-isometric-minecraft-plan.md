---
title: "feat: iOS isometric Minecraft-style game"
type: feat
status: active
date: 2026-03-30
---

# iOS Isometric Minecraft-Style Game

## Overview

A 2D isometric pixel art game in iOS Swift using SpriteKit. The player spawns in a procedurally generated world of grass, dirt, stone, wood, leaves, and water blocks. Tap to break blocks, long-press to place selected block type. Camera follows the player. Touch joystick for movement. Debug overlay shows live stats.

## Problem Frame

VoxParty team needs a playable iOS game foundation. This builds a real game on top of the existing iOS scaffold, replacing the hello world demo with a complete Minecraft-style experience.

## Requirements Trace

- R1. Procedurally generated isometric terrain (noise-based landmasses)
- R2. Multiple block types: grass, dirt, stone, wood, leaves, water
- R3. Pixel art aesthetic with nearest-neighbor texture filtering
- R4. Player movement on grid with virtual joystick
- R5. Block breaking (tap) and placing (long-press) mechanics
- R6. Camera follow with world bounds clamping
- R7. Visual debug overlay (FPS, player grid position, block count)
- R8. iOS touch controls: virtual joystick + tap/long-press gestures
- R9. Build succeeds on iOS Simulator

## Scope Boundaries

- Single-player, single-world (no persistence, no multiplayer, no saves)
- Fixed grid size: 64x64 tiles
- No inventory UI (cycle selected block with a button)
- No physics engine beyond basic collision
- No NPC enemies
- Targeting iOS Simulator (no device-specific haptics)

## Key Technical Decisions

- **Manual SKSpriteNode over SKTileMapNode**: True isometric diamond tiles require manual sprite spawning. SKTileMapNode's isometric mode is top-down only.
- **GameplayKit GKNoise for terrain**: Native Apple framework, no third-party dependencies.
- **Pixel art via SKTexture.filteringMode = .nearest**: Prevents sprite blurring on retina displays.
- **GridX + GridY depth sorting**: Isometric depth is a function of grid position, not screen Y.
- **Custom VirtualJoystick**: SpriteKit-compatible, tracks touches in scene for multi-touch reliability.
- **Block interaction via scene-level touch tracking**: Single touch handler maps tap vs long-press to break vs place.
- **SKCameraNode for follow**: Native camera with bounds clamping and lerp smoothing.

## High-Level Technical Design

```
GameScene (SKScene)
  ├── worldNode (SKNode)          — all tiles and player, z-sorted
  │     ├── tileNodes [y][x]       — SKSpriteNode per tile, pixel art
  │     └── playerNode             — SKSpriteNode, animates on movement
  ├── camera (SKCameraNode)         — follows player with lerp smoothing
  ├── joystick (VirtualJoystick)    — left side touch, quantized 8-dir
  ├── blockSelector (SKNode)        — bottom UI: current block type + cycle buttons
  └── debugOverlay (SKNode)        — top-left, FPS + grid pos + block count

GameState
  ├── world[y][x]: TileType       — 2D grid, generated once
  ├── player: (gridX, gridY)       — current grid position
  ├── selectedBlock: TileType       — block type to place
  └── Inventory: selected index

IsometricMath
  ├── gridToScreen(x,y) → CGPoint
  ├── screenToGrid(point) → (x,y)
  └── depthKey(x,y,z) → CGFloat   — zPosition = x + y + z*1000

TerrainGenerator
  └── generate() → [[TileType]]   — GKNoise with thresholds → block types
```

## Implementation Units

- [ ] **Unit 1: Replace SwiftUI scaffold with SpriteKit game scene**

**Goal:** Working SpriteKit scene with pixel-art rendering, XcodeGen project intact.

**Requirements:** R9

**Dependencies:** None

**Files:**
- Replace: `ios-hello-world/Sources/App.swift`
- Replace: `ios-hello-world/Sources/ContentView.swift`
- Create: `ios-hello-world/Sources/GameScene.swift`
- Create: `ios-hello-world/Sources/IsometricMath.swift`
- Create: `ios-hello-world/Sources/GameState.swift`
- Create: `ios-hello-world/Sources/GameViewController.swift`
- Create: `ios-hello-world/Resources/Assets.xcassets/` (textures)

**Approach:**
- App.swift: Launch `GameViewController` instead of SwiftUI ContentView
- GameViewController: Hosts `SKView` with `shouldCullNonVisibleNodes = true`, `ignoresSiblingOrder = true`, `preferredFramesPerSecond = 60`
- GameScene: Minimal scene — spawns world, shows "Loading..." text, then generates terrain
- SKView contentScaleFactor = 1.0 for pixel-perfect on simulator

**Test scenarios:**
- Happy path: App launches, black screen with "Loading..." appears, terrain generates
- Build: `xcodebuild ... build` → BUILD SUCCEEDED

**Verification:**
- App runs on simulator without crash
- SpriteKit scene initializes

---

- [ ] **Unit 2: Procedural pixel art block textures**

**Goal:** All block types render as distinct pixel art sprites with nearest-neighbor filtering.

**Requirements:** R2, R3

**Dependencies:** Unit 1

**Files:**
- Create: `ios-hello-world/Sources/Textures.swift`
- Create: `ios-hello-world/Resources/Textures/` (generated PNGs)

**Approach:**
- Generate procedural pixel art for each block type at 64x32 (isometric diamond):
  - Grass: green top surface, brown edges
  - Dirt: brown, darker edges
  - Stone: gray, darker edges
  - Wood: brown trunk, lighter center
  - Leaves: green cluster, transparent edges
  - Water: blue, semi-transparent, animated shimmer (optional)
- Each texture generated as PNG at 2x (128x64) for retina, loaded with `filteringMode = .nearest`
- Textures stored in Assets.xcassets or generated at runtime
- Water block: SKAction sequence to pulse alpha (0.6 to 1.0)

**Test scenarios:**
- Happy path: Each block type renders distinctly
- Visual: No blurring on retina simulator (nearest neighbor)
- Water (if implemented): Visible animation

**Verification:**
- All 6 block types visually distinct in game

---

- [ ] **Unit 3: Isometric tile grid rendering**

**Goal:** 64x64 grid of tiles renders in correct isometric order (depth-sorted).

**Requirements:** R1

**Dependencies:** Units 1, 2

**Files:**
- Modify: `ios-hello-world/Sources/GameScene.swift`
- Modify: `ios-hello-world/Sources/GameState.swift`

**Approach:**
- `GameState.world: [[TileType]]` — 64x64 2D array
- `TerrainGenerator.generate()` — GKNoise with frequency=4.0, 4 octaves, thresholds: <-0.3=deep water, -0.3..-0.1=water, -0.1..0.2=sand, 0.2..0.6=grass, 0.6..0.8=forest(dirt+wood+leaves), >0.8=mountain(stone)
- `GameScene.spawnTiles()` — iterates grid, creates SKSpriteNode per tile, positions via `gridToScreen`, sets zPosition via `depthKey`
- Depth order: ascending `gridX + gridY` — lower-left tiles render first (behind)
- Spawn visible subset (viewport culling for performance): only tiles within 2 screens of camera

**Technical design:**
```
For gy in 0..<gridHeight:
  For gx in 0..<gridWidth:
    let tile = world[gy][gx]
    if tile == .void: continue
    let sprite = SKSpriteNode(texture: textures[tile])
    sprite.size = CGSize(width:64, height:32)
    sprite.position = gridToScreen(gx, gy)
    sprite.zPosition = depthKey(gx, gy, 0)
    sprite.filteringMode = .nearest
    worldNode.addChild(sprite)
```

**Test scenarios:**
- Happy path: Terrain renders with correct isometric depth (player walks "over" tiles behind them)
- Edge: Water tiles render semi-transparently
- Edge: Forest tiles (wood/leaves) stack visually on grass

**Verification:**
- Walk the player in a full circle — tiles behind the player always appear behind

---

- [ ] **Unit 4: Player sprite and grid-based movement**

**Goal:** Player character moves on the grid with smooth visual transition between tiles.

**Requirements:** R4

**Dependencies:** Unit 3

**Files:**
- Create: `ios-hello-world/Sources/Player.swift`
- Modify: `ios-hello-world/Sources/GameScene.swift`

**Approach:**
- Player is a SKSpriteNode (32x32 blue square with pixel art character texture)
- Grid-based movement: player occupies exact tile center, moves one tile per input
- Movement cooldown: 150ms between moves (prevents diagonal spam)
- 8-direction quantization: joystick angle → nearest cardinal/diagonal direction
- Visual: smooth SKAction move to new grid position (0.1s), not instant snap
- Player zPosition: `depthKey(gridX, gridY, 1)` — always above tile at same grid position
- Collision: water and void tiles block movement

**Technical design:**
```
On joystick input:
  dir = joystick.direction  // 8 quantized directions
  newX = player.gridX + dir.dx
  newY = player.gridY + dir.dy
  if world.isWalkable(newX, newY):
    player.gridX = newX
    player.gridY = newY
    player.run(SKAction.move(to: gridToScreen(newX,newY), duration:0.1))
```

**Test scenarios:**
- Happy path: Press joystick right, player moves to adjacent tile
- Edge: Hold joystick, player moves tile-by-tile with 150ms cooldown
- Edge: Walk toward void tile, player stops at boundary
- Edge: Diagonal joystick, player moves diagonally

**Verification:**
- Player visually walks on top of tiles in correct depth order
- Player cannot walk into void or water

---

- [ ] **Unit 5: Camera follow with bounds**

**Goal:** Camera smoothly follows player and clamps to world edges.

**Requirements:** R6

**Dependencies:** Unit 4

**Files:**
- Modify: `ios-hello-world/Sources/GameScene.swift`

**Approach:**
- SKCameraNode centered on world, `camera` property of scene
- Smooth follow: lerp camera position toward player position each frame
- Lerp factor: 0.1 per frame (10% of remaining distance)
- World bounds clamping: camera stops at world edges so edges are visible
- Camera zPosition = 1000 (above all tiles)

**Technical design:**
```
Camera bounds:
  minX = (gridHeight/2) * TILE_H   // so north edge visible when camera at minY
  maxX = worldWidth - minX
  minY = minX
  maxY = worldHeight - minX

Update each frame:
  targetX = player.screenX
  targetY = player.screenY
  cam.x += (targetX - cam.x) * 0.1
  cam.y += (targetY - cam.y) * 0.1
  cam.x = clamp(cam.x, minX, maxX)
  cam.y = clamp(cam.y, minY, maxY)
```

**Test scenarios:**
- Happy path: Walk to world edge, camera stops at edge
- Edge: Walk back from edge, camera smoothly reacquires player

**Verification:**
- Player always centered, world edges never scroll out of view

---

- [ ] **Unit 6: Block breaking and placing**

**Goal:** Tap a tile to break it (removes from world). Long-press a tile to place selected block type.

**Requirements:** R5

**Dependencies:** Units 3, 4

**Files:**
- Create: `ios-hello-world/Sources/BlockInteraction.swift`
- Modify: `ios-hello-world/Sources/GameScene.swift`
- Modify: `ios-hello-world/Sources/GameState.swift`

**Approach:**
- Scene-level touch handling: single UITouch tracked for tap vs long-press
- Tap (< 0.3s): find tile under touch, remove it (set to void, animate out with scale-to-zero + fade)
- Long-press (>= 0.3s): find tile under touch, place selected block type, animate in
- Touch-to-grid: convert touch location to scene coords → `screenToGrid()`
- Only break non-void, non-water tiles
- Only place on void tiles adjacent to non-void
- Block selector UI: 3 SKSpriteNode buttons at bottom center, cycle through grass/dirt/stone/wood/leaves
- Visual feedback: highlight tile under finger (yellow outline sprite)

**Test scenarios:**
- Happy path: Tap grass tile, it disappears (becomes void)
- Happy path: Long-press void tile, selected block type appears
- Edge: Tap water tile, nothing happens
- Edge: Long-press non-void tile, nothing happens
- Edge: Long-press void at world edge, nothing happens
- Edge: Break all adjacent tiles around a tile, it stays (no falling)

**Verification:**
- Breaking and placing work with visual feedback
- Block selector cycles through all 5 placeable types

---

- [ ] **Unit 7: Virtual joystick touch controls**

**Goal:** Virtual joystick in lower-left corner for movement. iOS touch controls work on simulator.

**Requirements:** R4, R8

**Dependencies:** Unit 4

**Files:**
- Create: `ios-hello-world/Sources/VirtualJoystick.swift`
- Modify: `ios-hello-world/Sources/GameScene.swift`

**Approach:**
- VirtualJoystick: two SKSpriteNodes (base circle + knob)
- Base appears at touch-down location in left half of screen
- Knob follows finger within base radius (60pt max displacement)
- Deadzone: 10pt radius — below deadzone outputs nil
- Direction quantized to 8 directions: right, down-right, down, down-left, left, up-left, up, up-right
- Right half of screen: reserved for block interaction
- Multi-touch: joystick and block interaction tracked as separate touches

**Test scenarios:**
- Happy path: Touch left side, joystick appears, drag, player moves
- Edge: Touch and release without moving, player doesn't move
- Edge: Rapid touch on/off, no stuck joystick state

**Verification:**
- Virtual joystick functional on simulator with mouse drag

---

- [ ] **Unit 8: Visual debug overlay**

**Goal:** Persistent top-left overlay showing live FPS, player grid position, and block count.

**Requirements:** R7

**Dependencies:** Unit 4

**Files:**
- Create: `ios-hello-world/Sources/DebugOverlay.swift`
- Modify: `ios-hello-world/Sources/GameScene.swift`

**Approach:**
- SKLabelNode-based overlay (monospaced Courier font)
- zPosition = 9999 (above everything)
- FPS: frame counter incremented each `update()`, computed per second
- Grid position: reads `player.gridX, player.gridY`
- Block count: counts non-void tiles in `GameState.world`
- Semi-transparent black background rect behind labels
- Positioned top-left: `CGPoint(x: -frame.width/2 + 10, y: frame.height/2 - 20)`
- fontSize = 14, white text, 1pt black outline for readability

**Test scenarios:**
- Happy path: FPS updates every second, position updates every move, block count static
- Edge: After breaking tiles, block count decreases

**Verification:**
- Debug overlay always visible, stats accurate

## System-Wide Impact

- **Replaces SwiftUI hello world**: App.swift now launches SKView instead of ContentView
- **No shared state**: GameState is a simple struct, GameScene owns all mutable state
- **No persistence**: World regenerates on each launch

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| SpriteKit scene won't init in headless environment | Test on simulator only |
| Memory pressure from 64x64 texture atlas | Generate tiles in viewport batches, use texture atlases |
| Touch tracking conflicts between joystick and block interaction | Track two independent touches by touch ID |
| Camera lerp creates jitter on diagonal movement | Clamp lerp factor, ensure movement duration > 1 frame |
| XcodeGen project.yml regeneration needed | Keep project.yml as source of truth, update only when adding files |

## Documentation / Operational Notes

- To rebuild after code changes: `xcodegen generate` (only if project.yml or file structure changes)
- To run: open `HelloWorld.xcodeproj` in Xcode, Cmd+R on iPhone 17 Pro simulator
- `project.yml` is the authoritative project file — `.xcodeproj` is committed
