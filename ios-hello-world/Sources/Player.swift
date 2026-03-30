import SpriteKit

class Player: SKNode {
    var gridX: Int = 1
    var gridY: Int = 1
    var texture: SKTexture

    private let sprite: SKSpriteNode
    private var lastMoveTime: TimeInterval = 0
    private let moveCooldown: TimeInterval = 0.15

    init(texture: SKTexture) {
        self.texture = texture
        self.sprite = SKSpriteNode(texture: texture)
        super.init()

        sprite.size = CGSize(width: 32, height: 32)
        addChild(sprite)

        // Find valid spawn position
        position = IsometricMath.gridToScreen(x: gridX, y: gridY)
        position.y += 8 // offset from tile
    }

    required init?(coder aDecoder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    func move(direction: Direction, world: GameState, currentTime: TimeInterval) -> Bool {
        guard currentTime - lastMoveTime >= moveCooldown else { return false }

        let newX = gridX + direction.dx
        let newY = gridY + direction.dy

        guard world.isWalkable(x: newX, y: newY) else { return false }

        gridX = newX
        gridY = newY
        lastMoveTime = currentTime

        let targetPos = IsometricMath.gridToScreen(x: gridX, y: gridY)
        let moveAction = SKAction.move(to: CGPoint(x: targetPos.x, y: targetPos.y + 8), duration: 0.08)
        moveAction.timingMode = .easeOut
        run(moveAction)

        return true
    }
}

enum Direction: CaseIterable {
    case right, left, up, down, upRight, upLeft, downRight, downLeft

    var dx: Int {
        switch self {
        case .right, .upRight, .downRight: return 1
        case .left, .upLeft, .downLeft: return -1
        default: return 0
        }
    }

    var dy: Int {
        switch self {
        case .up, .upRight, .upLeft: return -1
        case .down, .downRight, .downLeft: return 1
        default: return 0
        }
    }

    static func fromAngle(_ angle: CGFloat) -> Direction {
        let degrees = angle * 180 / .pi
        switch degrees {
        case -157.5...(-112.5): return .left
        case -112.5...(-67.5): return .downLeft
        case -67.5...(-22.5): return .down
        case -22.5...(22.5): return .downRight
        case 22.5...(67.5): return .right
        case 67.5...(112.5): return .upRight
        case 112.5...(157.5): return .up
        default: return .upLeft
        }
    }
}
