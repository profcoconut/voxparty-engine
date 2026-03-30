import Foundation
import CoreGraphics
import SwiftUI

struct IsometricMath {
    static let tileWidth: CGFloat = 64
    static let tileHeight: CGFloat = 32

    static func gridToScreen(x: Int, y: Int) -> CGPoint {
        let screenX = CGFloat(x - y) * (tileWidth / 2)
        let screenY = CGFloat(x + y) * (tileHeight / 2)
        return CGPoint(x: screenX, y: screenY)
    }

    static func depthKey(x: Int, y: Int) -> Int {
        return x + y
    }

    static func screenToGrid(point: CGPoint, offset: CGSize) -> (Int, Int) {
        let adjustedX = point.x + offset.width
        let adjustedY = point.y + offset.height

        // Inverse of gridToScreen
        // screenX = (x - y) * tileWidth / 2
        // screenY = (x + y) * tileHeight / 2
        // Solving:
        // x = screenX / tileWidth + screenY / tileHeight
        // y = screenY / tileHeight - screenX / tileWidth
        let gridX = Int(round(adjustedX / (tileWidth / 2) + adjustedY / (tileHeight / 2)) / 2)
        let gridY = Int(round(adjustedY / (tileHeight / 2) - adjustedX / (tileWidth / 2)) / 2)

        return (gridX, gridY)
    }

    static func createDiamondPath(at point: CGPoint) -> Path {
        var path = Path()
        let halfW = tileWidth / 2
        let halfH = tileHeight / 2

        path.move(to: CGPoint(x: point.x, y: point.y - halfH))  // Top
        path.addLine(to: CGPoint(x: point.x + halfW, y: point.y))  // Right
        path.addLine(to: CGPoint(x: point.x, y: point.y + halfH))  // Bottom
        path.addLine(to: CGPoint(x: point.x - halfW, y: point.y))  // Left
        path.closeSubpath()

        return path
    }
}
