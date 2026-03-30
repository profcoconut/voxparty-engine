import SpriteKit

class DebugOverlay: SKNode {
    let fpsLabel = SKLabelNode(fontNamed: "Courier")
    let gridLabel = SKLabelNode(fontNamed: "Courier")
    let blockLabel = SKLabelNode(fontNamed: "Courier")
    let blockTypeLabel = SKLabelNode(fontNamed: "Courier")

    private var frameCount = 0
    private var lastTime: TimeInterval = 0
    private var currentFPS = 0

    override init() {
        super.init()
        setup()
    }

    required init?(coder aDecoder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func setup() {
        zPosition = 9999

        let labels = [fpsLabel, gridLabel, blockLabel, blockTypeLabel]
        for (i, label) in labels.enumerated() {
            label.fontSize = 14
            label.fontColor = .white
            label.horizontalAlignmentMode = .left
            label.position = CGPoint(x: 0, y: -CGFloat(i) * 20)
            label.zPosition = 9999
            addChild(label)
        }

        let bg = SKShapeNode(rectOf: CGSize(width: 200, height: 90), cornerRadius: 6)
        bg.fillColor = UIColor.black.withAlphaComponent(0.6)
        bg.strokeColor = .clear
        bg.zPosition = -1
        bg.position = CGPoint(x: 100, y: -35)
        addChild(bg)

        fpsLabel.text = "FPS: --"
        gridLabel.text = "Grid: (0, 0)"
        blockLabel.text = "Blocks: 0"
        blockTypeLabel.text = "Selected: Grass"
    }

    func update(currentTime: TimeInterval, player: Player?, gameState: GameState?, selectedBlock: TileType) {
        frameCount += 1

        if currentTime - lastTime >= 1.0 {
            currentFPS = frameCount
            frameCount = 0
            lastTime = currentTime
            fpsLabel.text = "FPS: \(currentFPS)"
        }

        if let p = player {
            gridLabel.text = String(format: "Grid: (%d, %d)", p.gridX, p.gridY)
        }

        if let state = gameState {
            blockLabel.text = "Blocks: \(state.blockCount)"
        }

        blockTypeLabel.text = "Selected: \(selectedBlock)"
    }
}
