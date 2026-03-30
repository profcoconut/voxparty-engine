import UIKit
import SpriteKit

class GameViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
        let skView = SKView(frame: view.bounds)
        skView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        skView.ignoresSiblingOrder = true
        skView.shouldCullNonVisibleNodes = true
        skView.preferredFramesPerSecond = 60
        skView.contentScaleFactor = 1.0 // pixel-perfect on simulator

        if let scene = GameScene(size: skView.bounds.size) as GameScene? {
            scene.scaleMode = .resizeFill
            skView.presentScene(scene)
        }

        view.addSubview(skView)
    }
}
