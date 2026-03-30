import SpriteKit

class GameScene: SKScene {
    var worldNode: SKNode!
    var cameraNode: SKCameraNode!
    var gameState: GameState!
    var tileNodes: [[SKSpriteNode?]]
    var player: Player!
    var blockHandler = BlockInteractionHandler()
    var highlightSprite: SKSpriteNode?
    var joystick: VirtualJoystick!
    var debugOverlay: DebugOverlay!

    override init(size: CGSize) {
        self.tileNodes = Array(repeating: Array(repeating: nil, count: IsometricMath.gridSize), count: IsometricMath.gridSize)
        super.init(size: size)
    }

    required init?(coder aDecoder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func didMove(to view: SKView) {
        backgroundColor = UIColor(red: 0.1, green: 0.1, blue: 0.18, alpha: 1.0)

        cameraNode = SKCameraNode()
        addChild(cameraNode)
        self.camera = cameraNode

        worldNode = SKNode()
        addChild(worldNode)

        // Initialize textures
        _ = Textures.shared

        gameState = GameState.generate(gridSize: IsometricMath.gridSize)
        spawnTiles()

        // Setup block handler
        blockHandler.scene = self
        blockHandler.gameState = gameState
        blockHandler.onTileHighlight = { [weak self] pos in
            self?.updateHighlight(pos)
        }

        // Spawn player at a walkable position
        let spawnPos = findSpawnPosition()
        player = Player(texture: Textures.shared.playerTexture())
        player.gridX = spawnPos.x
        player.gridY = spawnPos.y
        player.position = IsometricMath.gridToScreen(x: spawnPos.x, y: spawnPos.y)
        player.zPosition = IsometricMath.depthKey(x: spawnPos.x, y: spawnPos.y, z: 1) + 0.5
        worldNode.addChild(player)

        // Setup joystick
        joystick = VirtualJoystick()
        joystick.position = CGPoint(x: 100, y: 100)
        addChild(joystick)

        // Setup debug overlay
        debugOverlay = DebugOverlay()
        debugOverlay.position = CGPoint(x: -size.width/2 + 10, y: size.height/2 - 10)
        addChild(debugOverlay)
    }

    func spawnTiles() {
        let textures = Textures.shared.textures

        for gy in 0..<gameState.gridSize {
            for gx in 0..<gameState.gridSize {
                let tileType = gameState.getTile(at: gx, y: gy)
                guard tileType != .void, let tex = textures[tileType] else { continue }

                let sprite = SKSpriteNode(texture: tex)
                sprite.size = CGSize(width: 64, height: 32)
                sprite.position = IsometricMath.gridToScreen(x: gx, y: gy)
                sprite.zPosition = IsometricMath.depthKey(x: gx, y: gy, z: 0)
                sprite.name = "tile_\(gx)_\(gy)"

                tileNodes[gy][gx] = sprite
                worldNode.addChild(sprite)
            }
        }
    }

    func findSpawnPosition() -> (x: Int, y: Int) {
        // Find a walkable tile near center
        let centerX = gameState.gridSize / 2
        let centerY = gameState.gridSize / 2
        for r in 0..<gameState.gridSize {
            for dx in -r...r {
                for dy in -r...r {
                    let x = centerX + dx
                    let y = centerY + dy
                    if x >= 0 && x < gameState.gridSize && y >= 0 && y < gameState.gridSize {
                        if gameState.isWalkable(x: x, y: y) { return (x, y) }
                    }
                }
            }
        }
        // Last resort: scan entire grid
        for y in 0..<gameState.gridSize {
            for x in 0..<gameState.gridSize {
                if gameState.isWalkable(x: x, y: y) { return (x, y) }
            }
        }
        return (centerX, centerY)
    }

    func updateHighlight(_ pos: (x: Int, y: Int)?) {
        highlightSprite?.removeFromParent()
        highlightSprite = nil

        guard let pos = pos else { return }
        let hl = SKSpriteNode(color: UIColor.yellow.withAlphaComponent(0.3), size: CGSize(width: 66, height: 34))
        hl.position = IsometricMath.gridToScreen(x: pos.x, y: pos.y)
        hl.zPosition = IsometricMath.depthKey(x: pos.x, y: pos.y, z: 100)
        addChild(hl)
        highlightSprite = hl
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            let location = touch.location(in: self)

            if location.x <= size.width / 2 {
                // Left side: joystick
                joystick.handleTouch(touch, in: self)
            } else {
                // Right side: block interaction
                blockHandler.touchBegan(touch, in: self)
            }
        }
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            if joystick.trackingTouch == touch {
                joystick.handleTouch(touch, in: self)
            } else {
                blockHandler.touchMoved(touch, in: self)
            }
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            if joystick.trackingTouch == touch {
                joystick.endTouch(touch)
            } else {
                blockHandler.touchEnded(touch, in: self)
            }
        }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            if joystick.trackingTouch == touch {
                joystick.endTouch(touch)
            }
            blockHandler.touchEnded(touch, in: self)
        }
    }

    override func update(_ currentTime: TimeInterval) {
        // Camera follow
        guard let camera = camera else { return }
        camera.position.x += (player.position.x - camera.position.x) * 0.1
        camera.position.y += (player.position.y - camera.position.y) * 0.1

        let worldPixelWidth = CGFloat(gameState.gridSize) * IsometricMath.tileWidth / 2
        let worldPixelHeight = CGFloat(gameState.gridSize) * IsometricMath.tileHeight
        camera.position.x = max(0, min(worldPixelWidth, camera.position.x))
        camera.position.y = max(0, min(worldPixelHeight, camera.position.y))

        // Player movement
        if let dir = joystick.direction {
            _ = player.move(direction: dir, world: gameState, currentTime: currentTime)
        }

        // Update debug overlay
        debugOverlay.update(currentTime: currentTime, player: player, gameState: gameState, selectedBlock: blockHandler.selectedBlock)
    }
}
