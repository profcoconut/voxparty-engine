import SpriteKit

class VirtualJoystick: SKNode {
    var baseNode: SKSpriteNode!
    var knobNode: SKSpriteNode!

    private(set) var trackingTouch: UITouch?
    private let maxRadius: CGFloat = 50
    private let deadzone: CGFloat = 10

    var displacement: CGPoint = .zero

    var direction: Direction? {
        let len = sqrt(displacement.x * displacement.x + displacement.y * displacement.y)
        if len < deadzone { return nil }

        let angle = atan2(displacement.y, displacement.x)
        return Direction.fromAngle(angle)
    }

    override init() {
        super.init()
        setup()
    }

    required init?(coder aDecoder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func setup() {
        baseNode = SKSpriteNode(color: UIColor.gray.withAlphaComponent(0.4), size: CGSize(width: 120, height: 120))
        knobNode = SKSpriteNode(color: UIColor.white.withAlphaComponent(0.7), size: CGSize(width: 50, height: 50))

        baseNode.alpha = 0
        knobNode.alpha = 0
        baseNode.zPosition = 1000
        knobNode.zPosition = 1001

        addChild(baseNode)
        addChild(knobNode)
    }

    func handleTouch(_ touch: UITouch, in scene: SKScene) {
        let location = touch.location(in: scene)

        if trackingTouch == nil {
            trackingTouch = touch
            baseNode.position = location
            knobNode.position = CGPoint.zero
            baseNode.alpha = 1
            knobNode.alpha = 1
        }

        displacement = CGPoint(
            x: location.x - baseNode.position.x,
            y: location.y - baseNode.position.y
        )

        let len = sqrt(displacement.x * displacement.x + displacement.y * displacement.y)
        if len > maxRadius {
            displacement = CGPoint(
                x: displacement.x / len * maxRadius,
                y: displacement.y / len * maxRadius
            )
        }

        knobNode.position = displacement
    }

    func endTouch(_ touch: UITouch) {
        if trackingTouch == touch {
            trackingTouch = nil
            displacement = .zero
            knobNode.position = .zero
            baseNode.alpha = 0
            knobNode.alpha = 0
        }
    }
}
