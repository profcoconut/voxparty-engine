import UIKit
import SpriteKit

class BlockInteractionHandler {
    weak var scene: GameScene?
    var gameState: GameState?
    var selectedBlock: TileType = .grass

    private var touchStartTime: TimeInterval = 0
    private var touchStartLocation: CGPoint = .zero
    private var touch: UITouch?
    private let longPressThreshold: TimeInterval = 0.3

    var onTileHighlight: ((Int, Int)?) -> Void = { _ in }

    func touchBegan(_ t: UITouch, in scene: SKScene) {
        let location = t.location(in: scene)
        touchStartTime = CACurrentMediaTime()
        touchStartLocation = location
        touch = t
    }

    func touchMoved(_ t: UITouch, in scene: SKScene) {
        let location = t.location(in: scene)
        let gridPos = IsometricMath.screenToGrid(point: location)
        onTileHighlight((gridPos.x, gridPos.y))
    }

    func touchEnded(_ t: UITouch, in scene: SKScene) {
        let elapsed = CACurrentMediaTime() - touchStartTime

        if elapsed >= longPressThreshold {
            // Long press: place block
            let gridPos = IsometricMath.screenToGrid(point: touchStartLocation)
            placeBlock(at: gridPos.x, y: gridPos.y)
        } else {
            // Tap: break block
            let gridPos = IsometricMath.screenToGrid(point: touchStartLocation)
            breakBlock(at: gridPos.x, y: gridPos.y)
        }

        onTileHighlight(nil)
        touch = nil
    }

    func breakBlock(at x: Int, y: Int) {
        guard var state = gameState, let sc = scene else { return }
        let tileType = state.getTile(at: x, y: y)
        guard tileType != .void && tileType != .water else { return }

        state.setTile(.void, at: x, y: y)
        gameState = state

        if let sprite = sc.tileNodes[y][x] {
            let shrink = SKAction.scale(to: 0, duration: 0.15)
            let fade = SKAction.fadeOut(withDuration: 0.15)
            let group = SKAction.group([shrink, fade])
            sprite.run(group) { sprite.removeFromParent() }
            sc.tileNodes[y][x] = nil
        }
    }

    func placeBlock(at x: Int, y: Int) {
        guard var state = gameState, let sc = scene else { return }
        let tileType = state.getTile(at: x, y: y)
        guard tileType == .void else { return }

        state.setTile(selectedBlock, at: x, y: y)
        gameState = state

        guard let texture = Textures.shared.textures[selectedBlock] else { return }
        let sprite = SKSpriteNode(texture: texture)
        sprite.size = CGSize(width: 64, height: 32)
        sprite.position = IsometricMath.gridToScreen(x: x, y: y)
        sprite.zPosition = IsometricMath.depthKey(x: x, y: y, z: 0)
        sprite.setScale(0)
        sprite.name = "tile_\(x)_\(y)"

        sc.worldNode.addChild(sprite)
        let grow = SKAction.scale(to: 1, duration: 0.15)
        grow.timingMode = .easeOut
        sprite.run(grow)
        sc.tileNodes[y][x] = sprite
    }

    func cycleBlock() {
        let allPlaceable: [TileType] = [.grass, .dirt, .stone, .wood, .leaves]
        if let idx = allPlaceable.firstIndex(of: selectedBlock) {
            selectedBlock = allPlaceable[(idx + 1) % allPlaceable.count]
        } else {
            selectedBlock = .grass
        }
    }
}
