import Foundation

struct IsometricMath {
    static let tileWidth: CGFloat = 64
    static let tileHeight: CGFloat = 32
    static let gridSize: Int = 64

    /// Convert grid (x, y) to screen position
    static func gridToScreen(x: Int, y: Int) -> CGPoint {
        let screenX = CGFloat(x - y) * (tileWidth / 2)
        let screenY = CGFloat(x + y) * (tileHeight / 2)
        return CGPoint(x: screenX, y: screenY)
    }

    /// Convert screen position to grid coordinates
    static func screenToGrid(point: CGPoint) -> (x: Int, y: Int) {
        let gx = (point.x / (tileWidth / 2) + point.y / (tileHeight / 2)) / 2
        let gy = (point.y / (tileHeight / 2) - point.x / (tileWidth / 2)) / 2
        return (Int(round(gx)), Int(round(gy)))
    }

    /// Calculate zPosition for depth sorting
    static func depthKey(x: Int, y: Int, z: Int = 0) -> CGFloat {
        return CGFloat(x + y + z * 1000)
    }
}
